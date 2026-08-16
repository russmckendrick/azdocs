//! Self-contained SVG rendering of an `EstateGraph`: shares the draw.io
//! layout (via `absolutize`) and palette so both emitters agree on shape,
//! with icons embedded as data URIs so the file needs no external assets.

use std::fmt::Write as _;

use super::drawio::container_palette;
use super::graph::{DiagEdge, EdgeStyle, EstateGraph, NodeKind, truncate_label};
use super::icons;
use super::layout::{self, Placement};

const MARGIN: f64 = 20.0;
/// Vertical room above the content for the diagram title.
const TITLE_BAND: f64 = 48.0;
const ICON: f64 = 48.0;
const FONT: &str = "Arial, Helvetica, sans-serif";
const TEXT_PRIMARY: &str = "#323130";
const TEXT_SECONDARY: &str = "#605E5C";
const PEERING_CONNECTED: &str = "#107C10";
const PEERING_DISCONNECTED: &str = "#D13438";

/// Render the graph as a standalone SVG document.
pub fn render(graph: &EstateGraph) -> String {
    let placements = layout::absolutize(graph, &layout::layout(graph));
    let (content_width, content_height) = bounds(&placements);
    let width = content_width + 2.0 * MARGIN;
    let height = content_height + TITLE_BAND + MARGIN;

    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" font-family="{FONT}">"#,
        w = fmt(width),
        h = fmt(height),
    );
    let _ = writeln!(
        out,
        r#"  <rect width="{}" height="{}" fill="white"/>"#,
        fmt(width),
        fmt(height)
    );
    out.push_str(concat!(
        "  <defs>\n",
        "    <marker id=\"arrow\" viewBox=\"0 0 10 10\" refX=\"10\" refY=\"5\" ",
        "markerWidth=\"6\" markerHeight=\"6\" orient=\"auto-start-reverse\">\n",
        "      <path d=\"M 0 0 L 10 5 L 0 10 z\" fill=\"#605E5C\"/>\n",
        "    </marker>\n",
        "  </defs>\n",
    ));
    let _ = writeln!(
        out,
        r#"  <text x="{}" y="28" font-size="18" font-weight="bold" text-anchor="middle" fill="{TEXT_PRIMARY}">{}</text>"#,
        fmt(width / 2.0),
        escape(&graph.title),
    );
    let _ = writeln!(
        out,
        r#"  <g transform="translate({},{})">"#,
        fmt(MARGIN),
        fmt(TITLE_BAND)
    );

    // Containers first (parents precede children by construction), then
    // edges, then icons — so edges run under icons but over containers.
    for (index, node) in graph.nodes.iter().enumerate() {
        if node.kind.is_container() {
            container(&mut out, graph, index, &placements[index]);
        }
    }
    for edge in &graph.edges {
        edge_line(&mut out, edge, &placements);
    }
    for (index, node) in graph.nodes.iter().enumerate() {
        if let NodeKind::Resource { azure_type } = &node.kind {
            resource_icon(&mut out, node, azure_type, &placements[index]);
        }
    }

    out.push_str("  </g>\n</svg>\n");
    out
}

fn bounds(placements: &[Placement]) -> (f64, f64) {
    let width = placements
        .iter()
        .map(|p| p.x + p.width)
        .fold(0.0_f64, f64::max);
    let height = placements
        .iter()
        .map(|p| p.y + p.height)
        .fold(0.0_f64, f64::max);
    (width.max(360.0), height.max(120.0))
}

fn container(out: &mut String, graph: &EstateGraph, index: usize, placement: &Placement) {
    let node = &graph.nodes[index];
    let (fill, stroke) = container_palette(&node.kind);
    // Subnets and the synthetic "Standalone Resources" group are soft
    // boundaries, drawn dashed to match the draw.io/legacy convention.
    let dashed = node.kind == NodeKind::Subnet || node.label == "Standalone Resources";
    let dash = if dashed {
        r#" stroke-dasharray="6,4""#
    } else {
        ""
    };
    let _ = writeln!(
        out,
        r#"    <rect x="{}" y="{}" width="{}" height="{}" rx="8" fill="{fill}" stroke="{stroke}" stroke-width="1.5"{dash}/>"#,
        fmt(placement.x),
        fmt(placement.y),
        fmt(placement.width),
        fmt(placement.height),
    );
    let center_x = placement.x + placement.width / 2.0;
    let has_children = graph.nodes.iter().any(|n| n.parent == Some(index));
    // Childless containers (peering stubs) centre their label; populated ones
    // keep it in the 30px title band the layout reserves.
    let (label_y, sublabel_y) = if has_children {
        (placement.y + 15.0, placement.y + 26.0)
    } else if node.sublabel.is_some() {
        (
            placement.y + placement.height / 2.0 - 3.0,
            placement.y + placement.height / 2.0 + 11.0,
        )
    } else {
        (placement.y + placement.height / 2.0 + 4.0, 0.0)
    };
    let _ = writeln!(
        out,
        r#"    <text x="{}" y="{}" font-size="12" font-weight="bold" text-anchor="middle" fill="{TEXT_PRIMARY}">{}</text>"#,
        fmt(center_x),
        fmt(label_y),
        escape(&truncate_label(&node.label)),
    );
    if let Some(sublabel) = &node.sublabel {
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}" font-size="9" text-anchor="middle" fill="{TEXT_SECONDARY}">{}</text>"#,
            fmt(center_x),
            fmt(sublabel_y),
            escape(sublabel),
        );
    }
}

fn resource_icon(
    out: &mut String,
    node: &super::graph::Node,
    azure_type: &str,
    placement: &Placement,
) {
    let center_x = placement.x + placement.width / 2.0;
    let _ = writeln!(
        out,
        r#"    <image x="{}" y="{}" width="{}" height="{}" href="{}"/>"#,
        fmt(center_x - ICON / 2.0),
        fmt(placement.y + 4.0),
        fmt(ICON),
        fmt(ICON),
        icons::svg_data_uri(azure_type),
    );
    let mut line_y = placement.y + 4.0 + ICON + 14.0;
    for line in wrap_label(&truncate_label(&node.label)) {
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}" font-size="11" text-anchor="middle" fill="{TEXT_PRIMARY}">{}</text>"#,
            fmt(center_x),
            fmt(line_y),
            escape(&line),
        );
        line_y += 13.0;
    }
    if let Some(sublabel) = &node.sublabel {
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}" font-size="9" text-anchor="middle" fill="{TEXT_SECONDARY}">{}</text>"#,
            fmt(center_x),
            fmt(line_y),
            escape(sublabel),
        );
    }
}

fn edge_line(out: &mut String, edge: &DiagEdge, placements: &[Placement]) {
    let source = center(&placements[edge.source]);
    let target = center(&placements[edge.target]);
    let (stroke, dash, marker) = match edge.style {
        EdgeStyle::Solid => ("#605E5C", "", r##" marker-end="url(#arrow)""##),
        EdgeStyle::Dashed => {
            // Peerings: green when connected, red for any other state.
            let connected = edge
                .label
                .as_deref()
                .is_some_and(|state| state.eq_ignore_ascii_case("connected"));
            let stroke = if connected {
                PEERING_CONNECTED
            } else {
                PEERING_DISCONNECTED
            };
            (stroke, r#" stroke-dasharray="6,4""#, "")
        }
        EdgeStyle::Association => ("#8A8886", r#" stroke-dasharray="2,3""#, ""),
    };
    let _ = writeln!(
        out,
        r#"    <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{stroke}" stroke-width="1.5"{dash}{marker}/>"#,
        fmt(source.0),
        fmt(source.1),
        fmt(target.0),
        fmt(target.1),
    );
    if let Some(label) = &edge.label {
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}" font-size="10" text-anchor="middle" fill="{TEXT_SECONDARY}">{}</text>"#,
            fmt((source.0 + target.0) / 2.0),
            fmt((source.1 + target.1) / 2.0 - 6.0),
            escape(label),
        );
    }
}

fn center(placement: &Placement) -> (f64, f64) {
    (
        placement.x + placement.width / 2.0,
        placement.y + placement.height / 2.0,
    )
}

/// Split an (already truncated) label into at most two 15-char lines so it
/// fits the 150px layout slot at font-size 11.
fn wrap_label(value: &str) -> Vec<String> {
    const LINE: usize = 15;
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= LINE {
        return vec![value.to_owned()];
    }
    vec![
        chars[..LINE].iter().collect(),
        chars[LINE..].iter().collect(),
    ]
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn fmt(value: f64) -> String {
    if (value.round() - value).abs() < 0.01 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.1}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagram::graph::Node;

    #[test]
    fn escape_covers_xml_metacharacters() {
        assert_eq!(escape(r#"a<b>&"c""#), "a&lt;b&gt;&amp;&quot;c&quot;");
    }

    #[test]
    fn wrap_label_splits_long_names_into_two_lines() {
        assert_eq!(wrap_label("short"), vec!["short"]);
        assert_eq!(
            wrap_label("a-very-long-resource-name"),
            vec!["a-very-long-res", "ource-name"]
        );
    }

    #[test]
    fn render_embeds_icons_and_escapes_labels() {
        let graph = EstateGraph {
            title: "t & t".into(),
            nodes: vec![
                Node {
                    label: "rg".into(),
                    sublabel: None,
                    kind: NodeKind::ResourceGroup,
                    parent: None,
                },
                Node {
                    label: "vm \"one\"".into(),
                    sublabel: Some("Virtual Machine".into()),
                    kind: NodeKind::Resource {
                        azure_type: "microsoft.compute/virtualmachines".into(),
                    },
                    parent: Some(0),
                },
            ],
            edges: vec![],
        };

        let svg = render(&graph);

        assert!(svg.contains("t &amp; t"), "title escaped: {svg}");
        assert!(
            svg.contains("data:image/svg+xml;base64,"),
            "icon embedded: {svg}"
        );
        assert!(svg.contains("vm &quot;one&quot;"), "label escaped: {svg}");
    }
}
