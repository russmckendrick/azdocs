use std::fmt::Write as _;

use super::graph::{EdgeStyle, EstateGraph, NodeKind, has_aggregates, legend_kinds, node_label};
use super::page::DiagramDetail;
use crate::labels::DiagramLabels;

/// Emitters warn above this: deeply nested Mermaid becomes unreadable.
pub const NODE_WARN_THRESHOLD: usize = 150;

/// Render the graph as a `flowchart LR` with nested subgraphs for containers,
/// at full detail with a title.
pub fn render(graph: &EstateGraph, labels: &DiagramLabels) -> String {
    render_for(graph, DiagramDetail::Full, labels)
}

/// Render at a detail level. Aggregation already happened in the graph
/// builder, so the only thing the level decides here is the title: report
/// figures are captioned by the page and carry none.
pub fn render_for(graph: &EstateGraph, detail: DiagramDetail, labels: &DiagramLabels) -> String {
    let mut out = String::new();
    if detail.shows_title() {
        let _ = writeln!(out, "---\ntitle: {}\n---", graph.title);
    }
    out.push_str("flowchart LR\n");

    // Depth-first over containment so subgraph blocks nest correctly.
    let children = child_lists(graph);
    let roots: Vec<usize> = (0..graph.nodes.len())
        .filter(|&i| graph.nodes[i].parent.is_none())
        .collect();
    for root in roots {
        emit_node(graph, &children, root, 1, &mut out);
    }

    // Mermaid has two line styles; the third (association) is a finer dot,
    // applied by index afterwards because `linkStyle` is the only way to
    // set a dash pattern.
    let mut associations = Vec::new();
    for (index, edge) in graph.edges.iter().enumerate() {
        let arrow = match edge.style {
            EdgeStyle::Solid => "---",
            EdgeStyle::Dashed | EdgeStyle::Association => "-.-",
        };
        if matches!(edge.style, EdgeStyle::Association) {
            associations.push(index);
        }
        match &edge.label {
            Some(label) => {
                let _ = writeln!(
                    out,
                    "    n{}{arrow}|{}|n{}",
                    edge.source,
                    escape(label),
                    edge.target
                );
            }
            None => {
                let _ = writeln!(out, "    n{}{arrow}n{}", edge.source, edge.target);
            }
        }
    }
    for index in associations {
        let _ = writeln!(out, "    linkStyle {index} stroke-dasharray:1 3");
    }

    legend(graph, labels, &mut out);

    out.push_str(concat!(
        "    classDef subscription fill:#e8f1fa,stroke:#0078d4\n",
        "    classDef rg fill:#f3f2f1,stroke:#8a8886\n",
        "    classDef vnet fill:#e6f5e6,stroke:#107c10\n",
        "    classDef subnet fill:#fff,stroke:#8a8886\n",
        "    classDef zone fill:#fdf6ec,stroke:#d97706\n",
        "    classDef resource fill:#fff,stroke:#605e5c\n",
        "    classDef legend fill:#fff,stroke:#8a8886,stroke-dasharray:4 3\n",
    ));
    out
}

/// The same key the SVG draws: one node per container kind present, plus the
/// count convention when a tile carries one. Empty when there is nothing to
/// explain.
fn legend(graph: &EstateGraph, labels: &DiagramLabels, out: &mut String) {
    let kinds = legend_kinds(graph);
    let aggregates = has_aggregates(graph);
    if kinds.is_empty() && !aggregates {
        return;
    }
    let _ = writeln!(
        out,
        "    subgraph legend[\"{}\"]",
        escape(&labels.legend.title)
    );
    for kind in kinds {
        let (label, _) = super::svg::legend_words(&kind, labels);
        let class = class_for(&kind).unwrap_or("resource");
        let _ = writeln!(out, "        lg_{class}[\"{}\"]", escape(label));
        let _ = writeln!(out, "        class lg_{class} {class}");
    }
    if aggregates {
        let _ = writeln!(
            out,
            "        lg_aggregate[\"×N — {}\"]",
            escape(&labels.legend.aggregate)
        );
        out.push_str("        class lg_aggregate resource\n");
    }
    out.push_str("    end\n    class legend legend\n");
}

fn child_lists(graph: &EstateGraph) -> Vec<Vec<usize>> {
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); graph.nodes.len()];
    for (index, node) in graph.nodes.iter().enumerate() {
        if let Some(parent) = node.parent {
            children[parent].push(index);
        }
    }
    children
}

fn emit_node(
    graph: &EstateGraph,
    children: &[Vec<usize>],
    index: usize,
    depth: usize,
    out: &mut String,
) {
    let node = &graph.nodes[index];
    let indent = "    ".repeat(depth);
    let label = match &node.sublabel {
        Some(sub) => format!("{}<br/><i>{}</i>", escape(&node_label(node)), escape(sub)),
        None => escape(&node_label(node)),
    };
    if node.kind.is_container() && !children[index].is_empty() {
        let _ = writeln!(out, "{indent}subgraph n{index}[\"{label}\"]");
        for &child in &children[index] {
            emit_node(graph, children, child, depth + 1, out);
        }
        let _ = writeln!(out, "{indent}end");
    } else {
        let _ = writeln!(out, "{indent}n{index}[\"{label}\"]");
    }
    if let Some(class) = class_for(&node.kind) {
        let _ = writeln!(out, "{indent}class n{index} {class}");
    }
}

fn class_for(kind: &NodeKind) -> Option<&'static str> {
    match kind {
        NodeKind::Subscription => Some("subscription"),
        NodeKind::ResourceGroup => Some("rg"),
        NodeKind::Vnet => Some("vnet"),
        NodeKind::Subnet => Some("subnet"),
        NodeKind::Zone => Some("zone"),
        NodeKind::Resource { .. } => Some("resource"),
        NodeKind::Tenant => None,
    }
}

fn escape(value: &str) -> String {
    value.replace('"', "&quot;").replace('|', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::graph::{DiagEdge, LayoutMode, Node};

    fn labels() -> DiagramLabels {
        crate::labels::Labels::default().diagram
    }

    fn node(label: &str, kind: NodeKind, parent: Option<usize>) -> Node {
        Node {
            label: label.into(),
            sublabel: None,
            kind,
            parent,
        }
    }

    #[test]
    fn render_nests_containers_and_quotes_labels() {
        let graph = EstateGraph {
            title: "test".into(),
            nodes: vec![
                Node {
                    label: "Sub \"A\"".into(),
                    sublabel: None,
                    kind: NodeKind::Subscription,
                    parent: None,
                },
                Node {
                    label: "vm-1".into(),
                    sublabel: Some("Virtual Machine".into()),
                    kind: NodeKind::Resource {
                        azure_type: "microsoft.compute/virtualmachines".into(),
                    },
                    parent: Some(0),
                },
            ],
            edges: vec![],
            layout: LayoutMode::default(),
        };

        let output = render(&graph, &labels());

        assert!(
            output.contains("subgraph n0[\"Sub &quot;A&quot;\"]")
                && output.contains("n1[\"vm-1<br/><i>Virtual Machine</i>\"]"),
            "output:\n{output}"
        );
        assert!(output.starts_with("---\ntitle: test"));
        assert!(!output.contains("subgraph legend"), "nothing to explain");
    }

    #[test]
    fn unit_summary_detail_drops_the_title_and_keys_the_legend_to_present_kinds() {
        let graph = EstateGraph {
            title: "network".into(),
            nodes: vec![
                node("vnet-a", NodeKind::Vnet, None),
                node("app", NodeKind::Subnet, Some(0)),
                node(
                    "vm",
                    NodeKind::Resource {
                        azure_type: "t".into(),
                    },
                    Some(1),
                ),
                node("vnet-b", NodeKind::Vnet, None),
            ],
            edges: vec![
                DiagEdge {
                    source: 0,
                    target: 3,
                    label: Some("Connected".into()),
                    style: EdgeStyle::Dashed,
                },
                DiagEdge {
                    source: 2,
                    target: 3,
                    label: None,
                    style: EdgeStyle::Association,
                },
            ],
            layout: LayoutMode::default(),
        };

        let output = render_for(&graph, DiagramDetail::Summary, &labels());

        assert!(!output.contains("title:"));
        assert!(output.contains("classDef zone"));
        assert!(!output.contains("unnetworked"));
        assert!(output.contains("linkStyle 1 stroke-dasharray:1 3"));
        assert!(output.contains("lg_vnet") && output.contains("lg_subnet"));
        assert!(!output.contains("lg_zone"), "no zone drawn, none explained");
    }
}
