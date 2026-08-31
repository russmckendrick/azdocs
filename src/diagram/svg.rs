//! Self-contained SVG rendering of an `EstateGraph`: shares the draw.io
//! layout (via `absolutize`) and palette so both emitters agree on shape,
//! with icons embedded as data URIs so the file needs no external assets.

use std::fmt::Write as _;

use super::drawio::container_palette;
use super::graph::{DiagEdge, EdgeStyle, EstateGraph, NodeKind, truncate_label};
use super::icons;
use super::layout::{self, Placement};
use super::page::{A4_PORTRAIT, DiagramDetail, PageFraction, Rung};
use super::route;

const MARGIN: f64 = 16.0;
/// Vertical room above the content for the diagram title, on the exports that
/// draw one ([`DiagramDetail::shows_title`]).
const TITLE_BAND: f64 = 40.0;
/// Corner radius on connectors. Matches what draw.io's own `rounded=1` draws,
/// so the two emitters finally agree.
const CORNER: f64 = 8.0;
/// Room below the content for the border-convention key.
const LEGEND_BAND: f64 = 26.0;
const FONT: &str = "Arial, Helvetica, sans-serif";
const TEXT_PRIMARY: &str = "#323130";
const TEXT_SECONDARY: &str = "#605E5C";
const PEERING_CONNECTED: &str = "#107C10";
const PEERING_DISCONNECTED: &str = "#D13438";

/// Render the graph as a standalone SVG document.
pub fn render(graph: &EstateGraph) -> String {
    render_for(graph, DiagramDetail::default())
}

/// Render at a given detail level. A summary is snapped to a share of an A4
/// page; a full export keeps whatever canvas its content needs.
pub fn render_for(graph: &EstateGraph, detail: DiagramDetail) -> String {
    let rung = layout::rung(graph);
    let placements = layout::absolutize(graph, &layout::layout_for(graph, detail));
    let routes = route::route(graph, &placements, &rung);
    // Routes escape past a node edge, so the canvas has to include them or the
    // viewBox stops matching what the rasteriser draws.
    let content = bounds(&placements, &routes);
    let (content_width, content_height) = (content.width, content.height);

    // A report diagram is the width of the text column, full stop: the layout
    // has already justified its rows to the page width, so the width decides
    // the scale and the height simply follows the content. Snapping the height
    // up to a fixed share of the sheet instead — as this used to — meant the
    // height, not the width, set the scale, and every diagram ended up a
    // shrunken block adrift in white space.
    let top_band = if detail.shows_title() {
        TITLE_BAND
    } else {
        MARGIN
    };
    // A graph with no boundaries to explain — a resource neighbourhood — gets
    // no key, so it must not pay for the band either.
    let keys = legend_entries(graph);
    let legend_band = if keys.is_empty() { 0.0 } else { LEGEND_BAND };
    let width = if detail.snaps_to_page() {
        A4_PORTRAIT.width
    } else {
        content_width + 2.0 * MARGIN
    };
    let inner_width = width - 2.0 * MARGIN;
    // The one thing the content may not do is outgrow the sheet; past that it
    // scales down, which is the only case that leaves a horizontal margin.
    let ceiling = A4_PORTRAIT.height - top_band - legend_band - MARGIN;
    let scale = if detail.snaps_to_page() {
        (inner_width / content_width).min(ceiling / content_height)
    } else {
        1.0
    };
    let height = (top_band + content_height * scale + legend_band + MARGIN).round();
    // Centre horizontally; the content hangs from the top so the title band
    // and the legend keep their fixed positions on the canvas.
    // `content.x`/`content.y` are zero or negative: subtracting them brings an
    // upward- or leftward-escaping connector back inside the canvas.
    let offset_x = MARGIN + (inner_width - content_width * scale) / 2.0 - content.x * scale;
    let offset_y = top_band - content.y * scale;

    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" font-family="{FONT}" data-page-fraction="{f}">"#,
        w = fmt(width),
        h = fmt(height),
        f = if detail.snaps_to_page() {
            PageFraction::for_height(height).label()
        } else {
            "full"
        },
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
    if detail.shows_title() {
        let _ = writeln!(
            out,
            r#"  <text x="{}" y="28" font-size="18" font-weight="bold" text-anchor="middle" fill="{TEXT_PRIMARY}">{}</text>"#,
            fmt(width / 2.0),
            escape(&graph.title),
        );
    }
    let _ = writeln!(
        out,
        r#"  <g transform="translate({},{}) scale({:.4})">"#,
        fmt(offset_x),
        fmt(offset_y),
        scale,
    );

    // Containers first (parents precede children by construction), then
    // edges, then icons — so edges run under icons but over containers.
    for (index, node) in graph.nodes.iter().enumerate() {
        if node.kind.is_container() {
            container(&mut out, graph, index, &placements[index], &rung);
        }
    }
    for routed in &routes {
        edge_line(&mut out, &graph.edges[routed.edge], routed);
    }
    for (index, node) in graph.nodes.iter().enumerate() {
        if let NodeKind::Resource { azure_type } = &node.kind {
            resource_icon(&mut out, node, azure_type, &placements[index], &rung);
        }
    }

    out.push_str("  </g>\n");
    // The legend is canvas furniture, so it is drawn outside the scaled group
    // and stays the same size whatever the content had to shrink to.
    legend(&mut out, graph, &keys, width, height);

    out.push_str("</svg>\n");
    out
}

/// Border conventions this graph actually uses: `(stroke, label, dash)`. A
/// simple diagram is not captioned with boundaries it does not draw.
fn legend_entries(graph: &EstateGraph) -> Vec<(&'static str, &'static str, &'static str)> {
    [
        (NodeKind::Vnet, "Virtual network", "7,4"),
        (NodeKind::Subnet, "Subnet", "4,3"),
        (NodeKind::Zone, "Outside the topology", "7,4"),
    ]
    .iter()
    .filter(|(kind, _, _)| graph.nodes.iter().any(|node| &node.kind == kind))
    .map(|(kind, label, dash)| (container_palette(kind).1, *label, *dash))
    .collect()
}

/// Key to the border conventions, plus the count convention and the Azure mark.
fn legend(
    out: &mut String,
    graph: &EstateGraph,
    entries: &[(&str, &str, &str)],
    canvas_width: f64,
    canvas_height: f64,
) {
    if entries.is_empty() {
        return;
    }

    let y = canvas_height - 10.0;
    let mut x = MARGIN + 4.0;
    for (stroke, label, dash) in entries {
        let _ = writeln!(
            out,
            r#"    <rect x="{}" y="{}" width="22" height="12" rx="2" fill="none" stroke="{stroke}" stroke-width="1.5" stroke-dasharray="{dash}"/>"#,
            fmt(x),
            fmt(y - 9.0),
        );
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}" font-size="10" fill="{TEXT_SECONDARY}">{}</text>"#,
            fmt(x + 28.0),
            fmt(y),
            escape(label),
        );
        x += 28.0 + text_width(label, 10.0) + 22.0;
    }
    // The count convention only needs explaining when a tile actually carries one.
    if graph
        .nodes
        .iter()
        .any(|node| node.sublabel.as_deref().is_some_and(|s| s.starts_with('×')))
    {
        let _ = writeln!(
            out,
            r##"    <text x="{}" y="{}" font-size="10" fill="{TEXT_SECONDARY}"><tspan fill="#0078D4">×N</tspan>  aggregated resources of one type</text>"##,
            fmt(x),
            fmt(y),
        );
    }
    let _ = writeln!(
        out,
        r##"    <text x="{}" y="{}" font-size="10" text-anchor="end" fill="#0078D4">Microsoft Azure</text>"##,
        fmt(canvas_width - MARGIN),
        fmt(y),
    );
}

/// Extent of everything drawn, as a rectangle rather than a size.
///
/// A connector that escapes up or left of the content — often the only clear
/// channel between two boxes sharing a row — runs at a negative coordinate.
/// Measuring the maximum alone left it outside the viewBox; the title band
/// happened to absorb the overhang at the top, so only a leftward escape
/// actually clipped, and then silently, in the PDF, DOCX and HTML that take
/// their diagrams from this SVG. Measuring both corners makes it a guarantee
/// rather than a coincidence.
fn bounds(placements: &[Placement], routes: &[route::EdgeRoute]) -> Placement {
    let mut left = 0.0_f64;
    let mut top = 0.0_f64;
    let mut right = 0.0_f64;
    let mut bottom = 0.0_f64;
    for p in placements {
        left = left.min(p.x);
        top = top.min(p.y);
        right = right.max(p.x + p.width);
        bottom = bottom.max(p.y + p.height);
    }
    for routed in routes {
        for (x, y) in &routed.points {
            left = left.min(*x);
            top = top.min(*y);
            right = right.max(*x);
            bottom = bottom.max(*y);
        }
    }
    Placement {
        x: left,
        y: top,
        width: (right - left).max(360.0).ceil(),
        height: (bottom - top).max(120.0).ceil(),
    }
}

fn container(
    out: &mut String,
    graph: &EstateGraph,
    index: usize,
    placement: &Placement,
    rung: &Rung,
) {
    let node = &graph.nodes[index];
    let (fill, stroke) = container_palette(&node.kind);
    // Every logical boundary is dashed, each kind with its own rhythm, so the
    // legend can name them: a reader tells a VNet from a subnet from an
    // unnetworked zone by the border alone.
    let dash = match node.kind {
        NodeKind::Subnet => r#" stroke-dasharray="4,3""#,
        NodeKind::Vnet | NodeKind::Zone => r#" stroke-dasharray="7,4""#,
        NodeKind::ResourceGroup => r#" stroke-dasharray="6,5""#,
        _ => "",
    };
    let _ = writeln!(
        out,
        r#"    <rect x="{}" y="{}" width="{}" height="{}" rx="6" fill="{fill}" stroke="{stroke}" stroke-width="1.5"{dash}/>"#,
        fmt(placement.x),
        fmt(placement.y),
        fmt(placement.width),
        fmt(placement.height),
    );
    let has_children = graph.nodes.iter().any(|n| n.parent == Some(index));
    let font = rung.container_label_px;
    // An empty subnet is a strip: name left, CIDR right, the same reading
    // order as a populated one. Centring it would imply content that isn't there.
    if !has_children && node.kind == NodeKind::Subnet {
        let baseline = placement.y + placement.height / 2.0 + font / 2.0 - 1.0;
        let band_end = placement.x + placement.width - 12.0;
        let reserved = node
            .sublabel
            .as_deref()
            .map_or(0.0, |value| text_width(value, font - 1.0) + 12.0);
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}" font-size="{}" fill="{TEXT_PRIMARY}">{}</text>"#,
            fmt(placement.x + 12.0),
            fmt(baseline),
            fmt(font),
            escape(&ellipsize(
                &node.label,
                font,
                placement.width - 24.0 - reserved
            )),
        );
        if let Some(sublabel) = &node.sublabel {
            let _ = writeln!(
                out,
                r#"    <text x="{}" y="{}" font-size="{}" text-anchor="end" fill="{TEXT_SECONDARY}">{}</text>"#,
                fmt(band_end),
                fmt(baseline),
                fmt(font - 1.0),
                escape(sublabel),
            );
        }
        return;
    }

    if !has_children {
        // A resource-group tile or peering stub: a plain box with its name
        // centred. The name is wrapped and clipped to the box — Azure group
        // names run to forty characters, and drawn as one unbroken line they
        // ran clean across their neighbours.
        let center_x = placement.x + placement.width / 2.0;
        let (font, lines) = fit_lines(
            &truncate_label(&node.label),
            font,
            placement.width - 16.0,
            stub_label_lines(placement.height, font, node.sublabel.is_some()),
        );
        let line_height = font + 2.0;
        // Centre the whole block — name lines plus the count beneath them.
        let block =
            lines.len() as f64 * line_height + if node.sublabel.is_some() { font } else { 0.0 };
        let mut baseline = placement.y + (placement.height - block) / 2.0 + font;
        for line in &lines {
            let _ = writeln!(
                out,
                r#"    <text x="{}" y="{}" font-size="{}" font-weight="bold" text-anchor="middle" fill="{TEXT_PRIMARY}">{}</text>"#,
                fmt(center_x),
                fmt(baseline),
                fmt(font),
                escape(line),
            );
            baseline += line_height;
        }
        if let Some(sublabel) = &node.sublabel {
            let _ = writeln!(
                out,
                r#"    <text x="{}" y="{}" font-size="{}" text-anchor="middle" fill="{TEXT_SECONDARY}">{}</text>"#,
                fmt(center_x),
                fmt(baseline - 2.0),
                fmt(font - 2.0),
                escape(sublabel),
            );
        }
        return;
    }

    // Populated containers label top-left, the way the Azure reference
    // topologies do — a centred header competes with the content beneath it.
    // The band reads "[icon] Kind · name .......... CIDR".
    let mut cursor = placement.x + 12.0;
    let baseline = placement.y + font + 8.0;
    let colour = container_label_colour(&node.kind);
    let band_end = placement.x + placement.width - 12.0;

    if let Some(glyph) = header_icon(&node.kind) {
        let size = font + 3.0;
        let _ = writeln!(
            out,
            r#"    <image x="{}" y="{}" width="{}" height="{}" href="{}"/>"#,
            fmt(cursor),
            fmt(baseline - size + 2.0),
            fmt(size),
            fmt(size),
            icons::svg_data_uri(glyph),
        );
        cursor += size + 7.0;
    }
    // The kind reads as a quiet prefix so the name is what the eye lands on.
    if let Some(prefix) = header_prefix(&node.kind) {
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}" font-size="{}" fill="{TEXT_SECONDARY}">{} ·</text>"#,
            fmt(cursor),
            fmt(baseline),
            fmt(font),
            escape(prefix),
        );
        cursor += text_width(prefix, font) + font;
    }

    // The CIDR or count sits at the far right of the same band and the name
    // gets the rest. The band is for the name, so the right-hand half is
    // capped at a third and its own type shrinks to fit that — reserving less
    // room than the text needs is what let the two overprint each other.
    let reserved = (band_end - cursor) * 0.4;
    let (sublabel_font, sublabel_text) = node.sublabel.as_deref().map_or((0.0, None), |value| {
        let mut size = font - 1.0;
        while size > MIN_LABEL_PX && text_width(value, size) > reserved {
            size -= 1.0;
        }
        (size, Some(ellipsize(value, size, reserved)))
    });
    let name_end = band_end
        - sublabel_text
            .as_deref()
            .map_or(0.0, |value| text_width(value, sublabel_font) + 12.0);
    // The name shrinks to stay whole before it is allowed to be cut: a
    // subscription band is narrow in an estate-wide diagram, and
    // "node4-datawareho…" tells a reader less than the same name two points
    // smaller.
    let name_font = fit_font(&node.label, font, name_end - cursor);
    let _ = writeln!(
        out,
        r#"    <text x="{}" y="{}" font-size="{}" font-weight="bold" fill="{colour}">{}</text>"#,
        fmt(cursor),
        fmt(baseline),
        fmt(name_font),
        escape(&ellipsize(&node.label, name_font, name_end - cursor)),
    );
    if let Some(sublabel) = sublabel_text {
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}" font-size="{}" text-anchor="end" fill="{TEXT_SECONDARY}">{}</text>"#,
            fmt(band_end),
            fmt(baseline),
            fmt(sublabel_font),
            escape(&sublabel),
        );
    }
}

/// Azure glyph shown at the head of a container band, or `None` where the
/// reference topologies show none (a subnet is identified by its border).
fn header_icon(kind: &NodeKind) -> Option<&'static str> {
    match kind {
        NodeKind::ResourceGroup => Some("microsoft.resources/resourcegroups"),
        NodeKind::Vnet => Some("microsoft.network/virtualnetworks"),
        NodeKind::Subscription => Some("microsoft.resources/subscriptions"),
        _ => None,
    }
}

/// Quiet kind prefix before the name. A resource group and a subnet carry
/// their name alone — the icon and the border already say what they are.
fn header_prefix(kind: &NodeKind) -> Option<&'static str> {
    match kind {
        NodeKind::Vnet => Some("Virtual network"),
        _ => None,
    }
}

fn container_label_colour(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Vnet => "#0078D4",
        NodeKind::Zone => "#B45309",
        _ => TEXT_PRIMARY,
    }
}

fn resource_icon(
    out: &mut String,
    node: &super::graph::Node,
    azure_type: &str,
    placement: &Placement,
    rung: &Rung,
) {
    let center_x = placement.x + placement.width / 2.0;
    let icon = route::visual_box(placement, true, rung);
    let _ = writeln!(
        out,
        r#"    <image x="{}" y="{}" width="{}" height="{}" href="{}"/>"#,
        fmt(icon.x),
        fmt(icon.y),
        fmt(icon.width),
        fmt(icon.height),
        icons::svg_data_uri(azure_type),
    );
    let line_height = rung.label_px + 2.0;
    let mut line_y = icon.y + icon.height + rung.label_px + 3.0;
    for line in wrap_label(&truncate_label(&node.label), rung) {
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}" font-size="{}" text-anchor="middle" fill="{TEXT_PRIMARY}">{}</text>"#,
            fmt(center_x),
            fmt(line_y),
            fmt(rung.label_px),
            escape(&line),
        );
        line_y += line_height;
    }
    // The sublabel carries the count for an aggregate tile and the resource
    // name for a singleton; it is clipped to the slot so a long Azure name
    // cannot bleed across its neighbours.
    if let Some(sublabel) = &node.sublabel {
        let font = rung.label_px - 2.0;
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}" font-size="{}" text-anchor="middle" fill="{TEXT_SECONDARY}">{}</text>"#,
            fmt(center_x),
            fmt(line_y),
            fmt(font),
            escape(&ellipsize(sublabel, font, placement.width - 4.0)),
        );
    }
}

fn edge_line(out: &mut String, edge: &DiagEdge, routed: &route::EdgeRoute) {
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
        r#"    <path d="{}" fill="none" stroke="{stroke}" stroke-width="1.5" stroke-linejoin="round" stroke-linecap="round"{dash}{marker}/>"#,
        route::svg_path(&routed.points, CORNER),
    );
    if let Some(label) = &edge.label
        && let Some((x, y)) = routed.label_at
    {
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}" font-size="10" text-anchor="middle" fill="{TEXT_SECONDARY}">{}</text>"#,
            fmt(x),
            fmt(y),
            escape(label),
        );
    }
}

/// Approximate rendered width. Character count rather than a measured advance,
/// because the rasteriser resolves fonts from the system database and the
/// emitter cannot know which face will actually be used.
fn text_width(value: &str, font_px: f64) -> f64 {
    value.chars().count() as f64 * font_px * 0.55
}

/// Shorten to fit `limit` pixels, marking the cut.
fn ellipsize(value: &str, font_px: f64, limit: f64) -> String {
    if text_width(value, font_px) <= limit {
        return value.to_owned();
    }
    let budget = ((limit / (font_px * 0.55)).floor() as usize).saturating_sub(1);
    if budget == 0 {
        return String::new();
    }
    let mut out: String = value.chars().take(budget).collect();
    out.push('…');
    out
}

/// How many name lines a childless container has room for above its count.
fn stub_label_lines(height: f64, font: f64, has_sublabel: bool) -> usize {
    let reserved = if has_sublabel { font + 4.0 } else { 0.0 };
    (((height - 12.0 - reserved) / (font + 2.0)).floor() as usize).clamp(1, 3)
}

/// Floor on a fitted label. Past this the name is unreadable on paper, so a
/// cut is the better trade.
const MIN_LABEL_PX: f64 = 8.0;

/// Largest size at or below `font` whose wrap fits the box whole, and the lines
/// it produces.
///
/// The name is the only thing on a resource-group tile, and Azure group names
/// run past thirty characters: dropping a couple of points to keep one whole
/// beats printing `rg-app-auto-…` on two tiles that differ only in what was
/// cut.
/// Largest size at or below `font` that keeps `value` inside `width` on one
/// line, floored at [`MIN_LABEL_PX`].
fn fit_font(value: &str, font: f64, width: f64) -> f64 {
    let mut size = font;
    while size > MIN_LABEL_PX && text_width(value, size) > width {
        size -= 1.0;
    }
    size
}

fn fit_lines(value: &str, font: f64, width: f64, lines: usize) -> (f64, Vec<String>) {
    let mut smallest = None;
    let mut size = font;
    while size >= MIN_LABEL_PX {
        let wrapped = wrap_to_width(value, size, width, lines);
        if !wrapped.iter().any(|line| line.ends_with('…')) {
            return (size, wrapped);
        }
        smallest = Some((size, wrapped));
        size -= 1.0;
    }
    smallest.unwrap_or_else(|| (font, wrap_to_width(value, font, width, lines)))
}

/// Wrap to the rung's line budget, breaking at word boundaries.
fn wrap_label(value: &str, rung: &Rung) -> Vec<String> {
    wrap_to_width(
        value,
        rung.label_px,
        rung.wrap_chars as f64 * rung.label_px * 0.55,
        rung.wrap_lines,
    )
}

/// Wrap to `width` px in at most `lines`, breaking at word boundaries and
/// marking any cut.
///
/// Chunking by character count split "Log Analytics Workspace" into
/// "Log Analytics W" / "orkspace"; Azure type names are prose and hyphenated
/// resource names have natural break points, so both are honoured.
fn wrap_to_width(value: &str, font_px: f64, width: f64, lines: usize) -> Vec<String> {
    let width = width.max(font_px);
    if text_width(value, font_px) <= width {
        return vec![value.to_owned()];
    }
    // Break after spaces, hyphens and slashes, keeping the separator attached
    // so a rejoined line reads the same as the original.
    let mut pieces: Vec<String> = Vec::new();
    let mut current = String::new();
    for ch in value.chars() {
        current.push(ch);
        if matches!(ch, ' ' | '-' | '/' | '_' | '.') {
            pieces.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        pieces.push(current);
    }

    let budget = lines;
    let mut wrapped: Vec<String> = Vec::new();
    let mut line = String::new();
    // Whether the name ran out of lines. Counting characters instead misses a
    // separator the trim ate, which marked "Log Analytics Workspace" as cut.
    let mut cut = false;
    for piece in pieces {
        if !line.is_empty() && text_width(&(line.clone() + &piece), font_px) > width {
            wrapped.push(line.trim_end().to_owned());
            line = String::new();
            if wrapped.len() == budget {
                cut = true;
                break;
            }
        }
        line.push_str(&piece);
    }
    if wrapped.len() < budget && !line.is_empty() {
        wrapped.push(line.trim_end().to_owned());
    } else if !line.is_empty() {
        cut = true;
    }
    // A single piece longer than the budget still has to be cut somewhere.
    if wrapped.is_empty() {
        wrapped.push(ellipsize(value, font_px, width));
    }
    // A single unbreakable piece can still be wider than the box; clipping
    // every line is what guarantees a label never runs over its neighbour.
    let mut wrapped: Vec<String> = wrapped
        .into_iter()
        .map(|line| ellipsize(&line, font_px, width))
        .collect();
    // Running out of lines has to be *marked*. Left unmarked it reads as the
    // whole name, so `rg-n4-corp-dwh-dev` and `rg-n4-corp-dwh-prod` printed as
    // the same tile — and nothing upstream could tell the label had not fitted.
    if cut && let Some(last) = wrapped.last_mut() {
        *last = mark_cut(last, font_px, width);
    }
    wrapped
}

/// End a line with an ellipsis, dropping characters until it fits.
fn mark_cut(line: &str, font_px: f64, width: f64) -> String {
    let mut out: String = line.trim_end().to_owned();
    if out.ends_with('…') {
        return out;
    }
    out.push('…');
    while text_width(&out, font_px) > width && out.chars().count() > 1 {
        out.pop();
        out.pop();
        out.push('…');
    }
    out
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
    use crate::diagram::graph::LayoutMode;
    use crate::diagram::graph::Node;

    #[test]
    fn escape_covers_xml_metacharacters() {
        assert_eq!(escape(r#"a<b>&"c""#), "a&lt;b&gt;&amp;&quot;c&quot;");
    }

    #[test]
    fn unit_wrap_label_leaves_a_short_name_on_one_line() {
        assert_eq!(
            wrap_label("short", &crate::diagram::page::COMFORTABLE),
            vec!["short"]
        );
    }

    /// Chunking by character count split type names mid-word
    /// ("Log Analytics W" / "orkspace"); breaks belong at separators.
    #[test]
    fn unit_wrap_label_breaks_at_word_boundaries_when_the_name_is_long() {
        assert_eq!(
            wrap_label(
                "Log Analytics Workspace",
                &crate::diagram::page::COMFORTABLE
            ),
            vec!["Log Analytics", "Workspace"]
        );
    }

    #[test]
    fn unit_wrap_label_breaks_hyphenated_names_after_the_hyphen() {
        let lines = wrap_label(
            "a-very-long-resource-name",
            &crate::diagram::page::COMFORTABLE,
        );

        assert!(
            lines.iter().all(|line| line.len() <= 16),
            "over budget: {lines:?}"
        );
        assert!(lines[0].ends_with('-'), "split mid-token: {lines:?}");
    }

    /// Two resource groups whose names differ only in the suffix printed as
    /// the same tile, because the label was cut without being marked and
    /// nothing then tried a smaller size.
    #[test]
    fn unit_a_name_shrinks_to_stay_whole_before_it_is_cut() {
        let (font, lines) = fit_lines("rg-n4-corp-dwh-logicapps-prod", 11.0, 80.0, 2);

        assert!(font < 11.0, "kept the full size and cut instead");
        assert_eq!(lines.concat(), "rg-n4-corp-dwh-logicapps-prod");
    }

    #[test]
    fn unit_a_name_too_long_for_any_size_is_marked_as_cut() {
        let (_, lines) = fit_lines("averyveryverylongunbreakabletoken", 11.0, 40.0, 1);

        assert!(lines[0].ends_with('…'), "cut silently: {lines:?}");
    }

    /// The band is for the name; a count that wanted more room than was left
    /// used to be printed straight over it.
    #[test]
    fn unit_a_header_name_and_its_count_never_share_the_same_pixels() {
        let graph = EstateGraph {
            title: String::new(),
            nodes: vec![
                Node {
                    label: "node4-datawarehouse".into(),
                    sublabel: Some("137 resources".into()),
                    kind: NodeKind::Subscription,
                    parent: None,
                },
                Node {
                    label: "rg".into(),
                    sublabel: None,
                    kind: NodeKind::ResourceGroup,
                    parent: Some(0),
                },
            ],
            edges: vec![],
            layout: LayoutMode::default(),
        };

        let svg = render(&graph);

        assert!(svg.contains(">node4-datawarehouse<"), "name cut: {svg}");
        assert!(svg.contains(">137 resources<"), "count cut: {svg}");
    }

    #[test]
    fn unit_ellipsize_marks_a_name_it_had_to_cut() {
        let cut = ellipsize("an-extremely-long-resource-name", 11.0, 60.0);

        assert!(cut.ends_with('…') && cut.len() < 31, "got {cut:?}");
    }

    /// The sizing contract: a report diagram is exactly the width of the text
    /// column and never taller than the sheet, whatever the estate throws at it.
    #[test]
    fn unit_render_spans_the_page_width_and_never_outgrows_the_sheet() {
        for count in [1, 5, 20, 60] {
            let mut nodes = vec![Node {
                label: "rg".into(),
                sublabel: None,
                kind: NodeKind::ResourceGroup,
                parent: None,
            }];
            nodes.extend((0..count).map(|index| Node {
                label: format!("resource-{index}"),
                sublabel: None,
                kind: NodeKind::Resource {
                    azure_type: "microsoft.storage/storageaccounts".into(),
                },
                parent: Some(0),
            }));
            let graph = EstateGraph {
                title: "t".into(),
                nodes,
                edges: vec![],
                layout: LayoutMode::default(),
            };

            let svg = render(&graph);

            let (width, height) = viewbox(&svg);
            assert_eq!(
                width,
                crate::diagram::page::A4_PORTRAIT.width,
                "{count} resources produced a {width}px canvas"
            );
            assert!(
                height <= crate::diagram::page::A4_PORTRAIT.height,
                "{count} resources produced a {height}px canvas, taller than the sheet"
            );
        }
    }

    /// The content is drawn at the width it was laid out for, so the drawing
    /// reaches the canvas edges instead of floating in the middle of it.
    #[test]
    fn unit_render_leaves_no_horizontal_slack_around_the_content() {
        let mut nodes = vec![Node {
            label: "rg".into(),
            sublabel: None,
            kind: NodeKind::ResourceGroup,
            parent: None,
        }];
        nodes.extend((0..13).map(|index| Node {
            label: format!("resource-{index}"),
            sublabel: None,
            kind: NodeKind::Resource {
                azure_type: "microsoft.storage/storageaccounts".into(),
            },
            parent: Some(0),
        }));
        let graph = EstateGraph {
            title: "t".into(),
            nodes,
            edges: vec![],
            layout: LayoutMode::default(),
        };

        let svg = render(&graph);

        assert!(
            svg.contains(&format!(r#"transform="translate({MARGIN},"#)),
            "content is inset past the margin: {svg}"
        );
    }

    #[test]
    fn unit_a_report_diagram_has_no_title_but_a_standalone_export_does() {
        let graph = EstateGraph {
            title: "estate".into(),
            nodes: vec![Node {
                label: "rg".into(),
                sublabel: None,
                kind: NodeKind::ResourceGroup,
                parent: None,
            }],
            edges: vec![],
            layout: LayoutMode::default(),
        };

        assert!(!render_for(&graph, DiagramDetail::Summary).contains(">estate<"));
        assert!(render_for(&graph, DiagramDetail::Full).contains(">estate<"));
    }

    /// First and last numbers of the `viewBox`.
    fn viewbox(svg: &str) -> (f64, f64) {
        let attribute = svg
            .split_once(r#"viewBox="0 0 "#)
            .and_then(|(_, rest)| rest.split_once('"'))
            .expect("rendered SVG carries a viewBox");
        let mut parts = attribute.0.split_whitespace();
        let width = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
        let height = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0.0);
        (width, height)
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
            layout: LayoutMode::default(),
        };

        let svg = render_for(&graph, DiagramDetail::Full);

        assert!(svg.contains("t &amp; t"), "title escaped: {svg}");
        assert!(
            svg.contains("data:image/svg+xml;base64,"),
            "icon embedded: {svg}"
        );
        assert!(svg.contains("vm &quot;one&quot;"), "label escaped: {svg}");
    }

    /// A connector routed left of the content used to be cut off by the
    /// viewBox: the canvas was measured from the origin, not from the
    /// drawing's own top-left corner.
    #[test]
    fn unit_a_connector_left_of_the_content_stays_on_the_canvas() {
        let routes = [route::EdgeRoute {
            points: vec![(-40.0, 10.0), (-40.0, 90.0)],
            label_at: None,
            edge: 0,
            source_anchor: (route::Side::Left, 0.5),
            target_anchor: (route::Side::Left, 0.5),
        }];
        let placements = [Placement {
            x: 0.0,
            y: 0.0,
            width: 400.0,
            height: 200.0,
        }];

        let content = bounds(&placements, &routes);

        assert_eq!(content.x, -40.0);
        assert_eq!(content.width, 440.0);
    }
}
