pub mod csv;
pub mod html;
pub mod markdown;
pub mod site;
pub mod xlsx;

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::error::StoreError;
use crate::model::azure_types;
use crate::querypack::{QueryKind, QueryPack};
use crate::store::Store;

/// Everything the report emitters need, built once from the store and shaped
/// for direct serialization into templates.
#[derive(Debug, Serialize)]
pub struct ReportContext {
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
    pub categories: Vec<Category>,
    pub findings: Vec<FindingRow>,
    pub subscriptions: Vec<SubscriptionSection>,
}

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
    pub name: String,
    pub count: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct SeverityCounts {
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

#[derive(Debug, Serialize)]
pub struct TagCoverage {
    pub tagged: usize,
    pub untagged: usize,
    pub percent: u32,
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

impl ReportContext {
    pub fn build(store: &Store, snapshot_id: &str) -> Result<Self, StoreError> {
        let snapshot = store.get_snapshot(snapshot_id)?;
        let subscriptions = store.subscriptions(snapshot_id)?;
        let resource_groups = store.resource_groups(snapshot_id)?;
        let resources = store.resources(snapshot_id)?;
        let findings = store.findings(snapshot_id)?;

        let mut type_counts: BTreeMap<&str, usize> = BTreeMap::new();
        let mut location_counts: BTreeMap<&str, usize> = BTreeMap::new();
        let mut tagged = 0;
        for resource in &resources {
            *type_counts.entry(&resource.azure_type).or_default() += 1;
            *location_counts
                .entry(resource.location.as_deref().unwrap_or("(none)"))
                .or_default() += 1;
            if resource.tags.is_some() {
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
                name: name.to_owned(),
                count,
            })
            .collect();
        location_counts.sort_by(|a, b| b.count.cmp(&a.count).then(a.name.cmp(&b.name)));

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
        let percent = if resources.is_empty() {
            0
        } else {
            (tagged * 100 / resources.len()) as u32
        };

        // Category sections come from stored raw query results; the loader
        // failing here (e.g. a broken user query file) should not block
        // reporting on already-collected data.
        let pack = QueryPack::load().unwrap_or_default();
        let mut categories: BTreeMap<String, Vec<QuerySection>> = BTreeMap::new();
        for def in pack.all() {
            if def.kind != QueryKind::Inventory
                || matches!(
                    def.name.as_str(),
                    "all_resources" | "subscriptions" | "resource_groups"
                )
            {
                continue;
            }
            let rows = store.query_results(snapshot_id, &def.name)?;
            if rows.is_empty() {
                continue;
            }
            let columns = rows
                .first()
                .and_then(Value::as_object)
                .map(|map| map.keys().cloned().collect())
                .unwrap_or_default();
            categories
                .entry(def.category.clone())
                .or_default()
                .push(QuerySection {
                    name: def.name.clone(),
                    description: def.description.clone(),
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
                                location: r.location.clone(),
                                tags: r.tags.as_ref().map(std::string::ToString::to_string),
                            })
                            .collect();
                        rows.sort_by(|a, b| {
                            (&a.azure_type, &a.name).cmp(&(&b.azure_type, &b.name))
                        });
                        ResourceGroupSection {
                            name: rg.name.clone(),
                            location: rg.location.clone(),
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

        Ok(Self {
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
            categories,
            findings: finding_rows,
            subscriptions: subscription_sections,
        })
    }
}

/// Render a JSON cell for a table: strings bare, everything else compact JSON.
pub(crate) fn cell_to_string(value: Option<&Value>) -> String {
    match value {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(s)) => s.clone(),
        Some(other) => other.to_string(),
    }
}
