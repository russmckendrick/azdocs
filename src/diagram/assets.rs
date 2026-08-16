//! Prebuilt SVG+PNG diagram bundle for report emitters: the HTML site embeds
//! these today and the PDF/DOCX emitters consume the same seam later.

use crate::error::DiagramError;
use crate::store::Store;

use super::graph::{DiagramScope, EstateGraph};
use super::{png, svg};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramAssetKind {
    Hierarchy,
    Network,
    Peerings,
    Vnet,
    ResourceGroup,
}

#[derive(Debug)]
pub struct DiagramAsset {
    /// Unique across the whole set — safe as a file stem.
    pub slug: String,
    pub title: String,
    pub kind: DiagramAssetKind,
    pub svg: String,
    pub png: Vec<u8>,
}

/// Build every diagram for a snapshot, rendered to both SVG and PNG. Empty
/// graphs (e.g. peerings in a VNet-less estate) are omitted.
pub fn build_all(
    store: &Store,
    snapshot_id: &str,
    scope: &DiagramScope,
) -> Result<Vec<DiagramAsset>, DiagramError> {
    let mut assets = Vec::new();
    let mut push = |slug: String,
                    title: String,
                    kind: DiagramAssetKind,
                    graph: &EstateGraph|
     -> Result<(), DiagramError> {
        if graph.nodes.is_empty() {
            return Ok(());
        }
        let svg_text = svg::render(graph);
        let png_bytes = png::from_svg(&svg_text, png::DEFAULT_SCALE)?;
        assets.push(DiagramAsset {
            slug,
            title,
            kind,
            svg: svg_text,
            png: png_bytes,
        });
        Ok(())
    };

    let hierarchy = EstateGraph::hierarchy(store, snapshot_id)?;
    push(
        "hierarchy".to_owned(),
        hierarchy.title.clone(),
        DiagramAssetKind::Hierarchy,
        &hierarchy,
    )?;
    let network = EstateGraph::network(store, snapshot_id, scope)?;
    push(
        "network".to_owned(),
        network.title.clone(),
        DiagramAssetKind::Network,
        &network,
    )?;
    let peerings = EstateGraph::peerings(store, snapshot_id, scope)?;
    push(
        "peerings".to_owned(),
        peerings.title.clone(),
        DiagramAssetKind::Peerings,
        &peerings,
    )?;
    for named in EstateGraph::per_vnet(store, snapshot_id, scope)? {
        push(
            format!("vnet-{}", named.slug),
            named.sheet_name.clone(),
            DiagramAssetKind::Vnet,
            &named.graph,
        )?;
    }
    for named in EstateGraph::per_resource_group(store, snapshot_id, scope)? {
        push(
            format!("resource-group-{}", named.slug),
            named.sheet_name.clone(),
            DiagramAssetKind::ResourceGroup,
            &named.graph,
        )?;
    }
    Ok(assets)
}
