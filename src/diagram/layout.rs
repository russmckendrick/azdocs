//! Bottom-up container sizing and grid placement. Coordinates are relative to
//! the parent container, matching draw.io child geometry.
//!
//! The grid shape comes from [`super::page`], which knows the target sheet, so
//! layout, the SVG emitter and the report formats all agree on how much fits.

use super::graph::{EstateGraph, LayoutMode, Node, NodeKind};
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
    if graph.layout == LayoutMode::Relational {
        return hub_and_spoke(graph, &rung, detail.canvas_width());
    }
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

/// Lay a peering graph out around its best-connected VNet.
///
/// Azure peering is nearly always hub and spoke, and this draws it the way the
/// reference architectures do: the hub in a centre column with its spokes
/// stacked in two flanking columns, so every hub connector is a short
/// horizontal run and none of them crosses a box.
///
/// It has to degrade, because not every estate is hub and spoke. With no
/// clear hub — a full mesh, where every VNet has the same degree — the
/// best-connected node is chosen by name so the picture is stable, and the
/// mesh's remaining links become chords routed around the columns. VNets with
/// no peering at all are not part of the topology at all, so they go in a grid
/// below it rather than being threaded into a shape they play no part in.
fn hub_and_spoke(graph: &EstateGraph, rung: &Rung, canvas_width: f64) -> Vec<Placement> {
    let width = rung.pill_width;
    let height = rung.pill_height;
    let gutter = rung.gutter;

    let mut degree = vec![0usize; graph.nodes.len()];
    for edge in &graph.edges {
        degree[edge.source] += 1;
        degree[edge.target] += 1;
    }

    let peered: Vec<usize> = (0..graph.nodes.len()).filter(|&i| degree[i] > 0).collect();
    // Highest degree wins; ties go to the lowest node index, which `peerings`
    // has already ordered by name. Determinism is not a nicety here — the
    // golden files and `$skipToken` pagination both rest on it.
    let busiest = peered
        .iter()
        .copied()
        .max_by_key(|&i| (degree[i], std::cmp::Reverse(i)));

    // A hub only earns the centre if the spokes really are spokes. What makes
    // hub and spoke that shape is that spokes talk to the hub and not to each
    // other, so a single spoke-to-spoke peering means this is a mesh wearing a
    // hub's clothes — and a mesh drawn as one centre and two flanks is a row,
    // which is the shape that made every connector cross a box.
    let hub = busiest.filter(|&candidate| {
        graph
            .edges
            .iter()
            .all(|e| e.source == candidate || e.target == candidate)
    });

    if hub.is_none() && peered.len() > 2 {
        return ring(graph, &peered, rung, canvas_width);
    }

    let spokes: Vec<usize> = peered.iter().copied().filter(|&i| Some(i) != hub).collect();

    let mut placements = vec![
        Placement {
            x: 0.0,
            y: 0.0,
            width,
            height,
        };
        graph.nodes.len()
    ];

    // Three columns: spokes left and right, hub between them. The lane is wide
    // enough for a connector to turn in without grazing either box.
    let lane = (gutter * 3.0).max(72.0);
    let block = width * 3.0 + lane * 2.0;
    let left = ((canvas_width - block) / 2.0).max(0.0);
    let centre = left + width + lane;
    let right = centre + width + lane;

    let rows = spokes.len().div_ceil(2);
    let step = height + gutter;
    for (rank, &node) in spokes.iter().enumerate() {
        placements[node].x = if rank % 2 == 0 { left } else { right };
        placements[node].y = (rank / 2) as f64 * step;
    }
    if let Some(hub) = hub {
        placements[hub].x = centre;
        // Centred against the spoke stack, so the connectors fan symmetrically.
        placements[hub].y = ((rows.max(1) as f64 * step) - step) / 2.0;
    }

    // Anything with no peering sits below the topology, not inside it.
    let below = if hub.is_some() {
        rows.max(1) as f64 * step + gutter * 2.0
    } else {
        0.0
    };
    shelf(graph, &mut placements, below, rung, canvas_width);
    placements
}

/// Lay a mesh out as a polygon: three peered VNets make a triangle, four a
/// square, and so on around an ellipse.
///
/// A mesh has no centre to put anything in. Drawn as a row — or as a centre
/// with flanks, which for three nodes is the same thing — every link between
/// non-adjacent boxes has to cross the boxes between them. On a ring every
/// node is on the hull, so each link is a chord with clear space either side.
///
/// The radius comes from the chord between neighbours: it has to be at least a
/// box wide, or the ring overlaps itself. Width is capped to the canvas, so a
/// large mesh stretches into an ellipse and grows downward rather than off the
/// sheet.
fn ring(graph: &EstateGraph, peered: &[usize], rung: &Rung, canvas_width: f64) -> Vec<Placement> {
    let (width, height) = (rung.pill_width, rung.pill_height);
    let count = peered.len() as f64;
    let spread = (std::f64::consts::PI / count).sin().max(0.05);
    // Boxes are wide and short, so one radius for both axes over-spaces the
    // ring vertically while leaving side-by-side neighbours almost touching.
    // Each axis is sized from the dimension it actually has to clear.
    let mut rx = ((width + rung.gutter) / (2.0 * spread)).max(width / 2.0);
    let mut ry = ((height + rung.gutter) / (2.0 * spread)).max(height / 2.0);
    let ceiling = ((canvas_width - width) / 2.0).max(width / 2.0);
    rx = rx.min(ceiling);

    let mut placements = vec![
        Placement {
            x: 0.0,
            y: 0.0,
            width,
            height,
        };
        graph.nodes.len()
    ];
    // First node at twelve o'clock, then clockwise, so the same graph always
    // draws the same way round.
    let place = |rx: f64, ry: f64, placements: &mut [Placement]| {
        for (rank, &node) in peered.iter().enumerate() {
            let angle =
                -std::f64::consts::FRAC_PI_2 + (rank as f64) * std::f64::consts::TAU / count;
            placements[node].x = (rx + rx * angle.cos()).round();
            placements[node].y = (ry + ry * angle.sin()).round();
        }
    };
    place(rx, ry, &mut placements);
    // Trigonometry sizes the gap between *neighbours*; on a squashed ellipse
    // it is a box two places round that collides. Rather than solve for that,
    // grow the ellipse until the drawing is clear — a handful of steps, and
    // the result is checkable rather than merely argued.
    for _ in 0..24 {
        if !overlaps(peered, &placements, rung.gutter) {
            break;
        }
        rx = (rx * 1.12).min(ceiling);
        ry *= 1.12;
        place(rx, ry, &mut placements);
    }
    let below = 2.0 * ry + height + rung.gutter * 2.0;
    shelf(graph, &mut placements, below, rung, canvas_width);
    placements
}

/// Does any pair of ring members sit closer than `clearance`?
fn overlaps(members: &[usize], placements: &[Placement], clearance: f64) -> bool {
    members.iter().enumerate().any(|(i, &a)| {
        members[i + 1..].iter().any(|&b| {
            let (p, q) = (&placements[a], &placements[b]);
            p.x < q.x + q.width + clearance
                && q.x < p.x + p.width + clearance
                && p.y < q.y + q.height + clearance
                && q.y < p.y + p.height + clearance
        })
    })
}

/// Place the "not peered" zone at `top` and grid its members inside it.
///
/// The grid is balanced, not as wide as the canvas allows: a shelf of eight
/// boxes across the full measure is a strip of unreadable text, where two rows
/// of four are the same information at twice the size.
///
/// Members are children of the zone, so their placements are relative to it —
/// the same contract the containment layout uses, which is what lets draw.io
/// parent them and the SVG absolutise them without either knowing which layout
/// produced the graph.
fn shelf(
    graph: &EstateGraph,
    placements: &mut [Placement],
    top: f64,
    rung: &Rung,
    canvas_width: f64,
) {
    let Some(zone) = (0..graph.nodes.len())
        .find(|&i| graph.nodes[i].kind == NodeKind::Zone && graph.nodes[i].parent.is_none())
    else {
        return;
    };
    let members: Vec<usize> = (0..graph.nodes.len())
        .filter(|&i| graph.nodes[i].parent == Some(zone))
        .collect();
    if members.is_empty() {
        return;
    }
    let (width, height) = (rung.pill_width, rung.pill_height);
    let pad = rung.padding;
    let band = rung.title_band;
    let usable = (canvas_width - 2.0 * pad).max(width);
    let fits = (((usable + rung.gutter) / (width + rung.gutter)).floor() as usize).max(1);
    let columns = ((members.len() as f64).sqrt().ceil().max(1.0) as usize).min(fits);
    let rows = members.len().div_ceil(columns);

    for (rank, &node) in members.iter().enumerate() {
        placements[node].width = width;
        placements[node].height = height;
        placements[node].x = pad + (rank % columns) as f64 * (width + rung.gutter);
        placements[node].y = band + pad + (rank / columns) as f64 * (height + rung.gutter);
    }
    placements[zone].x = 0.0;
    placements[zone].y = top;
    placements[zone].width =
        2.0 * pad + columns as f64 * width + (columns.saturating_sub(1)) as f64 * rung.gutter;
    placements[zone].height =
        band + 2.0 * pad + rows as f64 * height + (rows.saturating_sub(1)) as f64 * rung.gutter;
}

/// Height the title band needs to hold this container's label at `width`.
///
/// `Rung::title_band` is a constant sized for one line. A VNet header is its
/// name *and* its CIDR, and in a narrow subnet column that pair wraps — so the
/// second line printed below the band, across the content beneath it.
///
/// Every consumer reads this, not the constant: the layout reserves the space,
/// draw.io declares the same figure as its `startSize`, and the SVG draws
/// inside it. draw.io positions a swimlane's children by their own geometry
/// regardless of `startSize`, so an emitter that grew the band on its own
/// would simply rule the divider through the first row.
pub(crate) fn title_band_for(node: &Node, width: f64, rung: &Rung) -> f64 {
    if !node.kind.is_container() {
        return rung.title_band;
    }
    let font = rung.container_label_px;
    let characters = node.label.chars().count()
        + node
            .sublabel
            .as_deref()
            .map_or(0, |s| s.chars().count() + 2);
    // The inset matches the 12px the emitters leave either side, plus the
    // stencil that shares the band. 0.58em is the average advance of the bold
    // sans a header is set in; erring wide keeps the estimate conservative.
    let usable = (width - 24.0 - (font + 6.0)).max(font * 4.0);
    let per_line = (usable / (font * 0.58)).floor().max(4.0);
    let lines = (characters as f64 / per_line).ceil().max(1.0);
    (lines * (font + 4.0) + 10.0).max(rung.title_band)
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
    placements[index].height = height + title_band_for(&graph.nodes[index], width, &metrics);
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
                title_band_for(&graph.nodes[node], slot, &inner),
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
        // Height decides. On a tie take the *narrowest* packing that still
        // costs the same rows: nine across and one over is the same height as
        // five by two and reads as a strip, and the slack columns bought
        // nothing. Preferring width here is what made everything a row.
        .min_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
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
        // Height decides; on a tie the *narrowest* packing wins. Six across
        // and one over is exactly as tall as four by two, and the two extra
        // columns buy nothing but a strip of tiles with the sheet's width
        // spread between them. This is the tie that made everything a row.
        if height < best.1 - 0.01 || (height < best.1 + 0.01 && columns < best.0) {
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
        title_band_for(&graph.nodes[node], slot, &inner),
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
            layout: LayoutMode::default(),
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

#[cfg(test)]
mod relational_tests {
    use super::*;
    use crate::diagram::graph::{DiagEdge, EdgeStyle, LayoutMode, Node, NodeKind};

    /// `hub` peered to `spoke-0..n`, plus `loose` VNets with no peering.
    fn peering_graph(spokes: usize, loose: usize) -> EstateGraph {
        let mut nodes = vec![Node {
            label: "hub".into(),
            sublabel: None,
            kind: NodeKind::Vnet,
            parent: None,
        }];
        let mut edges = Vec::new();
        for i in 0..spokes {
            nodes.push(Node {
                label: format!("spoke-{i}"),
                sublabel: None,
                kind: NodeKind::Vnet,
                parent: None,
            });
            edges.push(DiagEdge {
                source: 0,
                target: nodes.len() - 1,
                label: None,
                style: EdgeStyle::Dashed,
            });
        }
        // Mirrors `EstateGraph::peerings`: unpeered VNets live in a labelled
        // zone rather than floating under the drawing unexplained.
        if loose > 0 {
            nodes.push(Node {
                label: format!("Not peered  ·  {loose} virtual networks"),
                sublabel: None,
                kind: NodeKind::Zone,
                parent: None,
            });
            let zone = nodes.len() - 1;
            for i in 0..loose {
                nodes.push(Node {
                    label: format!("loose-{i}"),
                    sublabel: None,
                    kind: NodeKind::Vnet,
                    parent: Some(zone),
                });
            }
        }
        EstateGraph {
            title: "peerings".into(),
            nodes,
            edges,
            layout: LayoutMode::Relational,
        }
    }

    /// Every VNet in one row meant a peering had to cross whatever sat between
    /// its endpoints. The hub belongs between its spokes, not beside them.
    #[test]
    fn unit_the_hub_sits_between_its_spokes() {
        let graph = peering_graph(6, 0);
        let placed = layout_for(&graph, DiagramDetail::Full);

        let hub = placed[0].x;
        let lefts = (1..7).filter(|&i| placed[i].x < hub).count();
        let rights = (1..7).filter(|&i| placed[i].x > hub).count();

        assert_eq!((lefts, rights), (3, 3), "spokes did not flank the hub");
        assert!(
            (1..7).all(|i| placed[i].x != hub),
            "a spoke shares the hub's column"
        );
    }

    /// Spokes stack rather than spreading, so the sheet stays page-width
    /// however many of them there are.
    #[test]
    fn unit_spokes_stack_instead_of_widening_the_sheet() {
        let narrow = peering_graph(2, 0);
        let wide = peering_graph(12, 0);
        let extent = |g: &EstateGraph| -> f64 {
            layout_for(g, DiagramDetail::Full)
                .iter()
                .map(|p| p.x + p.width)
                .fold(0.0_f64, f64::max)
        };

        assert_eq!(extent(&narrow), extent(&wide));
    }

    /// A VNet with no peering is not part of the topology, so it must not be
    /// threaded into the shape as though it were — and the zone holding those
    /// has to say what it is, or it reads as part of the drawing that failed
    /// to connect.
    #[test]
    fn unit_an_unpeered_vnet_sits_below_the_topology_in_a_labelled_zone() {
        let graph = peering_graph(4, 3);
        let placed = layout_for(&graph, DiagramDetail::Full);

        let zone = graph
            .nodes
            .iter()
            .position(|n| n.kind == NodeKind::Zone)
            .expect("a zone for the unpeered VNets");
        let lowest_peered = (0..5)
            .map(|i| placed[i].y + placed[i].height)
            .fold(0.0, f64::max);

        assert!(
            placed[zone].y >= lowest_peered,
            "the zone was drawn inside the topology"
        );
        assert!(
            graph.nodes[zone].label.contains("Not peered"),
            "the zone does not say what it holds: {}",
            graph.nodes[zone].label
        );
        // Members are children, so they are placed relative to the zone and
        // must sit inside it.
        for (i, node) in graph.nodes.iter().enumerate() {
            if node.parent == Some(zone) {
                assert!(
                    placed[i].x >= 0.0
                        && placed[i].y >= 0.0
                        && placed[i].x + placed[i].width <= placed[zone].width
                        && placed[i].y + placed[i].height <= placed[zone].height,
                    "member {i} escapes the zone"
                );
            }
        }
    }

    /// A full mesh has no hub at all — every VNet has the same degree. It still
    /// has to draw the same way twice, or the goldens wobble.
    #[test]
    fn unit_a_mesh_without_a_hub_still_places_deterministically() {
        let mut graph = peering_graph(2, 0);
        graph.edges.push(DiagEdge {
            source: 1,
            target: 2,
            label: None,
            style: EdgeStyle::Dashed,
        });

        let once = layout_for(&graph, DiagramDetail::Full);
        let twice = layout_for(&graph, DiagramDetail::Full);

        assert!(
            once.iter()
                .zip(&twice)
                .all(|(a, b)| a.x == b.x && a.y == b.y)
        );
    }

    /// Nine tiles will not fit one row at full width, and every packing from
    /// five to eight columns comes out two rows tall. Taking the widest bought
    /// nothing but a strip with the sheet's width spread between the tiles;
    /// the narrowest is exactly as tall and reads as a grid.
    #[test]
    fn unit_equal_height_packings_take_the_narrowest_grid() {
        let nodes = (0..9)
            .map(|i| Node {
                label: format!("r{i}"),
                sublabel: None,
                kind: NodeKind::Resource {
                    azure_type: "microsoft.compute/virtualmachines".into(),
                },
                parent: None,
            })
            .collect();
        let graph = EstateGraph {
            title: "t".into(),
            nodes,
            edges: Vec::new(),
            layout: LayoutMode::default(),
        };

        let placed = layout_for(&graph, DiagramDetail::Full);
        let rows: std::collections::BTreeSet<i64> = placed.iter().map(|p| p.y as i64).collect();
        let top = *rows.first().expect("a placed row");
        let across = placed.iter().filter(|p| p.y as i64 == top).count();

        assert_eq!(rows.len(), 2, "the narrower packing cost a row");
        assert_eq!(across, 5, "packed {across} across, not the narrowest grid");
    }

    /// Sizing the ring from the neighbour chord alone let a box two places
    /// round collide on a squashed ellipse. Nothing may overlap, at any size.
    #[test]
    fn unit_a_ring_never_overlaps_itself_at_any_size() {
        for members in 3..=12 {
            let nodes = (0..members)
                .map(|i| Node {
                    label: format!("vnet-{i}"),
                    sublabel: None,
                    kind: NodeKind::Vnet,
                    parent: None,
                })
                .collect();
            // A full mesh, so no node can be mistaken for a hub.
            let mut edges = Vec::new();
            for a in 0..members {
                for b in (a + 1)..members {
                    edges.push(DiagEdge {
                        source: a,
                        target: b,
                        label: None,
                        style: EdgeStyle::Dashed,
                    });
                }
            }
            let graph = EstateGraph {
                title: "peerings".into(),
                nodes,
                edges,
                layout: LayoutMode::Relational,
            };

            let placed = layout_for(&graph, DiagramDetail::Full);

            for a in 0..members {
                for b in (a + 1)..members {
                    let (p, q) = (&placed[a], &placed[b]);
                    assert!(
                        p.x >= q.x + q.width
                            || q.x >= p.x + p.width
                            || p.y >= q.y + q.height
                            || q.y >= p.y + p.height,
                        "{members}-node ring overlaps {a} and {b}"
                    );
                }
            }
        }
    }
}
