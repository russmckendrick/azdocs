use std::fmt::Write as _;

use super::graph::{EdgeStyle, EstateGraph, NodeKind};

/// Emitters warn above this: deeply nested Mermaid becomes unreadable.
pub const NODE_WARN_THRESHOLD: usize = 150;

/// Render the graph as a `flowchart LR` with nested subgraphs for containers.
pub fn render(graph: &EstateGraph) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "---\ntitle: {}\n---", graph.title);
    out.push_str("flowchart LR\n");

    // Depth-first over containment so subgraph blocks nest correctly.
    let children = child_lists(graph);
    let roots: Vec<usize> = (0..graph.nodes.len())
        .filter(|&i| graph.nodes[i].parent.is_none())
        .collect();
    for root in roots {
        emit_node(graph, &children, root, 1, &mut out);
    }

    for edge in &graph.edges {
        let arrow = match edge.style {
            EdgeStyle::Solid => "---",
            EdgeStyle::Dashed => "-.-",
            EdgeStyle::Association => "-.-",
        };
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

    out.push_str(concat!(
        "    classDef subscription fill:#e8f1fa,stroke:#0078d4\n",
        "    classDef rg fill:#f3f2f1,stroke:#8a8886\n",
        "    classDef vnet fill:#e6f5e6,stroke:#107c10\n",
        "    classDef subnet fill:#f0faf0,stroke:#4c9a4c\n",
        "    classDef resource fill:#fff,stroke:#605e5c\n",
    ));
    out
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
        Some(sub) => format!("{}<br/><i>{}</i>", escape(&node.label), escape(sub)),
        None => escape(&node.label),
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
    use crate::diagram::graph::Node;

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
        };

        let output = render(&graph);

        assert!(
            output.contains("subgraph n0[\"Sub &quot;A&quot;\"]")
                && output.contains("n1[\"vm-1<br/><i>Virtual Machine</i>\"]"),
            "output:\n{output}"
        );
    }
}
