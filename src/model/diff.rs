//! Field-level comparison of two snapshots. Pure: the store loads both
//! sides, this module says what differs, and every surface (CLI, report,
//! desktop) renders the same `SnapshotChanges`.
//!
//! JSON is canonicalised (keys sorted) and volatile paths are ignored, so a
//! resource ARG happened to serialise in a different order, or whose
//! `provisioningState` flickered, is not reported as changed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Edge, Finding, Resource, ResourceGroup, Severity, Snapshot, Subscription};

const BUILTIN_IGNORE: &str = include_str!("../../data/diff_ignore.toml");

/// `<platform config dir>/azdocs/diff_ignore.toml`, merged over the built-in
/// list (same pattern as the display names and query overrides).
pub fn user_ignore_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "azdocs")
        .map(|dirs| dirs.config_dir().join("diff_ignore.toml"))
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct IgnoreFile {
    #[serde(default)]
    properties: IgnoreSection,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct IgnoreSection {
    #[serde(default)]
    paths: Vec<String>,
}

/// Which `properties` paths a comparison leaves out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IgnoreRules {
    patterns: Vec<Vec<String>>,
}

impl IgnoreRules {
    pub fn parse(content: &str) -> Result<Self, toml::de::Error> {
        let file: IgnoreFile = toml::from_str(content)?;
        Ok(Self::from_paths(file.properties.paths))
    }

    pub fn from_paths(paths: impl IntoIterator<Item = String>) -> Self {
        Self {
            patterns: paths
                .into_iter()
                .filter(|p| !p.trim().is_empty())
                .map(|p| p.split('.').map(str::to_owned).collect())
                .collect(),
        }
    }

    /// The built-in rules extended by the user file, if any.
    pub fn builtin() -> &'static IgnoreRules {
        static RULES: OnceLock<IgnoreRules> = OnceLock::new();
        RULES.get_or_init(|| {
            let user = user_ignore_path().and_then(|path| std::fs::read_to_string(path).ok());
            Self::load(user.as_deref())
        })
    }

    pub fn load(user_content: Option<&str>) -> Self {
        let mut rules =
            Self::parse(BUILTIN_IGNORE).expect("embedded diff_ignore.toml is valid; unit tested");
        if let Some(content) = user_content {
            match Self::parse(content) {
                Ok(extra) => rules.patterns.extend(extra.patterns),
                Err(err) => tracing::warn!("ignoring invalid diff_ignore.toml override: {err}"),
            }
        }
        rules
    }

    pub fn extend(&mut self, other: &IgnoreRules) {
        self.patterns.extend(other.patterns.iter().cloned());
    }

    /// `path` is a dotted leaf path under `properties`.
    pub fn ignores(&self, path: &str) -> bool {
        let segments: Vec<&str> = path.split('.').collect();
        self.patterns
            .iter()
            .any(|pattern| matches(pattern, &segments))
    }
}

fn matches(pattern: &[String], segments: &[&str]) -> bool {
    let mut p = 0;
    let mut s = 0;
    while p < pattern.len() {
        let part = pattern[p].as_str();
        let last = p == pattern.len() - 1;
        if last && part == "*" {
            // Trailing `*` swallows everything beneath the prefix.
            return s < segments.len();
        }
        let Some(segment) = segments.get(s) else {
            return false;
        };
        let ok = if let Some(prefix) = part.strip_suffix('*') {
            // `lastModified*` also covers `lastModifiedBy.name`: a trailing
            // wildcard segment matches the segment and anything beneath it.
            if last {
                return segment.starts_with(prefix);
            }
            segment.starts_with(prefix)
        } else {
            part == "*" || part == *segment
        };
        if !ok {
            return false;
        }
        p += 1;
        s += 1;
    }
    s == segments.len()
}

/// Sort object keys recursively so serialisation order never counts as change.
pub fn canonical(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<&String, Value> =
                map.iter().map(|(k, v)| (k, canonical(v))).collect();
            let mut out = serde_json::Map::new();
            for (k, v) in sorted {
                out.insert(k.clone(), v);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonical).collect()),
        other => other.clone(),
    }
}

/// Every leaf as `a.b.0.c` → value. Arrays are positional.
pub fn flatten(value: &Value) -> BTreeMap<String, Value> {
    let mut out = BTreeMap::new();
    flatten_into(value, String::new(), &mut out);
    out
}

fn flatten_into(value: &Value, prefix: String, out: &mut BTreeMap<String, Value>) {
    let join = |key: &str| {
        if prefix.is_empty() {
            key.to_owned()
        } else {
            format!("{prefix}.{key}")
        }
    };
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (key, inner) in map {
                flatten_into(inner, join(key), out);
            }
        }
        Value::Array(items) if !items.is_empty() => {
            for (index, inner) in items.iter().enumerate() {
                flatten_into(inner, join(&index.to_string()), out);
            }
        }
        leaf => {
            out.insert(prefix, leaf.clone());
        }
    }
}

/// Which stored column a change belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffField {
    Name,
    Type,
    Kind,
    Location,
    ResourceGroup,
    Sku,
    Identity,
    Tags,
    Properties,
}

impl DiffField {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Type => "type",
            Self::Kind => "kind",
            Self::Location => "location",
            Self::ResourceGroup => "resource_group",
            Self::Sku => "sku",
            Self::Identity => "identity",
            Self::Tags => "tags",
            Self::Properties => "properties",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldChange {
    pub field: DiffField,
    /// Dotted leaf path inside the field; empty for scalar columns.
    pub path: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceRef {
    pub id: String,
    pub display_id: String,
    pub name: String,
    pub azure_type: String,
    pub subscription_id: String,
    pub resource_group: Option<String>,
}

impl From<&Resource> for ResourceRef {
    fn from(resource: &Resource) -> Self {
        Self {
            id: resource.id.clone(),
            display_id: resource.display_id.clone(),
            name: resource.name.clone(),
            azure_type: resource.azure_type.clone(),
            subscription_id: resource.subscription_id.clone(),
            resource_group: resource.resource_group.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceChange {
    pub resource: ResourceRef,
    pub fields: Vec<FieldChange>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResourceChanges {
    pub added: Vec<ResourceRef>,
    pub removed: Vec<ResourceRef>,
    pub changed: Vec<ResourceChange>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FindingRef {
    pub query_name: String,
    pub category: String,
    pub severity: Severity,
    pub resource_id: Option<String>,
    pub title: String,
}

impl From<&Finding> for FindingRef {
    fn from(finding: &Finding) -> Self {
        Self {
            query_name: finding.query_name.clone(),
            category: finding.category.clone(),
            severity: finding.severity,
            resource_id: finding.resource_id.clone(),
            title: finding.title.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FindingChanges {
    pub added: Vec<FindingRef>,
    pub resolved: Vec<FindingRef>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EdgeRef {
    pub source_id: String,
    pub target_id: String,
    pub kind: String,
}

impl From<&Edge> for EdgeRef {
    fn from(edge: &Edge) -> Self {
        Self {
            source_id: edge.source_id.clone(),
            target_id: edge.target_id.clone(),
            kind: edge.kind.as_str().to_owned(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EdgeChanges {
    pub added: Vec<EdgeRef>,
    pub removed: Vec<EdgeRef>,
}

/// Ids that appear on only one side.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SetChange {
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SideCounts {
    pub resources: usize,
    pub findings: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
    pub edges: usize,
    pub subscriptions: usize,
    pub resource_groups: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CountDelta {
    pub base: SideCounts,
    pub target: SideCounts,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotRef {
    pub id: String,
    pub created_at: String,
    pub status: String,
}

impl From<&Snapshot> for SnapshotRef {
    fn from(snapshot: &Snapshot) -> Self {
        Self {
            id: snapshot.id.clone(),
            created_at: snapshot.created_at.to_rfc3339(),
            status: snapshot.status.as_str().to_owned(),
        }
    }
}

/// Everything that differs between a base and a target snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotChanges {
    pub base: SnapshotRef,
    pub target: SnapshotRef,
    pub resources: ResourceChanges,
    pub findings: FindingChanges,
    pub edges: EdgeChanges,
    pub subscriptions: SetChange,
    pub resource_groups: SetChange,
    pub counts: CountDelta,
}

impl SnapshotChanges {
    pub fn is_empty(&self) -> bool {
        self.resources.added.is_empty()
            && self.resources.removed.is_empty()
            && self.resources.changed.is_empty()
            && self.findings.added.is_empty()
            && self.findings.resolved.is_empty()
            && self.edges.added.is_empty()
            && self.edges.removed.is_empty()
            && self.subscriptions.added.is_empty()
            && self.subscriptions.removed.is_empty()
            && self.resource_groups.added.is_empty()
            && self.resource_groups.removed.is_empty()
    }

    /// Total field changes across every changed resource.
    pub fn field_changes(&self) -> usize {
        self.resources.changed.iter().map(|c| c.fields.len()).sum()
    }
}

/// One side of a comparison, as the store loads it.
#[derive(Debug, Clone)]
pub struct SnapshotSide {
    pub snapshot: Snapshot,
    pub subscriptions: Vec<Subscription>,
    pub resource_groups: Vec<ResourceGroup>,
    pub resources: Vec<Resource>,
    pub findings: Vec<Finding>,
    pub edges: Vec<Edge>,
}

impl SnapshotSide {
    fn counts(&self) -> SideCounts {
        let mut counts = SideCounts {
            resources: self.resources.len(),
            findings: self.findings.len(),
            edges: self.edges.len(),
            subscriptions: self.subscriptions.len(),
            resource_groups: self.resource_groups.len(),
            ..SideCounts::default()
        };
        for finding in &self.findings {
            match finding.severity {
                Severity::High => counts.high += 1,
                Severity::Medium => counts.medium += 1,
                Severity::Low => counts.low += 1,
                Severity::Info => counts.info += 1,
            }
        }
        counts
    }
}

pub fn compare(
    base: &SnapshotSide,
    target: &SnapshotSide,
    ignore: &IgnoreRules,
) -> SnapshotChanges {
    let base_resources: BTreeMap<&str, &Resource> =
        base.resources.iter().map(|r| (r.id.as_str(), r)).collect();
    let target_resources: BTreeMap<&str, &Resource> = target
        .resources
        .iter()
        .map(|r| (r.id.as_str(), r))
        .collect();

    let mut resources = ResourceChanges::default();
    for (id, before) in &base_resources {
        match target_resources.get(id) {
            None => resources.removed.push(ResourceRef::from(*before)),
            Some(after) => {
                let fields = field_changes(before, after, ignore);
                if !fields.is_empty() {
                    resources.changed.push(ResourceChange {
                        resource: ResourceRef::from(*after),
                        fields,
                    });
                }
            }
        }
    }
    for (id, after) in &target_resources {
        if !base_resources.contains_key(id) {
            resources.added.push(ResourceRef::from(*after));
        }
    }

    let finding_key = |f: &Finding| {
        (
            f.query_name.clone(),
            f.resource_id.clone().unwrap_or_default(),
            f.title.clone(),
        )
    };
    let base_findings: BTreeMap<_, &Finding> =
        base.findings.iter().map(|f| (finding_key(f), f)).collect();
    let target_findings: BTreeMap<_, &Finding> = target
        .findings
        .iter()
        .map(|f| (finding_key(f), f))
        .collect();
    let findings = FindingChanges {
        added: target_findings
            .iter()
            .filter(|(key, _)| !base_findings.contains_key(*key))
            .map(|(_, f)| FindingRef::from(*f))
            .collect(),
        resolved: base_findings
            .iter()
            .filter(|(key, _)| !target_findings.contains_key(*key))
            .map(|(_, f)| FindingRef::from(*f))
            .collect(),
    };

    let edge_key = |e: &Edge| (e.source_id.clone(), e.target_id.clone(), e.kind);
    let base_edges: BTreeSet<_> = base.edges.iter().map(edge_key).collect();
    let target_edges: BTreeSet<_> = target.edges.iter().map(edge_key).collect();
    let edges = EdgeChanges {
        added: target
            .edges
            .iter()
            .filter(|e| !base_edges.contains(&edge_key(e)))
            .map(EdgeRef::from)
            .collect(),
        removed: base
            .edges
            .iter()
            .filter(|e| !target_edges.contains(&edge_key(e)))
            .map(EdgeRef::from)
            .collect(),
    };

    SnapshotChanges {
        base: SnapshotRef::from(&base.snapshot),
        target: SnapshotRef::from(&target.snapshot),
        resources,
        findings,
        edges,
        subscriptions: set_change(
            base.subscriptions
                .iter()
                .map(|s| s.subscription_id.as_str()),
            target
                .subscriptions
                .iter()
                .map(|s| s.subscription_id.as_str()),
        ),
        resource_groups: set_change(
            base.resource_groups.iter().map(|g| g.id.as_str()),
            target.resource_groups.iter().map(|g| g.id.as_str()),
        ),
        counts: CountDelta {
            base: base.counts(),
            target: target.counts(),
        },
    }
}

fn set_change<'a>(
    base: impl Iterator<Item = &'a str>,
    target: impl Iterator<Item = &'a str>,
) -> SetChange {
    let base: BTreeSet<&str> = base.collect();
    let target: BTreeSet<&str> = target.collect();
    SetChange {
        added: target.difference(&base).map(|s| (*s).to_owned()).collect(),
        removed: base.difference(&target).map(|s| (*s).to_owned()).collect(),
    }
}

fn field_changes(before: &Resource, after: &Resource, ignore: &IgnoreRules) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    let scalar =
        |field: DiffField, a: Option<&str>, b: Option<&str>, out: &mut Vec<FieldChange>| {
            if a != b {
                out.push(FieldChange {
                    field,
                    path: String::new(),
                    before: a.map(|v| Value::String(v.to_owned())),
                    after: b.map(|v| Value::String(v.to_owned())),
                });
            }
        };
    scalar(
        DiffField::Name,
        Some(&before.name),
        Some(&after.name),
        &mut changes,
    );
    scalar(
        DiffField::Type,
        Some(&before.azure_type),
        Some(&after.azure_type),
        &mut changes,
    );
    scalar(
        DiffField::Kind,
        before.kind.as_deref(),
        after.kind.as_deref(),
        &mut changes,
    );
    scalar(
        DiffField::Location,
        before.location.as_deref(),
        after.location.as_deref(),
        &mut changes,
    );
    scalar(
        DiffField::ResourceGroup,
        before.resource_group.as_deref(),
        after.resource_group.as_deref(),
        &mut changes,
    );
    for (field, a, b, rules) in [
        (DiffField::Sku, &before.sku, &after.sku, None),
        (DiffField::Identity, &before.identity, &after.identity, None),
        (DiffField::Tags, &before.tags, &after.tags, None),
        (
            DiffField::Properties,
            &before.properties,
            &after.properties,
            Some(ignore),
        ),
    ] {
        changes.extend(json_changes(field, a.as_ref(), b.as_ref(), rules));
    }
    changes
}

fn json_changes(
    field: DiffField,
    before: Option<&Value>,
    after: Option<&Value>,
    ignore: Option<&IgnoreRules>,
) -> Vec<FieldChange> {
    let empty = Value::Object(serde_json::Map::new());
    let before = flatten(&canonical(before.unwrap_or(&empty)));
    let after = flatten(&canonical(after.unwrap_or(&empty)));
    let paths: BTreeSet<&String> = before.keys().chain(after.keys()).collect();
    paths
        .into_iter()
        .filter(|path| !ignore.is_some_and(|rules| rules.ignores(path)))
        .filter(|path| before.get(*path) != after.get(*path))
        .map(|path| FieldChange {
            field,
            path: path.clone(),
            before: before.get(path).cloned(),
            after: after.get(path).cloned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{EdgeKind, SnapshotStatus};
    use serde_json::json;

    fn snapshot(id: &str) -> Snapshot {
        Snapshot {
            id: id.into(),
            created_at: "2026-01-01T00:00:00Z".parse().unwrap(),
            tenant_id: "t".into(),
            tool_version: "0".into(),
            status: SnapshotStatus::Complete,
            notes: None,
            heartbeat_at: None,
            interrupted_at: None,
        }
    }

    fn resource(id: &str, properties: Value, tags: Value) -> Resource {
        Resource {
            id: id.into(),
            display_id: id.to_uppercase(),
            name: id.rsplit('/').next().unwrap().into(),
            azure_type: "microsoft.test/things".into(),
            kind: None,
            location: Some("uksouth".into()),
            resource_group: Some("rg".into()),
            subscription_id: "s".into(),
            tags: Some(tags),
            sku: None,
            identity: None,
            properties: Some(properties),
        }
    }

    fn side(id: &str, resources: Vec<Resource>) -> SnapshotSide {
        SnapshotSide {
            snapshot: snapshot(id),
            subscriptions: vec![],
            resource_groups: vec![],
            resources,
            findings: vec![],
            edges: vec![],
        }
    }

    fn rules() -> IgnoreRules {
        IgnoreRules::load(None)
    }

    #[test]
    fn unit_builtin_ignore_file_parses() {
        assert!(rules().ignores("provisioningState"));
        assert!(rules().ignores("subnets.0.properties.provisioningState"));
        assert!(rules().ignores("instanceView.statuses.0.code"));
        assert!(!rules().ignores("addressSpace.addressPrefixes.0"));
    }

    #[test]
    fn unit_diff_ignores_provisioning_state_when_only_it_changes() {
        let base = side(
            "a",
            vec![resource(
                "/r",
                json!({"provisioningState": "Updating", "size": "S"}),
                json!({}),
            )],
        );
        let target = side(
            "b",
            vec![resource(
                "/r",
                json!({"provisioningState": "Succeeded", "size": "S"}),
                json!({}),
            )],
        );

        let changes = compare(&base, &target, &rules());

        assert!(changes.is_empty(), "{changes:?}");
    }

    #[test]
    fn unit_diff_reports_tag_change_as_field_change() {
        let base = side("a", vec![resource("/r", json!({}), json!({"env": "dev"}))]);
        let target = side(
            "b",
            vec![resource(
                "/r",
                json!({}),
                json!({"env": "prod", "owner": "x"}),
            )],
        );

        let changes = compare(&base, &target, &rules());

        let fields = &changes.resources.changed[0].fields;
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].field, DiffField::Tags);
        assert_eq!(fields[0].path, "env");
        assert_eq!(fields[0].after, Some(json!("prod")));
        assert_eq!(fields[1].before, None);
    }

    #[test]
    fn unit_diff_treats_reordered_keys_as_equal() {
        let base = side(
            "a",
            vec![resource(
                "/r",
                json!({"b": 1, "a": {"y": 2, "x": 1}}),
                json!({}),
            )],
        );
        let target = side(
            "b",
            vec![resource(
                "/r",
                json!({"a": {"x": 1, "y": 2}, "b": 1}),
                json!({}),
            )],
        );

        assert!(compare(&base, &target, &rules()).is_empty());
    }

    #[test]
    fn unit_diff_reports_array_element_change_by_index() {
        let base = side(
            "a",
            vec![resource(
                "/r",
                json!({"prefixes": ["10.0.0.0/16"]}),
                json!({}),
            )],
        );
        let target = side(
            "b",
            vec![resource(
                "/r",
                json!({"prefixes": ["10.0.0.0/16", "10.1.0.0/16"]}),
                json!({}),
            )],
        );

        let changes = compare(&base, &target, &rules());

        let fields = &changes.resources.changed[0].fields;
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].path, "prefixes.1");
        assert_eq!(fields[0].before, None);
    }

    #[test]
    fn unit_diff_reports_added_and_removed_resources_sorted() {
        let base = side(
            "a",
            vec![
                resource("/b", json!({}), json!({})),
                resource("/a", json!({}), json!({})),
            ],
        );
        let target = side(
            "b",
            vec![
                resource("/c", json!({}), json!({})),
                resource("/a", json!({}), json!({})),
            ],
        );

        let changes = compare(&base, &target, &rules());

        assert_eq!(changes.resources.added[0].id, "/c");
        assert_eq!(changes.resources.removed[0].id, "/b");
        assert_eq!(
            (
                changes.counts.base.resources,
                changes.counts.target.resources
            ),
            (2, 2)
        );
    }

    #[test]
    fn unit_diff_resolves_findings_missing_in_target() {
        let finding = |title: &str| Finding {
            query_name: "q".into(),
            category: "c".into(),
            severity: Severity::High,
            resource_id: Some("/r".into()),
            title: title.into(),
            detail: None,
        };
        let mut base = side("a", vec![]);
        base.findings = vec![finding("open port"), finding("public blob")];
        let mut target = side("b", vec![]);
        target.findings = vec![finding("public blob"), finding("weak tls")];

        let changes = compare(&base, &target, &rules());

        assert_eq!(changes.findings.resolved[0].title, "open port");
        assert_eq!(changes.findings.added[0].title, "weak tls");
        assert_eq!(changes.counts.base.high, 2);
    }

    #[test]
    fn unit_diff_reports_edge_added_and_removed() {
        let edge = |target: &str| Edge {
            source_id: "/nic".into(),
            target_id: target.into(),
            kind: EdgeKind::AttachedTo,
            properties: None,
        };
        let mut base = side("a", vec![]);
        base.edges = vec![edge("/vm-old")];
        let mut target = side("b", vec![]);
        target.edges = vec![edge("/vm-new")];

        let changes = compare(&base, &target, &rules());

        assert_eq!(changes.edges.removed[0].target_id, "/vm-old");
        assert_eq!(changes.edges.added[0].target_id, "/vm-new");
    }

    #[test]
    fn unit_user_diff_ignore_extends_builtin_rules() {
        let rules = IgnoreRules::load(Some("[properties]\npaths = [\"myVolatile.*\"]\n"));

        assert!(rules.ignores("myVolatile.counter"));
        assert!(rules.ignores("provisioningState"), "built-ins are kept");
    }

    #[test]
    fn unit_scalar_column_changes_are_reported_without_a_path() {
        let mut moved = resource("/r", json!({}), json!({}));
        moved.location = Some("ukwest".into());
        let base = side("a", vec![resource("/r", json!({}), json!({}))]);
        let target = side("b", vec![moved]);

        let changes = compare(&base, &target, &rules());

        let field = &changes.resources.changed[0].fields[0];
        assert_eq!(
            (field.field, field.path.as_str()),
            (DiffField::Location, "")
        );
    }
}
