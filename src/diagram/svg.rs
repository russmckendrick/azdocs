//! Self-contained SVG rendering of an `EstateGraph`: shares the draw.io
//! layout (via `absolutize`) and palette so both emitters agree on shape,
//! with icons embedded as data URIs so the file needs no external assets.

use std::fmt::Write as _;

use super::drawio::container_palette;
use super::graph::{DiagEdge, EdgeStyle, EstateGraph, NodeKind, truncate_label};
use super::icons;
use super::layout::{self, Placement};
use super::page::{DiagramDetail, PageFraction, Rung};
use super::route;

const MARGIN: f64 = 16.0;
/// Vertical room above the content for the diagram title.
const TITLE_BAND: f64 = 40.0;
/// Corner radius on connectors. Matches what draw.io's own `rounded=1` draws,
/// so the two emitters finally agree.
const CORNER: f64 = 8.0;
/// Room below the content for the border-convention key.
const LEGEND_BAND: f64 = 26.0;
/// Never shrink content below this to win a smaller page share — past it the
/// labels stop being readable, which is the whole problem being solved.
const MIN_SCALE: f64 = 0.62;
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
    let (content_width, content_height) = bounds(&placements, &routes);

    // Snap the canvas to a share of an A4 portrait page rather than emitting
    // whatever the content happens to measure, so a report stacks predictable
    // blocks instead of a run of differently-downscaled rectangles.
    let fraction = PageFraction::fit(
        content_width + 2.0 * MARGIN,
        content_height + TITLE_BAND + LEGEND_BAND + MARGIN,
        MIN_SCALE,
    );
    let (width, height) = if detail.snaps_to_page() {
        fraction.canvas()
    } else {
        (
            content_width + 2.0 * MARGIN,
            content_height + TITLE_BAND + LEGEND_BAND + MARGIN,
        )
    };
    let inner_width = width - 2.0 * MARGIN;
    let inner_height = height - TITLE_BAND - LEGEND_BAND - MARGIN;
    let scale = (inner_width / content_width)
        .min(inner_height / content_height)
        .min(1.0);
    // Centre horizontally; the content hangs from the top so the title band
    // and the legend keep their fixed positions on the canvas.
    let offset_x = MARGIN + (inner_width - content_width * scale) / 2.0;

    let mut out = String::new();
    let _ = writeln!(
        out,
        r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}" font-family="{FONT}" data-page-fraction="{f}">"#,
        w = fmt(width),
        h = fmt(height),
        f = if detail.snaps_to_page() {
            fraction.label()
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
    let _ = writeln!(
        out,
        r#"  <text x="{}" y="28" font-size="18" font-weight="bold" text-anchor="middle" fill="{TEXT_PRIMARY}">{}</text>"#,
        fmt(width / 2.0),
        escape(&graph.title),
    );
    let _ = writeln!(
        out,
        r#"  <g transform="translate({},{}) scale({:.4})">"#,
        fmt(offset_x),
        fmt(TITLE_BAND),
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
    legend(&mut out, graph, width, height);

    out.push_str("</svg>\n");
    out
}

/// Key to the border conventions, drawn only for the kinds actually present so
/// a simple diagram is not captioned with boundaries it does not use.
fn legend(out: &mut String, graph: &EstateGraph, canvas_width: f64, canvas_height: f64) {
    let entries: Vec<(&str, &str, &str)> = [
        (NodeKind::Vnet, "Virtual network", "7,4"),
        (NodeKind::Subnet, "Subnet", "4,3"),
        (NodeKind::Unnetworked, "Not in a VNet", "7,4"),
    ]
    .iter()
    .filter(|(kind, _, _)| graph.nodes.iter().any(|node| &node.kind == kind))
    .map(|(kind, label, dash)| (container_palette(kind).1, *label, *dash))
    .collect();
    if entries.is_empty() {
        return;
    }

    let y = canvas_height - 10.0;
    let mut x = MARGIN + 4.0;
    for (stroke, label, dash) in &entries {
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

fn bounds(placements: &[Placement], routes: &[route::EdgeRoute]) -> (f64, f64) {
    let mut width = placements
        .iter()
        .map(|p| p.x + p.width)
        .fold(0.0_f64, f64::max);
    let mut height = placements
        .iter()
        .map(|p| p.y + p.height)
        .fold(0.0_f64, f64::max);
    for routed in routes {
        for (x, y) in &routed.points {
            width = width.max(*x);
            height = height.max(*y);
        }
    }
    (width.max(360.0).ceil(), height.max(120.0).ceil())
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
        NodeKind::Vnet | NodeKind::Unnetworked => r#" stroke-dasharray="7,4""#,
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
        // A peering stub is a plain labelled box, so its label is centred.
        let center_x = placement.x + placement.width / 2.0;
        let baseline = placement.y + placement.height / 2.0 + font / 2.0 - 2.0;
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}" font-size="{}" font-weight="bold" text-anchor="middle" fill="{TEXT_PRIMARY}">{}</text>"#,
            fmt(center_x),
            fmt(if node.sublabel.is_some() {
                baseline - font * 0.6
            } else {
                baseline
            }),
            fmt(font),
            escape(&truncate_label(&node.label)),
        );
        if let Some(sublabel) = &node.sublabel {
            let _ = writeln!(
                out,
                r#"    <text x="{}" y="{}" font-size="{}" text-anchor="middle" fill="{TEXT_SECONDARY}">{}</text>"#,
                fmt(center_x),
                fmt(baseline + font * 0.7),
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

    // The CIDR sits at the far right of the same band, so the name only gets
    // the width left over. Both are clipped rather than allowed to collide.
    let sublabel_width = node
        .sublabel
        .as_deref()
        .map_or(0.0, |value| text_width(value, font - 1.0) + 12.0)
        .min((band_end - cursor) / 2.0);
    let _ = writeln!(
        out,
        r#"    <text x="{}" y="{}" font-size="{}" font-weight="bold" fill="{colour}">{}</text>"#,
        fmt(cursor),
        fmt(baseline),
        fmt(font),
        escape(&ellipsize(
            &node.label,
            font,
            band_end - cursor - sublabel_width
        )),
    );
    if let Some(sublabel) = &node.sublabel {
        let _ = writeln!(
            out,
            r#"    <text x="{}" y="{}" font-size="{}" text-anchor="end" fill="{TEXT_SECONDARY}">{}</text>"#,
            fmt(band_end),
            fmt(baseline),
            fmt(font - 1.0),
            escape(&ellipsize(sublabel, font - 1.0, band_end - cursor)),
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
        NodeKind::Unnetworked => "#B45309",
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

/// Wrap to the rung's line budget, breaking at word boundaries.
///
/// Chunking by character count split "Log Analytics Workspace" into
/// "Log Analytics W" / "orkspace"; Azure type names are prose and hyphenated
/// resource names have natural break points, so both are honoured.
fn wrap_label(value: &str, rung: &Rung) -> Vec<String> {
    if value.chars().count() <= rung.wrap_chars {
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

    let mut lines: Vec<String> = Vec::new();
    let mut line = String::new();
    for piece in pieces {
        if !line.is_empty() && line.chars().count() + piece.chars().count() > rung.wrap_chars {
            lines.push(line.trim_end().to_owned());
            line = String::new();
            if lines.len() == rung.wrap_lines {
                break;
            }
        }
        line.push_str(&piece);
    }
    if lines.len() < rung.wrap_lines && !line.is_empty() {
        lines.push(line.trim_end().to_owned());
    }
    // A single piece longer than the budget still has to be cut somewhere.
    if lines.is_empty() {
        lines.push(value.chars().take(rung.wrap_chars).collect());
    }
    let consumed: usize = lines.iter().map(|l| l.chars().count()).sum();
    if consumed < value.trim_end().chars().count()
        && let Some(last) = lines.last_mut()
    {
        *last = ellipsize(last, 1.0, (rung.wrap_chars as f64 - 1.0) * 0.55);
    }
    lines
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

    #[test]
    fn unit_ellipsize_marks_a_name_it_had_to_cut() {
        let cut = ellipsize("an-extremely-long-resource-name", 11.0, 60.0);

        assert!(cut.ends_with('…') && cut.len() < 31, "got {cut:?}");
    }

    /// Every diagram has to land on one of the four page shares, whatever the
    /// estate throws at it — that is the sizing contract.
    #[test]
    fn unit_render_snaps_the_canvas_to_a_page_fraction() {
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
            };

            let svg = render(&graph);

            let allowed = crate::diagram::page::PageFraction::ALL
                .iter()
                .map(|fraction| {
                    let (width, height) = fraction.canvas();
                    format!(r#"viewBox="0 0 {} {}""#, fmt(width), fmt(height))
                })
                .collect::<Vec<_>>();
            assert!(
                allowed.iter().any(|candidate| svg.contains(candidate)),
                "{count} resources produced an off-grid canvas; expected one of {allowed:?}"
            );
        }
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
