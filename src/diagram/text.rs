//! Shared deterministic text measurement and fitting for diagram paint and routing.

use super::page::Rung;

/// Approximate rendered width. Character count rather than a measured advance,
/// because the rasteriser resolves fonts from the system database and the
/// emitter cannot know which face will actually be used.
pub(crate) fn text_width(value: &str, font_px: f64) -> f64 {
    value.chars().count() as f64 * font_px * 0.55
}

/// Shorten to fit `limit` pixels, marking the cut.
pub(crate) fn ellipsize(value: &str, font_px: f64, limit: f64) -> String {
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
pub(crate) fn stub_label_lines(height: f64, font: f64, has_sublabel: bool) -> usize {
    let reserved = if has_sublabel { font + 4.0 } else { 0.0 };
    (((height - 12.0 - reserved) / (font + 2.0)).floor() as usize).clamp(1, 3)
}

/// Floor on a fitted label. Past this the name is unreadable on paper, so a
/// cut is the better trade.
pub(crate) const MIN_LABEL_PX: f64 = 8.0;

/// Largest size at or below `font` whose wrap fits the box whole, and the lines
/// it produces.
///
/// The name is the only thing on a resource-group tile, and Azure group names
/// run past thirty characters: dropping a couple of points to keep one whole
/// beats printing `rg-app-auto-…` on two tiles that differ only in what was
/// cut.
/// Largest size at or below `font` that keeps `value` inside `width` on one
/// line, floored at [`MIN_LABEL_PX`].
pub(crate) fn fit_font(value: &str, font: f64, width: f64) -> f64 {
    let mut size = font;
    while size > MIN_LABEL_PX && text_width(value, size) > width {
        size -= 1.0;
    }
    size
}

pub(crate) fn fit_lines(value: &str, font: f64, width: f64, lines: usize) -> (f64, Vec<String>) {
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
#[cfg(test)]
pub(crate) fn wrap_label(value: &str, rung: &Rung) -> Vec<String> {
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
pub(crate) fn wrap_to_width(value: &str, font_px: f64, width: f64, lines: usize) -> Vec<String> {
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

/// Glyph and fitted text bounds, excluding unused space in the layout slot.
pub(crate) fn resource_bounds(
    node: &super::graph::Node,
    placement: &super::layout::Placement,
    rung: &Rung,
) -> Vec<super::layout::Placement> {
    use super::layout::Placement;
    let icon = super::route::visual_box(placement, true, rung);
    let center = placement.x + placement.width / 2.0;
    let (font, lines) = fit_lines(
        &node.label,
        rung.label_px,
        placement.width - 8.0,
        rung.wrap_lines,
    );
    let mut bounds = vec![icon];
    let mut baseline = icon.y + icon.height + rung.label_px + 3.0;
    for line in lines {
        let width = text_width(&line, font) + 4.0;
        bounds.push(Placement {
            x: center - width / 2.0,
            y: baseline - font,
            width,
            height: font + 3.0,
        });
        baseline += font + 2.0;
    }
    if let Some(label) = &node.sublabel {
        let font = rung.label_px - 2.0;
        let width = text_width(&ellipsize(label, font, placement.width - 4.0), font) + 4.0;
        bounds.push(Placement {
            x: center - width / 2.0,
            y: baseline - font,
            width,
            height: font + 3.0,
        });
    }
    bounds
}
