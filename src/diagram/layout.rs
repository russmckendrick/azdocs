//! Bottom-up container sizing and grid placement. Coordinates are relative to
//! the parent container, matching draw.io child geometry.

use super::graph::EstateGraph;

pub const LEAF_WIDTH: f64 = 150.0;
pub const LEAF_HEIGHT: f64 = 100.0;
const PADDING: f64 = 24.0;
const TITLE_BAND: f64 = 30.0;
const GAP: f64 = 30.0;

/// Placement for one node, `(x, y)` relative to its parent.
#[derive(Debug, Clone, Copy)]
pub struct Placement {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Compute placements for every node. Children are arranged in a wide grid;
/// containers grow to fit; roots are stacked in a grid at the top level.
pub fn layout(graph: &EstateGraph) -> Vec<Placement> {
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
            width: LEAF_WIDTH,
            height: LEAF_HEIGHT,
        };
        graph.nodes.len()
    ];
    for &root in &roots {
        size_node(graph, &children, root, &mut placements);
    }
    // Roots form a virtual grid so multiple top-level containers don't overlap.
    place_grid(&roots, &mut placements, 0.0);
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

/// Post-order: size children first, then this container around them.
fn size_node(
    graph: &EstateGraph,
    children: &[Vec<usize>],
    index: usize,
    placements: &mut [Placement],
) {
    for &child in &children[index] {
        size_node(graph, children, child, placements);
    }
    if !graph.nodes[index].kind.is_container() || children[index].is_empty() {
        if graph.nodes[index].kind.is_container() {
            placements[index].width = 200.0;
            placements[index].height = 60.0;
        }
        return;
    }
    let (width, height) = place_grid(&children[index], placements, TITLE_BAND);
    placements[index].width = width;
    placements[index].height = height;
}

/// Arrange `nodes` into a wide grid; returns the enclosing (width, height)
/// including padding and the title band offset.
fn place_grid(nodes: &[usize], placements: &mut [Placement], top_offset: f64) -> (f64, f64) {
    let count = nodes.len();
    if count == 0 {
        return (200.0, 60.0);
    }
    let columns = ((count as f64 * 1.6).sqrt().ceil() as usize).max(1);
    let mut x = PADDING;
    let mut y = top_offset + PADDING;
    let mut row_height: f64 = 0.0;
    let mut max_width: f64 = 0.0;
    for (position, &node) in nodes.iter().enumerate() {
        if position > 0 && position % columns == 0 {
            x = PADDING;
            y += row_height + GAP;
            row_height = 0.0;
        }
        placements[node].x = x;
        placements[node].y = y;
        x += placements[node].width + GAP;
        row_height = row_height.max(placements[node].height);
        max_width = max_width.max(x);
    }
    (max_width - GAP + PADDING, y + row_height + PADDING)
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

    #[test]
    fn container_grows_to_fit_children() {
        let graph = EstateGraph {
            title: String::new(),
            nodes: vec![
                Node {
                    label: "rg".into(),
                    sublabel: None,
                    kind: NodeKind::ResourceGroup,
                    parent: None,
                },
                leaf(Some(0)),
                leaf(Some(0)),
            ],
            edges: vec![],
        };

        let placements = layout(&graph);

        assert!(
            placements[0].width > 2.0 * LEAF_WIDTH && placements[0].height > LEAF_HEIGHT,
            "container: {:?}",
            placements[0]
        );
    }

    #[test]
    fn absolutize_offsets_children_by_ancestor_positions() {
        let graph = EstateGraph {
            title: String::new(),
            nodes: vec![
                Node {
                    label: "rg".into(),
                    sublabel: None,
                    kind: NodeKind::ResourceGroup,
                    parent: None,
                },
                Node {
                    label: "vnet".into(),
                    sublabel: None,
                    kind: NodeKind::Vnet,
                    parent: Some(0),
                },
                leaf(Some(1)),
            ],
            edges: vec![],
        };
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
        let graph = EstateGraph {
            title: String::new(),
            nodes: vec![
                Node {
                    label: "rg".into(),
                    sublabel: None,
                    kind: NodeKind::ResourceGroup,
                    parent: None,
                },
                leaf(Some(0)),
                leaf(Some(0)),
            ],
            edges: vec![],
        };

        let placements = layout(&graph);

        let a = placements[1];
        let b = placements[2];
        let overlap_x = a.x < b.x + b.width && b.x < a.x + a.width;
        let overlap_y = a.y < b.y + b.height && b.y < a.y + a.height;
        assert!(!(overlap_x && overlap_y), "a={a:?} b={b:?}");
    }
}
