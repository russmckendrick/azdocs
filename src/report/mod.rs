pub mod analysis;
pub mod branding;
pub mod csv;
pub mod details;
pub mod docx;
pub mod fonts;
pub mod governance;
pub mod html;
pub mod markdown;
pub mod pdf;
pub mod posture;
pub mod provenance;
pub mod site;
pub mod theme;
pub mod websites;
mod world_map;
pub mod xlsx;

mod document;

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::error::StoreError;
// Re-exported so the emitters can keep importing it from their parent module.
pub(crate) use crate::model::rows::cell_to_string;
use crate::model::{azure_types, azure_values, rows};
use crate::querypack::QueryPack;
use crate::store::Store;
// The governance judgements and the analysis that applies them live in one
// module; every surface reads them from here.
pub use governance::{
    FLAGGED_NON_COMPLIANT_SHARE, GovernanceAnalysis, HEALTHY_TAG_COVERAGE_PERCENT, TagCoverage,
    is_group_flagged,
};

/// What part of the estate a report describes. Applied once, right after the
/// store loads, so every chapter, table and figure downstream sees the same
/// subset and a scoped document is consistent with itself.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReportScope {
    pub subscription: Option<String>,
    pub resource_group: Option<String>,
    /// Keep findings at this severity or higher (`High` is the highest).
    pub min_severity: Option<crate::model::Severity>,
}

impl ReportScope {
    pub fn is_unscoped(&self) -> bool {
        *self == Self::default()
    }

    fn keeps_subscription(&self, subscription_id: &str) -> bool {
        self.subscription
            .as_ref()
            .is_none_or(|wanted| wanted.eq_ignore_ascii_case(subscription_id))
    }

    fn keeps_group(&self, group: Option<&str>) -> bool {
        self.resource_group
            .as_ref()
            .is_none_or(|wanted| group.is_some_and(|g| wanted.eq_ignore_ascii_case(g)))
    }

    fn keeps_resource(&self, resource: &crate::model::Resource) -> bool {
        self.keeps_subscription(&resource.subscription_id)
            && self.keeps_group(resource.resource_group.as_deref())
    }

    /// A finding that names no stored resource is kept while the scope is no
    /// narrower than a subscription its id (if any) belongs to. Narrowing to
    /// a group drops it: a group study should not carry estate-level noise.
    fn keeps_unresolved_finding(&self, resource_id: Option<&str>) -> bool {
        if self.resource_group.is_some() {
            return false;
        }
        match (&self.subscription, resource_id) {
            (Some(wanted), Some(id)) => id
                .to_ascii_lowercase()
                .contains(&format!("/subscriptions/{}/", wanted.to_ascii_lowercase())),
            _ => true,
        }
    }

    fn keeps_severity(&self, severity: crate::model::Severity) -> bool {
        self.min_severity.is_none_or(|min| severity <= min)
    }

    /// Rows of a recorded query stay only when their own subscription and
    /// group columns, where present, fall inside the scope.
    fn keeps_row(&self, row: &Value) -> bool {
        let text = |key: &str| row.get(key).and_then(Value::as_str);
        text("subscriptionId").is_none_or(|id| self.keeps_subscription(id))
            && (self.resource_group.is_none()
                || text("resourceGroup").is_none_or(|group| self.keeps_group(Some(group))))
    }
}

impl From<&ReportScope> for crate::diagram::DiagramScope {
    fn from(scope: &ReportScope) -> Self {
        Self {
            subscription: scope.subscription.clone(),
            resource_group: scope.resource_group.clone(),
        }
    }
}

/// Everything the report emitters need, built once from the store and shaped
/// for direct serialization into templates.
#[derive(Debug, Serialize)]
pub struct ReportContext {
    #[serde(skip)]
    pub posture: posture::PostureReport,
    #[serde(skip)]
    pub websites: websites::WebsiteReport,
    #[serde(skip)]
    pub analysis: analysis::ReportAnalysis,
    pub snapshot_id: String,
    pub created_at: String,
    pub tenant_id: String,
    pub status: String,
    pub notes: Option<String>,
    pub totals: Totals,
    pub type_counts: Vec<TypeCount>,
    pub location_counts: Vec<NameCount>,
    pub severity_counts: SeverityCounts,
    pub tag_coverage: TagCoverage,
    pub governance: GovernanceAnalysis,
    pub categories: Vec<Category>,
    pub findings: Vec<FindingRow>,
    pub subscriptions: Vec<SubscriptionSection>,
    pub details: Vec<details::ResourceGroupPage>,
    pub resource_types: Vec<ResourceTypeSection>,
    /// What changed since the previous usable snapshot of this tenant; None
    /// when this is the earliest one.
    pub changes: Option<crate::model::diff::SnapshotChanges>,
    /// The last usable snapshots of this tenant, oldest first, this one last.
    pub trend: Vec<crate::store::TrendPoint>,
    /// The scope this context was built for, so emitters can say so.
    #[serde(skip)]
    pub scope: ReportScope,
    /// How much each emitter prints before pointing at the data exports.
    pub limits: crate::config::ReportConfig,
}

/// How many snapshots the trend table looks back over.
pub const TREND_LIMIT: usize = 12;

#[derive(Debug, Serialize)]
pub struct Totals {
    pub subscriptions: usize,
    pub resource_groups: usize,
    pub resources: usize,
    pub findings: usize,
}

#[derive(Debug, Serialize)]
pub struct TypeCount {
    pub azure_type: String,
    pub display: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
pub struct NameCount {
    /// The stored Azure value, e.g. `uksouth`. Kept so anything joining or
    /// filtering on it still matches what SQLite holds.
    pub name: String,
    /// Friendly name for display, e.g. `UK South`. Unknown values pass through
    /// unchanged, so this is always safe to render.
    pub display: String,
    pub count: usize,
}

/// Friendly location for an optional stored value. Presentation only — the
/// stored code is what every join and filter still uses.
fn display_location_opt(location: Option<&str>) -> Option<String> {
    location.map(|value| azure_values::display_location(value).into_owned())
}

#[derive(Debug, Default, Serialize)]
pub struct SeverityCounts {
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

/// One inventory category with the shaped rows of each of its queries.
#[derive(Debug, Serialize)]
pub struct Category {
    pub name: String,
    pub queries: Vec<QuerySection>,
}

#[derive(Debug, Serialize)]
pub struct QuerySection {
    pub name: String,
    pub description: String,
    pub columns: Vec<String>,
    pub rows: Vec<Value>,
}

#[derive(Debug, Serialize)]
pub struct FindingRow {
    pub severity: String,
    pub category: String,
    pub query_name: String,
    pub title: String,
    pub resource_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SubscriptionSection {
    pub subscription_id: String,
    pub display_name: String,
    pub state: Option<String>,
    pub resource_count: usize,
    pub resource_groups: Vec<ResourceGroupSection>,
}

#[derive(Debug, Serialize)]
pub struct ResourceGroupSection {
    pub name: String,
    pub location: Option<String>,
    /// Relative link (from docs root, no extension) to the detail page.
    pub detail_path: String,
    pub resources: Vec<ResourceRow>,
}

#[derive(Debug, Serialize)]
pub struct ResourceRow {
    pub name: String,
    pub display_type: String,
    pub azure_type: String,
    pub location: Option<String>,
    pub tags: Option<String>,
}

/// Every resource of one Azure type, with its full detail. This is the
/// by-type view the print formats document resource by resource; `details`
/// holds the same resources grouped by resource group for the docs tree.
#[derive(Debug, Serialize)]
pub struct ResourceTypeSection {
    pub display: String,
    pub azure_type: String,
    pub resources: Vec<details::ResourceDetail>,
}

impl ReportContext {
    pub fn build(store: &Store, snapshot_id: &str) -> Result<Self, StoreError> {
        Self::build_scoped(store, snapshot_id, &ReportScope::default(), true)
    }

    /// Desktop metadata never reads screenshot blobs and never runs the
    /// previous-snapshot comparison: previews and the diff load on demand,
    /// so opening a large estate costs one snapshot read, not two.
    pub fn build_for_desktop(store: &Store, snapshot_id: &str) -> Result<Self, StoreError> {
        Self::build_inner(
            store,
            snapshot_id,
            &ReportScope::default(),
            false,
            crate::config::ReportConfig::default(),
            false,
        )
    }

    /// Build for one scope. `include_images` loads screenshot bytes, which
    /// only the formats that draw them need.
    pub fn build_scoped(
        store: &Store,
        snapshot_id: &str,
        scope: &ReportScope,
        include_images: bool,
    ) -> Result<Self, StoreError> {
        Self::build_with(
            store,
            snapshot_id,
            scope,
            include_images,
            crate::config::ReportConfig::default(),
        )
    }

    /// Build with the `[report]` caps the emitters honour.
    pub fn build_with(
        store: &Store,
        snapshot_id: &str,
        scope: &ReportScope,
        include_images: bool,
        limits: crate::config::ReportConfig,
    ) -> Result<Self, StoreError> {
        Self::build_inner(store, snapshot_id, scope, include_images, limits, true)
    }

    fn build_inner(
        store: &Store,
        snapshot_id: &str,
        scope: &ReportScope,
        include_images: bool,
        limits: crate::config::ReportConfig,
        with_changes: bool,
    ) -> Result<Self, StoreError> {
        let snapshot = store.get_snapshot(snapshot_id)?;
        let mut subscriptions = store.subscriptions(snapshot_id)?;
        let mut resource_groups = store.resource_groups(snapshot_id)?;
        let mut resources = store.resources(snapshot_id)?;
        let mut findings = store.findings(snapshot_id)?;
        let mut edges = store.edges(snapshot_id)?;
        if !scope.is_unscoped() {
            subscriptions.retain(|sub| scope.keeps_subscription(&sub.subscription_id));
            resource_groups.retain(|group| {
                scope.keeps_subscription(&group.subscription_id)
                    && scope.keeps_group(Some(&group.name))
            });
            resources.retain(|resource| scope.keeps_resource(resource));
            let kept: std::collections::HashSet<&str> =
                resources.iter().map(|r| r.id.as_str()).collect();
            findings.retain(|finding| {
                scope.keeps_severity(finding.severity)
                    && match finding.resource_id.as_deref() {
                        Some(id) if kept.contains(id) => true,
                        other => scope.keeps_unresolved_finding(other),
                    }
            });
            edges.retain(|edge| {
                kept.contains(edge.source_id.as_str()) || kept.contains(edge.target_id.as_str())
            });
        }
        let changes = if with_changes {
            store
                .previous_snapshot(snapshot_id)?
                .map(|previous| store.snapshot_changes(&previous.id, snapshot_id))
                .transpose()?
        } else {
            None
        };
        // The trend is scoped to this snapshot's tenant whatever the store's
        // selection, and stops at this snapshot so a report on an older
        // snapshot does not describe its future.
        let trend: Vec<_> = store
            .snapshot_trend_for(&snapshot.tenant_id, TREND_LIMIT)?
            .into_iter()
            .filter(|point| point.created_at <= snapshot.created_at.to_rfc3339())
            .collect();

        let mut type_counts: BTreeMap<&str, usize> = BTreeMap::new();
        let mut location_counts: BTreeMap<&str, usize> = BTreeMap::new();
        let mut tagged = 0;
        for resource in &resources {
            *type_counts.entry(&resource.azure_type).or_default() += 1;
            *location_counts
                .entry(resource.location.as_deref().unwrap_or("(none)"))
                .or_default() += 1;
            // `{}` is untagged too — one definition of "tagged", shared with
            // the governance analysis.
            if governance::tag_keys(resource).next().is_some() {
                tagged += 1;
            }
        }
        let mut type_counts: Vec<TypeCount> = type_counts
            .into_iter()
            .map(|(azure_type, count)| TypeCount {
                azure_type: azure_type.to_owned(),
                display: azure_types::display_name(azure_type).to_owned(),
                count,
            })
            .collect();
        type_counts.sort_by(|a, b| b.count.cmp(&a.count).then(a.azure_type.cmp(&b.azure_type)));
        let mut location_counts: Vec<NameCount> = location_counts
            .into_iter()
            .map(|(name, count)| NameCount {
                display: azure_values::display_location(name).into_owned(),
                name: name.to_owned(),
                count,
            })
            .collect();
        // Busiest first, then by what the reader sees, with the stored code as
        // the deterministic tie-breaker.
        location_counts.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then(a.display.cmp(&b.display))
                .then(a.name.cmp(&b.name))
        });

        let mut severity_counts = SeverityCounts::default();
        for finding in &findings {
            match finding.severity {
                crate::model::Severity::High => severity_counts.high += 1,
                crate::model::Severity::Medium => severity_counts.medium += 1,
                crate::model::Severity::Low => severity_counts.low += 1,
                crate::model::Severity::Info => severity_counts.info += 1,
            }
        }

        let untagged = resources.len() - tagged;
        let percent = governance::percent(tagged, resources.len());
        let governance =
            governance::analyse(&subscriptions, &resource_groups, &resources, &findings);

        // Enumerate the store first: removed overrides and renamed built-ins must
        // not hide historical evidence. Recorded metadata wins over today's pack.
        let pack = QueryPack::load().unwrap_or_default();
        let query_runs = store.query_runs(snapshot_id)?;
        let mut categories: BTreeMap<String, Vec<QuerySection>> = BTreeMap::new();
        for name in store.query_result_names(snapshot_id)? {
            if matches!(
                name.as_str(),
                "all_resources" | "subscriptions" | "resource_groups"
            ) {
                continue;
            }
            let mut rows = store.query_results(snapshot_id, &name)?;
            if !scope.is_unscoped() {
                rows.retain(|row| scope.keeps_row(row));
            }
            if rows.is_empty() {
                continue;
            }
            let run = query_runs.iter().find(|run| run.query_name == name);
            let recorded = run.and_then(|run| run.provenance.as_ref());
            let current = pack.get(&name);
            let category = run
                .map(|r| r.category.clone())
                .or_else(|| current.map(|d| d.category.clone()))
                .unwrap_or_else(|| "evidence".to_owned());
            let description = recorded
                .map(|p| p.description.clone())
                .or_else(|| current.map(|d| d.description.clone()))
                .unwrap_or_default();
            let columns = rows::columns(&rows);
            categories.entry(category).or_default().push(QuerySection {
                name,
                description,
                columns,
                rows,
            });
        }
        let categories = categories
            .into_iter()
            .map(|(name, queries)| Category { name, queries })
            .collect();

        let finding_rows = findings
            .iter()
            .map(|f| FindingRow {
                severity: f.severity.as_str().to_owned(),
                category: f.category.clone(),
                query_name: f.query_name.clone(),
                title: f.title.clone(),
                resource_id: f.resource_id.clone(),
            })
            .collect();

        let subscription_sections = subscriptions
            .iter()
            .map(|sub| {
                let mut groups: Vec<ResourceGroupSection> = resource_groups
                    .iter()
                    .filter(|rg| rg.subscription_id == sub.subscription_id)
                    .map(|rg| {
                        let mut rows: Vec<ResourceRow> = resources
                            .iter()
                            .filter(|r| {
                                r.subscription_id == sub.subscription_id
                                    && r.resource_group.as_deref() == Some(&rg.name.to_lowercase())
                            })
                            .map(|r| ResourceRow {
                                name: r.name.clone(),
                                display_type: azure_types::display_name(&r.azure_type).to_owned(),
                                azure_type: r.azure_type.clone(),
                                location: display_location_opt(r.location.as_deref()),
                                tags: r.tags.as_ref().map(std::string::ToString::to_string),
                            })
                            .collect();
                        rows.sort_by(|a, b| {
                            (&a.azure_type, &a.name).cmp(&(&b.azure_type, &b.name))
                        });
                        ResourceGroupSection {
                            name: rg.name.clone(),
                            location: display_location_opt(rg.location.as_deref()),
                            detail_path: format!(
                                "resources/{}/{}",
                                markdown::slug(&sub.display_name),
                                markdown::slug(&rg.name)
                            ),
                            resources: rows,
                        }
                    })
                    .collect();
                groups.sort_by(|a, b| a.name.cmp(&b.name));
                let resource_count = resources
                    .iter()
                    .filter(|r| r.subscription_id == sub.subscription_id)
                    .count();
                SubscriptionSection {
                    subscription_id: sub.subscription_id.clone(),
                    display_name: sub.display_name.clone(),
                    state: sub.state.clone(),
                    resource_count,
                    resource_groups: groups,
                }
            })
            .collect();

        let mut detail_pages = Vec::new();
        for sub in &subscriptions {
            for rg in resource_groups
                .iter()
                .filter(|rg| rg.subscription_id == sub.subscription_id)
            {
                let rg_lower = rg.name.to_lowercase();
                let members: Vec<details::ResourceDetail> = resources
                    .iter()
                    .filter(|r| {
                        r.subscription_id == sub.subscription_id
                            && r.resource_group.as_deref() == Some(rg_lower.as_str())
                    })
                    .map(|r| details::resource_detail(r, &sub.display_name, &findings, &edges))
                    .collect();
                if members.is_empty() {
                    continue;
                }
                detail_pages.push(details::ResourceGroupPage {
                    path: format!(
                        "resources/{}/{}",
                        markdown::slug(&sub.display_name),
                        markdown::slug(&rg.name)
                    ),
                    subscription_name: sub.display_name.clone(),
                    subscription_slug: markdown::slug(&sub.display_name),
                    resource_group: rg.name.clone(),
                    location: display_location_opt(rg.location.as_deref()),
                    group_key: details::group_key(&sub.subscription_id, &rg.name),
                    resources: members,
                });
            }
        }

        // The by-type view feeds the print formats' compliance index. Ordered
        // by resource count then type so the biggest estates surface first.
        let subscription_names: BTreeMap<&str, &str> = subscriptions
            .iter()
            .map(|sub| (sub.subscription_id.as_str(), sub.display_name.as_str()))
            .collect();
        let mut by_type: BTreeMap<&str, Vec<details::ResourceDetail>> = BTreeMap::new();
        for resource in &resources {
            by_type
                .entry(resource.azure_type.as_str())
                .or_default()
                .push(details::resource_detail(
                    resource,
                    subscription_names
                        .get(resource.subscription_id.as_str())
                        .copied()
                        .unwrap_or(&resource.subscription_id),
                    &findings,
                    &edges,
                ));
        }
        let mut resource_types: Vec<ResourceTypeSection> = by_type
            .into_iter()
            .map(|(azure_type, mut members)| {
                members.sort_by(|a, b| {
                    (a.name.to_lowercase(), &a.arm_id).cmp(&(b.name.to_lowercase(), &b.arm_id))
                });
                ResourceTypeSection {
                    display: azure_types::display_name(azure_type).to_owned(),
                    azure_type: azure_type.to_owned(),
                    resources: members,
                }
            })
            .collect();
        resource_types.sort_by(|a, b| {
            b.resources
                .len()
                .cmp(&a.resources.len())
                .then_with(|| a.display.cmp(&b.display))
        });

        let mut analysis = analysis::ReportAnalysis::build(
            &resources,
            &subscriptions,
            &resource_groups,
            &findings,
            &edges,
            query_runs,
        );
        for name in store.query_result_names(snapshot_id)? {
            let mut rows = store.query_results(snapshot_id, &name)?;
            if !scope.is_unscoped() {
                rows.retain(|row| scope.keeps_row(row));
            }
            analysis.recorded_queries.insert(name.clone(), rows);
        }

        let posture = posture::PostureReport::build(
            &analysis.recorded_queries,
            &analysis.query_runs,
            snapshot.created_at,
            &resources,
        );
        let mut websites = websites::WebsiteReport::build(store, snapshot_id, include_images)?;
        if !scope.is_unscoped() {
            let kept: std::collections::HashSet<&str> =
                resources.iter().map(|r| r.id.as_str()).collect();
            websites.retain_resources(&kept);
        }
        Ok(Self {
            posture,
            websites,
            analysis,
            changes,
            trend,
            scope: scope.clone(),
            limits,
            snapshot_id: snapshot.id.clone(),
            created_at: snapshot.created_at.to_rfc3339(),
            tenant_id: snapshot.tenant_id.clone(),
            status: snapshot.status.as_str().to_owned(),
            notes: snapshot.notes.clone(),
            totals: Totals {
                subscriptions: subscriptions.len(),
                resource_groups: resource_groups.len(),
                resources: resources.len(),
                findings: findings.len(),
            },
            type_counts,
            location_counts,
            severity_counts,
            tag_coverage: TagCoverage {
                tagged,
                untagged,
                percent,
            },
            governance,
            categories,
            findings: finding_rows,
            subscriptions: subscription_sections,
            details: detail_pages,
            resource_types,
        })
    }
}

#[cfg(test)]
mod scope_tests {
    use super::*;
    use crate::model::{Finding, Resource, Severity};

    fn resource(sub: &str, group: &str, name: &str) -> Resource {
        let id = format!(
            "/subscriptions/{sub}/resourceGroups/{group}/providers/Microsoft.Test/things/{name}"
        );
        Resource {
            id: id.to_lowercase(),
            display_id: id,
            name: name.into(),
            azure_type: "microsoft.test/things".into(),
            kind: None,
            location: None,
            resource_group: Some(group.to_lowercase()),
            subscription_id: sub.into(),
            tags: None,
            sku: None,
            identity: None,
            properties: None,
        }
    }

    fn finding(resource_id: Option<&str>, severity: Severity) -> Finding {
        Finding {
            query_name: "q".into(),
            category: "c".into(),
            severity,
            resource_id: resource_id.map(str::to_owned),
            title: "t".into(),
            detail: None,
        }
    }

    #[test]
    fn unit_scope_drops_other_subscriptions_and_their_findings() {
        let scope = ReportScope {
            subscription: Some("SUB-A".into()),
            ..ReportScope::default()
        };
        assert!(scope.keeps_resource(&resource("sub-a", "rg", "x")));
        assert!(!scope.keeps_resource(&resource("sub-b", "rg", "x")));
        assert!(scope.keeps_unresolved_finding(Some("/subscriptions/sub-a/resourcegroups/rg")));
        assert!(!scope.keeps_unresolved_finding(Some("/subscriptions/sub-b/resourcegroups/rg")));
    }

    #[test]
    fn unit_scope_keeps_estate_level_findings_without_group_filter() {
        let subscription_only = ReportScope {
            subscription: Some("sub-a".into()),
            ..ReportScope::default()
        };
        let group = ReportScope {
            resource_group: Some("rg".into()),
            ..ReportScope::default()
        };
        assert!(subscription_only.keeps_unresolved_finding(None));
        assert!(!group.keeps_unresolved_finding(None));
        assert!(!group.keeps_resource(&resource("sub-a", "other", "x")));
        assert!(group.keeps_resource(&resource("sub-a", "RG", "x")));
    }

    #[test]
    fn unit_min_severity_filters_findings_only() {
        let scope = ReportScope {
            min_severity: Some(Severity::Medium),
            ..ReportScope::default()
        };
        assert!(scope.keeps_severity(finding(None, Severity::High).severity));
        assert!(scope.keeps_severity(Severity::Medium));
        assert!(!scope.keeps_severity(Severity::Low));
        assert!(
            scope.keeps_resource(&resource("any", "rg", "x")),
            "resources are untouched"
        );
    }

    #[test]
    fn unit_scope_filters_recorded_rows_by_their_own_columns() {
        let scope = ReportScope {
            subscription: Some("sub-a".into()),
            resource_group: Some("RG".into()),
            ..ReportScope::default()
        };
        assert!(
            scope.keeps_row(&serde_json::json!({"subscriptionId": "sub-a", "resourceGroup": "rg"}))
        );
        assert!(
            !scope
                .keeps_row(&serde_json::json!({"subscriptionId": "sub-b", "resourceGroup": "rg"}))
        );
        assert!(
            !scope.keeps_row(
                &serde_json::json!({"subscriptionId": "sub-a", "resourceGroup": "other"})
            )
        );
        assert!(
            scope.keeps_row(&serde_json::json!({"type": "aggregate"})),
            "rows without scope columns stay"
        );
    }
}
