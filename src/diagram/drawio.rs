use quick_xml::events::{BytesDecl, BytesStart, Event};
use quick_xml::writer::Writer;

use super::graph::{EdgeStyle, EstateGraph, NodeKind};
use super::icons;
use super::layout::{self, Placement};

/// Render the graph as a draw.io `mxfile`. Containers are swimlanes with
/// children parented to them at relative coordinates; edges carry no manual
/// waypoints so draw.io routes them.
pub fn render(graph: &EstateGraph) -> String {
    let placements = layout::layout(graph);
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);

    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .expect("writing to Vec cannot fail");

    let mut mxfile = BytesStart::new("mxfile");
    mxfile.push_attribute(("host", "azdocs"));
    mxfile.push_attribute(("type", "device"));
    with_element(&mut writer, mxfile, |writer| {
        let mut diagram = BytesStart::new("diagram");
        diagram.push_attribute(("name", graph.title.as_str()));
        diagram.push_attribute(("id", "azdocs-0"));
        with_element(writer, diagram, |writer| {
            let mut model = BytesStart::new("mxGraphModel");
            for (key, value) in [
                ("dx", "1000"),
                ("dy", "800"),
                ("grid", "0"),
                ("gridSize", "10"),
                ("guides", "1"),
                ("tooltips", "1"),
                ("connect", "1"),
                ("arrows", "1"),
                ("fold", "1"),
                ("page", "1"),
                ("pageScale", "1"),
                ("pageWidth", "1169"),
                ("pageHeight", "826"),
                ("math", "0"),
                ("shadow", "0"),
            ] {
                model.push_attribute((key, value));
            }
            with_element(writer, model, |writer| {
                with_element(writer, BytesStart::new("root"), |writer| {
                    empty_cell(writer, &[("id", "0")]);
                    empty_cell(writer, &[("id", "1"), ("parent", "0")]);
                    for (index, node) in graph.nodes.iter().enumerate() {
                        node_cell(writer, graph, index, node, &placements[index]);
                    }
                    for (offset, edge) in graph.edges.iter().enumerate() {
                        edge_cell(writer, offset, edge);
                    }
                });
            });
        });
    });

    String::from_utf8(writer.into_inner()).expect("writer emits UTF-8")
}

fn with_element<F>(writer: &mut Writer<Vec<u8>>, start: BytesStart<'_>, body: F)
where
    F: FnOnce(&mut Writer<Vec<u8>>),
{
    let name = start.name().as_ref().to_vec();
    writer
        .write_event(Event::Start(start))
        .expect("writing to Vec cannot fail");
    body(writer);
    writer
        .write_event(Event::End(quick_xml::events::BytesEnd::new(
            String::from_utf8_lossy(&name).into_owned(),
        )))
        .expect("writing to Vec cannot fail");
}

fn empty_cell(writer: &mut Writer<Vec<u8>>, attributes: &[(&str, &str)]) {
    let mut cell = BytesStart::new("mxCell");
    for attribute in attributes {
        cell.push_attribute(*attribute);
    }
    writer
        .write_event(Event::Empty(cell))
        .expect("writing to Vec cannot fail");
}

fn node_cell(
    writer: &mut Writer<Vec<u8>>,
    graph: &EstateGraph,
    index: usize,
    node: &super::graph::Node,
    placement: &Placement,
) {
    let parent = node
        .parent
        .map(|p| format!("n{p}"))
        .unwrap_or_else(|| "1".to_owned());
    let label = match &node.sublabel {
        Some(sub) => format!("{}\n{}", node.label, sub),
        None => node.label.clone(),
    };
    let style = style_for_node(graph, index, node);

    let mut cell = BytesStart::new("mxCell");
    cell.push_attribute(("id", format!("n{index}").as_str()));
    cell.push_attribute(("value", label.as_str()));
    cell.push_attribute(("style", style.as_str()));
    cell.push_attribute(("parent", parent.as_str()));
    cell.push_attribute(("vertex", "1"));
    with_element(writer, cell, |writer| {
        let mut geometry = BytesStart::new("mxGeometry");
        geometry.push_attribute(("x", trim_float(placement.x).as_str()));
        geometry.push_attribute(("y", trim_float(placement.y).as_str()));
        geometry.push_attribute(("width", trim_float(placement.width).as_str()));
        geometry.push_attribute(("height", trim_float(placement.height).as_str()));
        geometry.push_attribute(("as", "geometry"));
        writer
            .write_event(Event::Empty(geometry))
            .expect("writing to Vec cannot fail");
    });
}

fn style_for_node(graph: &EstateGraph, index: usize, node: &super::graph::Node) -> String {
    let has_children = graph.nodes.iter().any(|n| n.parent == Some(index));
    match &node.kind {
        NodeKind::Resource { azure_type } => icons::style_for(azure_type),
        kind if has_children => {
            let (fill, stroke) = container_palette(kind);
            format!(
                "swimlane;html=1;startSize=30;rounded=1;arcSize=6;\
                 fillColor={fill};strokeColor={stroke};fontSize=12;fontStyle=1;\
                 verticalAlign=top;horizontal=1;collapsible=0;"
            )
        }
        kind => {
            let (fill, stroke) = container_palette(kind);
            format!(
                "rounded=1;html=1;whiteSpace=wrap;fillColor={fill};\
                 strokeColor={stroke};verticalAlign=middle;fontSize=12;"
            )
        }
    }
}

fn container_palette(kind: &NodeKind) -> (&'static str, &'static str) {
    match kind {
        NodeKind::Tenant => ("#FFFFFF", "#605E5C"),
        NodeKind::Subscription => ("#E8F1FA", "#0078D4"),
        NodeKind::ResourceGroup => ("#F3F2F1", "#8A8886"),
        NodeKind::Vnet => ("#E6F5E6", "#107C10"),
        NodeKind::Subnet => ("#F0FAF0", "#4C9A4C"),
        NodeKind::Resource { .. } => ("#FFFFFF", "#605E5C"),
    }
}

fn edge_cell(writer: &mut Writer<Vec<u8>>, offset: usize, edge: &super::graph::DiagEdge) {
    let style = match edge.style {
        EdgeStyle::Solid => {
            "edgeStyle=orthogonalEdgeStyle;rounded=1;html=1;endArrow=none;strokeColor=#605E5C;"
        }
        EdgeStyle::Dashed => {
            "edgeStyle=orthogonalEdgeStyle;rounded=1;html=1;dashed=1;endArrow=none;\
             startArrow=none;strokeColor=#0078D4;fontSize=10;"
        }
        EdgeStyle::Association => {
            "edgeStyle=orthogonalEdgeStyle;rounded=1;html=1;dashed=1;dashPattern=1 3;\
             endArrow=open;strokeColor=#8A8886;fontSize=10;"
        }
    };
    let mut cell = BytesStart::new("mxCell");
    let id = format!("e{offset}");
    cell.push_attribute(("id", id.as_str()));
    if let Some(label) = &edge.label {
        cell.push_attribute(("value", label.as_str()));
    }
    cell.push_attribute(("style", style));
    // Cross-container edges live on the root layer; draw.io resolves the
    // endpoints by cell id.
    cell.push_attribute(("parent", "1"));
    cell.push_attribute(("source", format!("n{}", edge.source).as_str()));
    cell.push_attribute(("target", format!("n{}", edge.target).as_str()));
    cell.push_attribute(("edge", "1"));
    with_element(writer, cell, |writer| {
        let mut geometry = BytesStart::new("mxGeometry");
        geometry.push_attribute(("relative", "1"));
        geometry.push_attribute(("as", "geometry"));
        writer
            .write_event(Event::Empty(geometry))
            .expect("writing to Vec cannot fail");
    });
}

fn trim_float(value: f64) -> String {
    if (value.round() - value).abs() < 0.01 {
        format!("{}", value.round() as i64)
    } else {
        format!("{value:.1}")
    }
}
