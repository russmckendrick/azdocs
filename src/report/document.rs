use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use super::branding::BrandingContext;
use super::{HEALTHY_TAG_COVERAGE_PERCENT, ReportContext, cell_to_string};
use crate::diagram::assets::{DiagramAsset, DiagramAssetKind};

#[derive(Debug, Serialize)]
pub(crate) struct PrintDocument<'a> {
    pub cover: Cover<'a>,
    pub toc_depth: u8,
    pub blocks: Vec<Block<'a>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Cover<'a> {
    pub product_mark: bool,
    pub title: Cow<'a, str>,
    pub subtitle: Cow<'a, str>,
    pub company: Cow<'a, str>,
    pub tenant: Cow<'a, str>,
    pub snapshot: Cow<'a, str>,
    pub collected: Cow<'a, str>,
    pub status: Cow<'a, str>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum Block<'a> {
    Chapter {
        title: Cow<'a, str>,
        break_before: bool,
        divider: bool,
    },
    Heading {
        level: u8,
        title: Cow<'a, str>,
        icon: Option<Cow<'a, str>>,
    },
    Paragraph {
        style: ParagraphStyle,
        runs: Vec<TextRun<'a>>,
    },
    Statistics {
        items: Vec<Statistic<'a>>,
    },
    Table {
        style: TableKind,
        columns: Vec<TableColumn<'a>>,
        rows: Vec<Vec<Cow<'a, str>>>,
    },
    Facts {
        items: Vec<Fact<'a>>,
    },
    ResourceIndex {
        items: Vec<ResourceIndexItem<'a>>,
    },
    ResourcePlate {
        name: Cow<'a, str>,
    },
    SubLabel {
        title: Cow<'a, str>,
    },
    Callout {
        severity: Cow<'a, str>,
        title: Cow<'a, str>,
        detail: Option<Cow<'a, str>>,
    },
    Diagram {
        slug: Cow<'a, str>,
        caption: Option<Cow<'a, str>>,
    },
    EmptyState {
        text: Cow<'a, str>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ParagraphStyle {
    Body,
    Muted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TextStyle {
    Normal,
    Strong,
    Mono,
    Severity,
}

#[derive(Debug, Serialize)]
pub(crate) struct TextRun<'a> {
    pub text: Cow<'a, str>,
    pub style: TextStyle,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<Cow<'a, str>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Statistic<'a> {
    pub value: Cow<'a, str>,
    pub label: Cow<'a, str>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Fact<'a> {
    pub label: Cow<'a, str>,
    pub value: Cow<'a, str>,
    pub mono: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct ResourceIndexItem<'a> {
    pub name: Cow<'a, str>,
    pub subscription: Cow<'a, str>,
    pub resource_group: Cow<'a, str>,
    pub location: Cow<'a, str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TableKind {
    Data,
}

#[derive(Debug, Serialize)]
pub(crate) struct TableColumn<'a> {
    pub label: Cow<'a, str>,
    pub mono: bool,
}

impl<'a> PrintDocument<'a> {
    pub fn build(
        report: &'a ReportContext,
        branding: &'a BrandingContext,
        diagrams: &'a [DiagramAsset],
    ) -> Self {
        let mut blocks = Vec::new();
        let group_diagrams: BTreeMap<&str, &DiagramAsset> = diagrams
            .iter()
            .filter(|asset| asset.kind == DiagramAssetKind::ResourceGroup)
            .filter_map(|asset| Some((asset.group_key.as_deref()?, asset)))
            .collect();
        let resource_diagrams: BTreeMap<&str, &DiagramAsset> = diagrams
            .iter()
            .filter(|asset| asset.kind == DiagramAssetKind::Resource)
            .filter_map(|asset| Some((asset.resource_id.as_deref()?, asset)))
            .collect();

        build_summary(report, &mut blocks);
        build_overviews(diagrams, &mut blocks);
        build_findings(report, &mut blocks);
        build_type_index(report, &mut blocks);
        build_estate(report, &group_diagrams, &resource_diagrams, &mut blocks);
        build_evidence(report, &mut blocks);

        Self {
            cover: Cover {
                product_mark: true,
                title: Cow::Borrowed(&branding.title),
                subtitle: Cow::Borrowed(&branding.subtitle),
                company: Cow::Borrowed(&branding.company),
                tenant: Cow::Borrowed(&report.tenant_id),
                snapshot: Cow::Borrowed(&report.snapshot_id),
                collected: Cow::Borrowed(&report.created_at),
                status: Cow::Borrowed(&report.status),
            },
            toc_depth: 1,
            blocks,
        }
    }

    pub fn icon_types(&self) -> impl Iterator<Item = &str> {
        self.blocks.iter().filter_map(|block| match block {
            Block::Heading {
                icon: Some(icon), ..
            } => Some(icon.as_ref()),
            _ => None,
        })
    }
}

fn build_summary<'a>(report: &'a ReportContext, blocks: &mut Vec<Block<'a>>) {
    major_chapter(blocks, "Executive Summary", false);
    blocks.push(Block::Statistics {
        items: vec![
            statistic(report.totals.subscriptions.to_string(), "Subscriptions"),
            statistic(report.totals.resource_groups.to_string(), "Resource groups"),
            statistic(report.totals.resources.to_string(), "Resources"),
            statistic(report.totals.findings.to_string(), "Findings"),
            statistic(format!("{}%", report.tag_coverage.percent), "Tag coverage"),
        ],
    });

    blocks.push(Block::Paragraph {
        style: ParagraphStyle::Body,
        runs: vec![
            normal("This snapshot covers "),
            strong(report.totals.resources.to_string()),
            normal(" resources across "),
            strong(report.totals.resource_groups.to_string()),
            normal(" resource groups in "),
            strong(report.totals.subscriptions.to_string()),
            normal(" subscriptions. It records "),
            strong(report.totals.findings.to_string()),
            normal(" findings, including "),
            severity_run(report.severity_counts.high.to_string(), "high"),
            normal(" high and "),
            severity_run(report.severity_counts.medium.to_string(), "medium"),
            normal(" medium priority items. "),
            strong(report.tag_coverage.tagged.to_string()),
            normal(" resources are tagged and "),
            strong(report.tag_coverage.untagged.to_string()),
            normal(" are untagged, giving "),
            strong(format!("{}%", report.tag_coverage.percent)),
            normal(if report.tag_coverage.is_healthy() {
                " tag coverage, at or above the "
            } else {
                " tag coverage, below the "
            }),
            strong(format!("{HEALTHY_TAG_COVERAGE_PERCENT}%")),
            normal(" this report treats as healthy."),
        ],
    });

    if !report.type_counts.is_empty() {
        heading(blocks, 2, "Largest resource types", None);
        blocks.push(Block::Facts {
            items: report
                .type_counts
                .iter()
                .take(8)
                .map(|item| fact(item.display.as_str(), resource_count(item.count), false))
                .collect(),
        });
    }

    if !report.location_counts.is_empty() {
        heading(blocks, 2, "Geographic footprint", None);
        blocks.push(Block::Facts {
            items: report
                .location_counts
                .iter()
                .take(6)
                .map(|item| fact(item.display.as_str(), resource_count(item.count), false))
                .collect(),
        });
    }

    let mut priority: Vec<_> = report
        .findings
        .iter()
        .filter(|finding| matches!(finding.severity.as_str(), "high" | "medium"))
        .collect();
    priority.sort_by(|left, right| {
        severity_rank(&left.severity)
            .cmp(&severity_rank(&right.severity))
            .then(left.category.cmp(&right.category))
            .then(left.title.cmp(&right.title))
    });
    if !priority.is_empty() {
        heading(blocks, 2, "Priority findings", None);
        for finding in priority.into_iter().take(5) {
            blocks.push(finding_callout(finding));
        }
    }
}

fn build_findings<'a>(report: &'a ReportContext, blocks: &mut Vec<Block<'a>>) {
    major_chapter(blocks, "Findings", true);
    if report.findings.is_empty() {
        empty(blocks, "No findings.");
        return;
    }
    blocks.push(Block::Statistics {
        items: vec![
            statistic(report.severity_counts.high.to_string(), "High"),
            statistic(report.severity_counts.medium.to_string(), "Medium"),
            statistic(report.severity_counts.low.to_string(), "Low"),
            statistic(report.severity_counts.info.to_string(), "Info"),
        ],
    });
    blocks.push(Block::Paragraph {
        style: ParagraphStyle::Body,
        runs: vec![normal(
            "Findings are grouped by priority so the most consequential work can be reviewed first. The evidence appendix retains the underlying query output.",
        )],
    });

    for (severity, title) in [
        ("high", "High priority"),
        ("medium", "Medium priority"),
        ("low", "Low priority"),
        ("info", "Informational"),
    ] {
        let findings: Vec<_> = report
            .findings
            .iter()
            .filter(|finding| finding.severity == severity)
            .collect();
        if findings.is_empty() {
            continue;
        }
        heading(blocks, 2, title, None);
        for finding in findings {
            blocks.push(finding_callout(finding));
        }
    }
}

fn build_evidence<'a>(report: &'a ReportContext, blocks: &mut Vec<Block<'a>>) {
    if report.categories.is_empty() {
        return;
    }
    major_chapter(blocks, "Evidence appendix", true);
    blocks.push(Block::Paragraph {
        style: ParagraphStyle::Body,
        runs: vec![normal(
            "The following sections preserve the collected inventory evidence. Compact results are presented as facts; larger result sets remain tables for comparison and audit use.",
        )],
    });
    for category in &report.categories {
        heading(blocks, 2, capitalise(&category.name), None);
        for query in &category.queries {
            heading(blocks, 3, query.name.as_str(), None);
            if !query.description.is_empty() {
                blocks.push(Block::Paragraph {
                    style: ParagraphStyle::Muted,
                    runs: vec![normal(query.description.as_str())],
                });
            }

            let table_columns: Vec<TableColumn<'a>> = query
                .print_columns
                .iter()
                .map(|column| TableColumn {
                    label: Cow::Owned(display_label(column)),
                    mono: looks_like_identifier(column),
                })
                .collect();
            let rows = query
                .rows
                .iter()
                .map(|row| {
                    query
                        .print_columns
                        .iter()
                        .map(|column| Cow::Owned(cell_to_string(row.get(column))))
                        .collect()
                })
                .collect();
            if table_columns.is_empty() || query.rows.is_empty() {
                empty(blocks, "No results.");
            } else if query.rows.len() == 1 && table_columns.len() <= 6 {
                let row = &query.rows[0];
                blocks.push(Block::Facts {
                    items: query
                        .print_columns
                        .iter()
                        .map(|column| {
                            fact(
                                display_label(column),
                                cell_to_string(row.get(column)),
                                looks_like_identifier(column),
                            )
                        })
                        .collect(),
                });
            } else {
                blocks.push(Block::Table {
                    style: TableKind::Data,
                    columns: table_columns,
                    rows,
                });
            }
        }
    }
}

fn build_type_index<'a>(report: &'a ReportContext, blocks: &mut Vec<Block<'a>>) {
    major_chapter(blocks, "Resources by type", true);
    blocks.push(Block::Paragraph {
        style: ParagraphStyle::Body,
        runs: vec![normal(
            "This index keeps a compliance sweep by resource type available before the estate is documented in its Azure hierarchy.",
        )],
    });
    for section in &report.resource_types {
        heading(
            blocks,
            2,
            section.display.as_str(),
            Some(section.azure_type.as_str()),
        );
        blocks.push(Block::ResourceIndex {
            items: section
                .resources
                .iter()
                .map(|detail| ResourceIndexItem {
                    name: Cow::Borrowed(detail.name.as_str()),
                    subscription: Cow::Borrowed(detail.subscription_name.as_str()),
                    resource_group: option_text(detail.resource_group.as_deref()),
                    location: option_text(detail.location.as_deref()),
                })
                .collect(),
        });
    }
}

fn build_estate<'a>(
    report: &'a ReportContext,
    group_diagrams: &BTreeMap<&str, &'a DiagramAsset>,
    resource_diagrams: &BTreeMap<&str, &'a DiagramAsset>,
    blocks: &mut Vec<Block<'a>>,
) {
    for subscription in &report.subscriptions {
        chapter(blocks, subscription.display_name.as_str(), true);
        let group_prefix = format!("{}/", subscription.subscription_id.to_lowercase());
        let pages: Vec<_> = report
            .details
            .iter()
            .filter(|page| page.group_key.starts_with(&group_prefix))
            .collect();
        let subscription_findings: usize = pages
            .iter()
            .flat_map(|page| &page.resources)
            .map(|detail| detail.findings.len())
            .sum();
        blocks.push(Block::Paragraph {
            style: ParagraphStyle::Body,
            runs: vec![
                strong(subscription.resource_count.to_string()),
                normal(" resources across "),
                strong(pages.len().to_string()),
                normal(" resource groups, with "),
                strong(subscription_findings.to_string()),
                normal(" findings recorded in this snapshot."),
            ],
        });
        blocks.push(Block::Paragraph {
            style: ParagraphStyle::Muted,
            runs: vec![mono(subscription.subscription_id.as_str())],
        });

        for page in pages {
            heading(blocks, 2, page.resource_group.as_str(), None);
            let types: BTreeSet<_> = page
                .resources
                .iter()
                .map(|detail| detail.azure_type.as_str())
                .collect();
            let group_findings: usize = page
                .resources
                .iter()
                .map(|detail| detail.findings.len())
                .sum();
            let mut runs = vec![
                strong(page.resources.len().to_string()),
                normal(" resources across "),
                strong(types.len().to_string()),
                normal(" resource types"),
            ];
            if let Some(location) = &page.location {
                runs.push(normal(" · "));
                runs.push(normal(location.as_str()));
            }
            runs.push(normal(" · "));
            runs.push(strong(group_findings.to_string()));
            runs.push(normal(" findings"));
            blocks.push(Block::Paragraph {
                style: ParagraphStyle::Muted,
                runs,
            });

            if let Some(asset) = group_diagrams.get(page.group_key.as_str()) {
                blocks.push(Block::Diagram {
                    slug: Cow::Borrowed(asset.slug.as_str()),
                    caption: Some(Cow::Borrowed(page.resource_group.as_str())),
                });
            }

            for detail in &page.resources {
                blocks.push(Block::ResourcePlate {
                    name: Cow::Borrowed(detail.name.as_str()),
                });
                let mut context = vec![normal(detail.display_type.as_str())];
                if let Some(location) = &detail.location {
                    context.push(normal(" · "));
                    context.push(normal(location.as_str()));
                }
                context.push(normal(" · "));
                context.push(normal(relationship_count(detail.related.len())));
                context.push(normal(" · "));
                context.push(normal(finding_count(detail.findings.len())));
                blocks.push(Block::Paragraph {
                    style: ParagraphStyle::Muted,
                    runs: context,
                });
                blocks.push(Block::Paragraph {
                    style: ParagraphStyle::Muted,
                    runs: vec![mono(detail.arm_id.as_str())],
                });

                let normalized_id = detail.arm_id.to_lowercase();
                if let Some(asset) = resource_diagrams.get(normalized_id.as_str()) {
                    sub_label(blocks, "Relationships");
                    blocks.push(Block::Diagram {
                        slug: Cow::Borrowed(asset.slug.as_str()),
                        caption: None,
                    });
                }

                sub_label(blocks, "Settings");
                if detail.settings.is_empty() {
                    empty(blocks, "No settings recorded.");
                } else {
                    blocks.push(Block::Facts {
                        items: detail
                            .settings
                            .iter()
                            .map(|setting| {
                                fact(setting.key.as_str(), setting.value.as_str(), false)
                            })
                            .collect(),
                    });
                }

                if !detail.findings.is_empty() {
                    sub_label(blocks, "Findings");
                    for callout in &detail.findings {
                        blocks.push(Block::Callout {
                            severity: Cow::Borrowed(callout.severity.as_str()),
                            title: Cow::Borrowed(callout.title.as_str()),
                            detail: None,
                        });
                    }
                }
                if !detail.related.is_empty() {
                    sub_label(blocks, "Related resources");
                    blocks.push(Block::Paragraph {
                        style: ParagraphStyle::Body,
                        runs: vec![normal(detail.related.join(" · "))],
                    });
                }
            }
        }
    }
}

fn build_overviews<'a>(diagrams: &'a [DiagramAsset], blocks: &mut Vec<Block<'a>>) {
    let overviews: Vec<&DiagramAsset> = diagrams
        .iter()
        .filter(|asset| {
            matches!(
                asset.kind,
                DiagramAssetKind::Hierarchy | DiagramAssetKind::Network
            )
        })
        .collect();
    if overviews.is_empty() {
        return;
    }
    major_chapter(blocks, "Estate overview", true);
    blocks.push(Block::Paragraph {
        style: ParagraphStyle::Body,
        runs: vec![normal(
            "These overview diagrams orient the detailed subscription and resource-group sections that follow. Resource-group diagrams remain with their own sections.",
        )],
    });
    for asset in overviews {
        blocks.push(Block::Diagram {
            slug: Cow::Borrowed(asset.slug.as_str()),
            caption: Some(Cow::Borrowed(asset.title.as_str())),
        });
    }
}

fn major_chapter<'a>(
    blocks: &mut Vec<Block<'a>>,
    title: impl Into<Cow<'a, str>>,
    break_before: bool,
) {
    blocks.push(Block::Chapter {
        title: title.into(),
        break_before,
        divider: true,
    });
}

fn chapter<'a>(blocks: &mut Vec<Block<'a>>, title: impl Into<Cow<'a, str>>, break_before: bool) {
    blocks.push(Block::Chapter {
        title: title.into(),
        break_before,
        divider: false,
    });
}

fn heading<'a>(
    blocks: &mut Vec<Block<'a>>,
    level: u8,
    title: impl Into<Cow<'a, str>>,
    icon: Option<&'a str>,
) {
    blocks.push(Block::Heading {
        level,
        title: title.into(),
        icon: icon.map(Cow::Borrowed),
    });
}

fn sub_label<'a>(blocks: &mut Vec<Block<'a>>, title: &'static str) {
    blocks.push(Block::SubLabel {
        title: Cow::Borrowed(title),
    });
}

fn empty<'a>(blocks: &mut Vec<Block<'a>>, text: &'static str) {
    blocks.push(Block::EmptyState {
        text: Cow::Borrowed(text),
    });
}

fn fact<'a>(
    label: impl Into<Cow<'a, str>>,
    value: impl Into<Cow<'a, str>>,
    mono: bool,
) -> Fact<'a> {
    Fact {
        label: label.into(),
        value: value.into(),
        mono,
    }
}

fn finding_callout<'a>(finding: &'a super::FindingRow) -> Block<'a> {
    Block::Callout {
        severity: Cow::Borrowed(finding.severity.as_str()),
        title: Cow::Borrowed(finding.title.as_str()),
        detail: Some(Cow::Owned(format!(
            "{} · {}",
            capitalise(&finding.category),
            finding.query_name
        ))),
    }
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "high" => 0,
        "medium" => 1,
        "low" => 2,
        "info" => 3,
        _ => 4,
    }
}

fn resource_count(count: usize) -> String {
    counted(count, "resource", "resources")
}

fn relationship_count(count: usize) -> String {
    counted(count, "relationship", "relationships")
}

fn finding_count(count: usize) -> String {
    counted(count, "finding", "findings")
}

fn counted(count: usize, singular: &str, plural: &str) -> String {
    format!("{count} {}", if count == 1 { singular } else { plural })
}

fn statistic<'a>(value: String, label: &'static str) -> Statistic<'a> {
    Statistic {
        value: Cow::Owned(value),
        label: Cow::Borrowed(label),
    }
}

fn normal<'a>(text: impl Into<Cow<'a, str>>) -> TextRun<'a> {
    TextRun {
        text: text.into(),
        style: TextStyle::Normal,
        severity: None,
    }
}

fn mono<'a>(text: impl Into<Cow<'a, str>>) -> TextRun<'a> {
    TextRun {
        text: text.into(),
        style: TextStyle::Mono,
        severity: None,
    }
}

fn strong<'a>(text: impl Into<Cow<'a, str>>) -> TextRun<'a> {
    TextRun {
        text: text.into(),
        style: TextStyle::Strong,
        severity: None,
    }
}

fn severity_run<'a>(text: String, severity: &'static str) -> TextRun<'a> {
    TextRun {
        text: Cow::Owned(text),
        style: TextStyle::Severity,
        severity: Some(Cow::Borrowed(severity)),
    }
}

fn option_text(value: Option<&str>) -> Cow<'_, str> {
    value.map_or_else(|| Cow::Borrowed(""), Cow::Borrowed)
}

fn capitalise(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn display_label(value: &str) -> String {
    let words = value.replace(['_', '-'], " ");
    capitalise(&words)
}

fn looks_like_identifier(value: &str) -> bool {
    matches!(
        value,
        "id" | "azure_type" | "resource_id" | "subscription_id" | "query_name"
    ) || value.ends_with("_id")
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::report::details::{Callout, ResourceDetail, ResourceGroupPage, Setting};
    use crate::report::{
        Category, FindingRow, QuerySection, ResourceTypeSection, SeverityCounts,
        SubscriptionSection, TagCoverage, Totals, TypeCount,
    };

    fn detail() -> ResourceDetail {
        ResourceDetail {
            name: "st-app".to_owned(),
            display_type: "Storage account".to_owned(),
            azure_type: "microsoft.storage/storageaccounts".to_owned(),
            arm_id: "/Subscriptions/S1/resourceGroups/RG-App/providers/Microsoft.Storage/storageAccounts/st-app"
                .to_owned(),
            subscription_name: "Production".to_owned(),
            resource_group: Some("rg-app".to_owned()),
            location: Some("uksouth".to_owned()),
            settings: vec![Setting {
                key: "Public access".to_owned(),
                value: "Disabled".to_owned(),
            }],
            findings: vec![Callout {
                severity: "high".to_owned(),
                title: "Public access enabled".to_owned(),
            }],
            related: vec!["nic-app (attached to)".to_owned()],
        }
    }

    fn report() -> ReportContext {
        ReportContext {
            snapshot_id: "snapshot-1".to_owned(),
            created_at: "2026-08-17T10:00:00+00:00".to_owned(),
            tenant_id: "tenant-1".to_owned(),
            status: "complete".to_owned(),
            notes: None,
            totals: Totals {
                subscriptions: 1,
                resource_groups: 1,
                resources: 1,
                findings: 1,
            },
            type_counts: vec![TypeCount {
                azure_type: "microsoft.storage/storageaccounts".to_owned(),
                display: "Storage account".to_owned(),
                count: 1,
            }],
            location_counts: Vec::new(),
            severity_counts: SeverityCounts {
                high: 1,
                ..SeverityCounts::default()
            },
            tag_coverage: TagCoverage {
                tagged: 1,
                untagged: 0,
                percent: 100,
            },
            categories: vec![Category {
                name: "storage".to_owned(),
                queries: vec![QuerySection {
                    name: "Storage configuration".to_owned(),
                    description: "Configuration inventory".to_owned(),
                    columns: vec![
                        "azure_type".to_owned(),
                        "resource_group".to_owned(),
                        "values".to_owned(),
                    ],
                    print_columns: vec![
                        "azure_type".to_owned(),
                        "resource_group".to_owned(),
                        "values".to_owned(),
                    ],
                    rows: vec![json!({
                        "azure_type": "microsoft.storage/storageaccounts",
                        "resource_group": "rg-app",
                        "values": [1, true]
                    })],
                }],
            }],
            findings: vec![FindingRow {
                severity: "high".to_owned(),
                category: "storage".to_owned(),
                query_name: "public_access".to_owned(),
                title: "Public access enabled".to_owned(),
                resource_id: Some("/subscriptions/s1/resourcegroups/rg-app/st-app".to_owned()),
            }],
            subscriptions: vec![SubscriptionSection {
                subscription_id: "S1".to_owned(),
                display_name: "Production".to_owned(),
                state: Some("Enabled".to_owned()),
                resource_count: 1,
                resource_groups: Vec::new(),
            }],
            details: vec![ResourceGroupPage {
                path: "resources/production/rg-app".to_owned(),
                subscription_name: "Production".to_owned(),
                subscription_slug: "production".to_owned(),
                resource_group: "rg-app".to_owned(),
                location: Some("uksouth".to_owned()),
                group_key: "s1/rg-app".to_owned(),
                resources: vec![detail()],
            }],
            resource_types: vec![ResourceTypeSection {
                display: "Storage account".to_owned(),
                azure_type: "microsoft.storage/storageaccounts".to_owned(),
                resources: vec![detail()],
            }],
        }
    }

    fn diagram(
        slug: &str,
        title: &str,
        kind: DiagramAssetKind,
        resource_id: Option<&str>,
        group_key: Option<&str>,
    ) -> DiagramAsset {
        DiagramAsset {
            slug: slug.to_owned(),
            title: title.to_owned(),
            kind,
            svg: "<svg/>".to_owned(),
            resource_id: resource_id.map(str::to_owned),
            group_key: group_key.map(str::to_owned),
        }
    }

    fn diagrams() -> Vec<DiagramAsset> {
        vec![
            diagram(
                "group",
                "rg-app",
                DiagramAssetKind::ResourceGroup,
                None,
                Some("s1/rg-app"),
            ),
            diagram(
                "resource",
                "st-app",
                DiagramAssetKind::Resource,
                Some(
                    "/subscriptions/s1/resourcegroups/rg-app/providers/microsoft.storage/storageaccounts/st-app",
                ),
                None,
            ),
            diagram(
                "hierarchy",
                "Azure hierarchy",
                DiagramAssetKind::Hierarchy,
                None,
                None,
            ),
            diagram(
                "network",
                "Azure network",
                DiagramAssetKind::Network,
                None,
                None,
            ),
        ]
    }

    fn chapter_index(document: &PrintDocument<'_>, title: &str) -> usize {
        document
            .blocks
            .iter()
            .position(
                |block| matches!(block, Block::Chapter { title: value, .. } if value == title),
            )
            .unwrap_or_else(|| panic!("chapter {title} missing"))
    }

    #[test]
    fn unit_build_orders_type_index_before_subscription_body() {
        let report = report();
        let diagrams = diagrams();
        let branding = BrandingContext::default();
        let document = PrintDocument::build(&report, &branding, &diagrams);

        assert!(
            chapter_index(&document, "Resources by type") < chapter_index(&document, "Production")
        );
    }

    #[test]
    fn unit_build_uses_level_two_icon_headings_for_type_index_entries() {
        let report = report();
        let diagrams = diagrams();
        let branding = BrandingContext::default();
        let document = PrintDocument::build(&report, &branding, &diagrams);

        assert!(document.blocks.iter().any(|block| matches!(
            block,
            Block::Heading { level: 2, title, icon: Some(icon) }
                if title == "Storage account" && icon == "microsoft.storage/storageaccounts"
        )));
    }

    #[test]
    fn unit_build_resolves_shared_fact_labels_and_json_cells() {
        let report = report();
        let diagrams = diagrams();
        let branding = BrandingContext::default();
        let document = PrintDocument::build(&report, &branding, &diagrams);

        let facts = document.blocks.iter().find_map(|block| match block {
            Block::Facts { items }
                if items.iter().any(|item| item.label == "Azure type")
                    && items.iter().any(|item| item.label == "Resource group")
                    && items.iter().any(|item| item.value == "[1,true]") =>
            {
                Some(())
            }
            _ => None,
        });
        assert_eq!(facts, Some(()));
    }

    #[test]
    fn unit_build_emits_shared_empty_states() {
        let mut report = report();
        report.findings.clear();
        report.details[0].resources[0].settings.clear();
        let diagrams = diagrams();
        let branding = BrandingContext::default();
        let document = PrintDocument::build(&report, &branding, &diagrams);

        let states: Vec<&str> = document
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::EmptyState { text } => Some(text.as_ref()),
                _ => None,
            })
            .collect();
        assert!(states.contains(&"No findings.") && states.contains(&"No settings recorded."));
    }

    #[test]
    fn unit_build_places_each_diagram_in_its_semantic_section() {
        let report = report();
        let diagrams = diagrams();
        let branding = BrandingContext::default();
        let document = PrintDocument::build(&report, &branding, &diagrams);

        let markers: Vec<(&str, Option<&str>)> = document
            .blocks
            .iter()
            .filter_map(|block| match block {
                Block::Diagram { slug, caption } => Some((slug.as_ref(), caption.as_deref())),
                _ => None,
            })
            .collect();
        assert_eq!(
            markers,
            vec![
                ("hierarchy", Some("Azure hierarchy")),
                ("network", Some("Azure network")),
                ("group", Some("rg-app")),
                ("resource", None),
            ]
        );
    }

    #[test]
    fn unit_build_uses_shared_cover_fields_and_toc_depth() {
        let report = report();
        let diagrams = diagrams();
        let branding = BrandingContext::default();
        let document = PrintDocument::build(&report, &branding, &diagrams);

        assert_eq!(
            (
                document.toc_depth,
                document.cover.product_mark,
                document.cover.tenant.as_ref(),
                document.cover.snapshot.as_ref(),
                document.cover.collected.as_ref(),
                document.cover.status.as_ref(),
            ),
            (
                1,
                true,
                "tenant-1",
                "snapshot-1",
                "2026-08-17T10:00:00+00:00",
                "complete",
            )
        );
    }
}
