use std::collections::{BTreeMap, HashSet};

use azdocs::model::{Edge, Finding, QueryRun, Resource, ResourceGroup, Subscription, azure_values};
use azdocs::report::{ReportContext, SeverityCounts};
use azdocs::store::{SnapshotCounts, SnapshotDiff};
use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields = nullable)]
pub struct AppBootstrap {
    pub database_path: String,
    pub config_path: String,
    pub config_found: bool,
    pub has_credentials: bool,
    pub required_tags: Vec<String>,
    pub report_theme: String,
    pub report_themes: Vec<String>,
    pub snapshots: Vec<SnapshotSummary>,
    pub latest_snapshot_id: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "QueryDefMeta", optional_fields = nullable)]
pub struct QueryDefDto {
    pub name: String,
    pub category: String,
    #[ts(type = "QueryKind")]
    pub kind: String,
    pub description: String,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "QueryRows", optional_fields = nullable)]
pub struct QueryRowsDto {
    pub query_name: String,
    pub columns: Vec<String>,
    #[ts(type = "Array<Record<string, unknown>>")]
    pub rows: Vec<Value>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields = nullable)]
pub struct SnapshotSummary {
    pub id: String,
    pub created_at: String,
    pub tenant_id: String,
    #[ts(type = "SnapshotStatus")]
    pub status: String,
    pub notes: Option<String>,
    pub subscriptions: u64,
    pub resources: u64,
    pub findings: u64,
}

impl From<SnapshotCounts> for SnapshotSummary {
    fn from(value: SnapshotCounts) -> Self {
        Self {
            id: value.snapshot.id,
            created_at: value.snapshot.created_at.to_rfc3339(),
            tenant_id: value.snapshot.tenant_id,
            status: value.snapshot.status.as_str().to_owned(),
            notes: value.snapshot.notes,
            subscriptions: value.subscriptions,
            resources: value.resources,
            findings: value.findings,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields = nullable)]
pub struct EstateSnapshot {
    pub id: String,
    pub created_at: String,
    pub tenant_id: String,
    #[ts(type = "SnapshotStatus")]
    pub status: String,
    pub notes: Option<String>,
    pub totals: TotalsDto,
    pub tag_coverage: TagCoverageDto,
    pub severity_counts: SeverityCountsDto,
    pub azure_metadata: AzureMetadataDto,
    pub governance_thresholds: GovernanceThresholdsDto,
    pub subscriptions: Vec<SubscriptionDto>,
    pub resource_groups: Vec<ResourceGroupDto>,
    /// Every group the relationship map can open, synthetic ones included.
    pub resource_group_summaries: Vec<ResourceGroupSummaryDto>,
    pub resources: Vec<ResourceDto>,
    pub resource_types: Vec<ResourceTypeDto>,
    pub locations: Vec<NameCountDto>,
    pub findings: Vec<FindingDto>,
    pub edges: Vec<EdgeDto>,
    pub query_runs: Vec<QueryRunDto>,
    pub previous_diff: Option<SnapshotComparison>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "AzureMetadata", optional_fields = nullable)]
pub struct AzureMetadataDto {
    pub locations: BTreeMap<String, String>,
    pub kinds: BTreeMap<String, String>,
}

impl AzureMetadataDto {
    fn build() -> Self {
        Self {
            locations: azure_values::location_display_names().clone(),
            kinds: azure_values::kind_display_names().clone(),
        }
    }
}

/// The judgements the backend applies to tag compliance, sent so the explorer
/// and the printed report call the same estate healthy.
///
/// Values, not verdicts, because the desktop still computes its governance
/// analysis in the frontend. Moving that analysis to Rust would let this carry
/// the verdict instead — see the cleanup notes.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "GovernanceThresholds", optional_fields = nullable)]
pub struct GovernanceThresholdsDto {
    /// Coverage at or above this reads as healthy.
    pub healthy_tag_coverage_percent: u32,
    /// A group past this share of non-compliant resources is called out.
    pub flagged_non_compliant_share: f64,
}

impl GovernanceThresholdsDto {
    fn build() -> Self {
        Self {
            healthy_tag_coverage_percent: azdocs::report::HEALTHY_TAG_COVERAGE_PERCENT,
            flagged_non_compliant_share: azdocs::report::FLAGGED_NON_COMPLIANT_SHARE,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "Totals", optional_fields = nullable)]
pub struct TotalsDto {
    pub subscriptions: usize,
    pub resource_groups: usize,
    pub resources: usize,
    pub findings: usize,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "TagCoverage", optional_fields = nullable)]
pub struct TagCoverageDto {
    pub tagged: usize,
    pub untagged: usize,
    pub percent: u32,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "SeverityCounts", optional_fields = nullable)]
pub struct SeverityCountsDto {
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

impl From<&SeverityCounts> for SeverityCountsDto {
    fn from(value: &SeverityCounts) -> Self {
        Self {
            high: value.high,
            medium: value.medium,
            low: value.low,
            info: value.info,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "Subscription", optional_fields = nullable)]
pub struct SubscriptionDto {
    pub id: String,
    pub display_name: String,
    pub state: Option<String>,
    #[ts(optional = nullable, type = "Record<string, unknown>")]
    pub tags: Option<Value>,
}

impl From<Subscription> for SubscriptionDto {
    fn from(value: Subscription) -> Self {
        Self {
            id: value.subscription_id,
            display_name: value.display_name,
            state: value.state,
            tags: value.tags,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "ResourceGroup", optional_fields = nullable)]
pub struct ResourceGroupDto {
    pub id: String,
    pub name: String,
    pub subscription_id: String,
    pub location: Option<String>,
    #[ts(optional = nullable, type = "Record<string, unknown>")]
    pub tags: Option<Value>,
}

impl From<ResourceGroup> for ResourceGroupDto {
    fn from(value: ResourceGroup) -> Self {
        Self {
            id: value.id,
            name: value.name,
            subscription_id: value.subscription_id,
            location: value.location,
            tags: value.tags,
        }
    }
}

/// A resource group as the relationship map needs it: display-ready, and
/// including groups synthesised for resources whose group row is missing.
///
/// This exists because the frontend was deriving exactly this — the join key,
/// the synthetic-group rule, the subscription-name lookup — in
/// `topology-model.ts`, in the production render path, from a second
/// implementation that had already drifted from the Rust one.
#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "ResourceGroupSummary", optional_fields = nullable)]
pub struct ResourceGroupSummaryDto {
    pub id: String,
    pub name: String,
    pub subscription_id: String,
    /// Resolved here so the UI never has to join against the subscription list.
    pub subscription_name: String,
    pub resource_count: usize,
    pub finding_count: usize,
    /// Resource ids in this group, ordered by name then id.
    pub resource_ids: Vec<String>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "Resource", optional_fields = nullable)]
pub struct ResourceDto {
    pub id: String,
    pub display_id: String,
    pub name: String,
    pub azure_type: String,
    pub kind: Option<String>,
    pub location: Option<String>,
    pub resource_group: Option<String>,
    pub subscription_id: String,
    #[ts(optional = nullable, type = "Record<string, unknown>")]
    pub tags: Option<Value>,
    #[ts(optional = nullable, type = "unknown")]
    pub sku: Option<Value>,
    #[ts(optional = nullable, type = "unknown")]
    pub identity: Option<Value>,
    #[ts(optional = nullable, type = "Record<string, unknown>")]
    pub properties: Option<Value>,
    pub finding_count: usize,
    pub edge_count: usize,
}

impl ResourceDto {
    fn from_resource(
        value: Resource,
        finding_counts: &BTreeMap<String, usize>,
        edge_counts: &BTreeMap<String, usize>,
    ) -> Self {
        let finding_count = finding_counts.get(&value.id).copied().unwrap_or_default();
        let edge_count = edge_counts.get(&value.id).copied().unwrap_or_default();
        Self {
            id: value.id,
            display_id: value.display_id,
            name: value.name,
            azure_type: value.azure_type,
            kind: value.kind,
            location: value.location,
            resource_group: value.resource_group,
            subscription_id: value.subscription_id,
            tags: value.tags,
            sku: value.sku,
            identity: value.identity,
            properties: value.properties,
            finding_count,
            edge_count,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "ResourceType", optional_fields = nullable)]
pub struct ResourceTypeDto {
    pub azure_type: String,
    pub display_name: String,
    pub count: usize,
    pub icon: String,
    pub color: String,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "NameCount", optional_fields = nullable)]
pub struct NameCountDto {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "Finding", optional_fields = nullable)]
pub struct FindingDto {
    pub query_name: String,
    pub category: String,
    #[ts(type = "Severity")]
    pub severity: String,
    pub resource_id: Option<String>,
    pub title: String,
    #[ts(optional = nullable, type = "unknown")]
    pub detail: Option<Value>,
}

impl From<Finding> for FindingDto {
    fn from(value: Finding) -> Self {
        Self {
            query_name: value.query_name,
            category: value.category,
            severity: value.severity.as_str().to_owned(),
            resource_id: value.resource_id,
            title: value.title,
            detail: value.detail,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "Edge", optional_fields = nullable)]
pub struct EdgeDto {
    pub source_id: String,
    pub target_id: String,
    #[ts(type = "EdgeKind")]
    pub kind: String,
    #[ts(optional = nullable, type = "unknown")]
    pub properties: Option<Value>,
}

impl From<Edge> for EdgeDto {
    fn from(value: Edge) -> Self {
        Self {
            source_id: value.source_id,
            target_id: value.target_id,
            kind: value.kind.as_str().to_owned(),
            properties: value.properties,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "QueryRun", optional_fields = nullable)]
pub struct QueryRunDto {
    pub query_name: String,
    pub category: String,
    pub row_count: Option<u64>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
}

impl From<QueryRun> for QueryRunDto {
    fn from(value: QueryRun) -> Self {
        Self {
            query_name: value.query_name,
            category: value.category,
            row_count: value.row_count,
            duration_ms: value.duration_ms,
            error: value.error,
        }
    }
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields = nullable)]
pub struct SnapshotComparison {
    pub base_snapshot_id: String,
    pub target_snapshot_id: String,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub changed: Vec<String>,
}

impl SnapshotComparison {
    pub fn from_diff(base: String, target: String, value: SnapshotDiff) -> Self {
        Self {
            base_snapshot_id: base,
            target_snapshot_id: target,
            added: value.added,
            removed: value.removed,
            changed: value.changed,
        }
    }
}

impl EstateSnapshot {
    #[allow(clippy::too_many_arguments)]
    pub fn build(
        context: ReportContext,
        subscriptions: Vec<Subscription>,
        resource_groups: Vec<ResourceGroup>,
        resources: Vec<Resource>,
        findings: Vec<Finding>,
        edges: Vec<Edge>,
        query_runs: Vec<QueryRun>,
        previous_diff: Option<SnapshotComparison>,
    ) -> Self {
        let mut finding_counts = BTreeMap::new();
        for finding in &findings {
            if let Some(resource_id) = &finding.resource_id {
                *finding_counts.entry(resource_id.clone()).or_insert(0) += 1;
            }
        }
        let known_ids: HashSet<String> = resources.iter().map(|r| r.id.clone()).collect();
        let edges = resolve_subnet_endpoints(edges, &known_ids);
        let mut edge_counts = BTreeMap::new();
        for edge in &edges {
            *edge_counts.entry(edge.source_id.clone()).or_insert(0) += 1;
            *edge_counts.entry(edge.target_id.clone()).or_insert(0) += 1;
        }

        let resource_types = context
            .type_counts
            .iter()
            .map(|item| ResourceTypeDto {
                azure_type: item.azure_type.clone(),
                display_name: item.display.clone(),
                count: item.count,
                icon: azdocs::diagram::icons::svg_data_uri(&item.azure_type),
                color: azdocs::diagram::icons::category_color(&item.azure_type).to_owned(),
            })
            .collect();
        let locations = context
            .location_counts
            .iter()
            .map(|item| NameCountDto {
                name: item.name.clone(),
                count: item.count,
            })
            .collect();
        let totals = TotalsDto {
            subscriptions: context.totals.subscriptions,
            resource_groups: context.totals.resource_groups,
            resources: context.totals.resources,
            findings: context.totals.findings,
        };
        let tag_coverage = TagCoverageDto {
            tagged: context.tag_coverage.tagged,
            untagged: context.tag_coverage.untagged,
            percent: context.tag_coverage.percent,
        };
        let severity_counts = SeverityCountsDto::from(&context.severity_counts);

        // Same bucketing the relationship map uses, so the two never disagree
        // about which groups exist or what a synthesised id looks like.
        let subscription_names: BTreeMap<&str, &str> = subscriptions
            .iter()
            .map(|s| (s.subscription_id.as_str(), s.display_name.as_str()))
            .collect();
        let (buckets, _) = crate::groups::bucket_resources(&resource_groups, &resources, &[]);
        let mut resource_group_summaries: Vec<ResourceGroupSummaryDto> = buckets
            .into_iter()
            .map(|bucket| {
                let mut members: Vec<&Resource> = bucket.resources;
                members.sort_by(|left, right| {
                    left.name
                        .cmp(&right.name)
                        .then_with(|| left.id.cmp(&right.id))
                });
                ResourceGroupSummaryDto {
                    subscription_name: subscription_names
                        .get(bucket.subscription_id.as_str())
                        .map(|name| (*name).to_owned())
                        .unwrap_or_else(|| bucket.subscription_id.clone()),
                    resource_count: members.len(),
                    finding_count: members
                        .iter()
                        .map(|r| finding_counts.get(&r.id).copied().unwrap_or_default())
                        .sum(),
                    resource_ids: members.iter().map(|r| r.id.clone()).collect(),
                    id: bucket.id,
                    name: bucket.name,
                    subscription_id: bucket.subscription_id,
                }
            })
            .collect();
        // Ordered for display: subscription, then group, then id as the
        // deterministic tie-breaker.
        resource_group_summaries.sort_by(|left, right| {
            left.subscription_name
                .cmp(&right.subscription_name)
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.id.cmp(&right.id))
        });

        Self {
            id: context.snapshot_id,
            created_at: context.created_at,
            tenant_id: context.tenant_id,
            status: context.status,
            notes: context.notes,
            totals,
            tag_coverage,
            severity_counts,
            azure_metadata: AzureMetadataDto::build(),
            governance_thresholds: GovernanceThresholdsDto::build(),
            subscriptions: subscriptions.into_iter().map(Into::into).collect(),
            resource_group_summaries,
            resource_groups: resource_groups.into_iter().map(Into::into).collect(),
            resources: resources
                .into_iter()
                .map(|resource| ResourceDto::from_resource(resource, &finding_counts, &edge_counts))
                .collect(),
            resource_types,
            locations,
            findings: findings.into_iter().map(Into::into).collect(),
            edges: edges.into_iter().map(Into::into).collect(),
            query_runs: query_runs.into_iter().map(Into::into).collect(),
            previous_diff,
        }
    }
}

/// Subnets are not rows in `resources` (ARG's resources table does not return
/// them), so edges that end on a subnet id would dangle and the frontend would
/// drop them. Collapse those endpoints onto the owning VNet — which is a row —
/// keeping the subnet name in the edge properties, and dedupe what collapsing
/// merges together. Self-loops (subnet→its own VNet) disappear entirely.
fn resolve_subnet_endpoints(edges: Vec<Edge>, known_ids: &HashSet<String>) -> Vec<Edge> {
    let mut resolved = Vec::new();
    let mut seen = HashSet::new();
    for mut edge in edges {
        let mut notes = Vec::new();
        for (endpoint, role) in [
            (&mut edge.source_id, "sourceSubnet"),
            (&mut edge.target_id, "targetSubnet"),
        ] {
            if !known_ids.contains(endpoint.as_str())
                && let Some((vnet_id, subnet_name)) = split_subnet_id(endpoint)
            {
                notes.push((role, subnet_name));
                *endpoint = vnet_id;
            }
        }
        if edge.source_id == edge.target_id {
            continue;
        }
        if !notes.is_empty() {
            let props = edge
                .properties
                .get_or_insert_with(|| Value::Object(serde_json::Map::new()));
            if let Some(map) = props.as_object_mut() {
                for (role, subnet_name) in notes {
                    map.insert(role.to_owned(), Value::String(subnet_name));
                }
            }
        }
        if seen.insert((edge.source_id.clone(), edge.target_id.clone(), edge.kind)) {
            resolved.push(edge);
        }
    }
    resolved
}

/// `…/virtualnetworks/<vnet>/subnets/<name>` → the VNet id and subnet name.
/// Ids are already normalized to lowercase by the model layer.
fn split_subnet_id(id: &str) -> Option<(String, String)> {
    let (vnet_id, subnet_name) = id.split_once("/subnets/")?;
    if !vnet_id.contains("/virtualnetworks/") || subnet_name.is_empty() {
        return None;
    }
    Some((vnet_id.to_owned(), subnet_name.to_owned()))
}

#[cfg(test)]
mod tests {
    use azdocs::model::EdgeKind;

    use super::*;

    fn subnet_edge(source: &str, target: &str, kind: EdgeKind) -> Edge {
        Edge {
            source_id: source.to_owned(),
            target_id: target.to_owned(),
            kind,
            properties: None,
        }
    }

    const VNET: &str =
        "/subscriptions/s1/resourcegroups/rg/providers/microsoft.network/virtualnetworks/vnet-1";

    #[test]
    fn resolve_collapses_dangling_subnet_endpoints_onto_the_vnet() {
        let nic = "/subscriptions/s1/resourcegroups/rg/providers/microsoft.network/networkinterfaces/nic-1";
        let subnet = format!("{VNET}/subnets/app");
        let known = HashSet::from([nic.to_owned(), VNET.to_owned()]);

        let resolved = resolve_subnet_endpoints(
            vec![subnet_edge(nic, &subnet, EdgeKind::NicInSubnet)],
            &known,
        );

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].target_id, VNET);
        assert_eq!(
            resolved[0].properties.as_ref().unwrap()["targetSubnet"],
            "app"
        );
    }

    #[test]
    fn resolve_drops_self_loops_and_merged_duplicates() {
        let known = HashSet::from([VNET.to_owned()]);
        let subnet_a = format!("{VNET}/subnets/a");
        let subnet_b = format!("{VNET}/subnets/b");

        let resolved = resolve_subnet_endpoints(
            vec![
                subnet_edge(&subnet_a, VNET, EdgeKind::SubnetOf),
                subnet_edge(&subnet_b, VNET, EdgeKind::SubnetOf),
            ],
            &known,
        );

        assert!(resolved.is_empty(), "subnet→own-vnet edges are self-loops");
    }

    #[test]
    fn resolve_leaves_real_resource_endpoints_alone() {
        let pe =
            "/subscriptions/s1/resourcegroups/rg/providers/microsoft.network/privateendpoints/pe-1";
        let sql = "/subscriptions/s1/resourcegroups/rg/providers/microsoft.sql/servers/sql-1";
        let known = HashSet::from([pe.to_owned(), sql.to_owned()]);

        let resolved = resolve_subnet_endpoints(
            vec![subnet_edge(pe, sql, EdgeKind::PrivateEndpointFor)],
            &known,
        );

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].source_id, pe);
        assert_eq!(resolved[0].target_id, sql);
        assert!(resolved[0].properties.is_none());
    }
}

#[derive(Debug, serde::Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CollectRequest", optional_fields = nullable)]
pub struct CollectRequestDto {
    pub subscriptions: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "CollectResult", optional_fields = nullable)]
pub struct CollectResultDto {
    pub snapshot_id: String,
    pub status: String,
    pub queries_run: usize,
    pub queries_failed: usize,
    pub rows_ingested: u64,
}

#[derive(Clone, Debug, Serialize, TS)]
// `rename_all` on an enum renames the *variants*; the fields of a struct
// variant need `rename_all_fields`. Without it this sent `output_count`
// while every other DTO field was camelCase, and the UI read `undefined`.
#[serde(
    tag = "event",
    content = "data",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(optional_fields = nullable)]
pub enum CollectionEvent {
    Phase { message: String },
    Complete { snapshot_id: String },
    Failed { message: String },
}

#[derive(Debug, serde::Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[ts(rename = "ExportRequest", optional_fields = nullable)]
pub struct ExportRequestDto {
    pub snapshot_id: String,
    pub destination: String,
    #[ts(type = "ExportKind")]
    pub export_kind: String,
    pub formats: Vec<String>,
    pub theme: Option<String>,
    pub diagram_type: Option<String>,
    pub subscription_id: Option<String>,
    pub resource_group: Option<String>,
}

#[derive(Debug, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(rename = "ExportResult", optional_fields = nullable)]
pub struct ExportResultDto {
    pub destination: String,
    pub outputs: Vec<String>,
}

#[derive(Clone, Debug, Serialize, TS)]
// `rename_all` on an enum renames the *variants*; the fields of a struct
// variant need `rename_all_fields`. Without it this sent `output_count`
// while every other DTO field was camelCase, and the UI read `undefined`.
#[serde(
    tag = "event",
    content = "data",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(optional_fields = nullable)]
pub enum ExportEvent {
    Phase { message: String },
    Complete { output_count: usize },
    Failed { message: String },
}

#[cfg(test)]
mod event_wire_tests {
    use super::{CollectionEvent, ExportEvent};

    /// Struct-variant fields need `rename_all_fields`; `rename_all` alone only
    /// renames the variants. Without it these carried snake_case payloads while
    /// the rest of the wire format was camelCase, and the Exports workspace
    /// rendered "Exported undefined artifacts".
    #[test]
    fn unit_serialises_event_payload_fields_as_camel_case_when_emitted() {
        let export = serde_json::to_string(&ExportEvent::Complete { output_count: 5 })
            .expect("serialise export event");
        assert_eq!(export, r#"{"event":"complete","data":{"outputCount":5}}"#);

        let collect = serde_json::to_string(&CollectionEvent::Complete {
            snapshot_id: "abc".to_owned(),
        })
        .expect("serialise collection event");
        assert_eq!(
            collect,
            r#"{"event":"complete","data":{"snapshotId":"abc"}}"#
        );
    }
}
