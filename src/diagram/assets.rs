//! Prebuilt SVG diagram bundle for report emitters: the HTML site and PDF
//! embed the overview diagrams (hierarchy + network) as vector images.
//!
//! Only overviews are built here, and only as SVG — the report emitters never
//! consume rasters or the per-VNet/per-RG fan-out, and PNG rendering is far
//! too slow to pay for output that would be thrown away (`azdocs diagram`
//! rasterises on demand instead).

use crate::error::DiagramError;
use crate::labels::DiagramLabels;
use crate::store::Store;

use super::graph::{DiagramScope, EstateGraph};
use super::page::DiagramDetail;
use super::svg;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagramAssetKind {
    Hierarchy,
    Network,
    /// One resource and its immediate neighbours, for the per-resource report
    /// sections.
    Resource,
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
    /// Set on [`DiagramAssetKind::Resource`] assets: the normalized ARM id the
    /// diagram belongs to, so a report section can find its own diagram.
    pub resource_id: Option<String>,
    /// Set on [`DiagramAssetKind::ResourceGroup`] assets: the
    /// `<subscription id>/<group name>` key its report section is filed under.
    /// Group names repeat across subscriptions, so the name alone will not do.
    pub group_key: Option<String>,
}

/// Ceiling on per-resource diagrams. Each one is a layout plus an SVG render,
/// and a large estate would otherwise spend minutes drawing pictures nobody
/// scrolls to. Exceeding it is logged, never silent.
pub const MAX_RESOURCE_DIAGRAMS: usize = 250;

/// Ceiling on per-group diagrams. Past this the Diagrams chapter is longer
/// than the report it belongs to. Exceeding it is logged, never silent.
pub const MAX_GROUP_DIAGRAMS: usize = 60;

/// Selected report figures use the same graph, page, layout, route, icon and
/// SVG pipeline as the other diagram exports. Selection never draws geometry.
pub fn build_assessment(
    analysis: &crate::report::analysis::ReportAnalysis,
    labels: &crate::labels::Labels,
) -> Vec<DiagramAsset> {
    super::graph::assessment::build(analysis, labels)
        .into_iter()
        .map(|named| DiagramAsset {
            svg: svg::render_for(&named.graph, DiagramDetail::Summary, &labels.diagram),
            kind: if named.group_key.is_some() {
                DiagramAssetKind::ResourceGroup
            } else {
                DiagramAssetKind::Network
            },
            slug: named.slug,
            title: named.sheet_name,
            resource_id: None,
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
                resource_id: None,
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
    let mut assets = Vec::new();
    // One summarised diagram per resource group. The estate-wide view cannot
    // show 294 resources on a page at any readable size, but a group at a time
    // can — with its resources aggregated by type, which is what keeps each
    // one inside a share of the sheet.
    let groups =
        EstateGraph::per_resource_group(store, snapshot_id, scope, DiagramDetail::Summary, labels)?;
    if groups.len() > MAX_GROUP_DIAGRAMS {
        tracing::warn!(
            total = groups.len(),
            cap = MAX_GROUP_DIAGRAMS,
            "too many resource groups to diagram individually; the remainder \
             are reported without a diagram"
        );
    }
    for named in groups.iter().take(MAX_GROUP_DIAGRAMS) {
        if named.graph.nodes.is_empty() {
            continue;
        }
        assets.push(DiagramAsset {
            slug: format!("rg-{}", named.slug),
            title: named.sheet_name.clone(),
            kind: DiagramAssetKind::ResourceGroup,
            svg: svg::render_for(&named.graph, DiagramDetail::Summary, labels),
            resource_id: None,
            group_key: named.group_key.clone(),
        });
    }
    Ok(assets)
}

/// One diagram per connected resource, for the per-resource report sections.
///
/// Resources with no relationships are skipped: a lone box tells the reader
/// nothing the settings table does not. The set is capped at
/// [`MAX_RESOURCE_DIAGRAMS`], and truncation is logged.
pub fn build_resource_diagrams(
    store: &Store,
    snapshot_id: &str,
    labels: &DiagramLabels,
) -> Result<Vec<DiagramAsset>, DiagramError> {
    let resources = store.resources(snapshot_id)?;
    let edges = store.edges(snapshot_id)?;
    let by_id: std::collections::HashMap<&str, &crate::model::Resource> =
        resources.iter().map(|r| (r.id.as_str(), r)).collect();

    let connected: Vec<&crate::model::Resource> = resources
        .iter()
        .filter(|resource| {
            edges
                .iter()
                .any(|edge| edge.source_id == resource.id || edge.target_id == resource.id)
        })
        .collect();
    if connected.len() > MAX_RESOURCE_DIAGRAMS {
        tracing::warn!(
            total = connected.len(),
            cap = MAX_RESOURCE_DIAGRAMS,
            "too many connected resources to diagram individually; \
             the remainder are reported without a diagram"
        );
    }

    let mut assets = Vec::new();
    // `resources` is already sorted deterministically by the store, so the
    // slugs — and any golden output — are stable.
    for (index, resource) in connected.iter().take(MAX_RESOURCE_DIAGRAMS).enumerate() {
        let graph = EstateGraph::neighbourhood(resource, &by_id, &edges, labels);
        // Counted on the resources, not the nodes: the graph also carries the
        // group frame they are drawn in.
        if graph
            .nodes
            .iter()
            .filter(|node| !node.kind.is_container())
            .count()
            < 2
        {
            continue;
        }
        assets.push(DiagramAsset {
            // Indexed rather than named: resource names are not unique across
            // groups and subscriptions, but this must be a unique file stem.
            slug: format!("resource-{index}-{}", super::graph::slugify(&resource.name)),
            title: resource.name.clone(),
            kind: DiagramAssetKind::Resource,
            svg: svg::render(&graph, labels),
            resource_id: Some(resource.id.clone()),
            group_key: None,
        });
    }
    Ok(assets)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels() -> DiagramLabels {
        crate::labels::Labels::default().diagram
    }
    use crate::model::{Edge, EdgeKind, Resource, normalize_arm_id};
    use std::collections::HashMap;

    fn resource(name: &str, azure_type: &str) -> Resource {
        let id = format!("/subscriptions/s1/resourceGroups/rg/providers/{azure_type}/{name}");
        Resource {
            id: normalize_arm_id(&id),
            display_id: id,
            name: name.to_owned(),
            azure_type: azure_type.to_owned(),
            kind: None,
            location: Some("uksouth".to_owned()),
            resource_group: Some("rg".to_owned()),
            subscription_id: "s1".to_owned(),
            tags: None,
            sku: None,
            identity: None,
            properties: None,
        }
    }

    fn attached(source: &Resource, target: &Resource) -> Edge {
        Edge {
            source_id: source.id.clone(),
            target_id: target.id.clone(),
            kind: EdgeKind::AttachedTo,
            properties: None,
        }
    }

    #[test]
    fn unit_neighbourhood_includes_only_directly_connected_resources() {
        let vm = resource("vm-01", "microsoft.compute/virtualmachines");
        let nic = resource("vm-01-nic", "microsoft.network/networkinterfaces");
        let pip = resource("vm-01-pip", "microsoft.network/publicipaddresses");
        let unrelated = resource("st1", "microsoft.storage/storageaccounts");
        let all = [&vm, &nic, &pip, &unrelated];
        let by_id: HashMap<&str, &Resource> = all.iter().map(|r| (r.id.as_str(), *r)).collect();
        // pip hangs off the nic, so it is two hops from the VM.
        let edges = vec![attached(&nic, &vm), attached(&pip, &nic)];

        let graph = EstateGraph::neighbourhood(&vm, &by_id, &edges, &labels());

        let labels: Vec<&str> = graph
            .nodes
            .iter()
            .filter(|node| !node.kind.is_container())
            .map(|node| node.label.as_str())
            .collect();
        assert_eq!(labels, vec!["vm-01", "vm-01-nic"]);
        assert_eq!(graph.edges.len(), 1);
    }

    #[test]
    fn unit_neighbourhood_has_no_title_because_the_report_supplies_one() {
        let vm = resource("vm-01", "microsoft.compute/virtualmachines");
        let by_id: HashMap<&str, &Resource> = HashMap::from([(vm.id.as_str(), &vm)]);

        let graph = EstateGraph::neighbourhood(&vm, &by_id, &[], &labels());

        assert!(graph.title.is_empty());
    }

    /// build_resource_diagrams drops any graph with fewer than two resources;
    /// this is the shape that triggers it.
    #[test]
    fn unit_neighbourhood_of_an_unconnected_resource_is_a_lone_node() {
        let lonely = resource("st1", "microsoft.storage/storageaccounts");
        let by_id: HashMap<&str, &Resource> = HashMap::from([(lonely.id.as_str(), &lonely)]);

        let graph = EstateGraph::neighbourhood(&lonely, &by_id, &[], &labels());

        assert_eq!(
            graph
                .nodes
                .iter()
                .filter(|node| !node.kind.is_container())
                .count(),
            1
        );
    }

    /// The frame is what makes a relationships diagram look like the rest of
    /// the document rather than icons dropped on the page.
    #[test]
    fn unit_neighbourhood_is_drawn_inside_its_resource_group_frame() {
        let vm = resource("vm-01", "microsoft.compute/virtualmachines");
        let by_id: HashMap<&str, &Resource> = HashMap::from([(vm.id.as_str(), &vm)]);

        let graph = EstateGraph::neighbourhood(&vm, &by_id, &[], &labels());

        assert_eq!(
            graph.nodes[0].kind,
            crate::diagram::graph::NodeKind::ResourceGroup
        );
        assert_eq!(graph.nodes[0].label, "rg");
        assert!(graph.nodes[1..].iter().all(|node| node.parent == Some(0)));
    }
}
