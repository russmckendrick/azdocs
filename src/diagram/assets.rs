//! Prebuilt diagram bundle for report emitters: overviews (hierarchy and
//! network) and one summarised picture per resource group, each rendered
//! once as SVG and once as Mermaid text. Markdown embeds both, the HTML site
//! and single-file HTML the SVG, the PDF the SVG through Typst, and the DOCX
//! rasterises the same SVG to PNG at embed time.
//!
//! The per-VNet fan-out is never built here: no report consumes it, and
//! `azdocs diagram` renders it on demand.

use crate::error::DiagramError;
use crate::labels::DiagramLabels;
use crate::store::Store;

use super::graph::{DiagramScope, EstateGraph};
use super::page::DiagramDetail;
use super::{mermaid, svg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramAssetKind {
    Hierarchy,
    Network,
    /// One resource group, summarised to fit a share of the page.
    ResourceGroup,
}

#[derive(Debug)]
pub struct DiagramAsset {
    /// Unique across the whole set — safe as a file stem.
    pub slug: String,
    pub title: String,
    pub kind: DiagramAssetKind,
    pub svg: String,
    /// The same graph as Mermaid text, for hosts that render it natively.
    pub mermaid: String,
    /// Set on [`DiagramAssetKind::ResourceGroup`] assets: the
    /// `<subscription id>/<group name>` key its report section is filed under.
    /// Group names repeat across subscriptions, so the name alone will not do.
    pub group_key: Option<String>,
}

/// Default ceiling on per-group diagrams. Past this the Diagrams chapter is
/// longer than the report it belongs to. Exceeding it is logged, never silent.
pub const MAX_GROUP_DIAGRAMS: usize = 60;

/// Selected report figures use the same graph, page, layout, route, icon and
/// SVG pipeline as the other diagram exports. Selection never draws geometry.
pub fn build_assessment(
    analysis: &crate::report::analysis::ReportAnalysis,
    labels: &crate::labels::Labels,
    max_figure_nodes: usize,
) -> Vec<DiagramAsset> {
    super::graph::assessment::build(analysis, labels, max_figure_nodes)
        .into_iter()
        .map(|named| DiagramAsset {
            svg: svg::render_for(&named.graph, DiagramDetail::Summary, &labels.diagram),
            mermaid: mermaid::render_for(&named.graph, DiagramDetail::Summary, &labels.diagram),
            kind: if named.group_key.is_some() {
                DiagramAssetKind::ResourceGroup
            } else {
                DiagramAssetKind::Network
            },
            slug: named.slug,
            title: named.sheet_name,
            group_key: named.group_key,
        })
        .collect()
}

/// Build the overview diagrams for a snapshot, rendered to SVG. Empty graphs
/// (e.g. the network view of a VNet-less estate) are omitted.
pub fn build_overviews(
    store: &Store,
    snapshot_id: &str,
    scope: &DiagramScope,
    labels: &DiagramLabels,
) -> Result<Vec<DiagramAsset>, DiagramError> {
    let mut assets = Vec::new();
    let mut push = |slug: &str, kind: DiagramAssetKind, graph: &EstateGraph| {
        if !graph.nodes.is_empty() {
            assets.push(DiagramAsset {
                slug: slug.to_owned(),
                title: graph.title.clone(),
                kind,
                svg: svg::render(graph, labels),
                mermaid: mermaid::render(graph, labels),
                group_key: None,
            });
        }
    };

    let hierarchy = EstateGraph::hierarchy(store, snapshot_id, scope, labels)?;
    push("hierarchy", DiagramAssetKind::Hierarchy, &hierarchy);
    let network = EstateGraph::network(store, snapshot_id, scope, labels)?;
    push("network", DiagramAssetKind::Network, &network);

    assets.extend(build_group_summaries(store, snapshot_id, scope, labels)?);
    Ok(assets)
}

/// Reference-only exports need group figures, without the unused estate
/// hierarchy and network overview rendered for the HTML site.
pub fn build_group_summaries(
    store: &Store,
    snapshot_id: &str,
    scope: &DiagramScope,
    labels: &DiagramLabels,
) -> Result<Vec<DiagramAsset>, DiagramError> {
    build_group_summaries_capped(store, snapshot_id, scope, labels, MAX_GROUP_DIAGRAMS)
}

/// The same, with the cap supplied by `[report] max_group_diagrams`.
pub fn build_group_summaries_capped(
    store: &Store,
    snapshot_id: &str,
    scope: &DiagramScope,
    labels: &DiagramLabels,
    cap: usize,
) -> Result<Vec<DiagramAsset>, DiagramError> {
    let mut assets = Vec::new();
    // One summarised diagram per resource group. The estate-wide view cannot
    // show 294 resources on a page at any readable size, but a group at a time
    // can — with its resources aggregated by type, which is what keeps each
    // one inside a share of the sheet.
    let groups =
        EstateGraph::per_resource_group(store, snapshot_id, scope, DiagramDetail::Summary, labels)?;
    if groups.len() > cap {
        tracing::warn!(
            total = groups.len(),
            cap,
            "too many resource groups to diagram individually; the remainder \
             are reported without a diagram"
        );
    }
    for named in groups.iter().take(cap) {
        if named.graph.nodes.is_empty() {
            continue;
        }
        assets.push(DiagramAsset {
            slug: format!("rg-{}", named.slug),
            title: named.sheet_name.clone(),
            kind: DiagramAssetKind::ResourceGroup,
            svg: svg::render_for(&named.graph, DiagramDetail::Summary, labels),
            mermaid: mermaid::render_for(&named.graph, DiagramDetail::Summary, labels),
            group_key: named.group_key.clone(),
        });
    }
    Ok(assets)
}
