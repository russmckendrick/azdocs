pub mod branding;
pub mod csv;
pub mod details;
pub mod docx;
pub mod html;
mod mark;
pub mod markdown;
pub mod pdf;
pub mod site;
pub mod theme;
pub mod xlsx;

mod document;

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::error::StoreError;
// Re-exported so the emitters can keep importing it from their parent module.
pub(crate) use crate::model::rows::cell_to_string;
use crate::model::{azure_types, azure_values, rows};
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
    pub details: Vec<details::ResourceGroupPage>,
    pub resource_types: Vec<ResourceTypeSection>,
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
    /// [`page_columns`] applied once here so the page-width emitters (PDF,
    /// DOCX) share one definition of "what fits" instead of each reimplementing
    /// the rule.
    pub print_columns: Vec<String>,
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
        let snapshot = store.get_snapshot(snapshot_id)?;
        let subscriptions = store.subscriptions(snapshot_id)?;
        let resource_groups = store.resource_groups(snapshot_id)?;
        let resources = store.resources(snapshot_id)?;
        let findings = store.findings(snapshot_id)?;
        let edges = store.edges(snapshot_id)?;

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
            let columns = rows::columns(&rows);
            categories
                .entry(def.category.clone())
                .or_default()
                .push(QuerySection {
                    name: def.name.clone(),
                    description: def.description.clone(),
                    print_columns: page_columns(&columns)
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
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
            details: detail_pages,
            resource_types,
        })
    }
}

/// Column subset for page-width emitters (DOCX; the Typst template applies
/// the same rule): drop the raw ARM `id` column, which never fits a printed
/// page, and cap at six. The full data lives in CSV/XLSX/HTML.
pub(crate) fn page_columns(columns: &[String]) -> Vec<&str> {
    columns
        .iter()
        .map(String::as_str)
        .filter(|c| *c != "id")
        .take(6)
        .collect()
}
