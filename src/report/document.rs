use std::borrow::Cow;
use std::collections::BTreeMap;

use serde::Serialize;

use super::branding::BrandingContext;
use super::{ReportContext, cell_to_string};
use crate::diagram::assets::{DiagramAsset, DiagramAssetKind};

#[derive(Debug, Serialize)]
pub(crate) struct PrintDocument<'a> {
    pub cover: Cover<'a>,
    pub toc_depth: u8,
    pub blocks: Vec<Block<'a>>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Cover<'a> {
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
    ResourcePlate {
        name: Cow<'a, str>,
    },
    SubLabel {
        title: Cow<'a, str>,
    },
    Callout {
        severity: Cow<'a, str>,
        title: Cow<'a, str>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TableKind {
    Data,
    Findings,
    Settings,
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
        build_findings(report, &mut blocks);
        build_categories(report, &mut blocks);
        build_type_index(report, &mut blocks);
        build_estate(report, &group_diagrams, &resource_diagrams, &mut blocks);
        build_overviews(diagrams, &mut blocks);

        Self {
            cover: Cover {
                title: Cow::Borrowed(&branding.title),
                subtitle: Cow::Borrowed(&branding.subtitle),
                company: Cow::Borrowed(&branding.company),
                tenant: Cow::Borrowed(&report.tenant_id),
                snapshot: Cow::Borrowed(&report.snapshot_id),
                collected: Cow::Borrowed(&report.created_at),
                status: Cow::Borrowed(&report.status),
            },
            toc_depth: 2,
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
    chapter(blocks, "Executive Summary", false);
    blocks.push(Block::Statistics {
        items: vec![
            statistic(report.totals.subscriptions.to_string(), "Subscriptions"),
            statistic(report.totals.resource_groups.to_string(), "Resource groups"),
            statistic(report.totals.resources.to_string(), "Resources"),
            statistic(report.totals.findings.to_string(), "Findings"),
            statistic(format!("{}%", report.tag_coverage.percent), "Tag coverage"),
        ],
    });

    let severity = &report.severity_counts;
    blocks.push(Block::Paragraph {
        style: ParagraphStyle::Body,
        runs: vec![
            normal("Findings by severity: "),
            severity_run(severity.high.to_string(), "high"),
            normal(" high, "),
            severity_run(severity.medium.to_string(), "medium"),
            normal(" medium, "),
            severity_run(severity.low.to_string(), "low"),
            normal(" low, "),
            severity_run(severity.info.to_string(), "info"),
            normal(" info. "),
            strong(report.tag_coverage.tagged.to_string()),
            normal(" resources are tagged and "),
            strong(report.tag_coverage.untagged.to_string()),
            normal(" are untagged."),
        ],
    });

    heading(blocks, 2, "Resources by type", None);
    blocks.push(Block::Table {
        style: TableKind::Data,
        columns: columns(&[("Type", false), ("Azure type", true), ("Count", false)]),
        rows: report
            .type_counts
            .iter()
            .map(|item| {
                vec![
                    Cow::Borrowed(item.display.as_str()),
                    Cow::Borrowed(item.azure_type.as_str()),
                    Cow::Owned(item.count.to_string()),
                ]
            })
            .collect(),
    });
}

fn build_findings<'a>(report: &'a ReportContext, blocks: &mut Vec<Block<'a>>) {
    chapter(blocks, "Findings", false);
    if report.findings.is_empty() {
        empty(blocks, "No findings.");
        return;
    }
    blocks.push(Block::Table {
        style: TableKind::Findings,
        columns: columns(&[
            ("Severity", false),
            ("Title", false),
            ("Category", false),
            ("Check", true),
        ]),
        rows: report
            .findings
            .iter()
            .map(|finding| {
                vec![
                    Cow::Borrowed(finding.severity.as_str()),
                    Cow::Borrowed(finding.title.as_str()),
                    Cow::Borrowed(finding.category.as_str()),
                    Cow::Borrowed(finding.query_name.as_str()),
                ]
            })
            .collect(),
    });
}

fn build_categories<'a>(report: &'a ReportContext, blocks: &mut Vec<Block<'a>>) {
    for category in &report.categories {
        chapter(blocks, capitalise(&category.name), false);
        for query in &category.queries {
            heading(blocks, 2, query.name.as_str(), None);
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
    chapter(blocks, "Resources by type", false);
    for section in &report.resource_types {
        heading(
            blocks,
            2,
            section.display.as_str(),
            Some(section.azure_type.as_str()),
        );
        blocks.push(Block::Table {
            style: TableKind::Data,
            columns: columns(&[
                ("Name", false),
                ("Subscription", false),
                ("Resource group", false),
                ("Location", false),
            ]),
            rows: section
                .resources
                .iter()
                .map(|detail| {
                    vec![
                        Cow::Borrowed(detail.name.as_str()),
                        Cow::Borrowed(detail.subscription_name.as_str()),
                        option_text(detail.resource_group.as_deref()),
                        option_text(detail.location.as_deref()),
                    ]
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
        blocks.push(Block::Paragraph {
            style: ParagraphStyle::Muted,
            runs: vec![
                mono(subscription.subscription_id.as_str()),
                normal(" · "),
                normal(subscription.resource_count.to_string()),
                normal(" resources"),
            ],
        });

        let group_prefix = format!("{}/", subscription.subscription_id.to_lowercase());
        for page in report
            .details
            .iter()
            .filter(|page| page.group_key.starts_with(&group_prefix))
        {
            heading(blocks, 2, page.resource_group.as_str(), None);
            let mut runs = vec![
                normal(page.resources.len().to_string()),
                normal(" resources"),
            ];
            if let Some(location) = &page.location {
                runs.push(normal(" · "));
                runs.push(normal(location.as_str()));
            }
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
                let mut context = vec![
                    normal(detail.display_type.as_str()),
                    normal(" · "),
                    normal(detail.subscription_name.as_str()),
                ];
                if let Some(resource_group) = &detail.resource_group {
                    context.push(normal(" · "));
                    context.push(normal(resource_group.as_str()));
                }
                if let Some(location) = &detail.location {
                    context.push(normal(" · "));
                    context.push(normal(location.as_str()));
                }
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
                    blocks.push(Block::Table {
                        style: TableKind::Settings,
                        columns: columns(&[("Setting", false), ("Value", false)]),
                        rows: detail
                            .settings
                            .iter()
                            .map(|setting| {
                                vec![
                                    Cow::Borrowed(setting.key.as_str()),
                                    Cow::Borrowed(setting.value.as_str()),
                                ]
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
    chapter(blocks, "Diagrams", false);
    for asset in overviews {
        blocks.push(Block::Diagram {
            slug: Cow::Borrowed(asset.slug.as_str()),
            caption: Some(Cow::Borrowed(asset.title.as_str())),
        });
    }
}

fn chapter<'a>(blocks: &mut Vec<Block<'a>>, title: impl Into<Cow<'a, str>>, break_before: bool) {
    blocks.push(Block::Chapter {
        title: title.into(),
        break_before,
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

fn columns<'a>(values: &[(&'static str, bool)]) -> Vec<TableColumn<'a>> {
    values
        .iter()
        .map(|(label, mono)| TableColumn {
            label: Cow::Borrowed(*label),
            mono: *mono,
        })
        .collect()
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
    fn unit_build_resolves_shared_table_labels_and_json_cells() {
        let report = report();
        let diagrams = diagrams();
        let branding = BrandingContext::default();
        let document = PrintDocument::build(&report, &branding, &diagrams);

        let table = document.blocks.iter().find_map(|block| match block {
            Block::Table { columns, rows, .. }
                if columns.iter().any(|column| column.label == "Azure type")
                    && columns
                        .iter()
                        .any(|column| column.label == "Resource group")
                    && rows
                        .iter()
                        .any(|row| row.iter().any(|value| value == "[1,true]")) =>
            {
                Some(())
            }
            _ => None,
        });
        assert_eq!(table, Some(()));
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
                ("group", Some("rg-app")),
                ("resource", None),
                ("hierarchy", Some("Azure hierarchy")),
                ("network", Some("Azure network")),
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
                document.cover.tenant.as_ref(),
                document.cover.snapshot.as_ref(),
                document.cover.collected.as_ref(),
                document.cover.status.as_ref(),
            ),
            (
                2,
                "tenant-1",
                "snapshot-1",
                "2026-08-17T10:00:00+00:00",
                "complete",
            )
        );
    }
}
