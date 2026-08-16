//! Bottom-up container sizing and grid placement. Coordinates are relative to
//! the parent container, matching draw.io child geometry.
//!
//! The grid shape comes from [`super::page`], which knows the target sheet, so
//! layout, the SVG emitter and the report formats all agree on how much fits.

use super::graph::EstateGraph;
use super::page::{A4_PORTRAIT, DiagramDetail, Rung, rung_for};

/// Placement for one node, `(x, y)` relative to its parent.
#[derive(Debug, Clone, Copy)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Number of drawable boxes — leaves plus childless containers — which is what
/// the density rung is chosen from.
pub fn box_count(graph: &EstateGraph) -> usize {
    graph
        .nodes
        .iter()
        .enumerate()
        .filter(|(index, node)| {
            !node.kind.is_container() || !graph.nodes.iter().any(|n| n.parent == Some(*index))
        })
        .count()
}

/// Density rung this graph will be drawn at. Emitters call it so their font
/// and icon sizes match the geometry laid out here.
pub fn rung(graph: &EstateGraph) -> Rung {
    rung_for(box_count(graph))
}

/// Compute placements for every node, sized for a single A4 portrait page.
///
/// Two passes. The first measures each subtree at its natural size, which is
/// what the column choice needs; the second justifies every row to the width
/// it was given, so containers span the page instead of huddling in the middle
/// with the sheet empty on either side.
pub fn layout(graph: &EstateGraph) -> Vec<Placement> {
    layout_for(graph, DiagramDetail::default())
}

/// Lay out for a given detail level: a report summary fits the portrait page,
/// a standalone export gets the wider canvas its full content needs.
pub fn layout_for(graph: &EstateGraph, detail: DiagramDetail) -> Vec<Placement> {
    let rung = rung(graph);
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); graph.nodes.len()];
    let mut roots = Vec::new();
    for (index, node) in graph.nodes.iter().enumerate() {
        match node.parent {
            Some(parent) => children[parent].push(index),
            None => roots.push(index),
        }
    }

    let mut placements = vec![
        Placement {
            x: 0.0,
            y: 0.0,
            width: rung.leaf_width,
            height: rung.leaf_height,
        };
        graph.nodes.len()
    ];
    for &root in &roots {
        measure(graph, &children, root, &rung, 0, &mut placements);
    }
    justify(
        graph,
        &children,
        &roots,
        &rung,
        0,
        detail.canvas_width(),
        0.0,
        &mut placements,
    );
    placements
}

/// Convert parent-relative placements (draw.io child geometry) into absolute
/// page coordinates (what SVG needs) by walking each node's parent chain.
pub fn absolutize(graph: &EstateGraph, placements: &[Placement]) -> Vec<Placement> {
    let mut absolute = placements.to_vec();
    for (index, node) in graph.nodes.iter().enumerate() {
        let mut parent = node.parent;
        while let Some(ancestor) = parent {
            absolute[index].x += placements[ancestor].x;
            absolute[index].y += placements[ancestor].y;
            parent = graph.nodes[ancestor].parent;
        }
    }
    absolute
}

/// Pass one, post-order: every subtree at the size its own content wants.
fn measure(
    graph: &EstateGraph,
    children: &[Vec<usize>],
    index: usize,
    rung: &Rung,
    depth: usize,
    placements: &mut [Placement],
) {
    for &child in &children[index] {
        measure(graph, children, child, rung, depth + 1, placements);
    }
    if !graph.nodes[index].kind.is_container() {
        placements[index].width = rung.leaf_width;
        placements[index].height = rung.leaf_height;
        return;
    }
    if children[index].is_empty() {
        // A childless container is a peering stub. It used to get a 200x60
        // slot whose 3.3:1 shape was the largest single contributor to the
        // hierarchy diagram's aspect ratio.
        placements[index].width = rung.pill_width;
        placements[index].height = empty_height(&graph.nodes[index].kind, rung);
        return;
    }
    let metrics = rung.at_depth(depth);
    let columns = best_columns(
        &children[index],
        &metrics,
        A4_PORTRAIT.width,
        depth,
        placements,
    );
    let (width, height) = extent_at(&children[index], columns, &metrics, placements);
    placements[index].width = width;
    placements[index].height = height + metrics.title_band;
}

/// Pass two, pre-order: lay `nodes` out across exactly `available` width and
/// return the height consumed. Every child in a row gets the same slot width,
/// so the row spans the full measure.
#[allow(clippy::too_many_arguments)]
fn justify(
    graph: &EstateGraph,
    children: &[Vec<usize>],
    nodes: &[usize],
    rung: &Rung,
    depth: usize,
    available: f64,
    top_offset: f64,
    placements: &mut [Placement],
) -> f64 {
    let metrics = rung.at_depth(depth);
    if nodes.is_empty() {
        return top_offset + metrics.pill_height;
    }
    // At the top level the roots *are* the content, so there is no enclosing
    // box to pad against — the canvas margin already provides that.
    let pad = if depth == 0 { 0.0 } else { metrics.padding };
    // Chosen with lookahead into how each child re-flows at the candidate
    // width, not from the natural sizes the measure pass recorded.
    let (columns, _) = plan(graph, children, nodes, rung, depth, available, top_offset);
    let slot = ((available - 2.0 * pad + metrics.gutter - columns as f64 * metrics.gutter)
        / columns as f64)
        .max(rung.leaf_width);

    for &node in nodes {
        placements[node].width = slot;
        placements[node].height = if !graph.nodes[node].kind.is_container() {
            rung.leaf_height
        } else if children[node].is_empty() {
            empty_height(&graph.nodes[node].kind, &metrics)
        } else {
            let inner = rung.at_depth(depth + 1);
            justify(
                graph,
                children,
                &children[node],
                rung,
                depth + 1,
                slot,
                inner.title_band,
                placements,
            )
        };
    }

    let mut x = pad;
    let mut y = top_offset + pad;
    let mut row_height: f64 = 0.0;
    for (position, &node) in nodes.iter().enumerate() {
        if position > 0 && position % columns == 0 {
            x = pad;
            y += row_height + metrics.gutter;
            row_height = 0.0;
        }
        placements[node].x = x;
        placements[node].y = y;
        x += slot + metrics.gutter;
        row_height = row_height.max(placements[node].height);
    }
    y + row_height + pad
}

/// Height of a container with nothing in it.
///
/// An empty subnet is a fact worth stating — the address space is allocated —
/// but a full-height box for it reads as a rendering fault, so it collapses to
/// a strip carrying just its name and CIDR. A peering stub keeps a pill,
/// because it stands for a whole VNet living elsewhere.
pub(crate) fn empty_height(kind: &crate::diagram::graph::NodeKind, rung: &Rung) -> f64 {
    if matches!(kind, crate::diagram::graph::NodeKind::Subnet) {
        rung.container_label_px + 18.0
    } else {
        rung.pill_height
    }
}

/// Extent a `columns`-wide grid of these specific children would occupy.
/// Children are sized by the time this runs, so this measures the real boxes
/// rather than assuming uniform leaves — which matters most at the top level,
/// where a VNet subtree sits beside a zone of aggregated tiles.
fn extent_at(nodes: &[usize], columns: usize, rung: &Rung, placements: &[Placement]) -> (f64, f64) {
    let mut width: f64 = 0.0;
    let mut height = rung.padding;
    for row in nodes.chunks(columns.max(1)) {
        let row_width: f64 = row.iter().map(|&n| placements[n].width).sum::<f64>()
            + (row.len().saturating_sub(1)) as f64 * rung.gutter;
        let row_height = row
            .iter()
            .map(|&n| placements[n].height)
            .fold(0.0_f64, f64::max);
        width = width.max(row_width);
        height += row_height + rung.gutter;
    }
    (
        width + 2.0 * rung.padding,
        height - rung.gutter + rung.padding,
    )
}

/// Column count for a row that will afterwards be justified to `available`.
///
/// Because the justify pass stretches every slot, "does the natural width fit"
/// is the wrong question — a single column always fills the measure perfectly
/// and would win every time, which is what left the two zones of a resource
/// group stacked instead of side by side. The question is how narrow a slot
/// each child can still be laid out in, and how many cells are left ragged.
fn best_columns(
    nodes: &[usize],
    rung: &Rung,
    available: f64,
    depth: usize,
    placements: &[Placement],
) -> usize {
    let pad = if depth == 0 { 0.0 } else { rung.padding };
    // Narrowest slot any child could still be laid out in: one tile plus the
    // chrome a container wraps around it.
    let floor = rung.leaf_width + 2.0 * rung.padding;
    (1..=nodes.len())
        .filter(|&columns| {
            let slot = (available - 2.0 * pad + rung.gutter - columns as f64 * rung.gutter)
                / columns as f64;
            slot >= floor
        })
        .map(|columns| {
            // Height is the objective. Optimising for a perfectly filled last
            // row instead made the layout taller to avoid a single empty cell,
            // and it is what kept a resource group's two zones stacked when
            // side by side was half the height.
            let (_, height) = extent_at(nodes, columns, rung, placements);
            (height, columns)
        })
        // On a tie take the wider packing: filling the sheet across beats
        // running down it.
        .min_by(|a, b| a.0.total_cmp(&b.0).then(b.1.cmp(&a.1)))
        .map_or(1, |(_, columns)| columns)
}

/// Height `nodes` would occupy if justified into `available`, without placing
/// anything.
///
/// A parent cannot pick its columns from its children's natural sizes: halving
/// the measure makes each child re-flow into a narrower, taller shape, and the
/// parent only wins by going side-by-side if that reflow still comes out
/// shorter. This is the lookahead that decides it.
fn planned_height(
    graph: &EstateGraph,
    children: &[Vec<usize>],
    nodes: &[usize],
    rung: &Rung,
    depth: usize,
    available: f64,
    top_offset: f64,
) -> f64 {
    let metrics = rung.at_depth(depth);
    if nodes.is_empty() {
        return top_offset + metrics.pill_height;
    }
    let (_, height) = plan(graph, children, nodes, rung, depth, available, top_offset);
    height
}

/// Column count and resulting height for a justified row, chosen together.
fn plan(
    graph: &EstateGraph,
    children: &[Vec<usize>],
    nodes: &[usize],
    rung: &Rung,
    depth: usize,
    available: f64,
    top_offset: f64,
) -> (usize, f64) {
    let metrics = rung.at_depth(depth);
    let pad = if depth == 0 { 0.0 } else { metrics.padding };
    let floor = slot_floor(graph, children, nodes, rung);

    let mut best = (1usize, f64::INFINITY);
    for columns in 1..=nodes.len() {
        let slot = (available - 2.0 * pad + metrics.gutter - columns as f64 * metrics.gutter)
            / columns as f64;
        if slot < floor {
            continue;
        }
        let mut height = top_offset + pad;
        for row in nodes.chunks(columns) {
            let row_height = row
                .iter()
                .map(|&node| child_height(graph, children, node, rung, depth, slot))
                .fold(0.0_f64, f64::max);
            height += row_height + metrics.gutter;
        }
        height = height - metrics.gutter + pad;
        // Strictly less, so an equal-height wider packing wins the tie.
        if height < best.1 - 0.01 || (height < best.1 + 0.01 && columns > best.0) {
            best = (columns, height);
        }
    }
    if best.1.is_finite() {
        best
    } else {
        (1, top_offset + 2.0 * pad + rung.leaf_height)
    }
}

/// Height one child would take in a `slot`-wide cell.
fn child_height(
    graph: &EstateGraph,
    children: &[Vec<usize>],
    node: usize,
    rung: &Rung,
    depth: usize,
    slot: f64,
) -> f64 {
    if !graph.nodes[node].kind.is_container() {
        return rung.leaf_height;
    }
    if children[node].is_empty() {
        return empty_height(&graph.nodes[node].kind, &rung.at_depth(depth));
    }
    let inner = rung.at_depth(depth + 1);
    planned_height(
        graph,
        children,
        &children[node],
        rung,
        depth + 1,
        slot,
        inner.title_band,
    )
}

/// Narrowest slot every child in this row could still be laid out in. Leaves
/// need only their own width; a container also has to fit its chrome around
/// one of them.
fn slot_floor(graph: &EstateGraph, children: &[Vec<usize>], nodes: &[usize], rung: &Rung) -> f64 {
    nodes
        .iter()
        .map(|&node| {
            if graph.nodes[node].kind.is_container() && !children[node].is_empty() {
                rung.leaf_width + 4.0 * rung.padding
            } else {
                rung.leaf_width
            }
        })
        .fold(0.0_f64, f64::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::graph::{Node, NodeKind};

    fn leaf(parent: Option<usize>) -> Node {
        Node {
            label: "r".into(),
            sublabel: None,
            kind: NodeKind::Resource {
                azure_type: "t".into(),
            },
            parent,
        }
    }

    fn container(kind: NodeKind, parent: Option<usize>) -> Node {
        Node {
            label: "c".into(),
            sublabel: None,
            kind,
            parent,
        }
    }

    fn graph_with(nodes: Vec<Node>) -> EstateGraph {
        EstateGraph {
            title: String::new(),
            nodes,
            edges: vec![],
        }
    }

    #[test]
    fn container_grows_to_fit_children() {
        let graph = graph_with(vec![
            container(NodeKind::ResourceGroup, None),
            leaf(Some(0)),
            leaf(Some(0)),
        ]);

        let placements = layout(&graph);

        let rung = rung(&graph);
        assert!(
            placements[0].width > rung.leaf_width && placements[0].height > rung.leaf_height,
            "container: {:?}",
            placements[0]
        );
    }

    #[test]
    fn absolutize_offsets_children_by_ancestor_positions() {
        let graph = graph_with(vec![
            container(NodeKind::ResourceGroup, None),
            container(NodeKind::Vnet, Some(0)),
            leaf(Some(1)),
        ]);
        let placements = vec![
            Placement {
                x: 100.0,
                y: 50.0,
                width: 400.0,
                height: 300.0,
            },
            Placement {
                x: 20.0,
                y: 30.0,
                width: 200.0,
                height: 150.0,
            },
            Placement {
                x: 5.0,
                y: 7.0,
                width: 150.0,
                height: 100.0,
            },
        ];

        let absolute = absolutize(&graph, &placements);

        assert_eq!((absolute[0].x, absolute[0].y), (100.0, 50.0));
        assert_eq!((absolute[1].x, absolute[1].y), (120.0, 80.0));
        assert_eq!((absolute[2].x, absolute[2].y), (125.0, 87.0));
        assert_eq!(
            (absolute[2].width, absolute[2].height),
            (150.0, 100.0),
            "sizes are untouched"
        );
    }

    #[test]
    fn siblings_do_not_overlap() {
        let graph = graph_with(vec![
            container(NodeKind::ResourceGroup, None),
            leaf(Some(0)),
            leaf(Some(0)),
        ]);

        let placements = layout(&graph);

        let a = placements[1];
        let b = placements[2];
        let overlap_x = a.x < b.x + b.width && b.x < a.x + a.width;
        let overlap_y = a.y < b.y + b.height && b.y < a.y + a.height;
        assert!(!(overlap_x && overlap_y), "a={a:?} b={b:?}");
    }

    #[test]
    fn unit_box_count_counts_leaves_and_childless_containers_only() {
        let graph = graph_with(vec![
            container(NodeKind::ResourceGroup, None),
            leaf(Some(0)),
            leaf(Some(0)),
            container(NodeKind::Vnet, None),
        ]);

        // Two leaves plus the childless VNet stub; the populated group is not a box.
        assert_eq!(box_count(&graph), 3);
    }

    /// The redesign's central promise: whatever the estate throws at it, a
    /// diagram fits the page width without downscaling into illegibility.
    #[test]
    fn unit_layout_never_exceeds_the_page_width_for_a_flat_group() {
        for count in [3, 13, 28, 42, 59, 120] {
            let mut nodes = vec![container(NodeKind::ResourceGroup, None)];
            nodes.extend((0..count).map(|_| leaf(Some(0))));
            let graph = graph_with(nodes);

            let placements = layout(&graph);

            assert!(
                placements[0].width <= A4_PORTRAIT.width,
                "{count} resources laid out {}px wide, page is {}px",
                placements[0].width,
                A4_PORTRAIT.width
            );
        }
    }

    /// A peering stub keeps the rung's height but is stretched to its slot,
    /// because every row spans the full measure.
    #[test]
    fn unit_childless_containers_keep_the_pill_height_and_stretch_to_the_slot() {
        let graph = graph_with(vec![
            container(NodeKind::Vnet, None),
            container(NodeKind::Vnet, None),
        ]);

        let placements = layout(&graph);

        let rung = rung(&graph);
        assert_eq!(placements[0].height, rung.pill_height);
        assert_eq!(
            placements[0].width, placements[1].width,
            "siblings share a slot width"
        );
    }

    /// The point of the justify pass: a row spans the width it was given
    /// rather than huddling in the middle of the sheet.
    #[test]
    fn unit_rows_span_the_full_page_width() {
        let graph = graph_with(vec![
            container(NodeKind::ResourceGroup, None),
            leaf(Some(0)),
            leaf(Some(0)),
            leaf(Some(0)),
        ]);

        let placements = layout(&graph);

        assert!(
            (placements[0].width - A4_PORTRAIT.width).abs() < 1.0,
            "root spans {}px of {}px",
            placements[0].width,
            A4_PORTRAIT.width
        );
    }
}
