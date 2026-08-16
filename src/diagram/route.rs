//! Orthogonal connector routing shared by every emitter.
//!
//! Edges used to be drawn straight from node centre to node centre. For a
//! container that centre sits *inside* the box, so a VNet peering erupted from
//! the middle of a subnet and crossed the icons on its way out. Routing here
//! leaves from a boundary anchor and turns only at right angles, which is what
//! draw.io already does for its own edges (`edgeStyle=orthogonalEdgeStyle`) —
//! this brings the SVG, and therefore the PDF/DOCX/HTML reports, into line.
//!
//! Everything is closed-form: no obstacle search, because routing is paid once
//! per edge per diagram and a report renders hundreds of diagrams. A connector
//! may still cross an unrelated box; that is accepted in exchange for staying
//! fast and deterministic.

use std::collections::BTreeMap;

use super::graph::EstateGraph;
use super::layout::Placement;
use super::page::Rung;

/// Distance a connector runs perpendicular to the boundary before it may turn,
/// so it never appears to start on the border line itself.
const ESCAPE: f64 = 20.0;
/// Mid-segment coordinates snap to this grid, so parallel trunks stack into a
/// bus instead of scattering.
const LANE: f64 = 8.0;
/// Guard against a cycle in the parent chain; real graphs nest three deep.
const MAX_DEPTH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
}

impl Side {
    fn opposite(self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Top => Self::Bottom,
            Self::Bottom => Self::Top,
        }
    }

    fn is_horizontal(self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }
}

/// One routed edge. `points` always has at least two entries and is in draw
/// order from source to target.
#[derive(Debug, Clone)]
pub struct EdgeRoute {
    pub points: Vec<(f64, f64)>,
    /// Set when the edge carries a label: where to put it, already offset off
    /// the line.
    pub label_at: Option<(f64, f64)>,
    /// Index into `graph.edges`; edges skipped by containment are absent.
    pub edge: usize,
}

/// The one anchor rectangle an edge attaches to.
///
/// A resource leaf's slot is mostly label whitespace, so edges anchored to the
/// slot met empty space below the glyph. This returns the glyph itself.
pub fn visual_box(placement: &Placement, is_leaf: bool, rung: &Rung) -> Placement {
    if !is_leaf {
        return *placement;
    }
    Placement {
        x: placement.x + ((placement.width - rung.icon) / 2.0).round(),
        y: placement.y + 4.0,
        width: rung.icon,
        height: rung.icon,
    }
}

fn centre(placement: &Placement) -> (f64, f64) {
    (
        placement.x + placement.width / 2.0,
        placement.y + placement.height / 2.0,
    )
}

/// True when `ancestor` encloses `node` through the parent chain.
fn encloses(graph: &EstateGraph, ancestor: usize, node: usize) -> bool {
    let mut current = graph.nodes[node].parent;
    for _ in 0..MAX_DEPTH {
        match current {
            Some(index) if index == ancestor => return true,
            Some(index) => current = graph.nodes[index].parent,
            None => return false,
        }
    }
    false
}

/// Point on `side` of `box`, at fraction `f` along that side.
fn anchor_point(rect: &Placement, side: Side, fraction: f64) -> (f64, f64) {
    match side {
        Side::Right => (
            rect.x + rect.width,
            (rect.y + rect.height * fraction).round(),
        ),
        Side::Left => (rect.x, (rect.y + rect.height * fraction).round()),
        Side::Bottom => (
            (rect.x + rect.width * fraction).round(),
            rect.y + rect.height,
        ),
        Side::Top => ((rect.x + rect.width * fraction).round(), rect.y),
    }
}

/// Side of the source facing the target. Total, with every tie resolving the
/// same way so the result never wobbles on a float comparison.
fn side_toward(from: (f64, f64), to: (f64, f64)) -> Side {
    let (dx, dy) = (to.0 - from.0, to.1 - from.1);
    if dx.abs() >= dy.abs() {
        if dx > 0.0 { Side::Right } else { Side::Left }
    } else if dy > 0.0 {
        Side::Bottom
    } else {
        Side::Top
    }
}

/// Route every edge in `graph`. One entry per drawn edge, in edge order;
/// containment edges are omitted because the nesting already shows them.
pub fn route(graph: &EstateGraph, absolute: &[Placement], rung: &Rung) -> Vec<EdgeRoute> {
    let rects: Vec<Placement> = absolute
        .iter()
        .enumerate()
        .map(|(index, placement)| {
            let is_leaf = !graph.nodes[index].kind.is_container();
            visual_box(placement, is_leaf, rung)
        })
        .collect();

    // Which edges are drawn at all, and which side each end leaves from.
    let mut drawn: Vec<(usize, Side, Side)> = Vec::new();
    for (index, edge) in graph.edges.iter().enumerate() {
        if edge.source == edge.target
            || encloses(graph, edge.source, edge.target)
            || encloses(graph, edge.target, edge.source)
        {
            continue;
        }
        let source_side = side_toward(centre(&rects[edge.source]), centre(&rects[edge.target]));
        drawn.push((index, source_side, source_side.opposite()));
    }

    // Spread the anchors: several edges meeting one box fan out along its side
    // instead of piling onto the midpoint. BTreeMap and index tie-breaks keep
    // this deterministic, which the golden tests depend on.
    let mut buckets: BTreeMap<(usize, Side), Vec<usize>> = BTreeMap::new();
    for (position, &(index, source_side, target_side)) in drawn.iter().enumerate() {
        buckets
            .entry((graph.edges[index].source, source_side))
            .or_default()
            .push(position);
        buckets
            .entry((graph.edges[index].target, target_side))
            .or_default()
            .push(position);
    }
    let mut fractions: BTreeMap<(usize, usize), f64> = BTreeMap::new();
    for ((node, side), mut members) in buckets {
        members.sort_by(|&a, &b| {
            let key = |position: usize| {
                let edge = &graph.edges[drawn[position].0];
                let other = if edge.source == node {
                    edge.target
                } else {
                    edge.source
                };
                let point = centre(&rects[other]);
                if side.is_horizontal() {
                    point.1
                } else {
                    point.0
                }
            };
            key(a).total_cmp(&key(b)).then(a.cmp(&b))
        });
        let total = members.len();
        for (rank, position) in members.into_iter().enumerate() {
            fractions.insert((position, node), (rank + 1) as f64 / (total + 1) as f64);
        }
    }

    let mut routes: Vec<EdgeRoute> = drawn
        .iter()
        .enumerate()
        .map(|(position, &(index, source_side, target_side))| {
            let edge = &graph.edges[index];
            let start = anchor_point(
                &rects[edge.source],
                source_side,
                fractions
                    .get(&(position, edge.source))
                    .copied()
                    .unwrap_or(0.5),
            );
            let end = anchor_point(
                &rects[edge.target],
                target_side,
                fractions
                    .get(&(position, edge.target))
                    .copied()
                    .unwrap_or(0.5),
            );
            EdgeRoute {
                points: shape(start, end, source_side),
                label_at: None,
                edge: index,
            }
        })
        .collect();

    deconflict(&mut routes);
    for route in &mut routes {
        route.label_at = Some(label_anchor(&route.points));
    }
    routes
}

/// Two elbows in a channel between the anchors, or a U when they face away
/// from each other.
fn shape(start: (f64, f64), end: (f64, f64), side: Side) -> Vec<(f64, f64)> {
    let points = if side.is_horizontal() {
        let forward = if side == Side::Right {
            end.0 - start.0
        } else {
            start.0 - end.0
        };
        if forward >= 2.0 * ESCAPE {
            let mid = ((start.0 + end.0) / 2.0).round();
            vec![start, (mid, start.1), (mid, end.1), end]
        } else {
            // Anchors face away: escape past the further edge and come back.
            let mid = if side == Side::Right {
                start.0.max(end.0) + ESCAPE
            } else {
                start.0.min(end.0) - ESCAPE
            };
            vec![start, (mid, start.1), (mid, end.1), end]
        }
    } else {
        let forward = if side == Side::Bottom {
            end.1 - start.1
        } else {
            start.1 - end.1
        };
        if forward >= 2.0 * ESCAPE {
            let mid = ((start.1 + end.1) / 2.0).round();
            vec![start, (start.0, mid), (end.0, mid), end]
        } else {
            let mid = if side == Side::Bottom {
                start.1.max(end.1) + ESCAPE
            } else {
                start.1.min(end.1) - ESCAPE
            };
            vec![start, (start.0, mid), (end.0, mid), end]
        }
    };
    dedupe(points)
}

/// Snap mid-segments to a lane grid and fan out any that would coincide, so
/// parallel trunks read as a bus rather than a smear.
fn deconflict(routes: &mut [EdgeRoute]) {
    let mut lanes: BTreeMap<(bool, i64), Vec<usize>> = BTreeMap::new();
    for (index, route) in routes.iter_mut().enumerate() {
        if route.points.len() != 4 {
            continue;
        }
        let vertical_channel = (route.points[1].0 - route.points[2].0).abs() < 0.01;
        let value = if vertical_channel {
            route.points[1].0
        } else {
            route.points[1].1
        };
        let snapped = (value / LANE).round() * LANE;
        if vertical_channel {
            route.points[1].0 = snapped;
            route.points[2].0 = snapped;
        } else {
            route.points[1].1 = snapped;
            route.points[2].1 = snapped;
        }
        lanes
            .entry((vertical_channel, snapped as i64))
            .or_default()
            .push(index);
    }
    for ((vertical_channel, _), members) in lanes {
        let total = members.len();
        if total < 2 {
            continue;
        }
        for (rank, index) in members.into_iter().enumerate() {
            let shift = (rank as f64 - (total as f64 - 1.0) / 2.0) * LANE;
            let route = &mut routes[index];
            if vertical_channel {
                route.points[1].0 += shift;
                route.points[2].0 += shift;
            } else {
                route.points[1].1 += shift;
                route.points[2].1 += shift;
            }
        }
    }
}

/// Drop points that repeat or sit mid-way along a straight run, so the emitted
/// path has no zero-length or collinear segments.
fn dedupe(points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    const EPSILON: f64 = 0.01;
    let mut out: Vec<(f64, f64)> = Vec::with_capacity(points.len());
    for point in points {
        if out.last().is_some_and(|last: &(f64, f64)| {
            (last.0 - point.0).abs() < EPSILON && (last.1 - point.1).abs() < EPSILON
        }) {
            continue;
        }
        if out.len() >= 2 {
            let previous = out[out.len() - 2];
            let last = out[out.len() - 1];
            let straight_x =
                (previous.0 - last.0).abs() < EPSILON && (last.0 - point.0).abs() < EPSILON;
            let straight_y =
                (previous.1 - last.1).abs() < EPSILON && (last.1 - point.1).abs() < EPSILON;
            if straight_x || straight_y {
                out.pop();
            }
        }
        out.push(point);
    }
    if out.len() < 2 {
        out.push(*out.last().unwrap_or(&(0.0, 0.0)));
    }
    out
}

/// Midpoint of the longest segment, lifted clear of the line. Labels used to
/// sit at the straight-line midpoint, which for a routed edge is frequently
/// nowhere near the connector.
fn label_anchor(points: &[(f64, f64)]) -> (f64, f64) {
    points
        .windows(2)
        .enumerate()
        .map(|(index, pair)| {
            let length = (pair[1].0 - pair[0].0).abs() + (pair[1].1 - pair[0].1).abs();
            (
                length,
                std::cmp::Reverse(index),
                (
                    (pair[0].0 + pair[1].0) / 2.0,
                    (pair[0].1 + pair[1].1) / 2.0 - 6.0,
                ),
            )
        })
        .max_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)))
        .map(|(_, _, point)| point)
        .unwrap_or_default()
}

/// Render `points` as an SVG path with rounded corners. `radius` of 0 gives
/// the square corners of a classic reference architecture.
pub fn svg_path(points: &[(f64, f64)], radius: f64) -> String {
    use std::fmt::Write as _;

    let mut out = String::new();
    let _ = write!(out, "M {:.1} {:.1}", points[0].0, points[0].1);
    if radius <= 0.0 || points.len() < 3 {
        for point in &points[1..] {
            let _ = write!(out, " L {:.1} {:.1}", point.0, point.1);
        }
        return out;
    }
    for index in 1..points.len() - 1 {
        let (previous, corner, next) = (points[index - 1], points[index], points[index + 1]);
        let incoming = (corner.0 - previous.0).abs() + (corner.1 - previous.1).abs();
        let outgoing = (next.0 - corner.0).abs() + (next.1 - corner.1).abs();
        let r = radius.min(incoming / 2.0).min(outgoing / 2.0).floor();
        let toward = |from: (f64, f64), to: (f64, f64), distance: f64| {
            let length = (to.0 - from.0).abs() + (to.1 - from.1).abs();
            if length <= 0.0 {
                return from;
            }
            (
                from.0 + (to.0 - from.0) / length * distance,
                from.1 + (to.1 - from.1) / length * distance,
            )
        };
        let entry = toward(corner, previous, r);
        let exit = toward(corner, next, r);
        let _ = write!(out, " L {:.1} {:.1}", entry.0, entry.1);
        let _ = write!(
            out,
            " Q {:.1} {:.1} {:.1} {:.1}",
            corner.0, corner.1, exit.0, exit.1
        );
    }
    let last = points[points.len() - 1];
    let _ = write!(out, " L {:.1} {:.1}", last.0, last.1);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::graph::{DiagEdge, EdgeStyle, Node, NodeKind};
    use crate::diagram::page::COMFORTABLE;

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

    fn container(parent: Option<usize>) -> Node {
        Node {
            label: "c".into(),
            sublabel: None,
            kind: NodeKind::Vnet,
            parent,
        }
    }

    fn edge(source: usize, target: usize) -> DiagEdge {
        DiagEdge {
            source,
            target,
            label: None,
            style: EdgeStyle::Solid,
        }
    }

    fn box_at(x: f64, y: f64, width: f64, height: f64) -> Placement {
        Placement {
            x,
            y,
            width,
            height,
        }
    }

    fn assert_orthogonal(points: &[(f64, f64)]) {
        for pair in points.windows(2) {
            let horizontal = (pair[0].1 - pair[1].1).abs() < 0.01;
            let vertical = (pair[0].0 - pair[1].0).abs() < 0.01;
            assert!(
                horizontal || vertical,
                "diagonal segment {:?} -> {:?} in {points:?}",
                pair[0],
                pair[1]
            );
        }
    }

    #[test]
    fn unit_every_segment_is_axis_aligned_when_boxes_are_diagonally_apart() {
        let graph = EstateGraph {
            title: String::new(),
            nodes: vec![container(None), container(None)],
            edges: vec![edge(0, 1)],
        };
        let absolute = vec![
            box_at(0.0, 0.0, 100.0, 100.0),
            box_at(400.0, 300.0, 100.0, 100.0),
        ];

        let routes = route(&graph, &absolute, &COMFORTABLE);

        assert_eq!(routes.len(), 1);
        assert_orthogonal(&routes[0].points);
    }

    #[test]
    fn unit_route_starts_on_the_boundary_when_source_is_a_wide_container() {
        let graph = EstateGraph {
            title: String::new(),
            nodes: vec![container(None), container(None)],
            edges: vec![edge(0, 1)],
        };
        let absolute = vec![
            box_at(0.0, 0.0, 600.0, 200.0),
            box_at(900.0, 60.0, 100.0, 100.0),
        ];

        let routes = route(&graph, &absolute, &COMFORTABLE);

        assert_eq!(
            routes[0].points[0].0, 600.0,
            "leaves the east edge, not the centre"
        );
    }

    /// The pathology the redesign exists to remove: an edge from a container to
    /// something it already contains was drawn straight across its own children.
    #[test]
    fn unit_containment_edges_are_not_drawn() {
        let graph = EstateGraph {
            title: String::new(),
            nodes: vec![container(None), leaf(Some(0))],
            edges: vec![edge(0, 1)],
        };
        let absolute = vec![
            box_at(0.0, 0.0, 400.0, 400.0),
            box_at(100.0, 100.0, 100.0, 100.0),
        ];

        assert!(route(&graph, &absolute, &COMFORTABLE).is_empty());
    }

    #[test]
    fn unit_edges_sharing_a_side_fan_out_to_distinct_anchors() {
        let graph = EstateGraph {
            title: String::new(),
            nodes: vec![container(None), container(None), container(None)],
            edges: vec![edge(0, 1), edge(0, 2)],
        };
        let absolute = vec![
            box_at(0.0, 0.0, 100.0, 300.0),
            box_at(400.0, 0.0, 100.0, 100.0),
            box_at(400.0, 200.0, 100.0, 100.0),
        ];

        let routes = route(&graph, &absolute, &COMFORTABLE);

        assert_ne!(
            routes[0].points[0], routes[1].points[0],
            "both edges left from the same point"
        );
    }

    #[test]
    fn unit_routes_are_deterministic_when_computed_repeatedly() {
        let graph = EstateGraph {
            title: String::new(),
            nodes: vec![container(None), container(None), container(None)],
            edges: vec![edge(0, 1), edge(0, 2), edge(1, 2)],
        };
        let absolute = vec![
            box_at(0.0, 0.0, 120.0, 200.0),
            box_at(500.0, 40.0, 120.0, 100.0),
            box_at(260.0, 400.0, 120.0, 100.0),
        ];

        let first = route(&graph, &absolute, &COMFORTABLE);
        let second = route(&graph, &absolute, &COMFORTABLE);

        assert_eq!(
            first.iter().map(|r| r.points.clone()).collect::<Vec<_>>(),
            second.iter().map(|r| r.points.clone()).collect::<Vec<_>>()
        );
    }

    #[test]
    fn unit_visual_box_anchors_a_leaf_to_its_icon_not_its_label_slot() {
        let slot = box_at(0.0, 0.0, 152.0, 112.0);

        let anchored = visual_box(&slot, true, &COMFORTABLE);

        assert_eq!(anchored.width, COMFORTABLE.icon);
        assert_eq!(anchored.height, COMFORTABLE.icon);
        assert!(anchored.height < slot.height, "icon is not the whole slot");
    }

    #[test]
    fn unit_svg_path_is_a_plain_polyline_when_radius_is_zero() {
        let path = svg_path(&[(0.0, 0.0), (0.0, 10.0), (20.0, 10.0)], 0.0);

        assert!(!path.contains('Q'), "square corners requested: {path}");
    }

    #[test]
    fn unit_svg_path_rounds_interior_corners_when_radius_is_set() {
        let path = svg_path(&[(0.0, 0.0), (0.0, 40.0), (60.0, 40.0)], 8.0);

        assert!(path.contains('Q'), "no curve emitted: {path}");
    }

    #[test]
    fn unit_label_anchor_sits_on_the_longest_segment() {
        let anchor = label_anchor(&[(0.0, 0.0), (0.0, 10.0), (200.0, 10.0), (200.0, 20.0)]);

        assert_eq!(anchor.0, 100.0, "not centred on the long run");
    }
}
