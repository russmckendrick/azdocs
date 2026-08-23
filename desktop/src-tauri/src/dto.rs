use std::collections::BTreeMap;

use azdocs::model::{Edge, Finding, QueryRun, Resource, ResourceGroup, Subscription};
use azdocs::report::{ReportContext, SeverityCounts};
use azdocs::store::{SnapshotCounts, SnapshotDiff};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBootstrap {
    pub database_path: String,
    pub config_path: String,
    pub config_found: bool,
    pub has_credentials: bool,
    pub snapshots: Vec<SnapshotSummary>,
    pub latest_snapshot_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotSummary {
    pub id: String,
    pub created_at: String,
    pub tenant_id: String,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EstateSnapshot {
    pub id: String,
    pub created_at: String,
    pub tenant_id: String,
    pub status: String,
    pub notes: Option<String>,
    pub totals: TotalsDto,
    pub tag_coverage: TagCoverageDto,
    pub severity_counts: SeverityCountsDto,
    pub subscriptions: Vec<SubscriptionDto>,
    pub resource_groups: Vec<ResourceGroupDto>,
    pub resources: Vec<ResourceDto>,
    pub resource_types: Vec<ResourceTypeDto>,
    pub locations: Vec<NameCountDto>,
    pub findings: Vec<FindingDto>,
    pub edges: Vec<EdgeDto>,
    pub query_runs: Vec<QueryRunDto>,
    pub previous_diff: Option<SnapshotComparison>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotalsDto {
    pub subscriptions: usize,
    pub resource_groups: usize,
    pub resources: usize,
    pub findings: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TagCoverageDto {
    pub tagged: usize,
    pub untagged: usize,
    pub percent: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SubscriptionDto {
    pub id: String,
    pub display_name: String,
    pub state: Option<String>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceGroupDto {
    pub id: String,
    pub name: String,
    pub subscription_id: String,
    pub location: Option<String>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceDto {
    pub id: String,
    pub display_id: String,
    pub name: String,
    pub azure_type: String,
    pub kind: Option<String>,
    pub location: Option<String>,
    pub resource_group: Option<String>,
    pub subscription_id: String,
    pub tags: Option<Value>,
    pub sku: Option<Value>,
    pub identity: Option<Value>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceTypeDto {
    pub azure_type: String,
    pub display_name: String,
    pub count: usize,
    pub icon: String,
    pub color: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NameCountDto {
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingDto {
    pub query_name: String,
    pub category: String,
    pub severity: String,
    pub resource_id: Option<String>,
    pub title: String,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeDto {
    pub source_id: String,
    pub target_id: String,
    pub kind: String,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
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

        Self {
            id: context.snapshot_id,
            created_at: context.created_at,
            tenant_id: context.tenant_id,
            status: context.status,
            notes: context.notes,
            totals,
            tag_coverage,
            severity_counts,
            subscriptions: subscriptions.into_iter().map(Into::into).collect(),
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

#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectRequestDto {
    pub subscriptions: Vec<String>,
    pub notes: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectResultDto {
    pub snapshot_id: String,
    pub status: String,
    pub queries_run: usize,
    pub queries_failed: usize,
    pub rows_ingested: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "event", content = "data", rename_all = "camelCase")]
pub enum CollectionEvent {
    Phase { message: String },
    Complete { snapshot_id: String },
    Failed { message: String },
}
