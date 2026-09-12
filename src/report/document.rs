#[path = "assessment.rs"]
mod assessment;
#[path = "metadata.rs"]
mod metadata;

use std::borrow::Cow;

use serde::Serialize;

use super::branding::BrandingContext;
use super::{ReportContext, cell_to_string};
use crate::diagram::assets::DiagramAsset;
use crate::labels::Labels;

#[derive(Debug, Serialize)]
pub(crate) struct PrintDocument<'a> {
    pub technical_reference: bool,
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
    RasterImage {
        slug: String,
        #[serde(skip)]
        png: &'a [u8],
        caption: Cow<'a, str>,
    },
    ExternalLink {
        title: Cow<'a, str>,
        url: Cow<'a, str>,
    },
    Metadata {
        groups: Vec<MetadataGroup>,
        keep_together: bool,
    },
    Section {
        level: u8,
        title: Cow<'a, str>,
        id: String,
        break_before: bool,
    },
    CrossReference {
        target: String,
        title: Cow<'a, str>,
    },
    Chart {
        slug: String,
        svg: String,
        caption: Cow<'a, str>,
    },
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
    BulletList {
        items: Vec<Cow<'a, str>>,
    },
    Statistics {
        items: Vec<Statistic<'a>>,
    },
    Table {
        style: TableKind,
        keep_together: bool,
        columns: Vec<TableColumn<'a>>,
        rows: Vec<Vec<Cow<'a, str>>>,
        links: Vec<TableLink>,
    },
    Facts {
        items: Vec<Fact<'a>>,
    },
    ResourceIndex {
        items: Vec<ResourceIndexItem<'a>>,
    },
    ResourcePlate {
        id: String,
        name: Cow<'a, str>,
        icon: Cow<'a, str>,
        subtitle: Cow<'a, str>,
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

pub(super) fn external_link_svg(color: &str) -> String {
    format!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="{color}" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M15 3h6v6M10 14 21 3"/><path d="M21 14v5a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5"/></svg>"#
    )
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
    Mono,
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
    pub target: Option<String>,
    pub name: Cow<'a, str>,
    pub subscription: Cow<'a, str>,
    pub resource_group: Cow<'a, str>,
    pub location: Cow<'a, str>,
}

#[derive(Debug, Serialize)]
pub(crate) struct TableLink {
    pub row: usize,
    pub column: usize,
    pub target: String,
    pub external: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct MetadataGroup {
    pub title: String,
    pub rows: Vec<Vec<String>>,
    pub links: Vec<TableLink>,
}

/// Short, collision-free bookmarks within the selected snapshot. Identity is
/// the normalized ARM ID, never a resource's potentially duplicated name.
fn resource_anchor(report: &ReportContext, id: &str) -> Option<String> {
    let id = crate::model::normalize_arm_id(id);
    report
        .analysis
        .resources
        .keys()
        .position(|key| key == &id)
        .map(|index| format!("resource_{index}"))
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
    pub weight: u32,
}

/// A shared column policy keeps both print formats readable. Capping the
/// content contribution prevents identifiers from squeezing numeric headers.
pub(crate) fn table_weights(headers: &[&str], rows: &[Vec<String>]) -> Vec<u32> {
    headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            let longest = rows
                .iter()
                .filter_map(|row| row.get(index))
                .map(|value| value.chars().count())
                .max()
                .unwrap_or(0);
            longest.clamp(10, 30).max(header.chars().count().min(30)) as u32
        })
        .collect()
}

impl<'a> PrintDocument<'a> {
    pub fn build(
        report: &'a ReportContext,
        branding: &'a BrandingContext,
        diagrams: &'a [DiagramAsset],
    ) -> Self {
        let mut blocks = Vec::new();
        assessment::build(report, branding, diagrams, &mut blocks);

        Self {
            technical_reference: false,
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
            toc_depth: 2,
            blocks,
        }
    }

    pub fn reference(
        report: &'a ReportContext,
        branding: &'a BrandingContext,
        diagrams: &'a [DiagramAsset],
    ) -> Self {
        let mut document = Self::build(report, branding, &[]);
        document.technical_reference = true;
        document.cover.title = Cow::Borrowed(&branding.labels.report.assessment.reference_title);
        document.blocks.clear();
        assessment::reference(report, branding, diagrams, &mut document.blocks);
        for block in &mut document.blocks {
            if let Block::Facts { items } = block {
                let headers = [
                    &branding.labels.common.columns.setting,
                    &branding.labels.common.columns.value,
                ];
                let rows: Vec<Vec<Cow<'a, str>>> = std::mem::take(items)
                    .into_iter()
                    .map(|item| vec![item.label, item.value])
                    .collect();
                *block = Block::Table {
                    style: TableKind::Data,
                    keep_together: false,
                    links: Vec::new(),
                    columns: headers
                        .into_iter()
                        .enumerate()
                        .map(|(index, label)| TableColumn {
                            label: Cow::Borrowed(label),
                            mono: false,
                            weight: if index == 0 { 1 } else { 2 },
                        })
                        .collect(),
                    rows,
                };
            }
        }
        document
    }

    /// Both native renderers apply the same theme-defined reference density.
    pub fn render_branding(&self, branding: &BrandingContext) -> BrandingContext {
        let mut resolved = branding.clone();
        if self.technical_reference {
            resolved.tokens.layout.table_inset_pt = resolved.tokens.layout.reference_table_inset_pt;
            let typ = &mut resolved.tokens.typography;
            let scale = typ.reference_scale;
            typ.base_pt = (typ.base_pt * scale).max(8.0);
            typ.small_pt = (typ.small_pt * scale).max(8.0);
            typ.table_pt = (typ.table_pt * scale).max(8.0);
            typ.table_header_pt = (typ.table_header_pt * scale).max(8.0);
        }
        resolved
    }

    pub fn icon_types(&self) -> impl Iterator<Item = &str> {
        self.blocks.iter().filter_map(|block| match block {
            Block::Heading {
                icon: Some(icon), ..
            } => Some(icon.as_ref()),
            Block::ResourcePlate { icon, .. } => Some(icon.as_ref()),
            _ => None,
        })
    }
}

fn build_type_index<'a>(
    report: &'a ReportContext,
    labels: &'a Labels,
    blocks: &mut Vec<Block<'a>>,
) {
    let index = &labels.report.type_index;
    major_chapter(blocks, index.chapter.as_str(), true);
    blocks.push(Block::Paragraph {
        style: ParagraphStyle::Body,
        runs: vec![normal(index.intro.as_str())],
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
                    target: resource_anchor(report, &detail.arm_id),
                    name: Cow::Borrowed(detail.name.as_str()),
                    subscription: Cow::Borrowed(detail.subscription_name.as_str()),
                    resource_group: option_text(detail.resource_group.as_deref()),
                    location: option_text(detail.location.as_deref()),
                })
                .collect(),
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

fn sub_label<'a>(blocks: &mut Vec<Block<'a>>, title: &'a str) {
    blocks.push(Block::SubLabel {
        title: Cow::Borrowed(title),
    });
}

fn empty<'a>(blocks: &mut Vec<Block<'a>>, text: &'a str) {
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

fn statistic<'a>(value: String, label: &'a str) -> Statistic<'a> {
    Statistic {
        value: Cow::Owned(value),
        label: Cow::Borrowed(label),
    }
}

pub(super) fn normal<'a>(text: impl Into<Cow<'a, str>>) -> TextRun<'a> {
    TextRun {
        text: text.into(),
        style: TextStyle::Normal,
        severity: None,
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
