//! Prebuilt SVG diagram bundle for report emitters: the HTML site and PDF
//! embed the overview diagrams (hierarchy + network) as vector images.
//!
//! Only overviews are built here, and only as SVG — the report emitters never
//! consume rasters or the per-VNet/per-RG fan-out, and PNG rendering is far
//! too slow to pay for output that would be thrown away (`azdocs diagram`
//! rasterises on demand instead).

use crate::error::DiagramError;
use crate::store::Store;

use super::graph::{DiagramScope, EstateGraph};
use super::svg;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramAssetKind {
    Hierarchy,
    Network,
}

#[derive(Debug)]
pub struct DiagramAsset {
    /// Unique across the whole set — safe as a file stem.
    pub slug: String,
    pub title: String,
    pub kind: DiagramAssetKind,
    pub svg: String,
}

/// Build the overview diagrams for a snapshot, rendered to SVG. Empty graphs
/// (e.g. the network view of a VNet-less estate) are omitted.
pub fn build_overviews(
    store: &Store,
    snapshot_id: &str,
    scope: &DiagramScope,
) -> Result<Vec<DiagramAsset>, DiagramError> {
    let mut assets = Vec::new();
    let mut push = |slug: &str, kind: DiagramAssetKind, graph: &EstateGraph| {
        if !graph.nodes.is_empty() {
            assets.push(DiagramAsset {
                slug: slug.to_owned(),
                title: graph.title.clone(),
                kind,
                svg: svg::render(graph),
            });
        }
    };

    let hierarchy = EstateGraph::hierarchy(store, snapshot_id)?;
    push("hierarchy", DiagramAssetKind::Hierarchy, &hierarchy);
    let network = EstateGraph::network(store, snapshot_id, scope)?;
    push("network", DiagramAssetKind::Network, &network);
    Ok(assets)
}
