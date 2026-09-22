use quick_xml::events::{BytesDecl, BytesStart, Event};
use quick_xml::writer::Writer;

use super::graph::{EdgeStyle, EstateGraph, NodeKind, has_aggregates, legend_kinds, node_label};
use super::icons;
use super::layout::{self, Placement};
use super::page::{DiagramDetail, Rung};
use super::route;
use crate::labels::DiagramLabels;

/// Guard against a cycle in the parent chain; real graphs nest three deep.
const MAX_DEPTH: usize = 8;

/// Gap between a container's left edge and its stencil.
const HEADER_INSET: f64 = 8.0;

/// Does this node contain anything? Decides swimlane versus plain box, and
/// with it where the stencil and the label sit.
fn has_children(graph: &EstateGraph, index: usize) -> bool {
    graph.nodes.iter().any(|n| n.parent == Some(index))
}

/// Render the graph as a single-sheet draw.io `mxfile` at the default detail.
pub fn render(graph: &EstateGraph, labels: &DiagramLabels) -> String {
    render_for(graph, DiagramDetail::default(), labels)
}

/// Render one sheet at a given detail level. Containers are swimlanes with
/// children parented to them at relative coordinates; edges anchor to the
/// boundary sides [`super::route`] picked and are otherwise routed by draw.io,
/// so a reader who drags a node keeps a sensible connector.
pub fn render_for(graph: &EstateGraph, detail: DiagramDetail, labels: &DiagramLabels) -> String {
    // The empty prefix keeps single-sheet cell ids `n{i}`/`e{i}`; the id scheme
    // is frozen even though the geometry under it is not.
    render_file(&[(graph.title.as_str(), graph)], &[], false, detail, labels)
}

/// Render several graphs as one multi-sheet `mxfile` workbook. Sheet ids are
/// `azdocs-{i}`; cell ids are prefixed `s{i}-` so they are file-wide unique.
pub fn render_workbook(sheets: &[(&str, &EstateGraph)], labels: &DiagramLabels) -> String {
    render_workbook_for(sheets, DiagramDetail::default(), labels)
}

/// Render a workbook at a given detail level.
pub fn render_workbook_for(
    sheets: &[(&str, &EstateGraph)],
    detail: DiagramDetail,
    labels: &DiagramLabels,
) -> String {
    render_file(sheets, &[], true, detail, labels)
}

/// A workbook sheet that is a picture rather than a graph, such as the
/// resource-locations map: the SVG as one image cell, each label an editable
/// pill placed beside its point, and an optional caption beneath.
pub struct ImageSheet<'a> {
    pub name: &'a str,
    pub svg: &'a str,
    pub width: f64,
    pub height: f64,
    /// In placement priority: earlier labels get the nearer spots.
    pub labels: &'a [ImageLabel],
    pub caption: Option<&'a str>,
    pub colors: LabelColors<'a>,
}

/// A label for one point of an image sheet, in the image's pixels.
pub struct ImageLabel {
    pub text: String,
    pub x: f64,
    pub y: f64,
    /// Clearance around the point that no label may cover.
    pub radius: f64,
}

/// `#rrggbb` colours for an image sheet's pills, caption and leader lines.
pub struct LabelColors<'a> {
    pub fill: &'a str,
    pub stroke: &'a str,
    pub text: &'a str,
    pub muted: &'a str,
}

/// A workbook of graph sheets followed by image sheets. Image sheets take the
/// next `azdocs-{i}` ids and `s{i}-` cell prefixes, so the scheme is unchanged.
pub fn render_workbook_with(
    sheets: &[(&str, &EstateGraph)],
    images: &[ImageSheet<'_>],
    detail: DiagramDetail,
    labels: &DiagramLabels,
) -> String {
    render_file(sheets, images, true, detail, labels)
}

fn render_file(
    sheets: &[(&str, &EstateGraph)],
    images: &[ImageSheet<'_>],
    prefixed: bool,
    detail: DiagramDetail,
    labels: &DiagramLabels,
) -> String {
    let mut writer = Writer::new_with_indent(Vec::new(), b' ', 2);

    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .expect("writing to Vec cannot fail");

    let mut mxfile = BytesStart::new("mxfile");
    mxfile.push_attribute(("host", "azdocs"));
    mxfile.push_attribute(("type", "device"));
    with_element(&mut writer, mxfile, |writer| {
        for (index, (name, graph)) in sheets.iter().enumerate() {
            let prefix = if prefixed {
                format!("s{index}-")
            } else {
                String::new()
            };
            sheet(writer, index, name, graph, &prefix, detail, labels);
        }
        for (offset, image) in images.iter().enumerate() {
            let index = sheets.len() + offset;
            image_sheet(writer, index, image, &format!("s{index}-"));
        }
    });

    String::from_utf8(writer.into_inner()).expect("writer emits UTF-8")
}

fn sheet(
    writer: &mut Writer<Vec<u8>>,
    index: usize,
    name: &str,
    graph: &EstateGraph,
    prefix: &str,
    detail: DiagramDetail,
    labels: &DiagramLabels,
) {
    let rung = layout::rung(graph);
    let depths = depths(graph);
    let placements = layout::layout_for(graph, detail);
    // Routing works in page coordinates; draw.io child geometry is relative to
    // the parent, so the two live side by side here.
    let absolute = layout::absolutize(graph, &placements);
    let routes = route::route(graph, &absolute, &rung);
    let (content_width, content_height) = content_bounds(&absolute, &routes);
    let stamp_top = content_height + STAMP_GAP;
    let legend_entries = legend_entries(graph, labels);
    let legend_height = if legend_entries.is_empty() {
        0.0
    } else {
        LEGEND_ROW
    };
    let (page_width, page_height) =
        page_size(content_width, stamp_top + STAMP_SIZE.max(legend_height));
    let mut diagram = BytesStart::new("diagram");
    diagram.push_attribute(("name", name));
    diagram.push_attribute(("id", format!("azdocs-{index}").as_str()));
    with_element(writer, diagram, |writer| {
        let mut model = BytesStart::new("mxGraphModel");
        // The page is sized to the drawing rather than the drawing to A4: this
        // is the editable artefact, and the svg/png exports are the print ones.
        // Left at A4 landscape, every sheet wider than 1169px got a page break
        // ruled through it.
        let (page_width, page_height) = (trim_float(page_width), trim_float(page_height));
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
            ("pageWidth", page_width.as_str()),
            ("pageHeight", page_height.as_str()),
            ("math", "0"),
            ("shadow", "0"),
        ] {
            model.push_attribute((key, value));
        }
        with_element(writer, model, |writer| {
            with_element(writer, BytesStart::new("root"), |writer| {
                empty_cell(writer, &[("id", format!("{prefix}0").as_str())]);
                empty_cell(
                    writer,
                    &[
                        ("id", format!("{prefix}1").as_str()),
                        ("parent", format!("{prefix}0").as_str()),
                    ],
                );
                for (offset, node) in graph.nodes.iter().enumerate() {
                    // Chrome decays with nesting depth, and layout already
                    // reserved the decayed band: declaring the undecayed one
                    // put a nested container's children inside its own title.
                    let metrics = rung.at_depth(depths[offset]);
                    node_cell(
                        writer,
                        graph,
                        offset,
                        node,
                        &placements[offset],
                        &metrics,
                        prefix,
                    );
                    header_cell(
                        writer,
                        offset,
                        node,
                        &placements[offset],
                        &metrics,
                        has_children(graph, offset),
                        prefix,
                    );
                }
                // Only routed edges are drawn: `route` drops the containment
                // ones, which draw.io rendered as a line from a container to
                // its own child.
                for routed in &routes {
                    edge_cell(writer, routed, &graph.edges[routed.edge], prefix);
                }
                stamp_cell(writer, stamp_top, prefix);
                legend_cells(writer, &legend_entries, stamp_top, prefix);
            });
        });
    });
}

const LABEL_HEIGHT: f64 = 20.0;
const LABEL_FONT: f64 = 11.0;
/// Space between a point's clearance and its label.
const LABEL_GAP: f64 = 4.0;
const CAPTION_HEIGHT: f64 = 22.0;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Rect {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl Rect {
    fn around(x: f64, y: f64, radius: f64) -> Self {
        Self {
            x: x - radius,
            y: y - radius,
            width: radius * 2.0,
            height: radius * 2.0,
        }
    }

    fn overlaps(&self, other: &Rect) -> bool {
        self.x < other.x + other.width
            && other.x < self.x + self.width
            && self.y < other.y + other.height
            && other.y < self.y + self.height
    }

    fn inside(&self, width: f64, height: f64) -> bool {
        self.x >= 0.0
            && self.y >= 0.0
            && self.x + self.width <= width
            && self.y + self.height <= height
    }
}

/// A label's box and whether it sits away from its point, needing a leader.
struct PlacedLabel {
    rect: Rect,
    leader: bool,
}

/// Width of a pill for `text`: an estimate, since draw.io measures the font
/// itself, erring wide so a pill never clips its text.
fn label_width(text: &str) -> f64 {
    (text.chars().count() as f64 * LABEL_FONT * 0.6 + 16.0).ceil()
}

/// Place every label beside its point without covering another label or
/// point: right, left, above, below; then further out with a leader line,
/// diagonals included. Closed-form candidates in a fixed order keep the
/// output deterministic; a label with nowhere free takes its first in-bounds
/// spot rather than being dropped.
fn place_labels(image: &ImageSheet<'_>) -> Vec<PlacedLabel> {
    let points: Vec<Rect> = image
        .labels
        .iter()
        .map(|label| Rect::around(label.x, label.y, label.radius))
        .collect();
    let mut taken: Vec<Rect> = Vec::new();
    let mut placed = Vec::new();
    for label in image.labels {
        let (w, h) = (label_width(&label.text), LABEL_HEIGHT);
        let rect = |x: f64, y: f64| Rect {
            x,
            y,
            width: w,
            height: h,
        };
        let mut candidates = Vec::new();
        for (ring, leader) in [(0.0, false), (26.0, true), (60.0, true)] {
            let near = label.radius + LABEL_GAP + ring;
            candidates.extend(
                [
                    rect(label.x + near, label.y - h / 2.0),
                    rect(label.x - near - w, label.y - h / 2.0),
                    rect(label.x - w / 2.0, label.y - near - h),
                    rect(label.x - w / 2.0, label.y + near),
                ]
                .map(|r| (r, leader)),
            );
            if leader {
                let d = near * std::f64::consts::FRAC_1_SQRT_2;
                candidates.extend(
                    [
                        rect(label.x + d, label.y - d - h),
                        rect(label.x + d, label.y + d),
                        rect(label.x - d - w, label.y - d - h),
                        rect(label.x - d - w, label.y + d),
                    ]
                    .map(|r| (r, true)),
                );
            }
        }
        let fits = |r: &Rect| r.inside(image.width, image.height);
        let free = |r: &Rect| {
            fits(r)
                && !taken.iter().any(|other| other.overlaps(r))
                && !points.iter().any(|point| point.overlaps(r))
        };
        let (rect, leader) = candidates
            .iter()
            .find(|(r, _)| free(r))
            .or_else(|| candidates.iter().find(|(r, _)| fits(r)))
            .copied()
            .unwrap_or((candidates[0].0, false));
        taken.push(rect);
        placed.push(PlacedLabel { rect, leader });
    }
    placed
}

fn image_sheet(writer: &mut Writer<Vec<u8>>, index: usize, image: &ImageSheet<'_>, prefix: &str) {
    let caption_top = image.height + LABEL_GAP;
    let page_height = if image.caption.is_some() {
        caption_top + CAPTION_HEIGHT
    } else {
        image.height
    };
    let placed = place_labels(image);
    let colors = &image.colors;
    let layer = format!("{prefix}1");
    let mut diagram = BytesStart::new("diagram");
    diagram.push_attribute(("name", image.name));
    diagram.push_attribute(("id", format!("azdocs-{index}").as_str()));
    with_element(writer, diagram, |writer| {
        let mut model = BytesStart::new("mxGraphModel");
        let (page_width, page_height) = (trim_float(image.width), trim_float(page_height));
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
            ("pageWidth", page_width.as_str()),
            ("pageHeight", page_height.as_str()),
            ("math", "0"),
            ("shadow", "0"),
        ] {
            model.push_attribute((key, value));
        }
        with_element(writer, model, |writer| {
            with_element(writer, BytesStart::new("root"), |writer| {
                empty_cell(writer, &[("id", format!("{prefix}0").as_str())]);
                empty_cell(
                    writer,
                    &[
                        ("id", layer.as_str()),
                        ("parent", format!("{prefix}0").as_str()),
                    ],
                );
                use base64::Engine as _;
                let encoded = base64::engine::general_purpose::STANDARD.encode(image.svg);
                // Locked, so dragging a label never moves the map from under it.
                let style = format!(
                    "image;html=1;aspect=fixed;imageAspect=1;connectable=0;\
                     movable=0;resizable=0;deletable=0;editable=0;\
                     image=data:image/svg+xml,{encoded}"
                );
                geometry_cell(
                    writer,
                    &[
                        ("id", format!("{prefix}image").as_str()),
                        ("value", ""),
                        ("style", style.as_str()),
                        ("parent", layer.as_str()),
                        ("vertex", "1"),
                    ],
                    (0.0, 0.0, image.width, image.height),
                );
                // Leaders before pills, so every pill is drawn over its line.
                let leader_style = format!(
                    "endArrow=none;html=1;strokeColor={};strokeWidth=1;",
                    colors.muted
                );
                for (offset, (label, spot)) in image.labels.iter().zip(&placed).enumerate() {
                    if !spot.leader {
                        continue;
                    }
                    let r = spot.rect;
                    let end = (
                        label.x.clamp(r.x, r.x + r.width),
                        label.y.clamp(r.y, r.y + r.height),
                    );
                    leader_cell(
                        writer,
                        &format!("{prefix}leader{offset}"),
                        &layer,
                        &leader_style,
                        (label.x, label.y),
                        end,
                    );
                }
                let pill_style = format!(
                    "rounded=1;arcSize=50;whiteSpace=nowrap;html=1;fontSize={LABEL_FONT};\
                     fillColor={};strokeColor={};fontColor={};",
                    colors.fill, colors.stroke, colors.text
                );
                for (offset, (label, spot)) in image.labels.iter().zip(&placed).enumerate() {
                    let r = spot.rect;
                    geometry_cell(
                        writer,
                        &[
                            ("id", format!("{prefix}label{offset}").as_str()),
                            ("value", label.text.as_str()),
                            ("style", pill_style.as_str()),
                            ("parent", layer.as_str()),
                            ("vertex", "1"),
                        ],
                        (r.x, r.y, r.width, r.height),
                    );
                }
                if let Some(caption) = image.caption {
                    let style = format!(
                        "text;html=1;align=left;verticalAlign=middle;fontSize={LABEL_FONT};fontColor={};",
                        colors.muted
                    );
                    geometry_cell(
                        writer,
                        &[
                            ("id", format!("{prefix}caption").as_str()),
                            ("value", caption),
                            ("style", style.as_str()),
                            ("parent", layer.as_str()),
                            ("vertex", "1"),
                        ],
                        (0.0, caption_top, image.width, CAPTION_HEIGHT),
                    );
                }
            });
        });
    });
}

/// A free-standing line from `from` to `to`.
fn leader_cell(
    writer: &mut Writer<Vec<u8>>,
    id: &str,
    parent: &str,
    style: &str,
    from: (f64, f64),
    to: (f64, f64),
) {
    let mut cell = BytesStart::new("mxCell");
    cell.push_attribute(("id", id));
    cell.push_attribute(("style", style));
    cell.push_attribute(("parent", parent));
    cell.push_attribute(("edge", "1"));
    with_element(writer, cell, |writer| {
        let mut geometry = BytesStart::new("mxGeometry");
        geometry.push_attribute(("relative", "1"));
        geometry.push_attribute(("as", "geometry"));
        with_element(writer, geometry, |writer| {
            for ((x, y), role) in [(from, "sourcePoint"), (to, "targetPoint")] {
                let mut point = BytesStart::new("mxPoint");
                point.push_attribute(("x", trim_float(x).as_str()));
                point.push_attribute(("y", trim_float(y).as_str()));
                point.push_attribute(("as", role));
                writer
                    .write_event(Event::Empty(point))
                    .expect("writing to Vec cannot fail");
            }
        });
    });
}

/// A vertex with its geometry as the only child.
fn geometry_cell(
    writer: &mut Writer<Vec<u8>>,
    attributes: &[(&str, &str)],
    (x, y, width, height): (f64, f64, f64, f64),
) {
    let mut cell = BytesStart::new("mxCell");
    for attribute in attributes {
        cell.push_attribute(*attribute);
    }
    with_element(writer, cell, |writer| {
        let mut geometry = BytesStart::new("mxGeometry");
        geometry.push_attribute(("x", trim_float(x).as_str()));
        geometry.push_attribute(("y", trim_float(y).as_str()));
        geometry.push_attribute(("width", trim_float(width).as_str()));
        geometry.push_attribute(("height", trim_float(height).as_str()));
        geometry.push_attribute(("as", "geometry"));
        writer
            .write_event(Event::Empty(geometry))
            .expect("writing to Vec cannot fail");
    });
}

fn with_element<F>(writer: &mut Writer<Vec<u8>>, start: BytesStart<'_>, body: F)
where
    F: FnOnce(&mut Writer<Vec<u8>>),
{
    let name = start.name().as_ref().to_owned();
    writer
        .write_event(Event::Start(start))
        .expect("writing to Vec cannot fail");
    body(writer);
    writer
        .write_event(Event::End(quick_xml::events::BytesEnd::new(name)))
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
    rung: &Rung,
    prefix: &str,
) {
    let parent = node
        .parent
        .map(|p| format!("{prefix}n{p}"))
        .unwrap_or_else(|| format!("{prefix}1"));
    // `<br>`, not a newline: XML normalises a literal newline inside an
    // attribute value to a space, so the break between a resource's name and
    // its type was being dropped and draw.io just wrapped the run-on. The
    // style carries `html=1`, so the tag is what draw.io wants here anyway.
    //
    // Both halves are budgeted, because which one holds the long resource name
    // depends on the builder: a subnet member is name-then-type, an
    // out-of-network tile is type-then-name. Capping one of them left the
    // other to wrap to five lines and run into the row below.
    // A container reads on one line — "name  CIDR", the SVG's order — and its
    // label is deliberately never truncated: it is often the only place a
    // count appears. A leaf stacks its name over its type, and *is* budgeted,
    // because a slot is narrow and the row beneath it is close.
    let label = match (&node.sublabel, node.kind.is_container()) {
        (Some(sub), true) => format!("{}  {sub}", node_label(node)),
        (Some(sub), false) => format!(
            "{}<br>{}",
            fit_label(&node_label(node), rung),
            fit_label(sub, rung)
        ),
        (None, true) => node_label(node),
        (None, false) => fit_label(&node_label(node), rung),
    };
    let style = style_for_node(graph, index, node, placement, rung);

    // The glyph is centred in its layout slot and the slot's remaining height
    // is the label's. The size has to come from the rung: a fixed 64px icon
    // was taller than a whole dense slot (64px) and its label then ran over
    // the row beneath.
    let is_icon = matches!(node.kind, NodeKind::Resource { .. });
    let (x, y, width, height) = if is_icon {
        (
            placement.x + ((placement.width - rung.icon) / 2.0).round(),
            placement.y + 4.0,
            rung.icon,
            rung.icon,
        )
    } else {
        (placement.x, placement.y, placement.width, placement.height)
    };

    let mut cell = BytesStart::new("mxCell");
    cell.push_attribute(("id", format!("{prefix}n{index}").as_str()));
    cell.push_attribute(("value", label.as_str()));
    cell.push_attribute(("style", style.as_str()));
    cell.push_attribute(("parent", parent.as_str()));
    cell.push_attribute(("vertex", "1"));
    with_element(writer, cell, |writer| {
        let mut geometry = BytesStart::new("mxGeometry");
        geometry.push_attribute(("x", trim_float(x).as_str()));
        geometry.push_attribute(("y", trim_float(y).as_str()));
        geometry.push_attribute(("width", trim_float(width).as_str()));
        geometry.push_attribute(("height", trim_float(height).as_str()));
        geometry.push_attribute(("as", "geometry"));
        writer
            .write_event(Event::Empty(geometry))
            .expect("writing to Vec cannot fail");
    });
}

/// Cap one half of a leaf label to the lines draw.io has room to wrap it into.
///
/// draw.io wraps at `labelWidth` and never stops, so a long Azure name — a
/// restore-point collection carries a 19-digit suffix — ran to five lines in a
/// slot with room for two and printed over the tiles beneath it. The budget is
/// derived from the width and size the label is actually drawn at rather than
/// a fixed character count, so it holds at every density rung.
fn fit_label(value: &str, rung: &Rung) -> String {
    // 0.55em is the average advance of the sans draw.io renders labels in;
    // close enough to pick a line count, and erring narrow keeps it inside.
    let per_line = ((rung.leaf_width / (rung.label_px * 0.55)).floor() as usize).max(4);
    // Lines are budgeted from the room actually left under the glyph, not from
    // `wrap_lines`: that is a per-label allowance, and a leaf spends it twice
    // because it prints a name *and* a type. Two lines each is what overran a
    // compact tile, which has room for three between them.
    let room = rung.leaf_height - rung.icon - 4.0;
    let total = ((room / (rung.label_px * 1.25)).floor() as usize).max(2);
    crate::model::truncate(value, per_line * (total / 2).max(1))
}

fn style_for_node(
    graph: &EstateGraph,
    index: usize,
    node: &super::graph::Node,
    placement: &Placement,
    rung: &Rung,
) -> String {
    let has_children = has_children(graph, index);
    let font = trim_float(rung.container_label_px);
    // The same figure layout reserved. Declaring the rung constant instead
    // ruled the divider through the first row whenever a header wrapped.
    let band_px = layout::title_band_for(node, placement.width, rung);
    match &node.kind {
        NodeKind::Resource { azure_type } => icons::style_for(azure_type, rung),
        kind if has_children => {
            let (fill, stroke) = container_palette(kind);
            let band = trim_float(band_px);
            // `whiteSpace=wrap` is what stops a long subnet name — and its
            // address prefix — running clean over the neighbouring lane's
            // title. Without it draw.io lays the title out on one line at any
            // width.
            format!(
                "swimlane;html=1;startSize={band};rounded=1;arcSize=6;\
                 fillColor={fill};strokeColor={stroke};fontSize={font};fontStyle=0;\
                 verticalAlign=top;horizontal=1;collapsible=0;whiteSpace=wrap;"
            )
        }
        kind => {
            let (fill, stroke) = container_palette(kind);
            // The stencil shares this line with the label, so the label starts
            // clear of it — clear of its *drawn* width, which for a landscape
            // stencil like a VNet is two thirds again its height.
            let stencil = icons::header_style(kind)
                .map_or(0.0, |(_, aspect)| (rung.title_band + 4.0) * aspect);
            let indent = trim_float(HEADER_INSET * 2.0 + stencil);
            format!(
                "rounded=1;html=1;whiteSpace=wrap;fillColor={fill};\
                 strokeColor={stroke};verticalAlign=middle;fontSize={font};\
                 align=left;spacingLeft={indent};"
            )
        }
    }
}

/// Depth of every node in the containment tree, for the metrics that decay
/// with nesting.
fn depths(graph: &EstateGraph) -> Vec<usize> {
    graph
        .nodes
        .iter()
        .map(|node| {
            let mut depth = 0;
            let mut parent = node.parent;
            // Bounded like `route`'s walk: a cycle would otherwise hang here.
            while let Some(index) = parent {
                depth += 1;
                if depth > MAX_DEPTH {
                    break;
                }
                parent = graph.nodes[index].parent;
            }
            depth
        })
        .collect()
}

/// The container's own stencil, drawn in its title band.
///
/// A separate cell rather than a style on the swimlane: mxGraph paints an
/// `image=` only for image and label shapes, so a swimlane silently ignores
/// one. The cell is locked down so a reader dragging the diagram about cannot
/// pick the chrome up by accident.
fn header_cell(
    writer: &mut Writer<Vec<u8>>,
    index: usize,
    node: &super::graph::Node,
    placement: &Placement,
    rung: &Rung,
    banded: bool,
    prefix: &str,
) {
    let Some((style, aspect)) = icons::header_style(&node.kind) else {
        return;
    };
    // A swimlane hangs its stencil in the title band. A childless container —
    // a peering stub — has no band: its label is centred in the box, so the
    // stencil pairs with it on that line instead of stranding itself in a
    // corner half a box above the name it belongs to.
    // `size` is the stencil's height; its width follows from the aspect.
    let (size, y) = if banded {
        let size = (rung.title_band - 6.0).max(10.0);
        (size, (rung.title_band - size) / 2.0)
    } else {
        let size = (rung.title_band + 4.0).min(placement.height - 8.0);
        (size, (placement.height - size) / 2.0)
    };
    let drawn = size * aspect;
    let mut cell = BytesStart::new("mxCell");
    // `h{i}` parallels the frozen `n{i}`/`e{i}` scheme rather than renumbering
    // it: node cell ids are indices into `graph.nodes` and must stay that way.
    cell.push_attribute(("id", format!("{prefix}h{index}").as_str()));
    cell.push_attribute(("value", ""));
    cell.push_attribute(("style", style.as_str()));
    cell.push_attribute(("parent", format!("{prefix}n{index}").as_str()));
    cell.push_attribute(("vertex", "1"));
    with_element(writer, cell, |writer| {
        let mut geometry = BytesStart::new("mxGeometry");
        geometry.push_attribute(("x", trim_float(HEADER_INSET).as_str()));
        geometry.push_attribute(("y", trim_float(y).as_str()));
        geometry.push_attribute(("width", trim_float(drawn).as_str()));
        geometry.push_attribute(("height", trim_float(size).as_str()));
        geometry.push_attribute(("as", "geometry"));
        writer
            .write_event(Event::Empty(geometry))
            .expect("writing to Vec cannot fail");
    });
}

/// Size of the azdocs stamp on each sheet. The glyph is square.
const STAMP_SIZE: f64 = 28.0;
const STAMP_GAP: f64 = 24.0;

/// Provenance, bottom-left, once per sheet.
///
/// A sheet that is pasted into a deck otherwise carries nothing saying where it
/// came from. It is a stamp rather than a title block because the outermost
/// container already names the sheet, and a header would say it twice.
fn stamp_cell(writer: &mut Writer<Vec<u8>>, top: f64, prefix: &str) {
    // draw.io reads `data:image/svg+xml,<base64>`; the alphabet has no `;` or
    // `=` to be confused with the style syntax around it.
    //
    // The glyph is the image and the word is the cell's *label*, which is the
    // whole reason this is not the ready-made lockup asset: that one carries
    // its wordmark as vector outlines, and 4.7KB of them on every sheet more
    // than doubled a 54-sheet workbook. Rendered as text it is a few bytes.
    use base64::Engine as _;
    let encoded = base64::engine::general_purpose::STANDARD.encode(crate::mark::primary_svg());
    let style = format!(
        "image;aspect=fixed;html=1;points=[];\
         movable=0;resizable=0;deletable=0;connectable=0;editable=0;\
         labelPosition=right;align=left;verticalLabelPosition=middle;\
         verticalAlign=middle;spacingLeft=6;fontSize=12;fontStyle=1;\
         fontColor=#0754BD;image=data:image/svg+xml,{encoded}"
    );
    let mut cell = BytesStart::new("mxCell");
    cell.push_attribute(("id", format!("{prefix}mark").as_str()));
    cell.push_attribute(("value", "azdocs"));
    cell.push_attribute(("style", style.as_str()));
    cell.push_attribute(("parent", format!("{prefix}1").as_str()));
    cell.push_attribute(("vertex", "1"));
    with_element(writer, cell, |writer| {
        let mut geometry = BytesStart::new("mxGeometry");
        geometry.push_attribute(("x", "0"));
        geometry.push_attribute(("y", trim_float(top).as_str()));
        geometry.push_attribute(("width", trim_float(STAMP_SIZE).as_str()));
        geometry.push_attribute(("height", trim_float(STAMP_SIZE).as_str()));
        geometry.push_attribute(("as", "geometry"));
        writer
            .write_event(Event::Empty(geometry))
            .expect("writing to Vec cannot fail");
    });
}

/// Height of the legend row beside the stamp.
const LEGEND_ROW: f64 = 20.0;
const LEGEND_SWATCH: f64 = 28.0;
const LEGEND_LEFT: f64 = 60.0;

/// The same key the SVG draws: `(fill, stroke, dash, caption)` per container
/// kind present, then the count convention. Empty for a graph with nothing
/// to explain.
fn legend_entries(
    graph: &EstateGraph,
    labels: &DiagramLabels,
) -> Vec<(&'static str, &'static str, &'static str, String)> {
    let mut entries: Vec<_> = legend_kinds(graph)
        .into_iter()
        .map(|kind| {
            let (fill, stroke) = container_palette(&kind);
            let (label, dash) = super::svg::legend_words(&kind, labels);
            (fill, stroke, dash, label.to_owned())
        })
        .collect();
    if has_aggregates(graph) {
        entries.push((
            "#FFFFFF",
            "#0078D4",
            "",
            format!("×N — {}", labels.legend.aggregate),
        ));
    }
    entries
}

/// Legend swatches to the right of the stamp. Ids are a new `l{i}` family:
/// the frozen `n{i}`/`e{i}` scheme is untouched, and a workbook prefixes them
/// like every other cell.
fn legend_cells(
    writer: &mut Writer<Vec<u8>>,
    entries: &[(&str, &str, &str, String)],
    top: f64,
    prefix: &str,
) {
    let mut x = LEGEND_LEFT;
    for (index, (fill, stroke, dash, caption)) in entries.iter().enumerate() {
        let dashed = if dash.is_empty() {
            String::new()
        } else {
            format!("dashed=1;dashPattern={};", dash.replace(',', " "))
        };
        let style = format!(
            "rounded=1;html=1;whiteSpace=wrap;fillColor={fill};strokeColor={stroke};{dashed}\
             labelPosition=right;align=left;verticalLabelPosition=middle;verticalAlign=middle;\
             spacingLeft=4;fontSize=10;fontColor=#605E5C;movable=0;resizable=0;connectable=0;"
        );
        let mut cell = BytesStart::new("mxCell");
        cell.push_attribute(("id", format!("{prefix}l{index}").as_str()));
        cell.push_attribute(("value", caption.as_str()));
        cell.push_attribute(("style", style.as_str()));
        cell.push_attribute(("parent", format!("{prefix}1").as_str()));
        cell.push_attribute(("vertex", "1"));
        with_element(writer, cell, |writer| {
            let mut geometry = BytesStart::new("mxGeometry");
            geometry.push_attribute(("x", trim_float(x).as_str()));
            geometry.push_attribute(("y", trim_float(top + 4.0).as_str()));
            geometry.push_attribute(("width", trim_float(LEGEND_SWATCH).as_str()));
            geometry.push_attribute(("height", trim_float(LEGEND_ROW).as_str()));
            geometry.push_attribute(("as", "geometry"));
            writer
                .write_event(Event::Empty(geometry))
                .expect("writing to Vec cannot fail");
        });
        // Rough advance: the caption sits to the right of the swatch and the
        // next swatch must clear it. 6px per character at 10pt is generous.
        x += LEGEND_SWATCH + caption.chars().count() as f64 * 6.0 + 24.0;
    }
}

/// Extent of everything drawn, boxes and routed connectors alike.
fn content_bounds(absolute: &[Placement], routes: &[route::EdgeRoute]) -> (f64, f64) {
    let mut width = absolute
        .iter()
        .map(|p| p.x + p.width)
        .fold(0.0_f64, f64::max);
    let mut height = absolute
        .iter()
        .map(|p| p.y + p.height)
        .fold(0.0_f64, f64::max);
    for routed in routes {
        for (x, y) in &routed.points {
            width = width.max(*x);
            height = height.max(*y);
        }
    }
    (width, height)
}

/// Page the sheet declares, sized to the drawing plus a margin.
///
/// The floor used to be A4 landscape, which meant the median sheet filled a
/// third of its page and the drawing sat in a corner of a mostly empty
/// rectangle. The page here is the print grid, not a sheet of paper the
/// content has to be poured onto — hugging the drawing is what makes a
/// workbook of 54 sheets look deliberate.
fn page_size(width: f64, height: f64) -> (f64, f64) {
    const MARGIN: f64 = 40.0;
    const FLOOR: f64 = 240.0;
    (
        (width + MARGIN).ceil().max(FLOOR),
        (height + MARGIN).ceil().max(FLOOR),
    )
}

pub(crate) fn container_palette(kind: &NodeKind) -> (&'static str, &'static str) {
    match kind {
        NodeKind::Tenant => ("#FFFFFF", "#605E5C"),
        NodeKind::Subscription => ("#E8F1FA", "#0078D4"),
        NodeKind::ResourceGroup => ("#F3F2F1", "#8A8886"),
        NodeKind::Vnet => ("#EFF6FC", "#0078D4"),
        NodeKind::Subnet => ("#FFFFFF", "#8A8886"),
        NodeKind::Zone => ("#FDF6EC", "#D97706"),
        NodeKind::Resource { .. } => ("#FFFFFF", "#605E5C"),
    }
}

/// `exitX`/`exitY`/`entryX`/`entryY` for one end, from the side and the
/// distance along it that [`route`] chose.
///
/// The anchors are handed over rather than the routed points themselves: a
/// draw.io file exists to be edited, and hard waypoints go stale the moment a
/// reader drags a node, where an anchor keeps re-routing from the right side
/// of the box.
fn anchor_style(prefix: &str, (side, along): (route::Side, f64)) -> String {
    let (x, y) = side.fractions(along);
    format!(
        "{prefix}X={};{prefix}Y={};{prefix}Dx=0;{prefix}Dy=0;",
        trim_float(x),
        trim_float(y)
    )
}

/// Shared semantic peering colours. An unrecorded state cannot imply failure.
pub(crate) fn peering_color(label: Option<&str>) -> &'static str {
    match label.map(str::to_ascii_lowercase).as_deref() {
        Some("connected") => "#107C10",
        Some("disconnected" | "initiated") => "#D13438",
        _ => "#605E5C",
    }
}

fn edge_cell(
    writer: &mut Writer<Vec<u8>>,
    routed: &route::EdgeRoute,
    edge: &super::graph::DiagEdge,
    prefix: &str,
) {
    let base = match edge.style {
        EdgeStyle::Solid => {
            "edgeStyle=orthogonalEdgeStyle;rounded=1;html=1;endArrow=none;strokeColor=#605E5C;"
        }
        EdgeStyle::Dashed => {
            "edgeStyle=orthogonalEdgeStyle;rounded=1;html=1;dashed=1;endArrow=none;\
             startArrow=none;fontSize=10;"
        }
        EdgeStyle::Association => {
            "edgeStyle=orthogonalEdgeStyle;rounded=1;html=1;dashed=1;dashPattern=1 3;\
             endArrow=open;strokeColor=#8A8886;fontSize=10;"
        }
    };
    let style = format!(
        "{base}{}{}{}",
        if edge.style == EdgeStyle::Dashed {
            format!("strokeColor={};", peering_color(edge.label.as_deref()))
        } else {
            String::new()
        },
        anchor_style("exit", routed.source_anchor),
        anchor_style("entry", routed.target_anchor)
    );
    let mut cell = BytesStart::new("mxCell");
    let id = format!("{prefix}e{}", routed.edge);
    cell.push_attribute(("id", id.as_str()));
    if let Some(label) = &edge.label {
        cell.push_attribute(("value", label.as_str()));
    }
    cell.push_attribute(("style", style.as_str()));
    // Cross-container edges live on the root layer; draw.io resolves the
    // endpoints by cell id.
    cell.push_attribute(("parent", format!("{prefix}1").as_str()));
    cell.push_attribute(("source", format!("{prefix}n{}", edge.source).as_str()));
    cell.push_attribute(("target", format!("{prefix}n{}", edge.target).as_str()));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn labels() -> DiagramLabels {
        crate::labels::Labels::default().diagram
    }

    fn map_sheet<'a>(labels: &'a [ImageLabel], caption: Option<&'a str>) -> ImageSheet<'a> {
        ImageSheet {
            name: "Regions",
            svg: "<svg xmlns=\"http://www.w3.org/2000/svg\"/>",
            width: 1200.0,
            height: 600.0,
            labels,
            caption,
            colors: LabelColors {
                fill: "#ffffff",
                stroke: "#d6e1eb",
                text: "#14233a",
                muted: "#53677e",
            },
        }
    }

    fn point(text: &str, x: f64, y: f64) -> ImageLabel {
        ImageLabel {
            text: text.to_owned(),
            x,
            y,
            radius: 8.0,
        }
    }

    #[test]
    fn unit_image_sheet_follows_the_graph_sheets_with_the_next_ids() {
        let graph = graph(1);
        let points = [point("UK South · 3", 580.0, 150.0)];

        let xml = render_workbook_with(
            &[("Network", &graph)],
            &[map_sheet(&points, Some("Not on the map: Global · 1"))],
            DiagramDetail::Full,
            &labels(),
        );

        assert!(xml.contains(r#"name="Regions" id="azdocs-1""#), "{xml}");
        assert!(xml.contains(r#"id="s1-image""#));
        assert!(xml.contains("image=data:image/svg+xml,"));
        assert!(xml.contains(r#"id="s1-label0" value="UK South · 3""#));
        assert!(xml.contains(r#"id="s1-caption" value="Not on the map: Global · 1""#));
    }

    #[test]
    fn unit_labels_sit_beside_their_point_when_there_is_room() {
        let points = [point("UK South · 3", 580.0, 150.0)];

        let placed = place_labels(&map_sheet(&points, None));

        let rect = placed[0].rect;
        assert!(!placed[0].leader);
        assert_eq!(rect.x, 580.0 + 8.0 + LABEL_GAP, "to the right");
        assert_eq!(rect.y + rect.height / 2.0, 150.0, "centred on the point");
    }

    #[test]
    fn unit_crowded_labels_never_overlap_each_other_or_a_point() {
        // A northern-Europe cluster, as close as the real regions sit.
        let points = [
            point("UK South · 255", 580.0, 150.0),
            point("West Europe · 11", 600.0, 140.0),
            point("Sweden Central · 2", 610.0, 110.0),
            point("North Europe · 4", 570.0, 135.0),
        ];
        let sheet = map_sheet(&points, None);

        let placed = place_labels(&sheet);

        for (a, first) in placed.iter().enumerate() {
            assert!(first.rect.inside(sheet.width, sheet.height));
            for second in &placed[a + 1..] {
                assert!(!first.rect.overlaps(&second.rect), "label {a} overlaps");
            }
            for p in &points {
                let clearance = Rect::around(p.x, p.y, p.radius);
                assert!(!first.rect.overlaps(&clearance), "label {a} covers a point");
            }
        }
    }

    #[test]
    fn unit_a_label_at_the_right_edge_turns_inward() {
        let points = [point("East US · 2", 1195.0, 300.0)];

        let placed = place_labels(&map_sheet(&points, None));

        assert!(placed[0].rect.x + placed[0].rect.width <= 1200.0);
    }
    use crate::diagram::graph::LayoutMode;
    use crate::diagram::graph::{DiagEdge, Node};

    /// A VNet holding one subnet with `leaves` resources in it, plus a second
    /// VNet, so there is something for the layout to place side by side.
    fn graph(leaves: usize) -> EstateGraph {
        let mut nodes = vec![
            Node {
                label: "vnet-app".into(),
                sublabel: None,
                kind: NodeKind::Vnet,
                parent: None,
            },
            Node {
                label: "snet-a-very-long-subnet-name".into(),
                sublabel: Some("10.1.0.0/24".into()),
                kind: NodeKind::Subnet,
                parent: Some(0),
            },
            Node {
                label: "vnet-hub".into(),
                sublabel: None,
                kind: NodeKind::Vnet,
                parent: None,
            },
        ];
        for index in 0..leaves {
            nodes.push(Node {
                label: format!("vm-{index:03}"),
                sublabel: Some("Virtual Machine".into()),
                kind: NodeKind::Resource {
                    azure_type: "microsoft.compute/virtualmachines".into(),
                },
                parent: Some(1),
            });
        }
        EstateGraph {
            title: "test".into(),
            nodes,
            edges: Vec::new(),
            layout: LayoutMode::default(),
        }
    }

    fn page_width(xml: &str) -> f64 {
        let rest = xml.split_once("pageWidth=\"").expect("a page width").1;
        rest.split_once('"')
            .expect("a closing quote")
            .0
            .parse()
            .expect("a numeric page width")
    }

    /// The emitter laid out at the A4 *portrait* text column (680px) while
    /// declaring an A4 *landscape* page, so every sheet was a narrow ribbon
    /// with half the page blank beside it.
    #[test]
    fn unit_a_full_detail_sheet_is_wider_than_a_summary_one() {
        let graph = graph(4);

        let summary = page_width(&render_for(&graph, DiagramDetail::Summary, &labels()));
        let full = page_width(&render_for(&graph, DiagramDetail::Full, &labels()));

        assert!(
            full > summary,
            "full {full} did not exceed summary {summary}"
        );
    }

    #[test]
    fn unit_the_page_is_sized_to_the_drawing_rather_than_to_a4() {
        let wide = page_width(&render_for(&graph(24), DiagramDetail::Full, &labels()));

        assert!(wide > 1169.0, "still clipped to A4 landscape: {wide}");
    }

    /// Without wrapping, draw.io lays a swimlane title out on one line at any
    /// width, so neighbouring subnet titles printed over each other.
    #[test]
    fn unit_container_titles_wrap() {
        let xml = render_for(&graph(2), DiagramDetail::Full, &labels());

        for style in xml
            .split("style=\"")
            .skip(1)
            .filter(|s| s.starts_with("swimlane"))
        {
            assert!(style.contains("whiteSpace=wrap;"), "unwrapped: {style}");
        }
    }

    /// A fixed 64px glyph was taller than a whole dense slot, so its label ran
    /// over the row beneath it.
    #[test]
    fn unit_the_glyph_never_outgrows_its_layout_slot() {
        let xml = render_for(&graph(64), DiagramDetail::Full, &labels());
        let rung = layout::rung(&graph(64));

        // Container header icons are `image;` cells too, so match the
        // resource stencil rather than the first image in the document.
        let icon = xml
            .split("azure2/compute/")
            .skip(1)
            .filter_map(|cell| cell.split_once("height=\"")?.1.split_once('"'))
            .map(|(value, _)| value.parse::<f64>().expect("a numeric height"))
            .next()
            .expect("a resource cell");

        assert_eq!(icon, rung.icon);
        assert!(icon <= rung.leaf_height, "{icon} overflows the slot");
    }

    #[test]
    fn unit_edges_anchor_to_the_side_the_router_chose() {
        let mut graph = graph(2);
        graph.edges.push(DiagEdge {
            source: 3,
            target: 4,
            label: None,
            style: EdgeStyle::Solid,
        });

        let xml = render_for(&graph, DiagramDetail::Full, &labels());

        let style = xml
            .split("style=\"")
            .find(|s| s.starts_with("edgeStyle"))
            .expect("an edge cell");
        assert!(style.contains("exitX="), "no exit anchor: {style}");
        assert!(style.contains("entryX="), "no entry anchor: {style}");
    }

    /// draw.io drew a containment edge as a line from a container to a box
    /// already inside it. `route` drops them; the emitter follows.
    #[test]
    fn unit_a_containment_edge_is_not_drawn() {
        let mut graph = graph(1);
        graph.edges.push(DiagEdge {
            source: 1,
            target: 3,
            label: None,
            style: EdgeStyle::Solid,
        });

        let xml = render_for(&graph, DiagramDetail::Full, &labels());

        assert!(
            !xml.contains("edge=\"1\""),
            "drew a containment edge: {xml}"
        );
    }

    /// A swimlane ignores an `image=` in its own style, so the stencil is a
    /// child cell — one that must not renumber the frozen `n{i}` scheme.
    #[test]
    fn unit_a_container_carries_its_stencil_without_taking_a_node_id() {
        let xml = render_for(&graph(2), DiagramDetail::Full, &labels());

        assert!(
            xml.contains("azure2/networking/Virtual_Networks.svg"),
            "no vnet stencil: {xml}"
        );
        assert!(xml.contains("id=\"h0\""), "stencil took a node id: {xml}");
        assert!(xml.contains("id=\"n0\""), "node id was renumbered: {xml}");
    }

    /// The band draw.io declares has to be the one layout reserved. Too small
    /// and a wrapped header prints across the content; too large and the
    /// divider is ruled through the first row of children — draw.io places a
    /// swimlane's children by their own geometry, whatever `startSize` says.
    #[test]
    fn unit_no_swimlane_rules_its_divider_through_a_child() {
        // A long name and a CIDR, which is the pair that wraps in a narrow
        // column and was printing below the band.
        let mut graph = graph(6);
        graph.nodes[1].label = "snet-n4-corp-dwh-shared-dev-uks".into();
        graph.nodes[1].sublabel = Some("10.221.41.64/26".into());
        let xml = render_for(&graph, DiagramDetail::Full, &labels());

        let mut bands: Vec<(String, f64)> = Vec::new();
        let mut children: Vec<(String, f64)> = Vec::new();
        for cell in xml.split("<mxCell ").skip(1) {
            let id = field(cell, "id=\"").expect("an id");
            if let Some(style) = cell.split_once("style=\"").map(|(_, rest)| rest)
                && style.starts_with("swimlane")
                && let Some(size) = field(cell, "startSize=")
            {
                bands.push((
                    id.clone(),
                    size.trim_end_matches(';').parse().expect("a band"),
                ));
            }
            // `h{i}` is the stencil that sits *in* the band by design.
            if !id.starts_with('h')
                && let (Some(parent), Some(y)) = (field(cell, "parent=\""), field(cell, "y=\""))
            {
                children.push((parent, y.parse().expect("a y")));
            }
        }

        assert!(!bands.is_empty(), "no swimlane rendered: {xml}");
        for (id, band) in &bands {
            for (parent, y) in &children {
                if parent == id {
                    assert!(
                        *y >= *band,
                        "child at y={y} sits inside {id}'s {band}px band"
                    );
                }
            }
        }
    }

    /// Read `key`'s value up to the next `"` or `;`, whichever ends it.
    fn field(cell: &str, key: &str) -> Option<String> {
        let rest = cell.split_once(key)?.1;
        let end = rest.find(['"', ';']).unwrap_or(rest.len());
        Some(rest[..end].to_owned())
    }

    #[test]
    fn unit_every_sheet_carries_the_product_mark() {
        let xml = render_for(&graph(2), DiagramDetail::Full, &labels());

        assert!(xml.contains("value=\"azdocs\""), "no wordmark: {xml}");
        assert!(xml.contains("image=data:image/svg+xml,"), "no glyph: {xml}");
    }

    /// The stamp sits below the drawing, so the page has to grow to hold it.
    #[test]
    fn unit_the_page_makes_room_for_the_stamp() {
        let graph = graph(24);
        let placements =
            layout::absolutize(&graph, &layout::layout_for(&graph, DiagramDetail::Full));
        let content = placements
            .iter()
            .map(|p| p.y + p.height)
            .fold(0.0_f64, f64::max);

        let xml = render_for(&graph, DiagramDetail::Full, &labels());
        let height: f64 = xml
            .split_once("pageHeight=\"")
            .expect("a page height")
            .1
            .split_once('"')
            .expect("a closing quote")
            .0
            .parse()
            .expect("a numeric page height");

        assert!(height > content + STAMP_SIZE, "stamp clipped: {height}");
    }

    /// XML normalises a literal newline inside an attribute value to a space,
    /// so a `\n` between a resource's name and its type never reached draw.io
    /// as a break — it silently became a run-on that word-wrapped.
    #[test]
    fn unit_a_leaf_label_breaks_between_its_name_and_its_type() {
        let xml = render_for(&graph(2), DiagramDetail::Full, &labels());

        assert!(xml.contains("&lt;br&gt;"), "no line break emitted: {xml}");
        for value in xml
            .split("value=\"")
            .skip(1)
            .filter_map(|v| v.split_once('"'))
        {
            assert!(
                !value.0.contains('\n'),
                "raw newline in an attribute: {:?}",
                value.0
            );
        }
    }

    /// draw.io wraps at `labelWidth` and never stops, so a long Azure name ran
    /// to five lines in a slot with room for two and printed over the tiles
    /// beneath it.
    #[test]
    fn unit_a_long_name_is_capped_to_the_room_under_its_glyph() {
        for rung in [
            crate::diagram::page::COMFORTABLE,
            crate::diagram::page::COMPACT,
            crate::diagram::page::DENSE,
        ] {
            let fitted = fit_label(
                "AzureBackup_vm-mongodb-surveil-prod-uks-001_8566620548315644073",
                &rung,
            );
            let per_line = (rung.leaf_width / (rung.label_px * 0.55)).floor();
            let lines = (fitted.chars().count() as f64 / per_line).ceil();
            let room = rung.leaf_height - rung.icon - 4.0;

            // Two halves are printed, so one half may claim half the room.
            assert!(
                lines * rung.label_px * 1.25 <= room / 2.0 + rung.label_px,
                "{} overruns at {}: {lines} lines in {room}px",
                fitted,
                rung.name
            );
        }
    }

    /// The page floor was A4 landscape, so a short sheet filled a third of it
    /// and the drawing sat along the top of an empty rectangle. Width still
    /// spans the canvas — layout justifies to it — but height must not.
    #[test]
    fn unit_the_page_hugs_a_short_drawing() {
        let xml = render_for(&graph(2), DiagramDetail::Full, &labels());
        let height: f64 = xml
            .split_once("pageHeight=\"")
            .expect("a page height")
            .1
            .split_once('"')
            .expect("a closing quote")
            .0
            .parse()
            .expect("a numeric page height");

        assert!(height < 826.0, "still floored at A4 landscape: {height}");
    }
}
