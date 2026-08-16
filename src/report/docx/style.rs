//! Document setup and the themed building blocks the DOCX sections assemble.
//!
//! Everything visual comes from [`ThemeTokens`]; this module only implements
//! the closed set of layout strategies, exactly like `templates/typst/theme.typ`
//! does for the PDF.

use docx_rs::{
    AlignmentType, BorderType, Docx, LineSpacing, PageMargin, Paragraph, Run, RunFonts, Shading,
    ShdType, Style, StyleType, Table, TableBorder, TableBorderPosition, TableBorders, TableCell,
    TableCellMargins, TableLayoutType, TableRow, VAlignType, WidthType,
};

use crate::report::branding::BrandingContext;
use crate::report::theme::{TableStyle, ThemeTokens};

/// Twips (twentieths of a point) per inch — the unit Word measures pages in.
const TWIPS_PER_INCH: f32 = 1440.0;
/// EMU per twip, for image sizing.
pub const EMU_PER_TWIP: u32 = 635;

/// Twips per point, the other half of the twip → EMU conversion. Dropping this
/// factor renders images at 1/20th of their intended size.
const TWIPS_PER_POINT: f32 = 20.0;

/// Word sizes runs in half-points.
pub fn half_points(pt: f32) -> usize {
    (pt * 2.0).round().max(1.0) as usize
}

/// Points to EMU, for sizing an inline image against the type scale.
pub fn pt_to_emu(pt: f32) -> u32 {
    (pt * TWIPS_PER_POINT * EMU_PER_TWIP as f32)
        .round()
        .max(1.0) as u32
}

/// Twips to EMU, for sizing an image against the page.
pub fn twips_to_emu(twips: u32) -> u32 {
    twips.saturating_mul(EMU_PER_TWIP)
}

/// Word measures borders in eighths of a point.
fn eighths(pt: f32) -> usize {
    (pt * 8.0).round().max(1.0) as usize
}

/// OOXML colour attributes are bare hex, with no leading `#`.
pub fn hex(color: &str) -> String {
    color.trim_start_matches('#').to_owned()
}

/// Page dimensions in twips for the paper names the PDF also accepts. Word
/// needs explicit numbers, so only the common sizes are mapped; anything else
/// falls back to A4 (see [`page_size`]).
fn paper_twips(name: &str) -> Option<(u32, u32)> {
    match name.trim().to_ascii_lowercase().as_str() {
        "a3" => Some((16838, 23811)),
        "a4" => Some((11906, 16838)),
        "a5" => Some((8391, 11906)),
        "us-letter" | "letter" => Some((12240, 15840)),
        "us-legal" | "legal" => Some((12240, 20160)),
        "us-tabloid" | "tabloid" => Some((15840, 24480)),
        _ => None,
    }
}

/// Parse a Typst-style length (`2cm`, `1in`, `18mm`, `36pt`) into twips.
fn length_twips(value: &str) -> Option<u32> {
    let trimmed = value.trim();
    let split = trimmed.find(|c: char| c.is_ascii_alphabetic())?;
    let (number, unit) = trimmed.split_at(split);
    let number: f32 = number.trim().parse().ok()?;
    if !number.is_finite() || number < 0.0 {
        return None;
    }
    let per_unit = match unit.trim().to_ascii_lowercase().as_str() {
        "in" => TWIPS_PER_INCH,
        "cm" => TWIPS_PER_INCH / 2.54,
        "mm" => TWIPS_PER_INCH / 25.4,
        "pt" => 20.0,
        _ => return None,
    };
    Some((number * per_unit).round() as u32)
}

/// Everything a section needs to render itself: the palette and type scale,
/// plus the text width tables and images must fit inside.
pub struct Ctx<'a> {
    pub tokens: &'a ThemeTokens,
    pub usable_twips: u32,
}

impl<'a> Ctx<'a> {
    pub fn sans(&self) -> RunFonts {
        let family = &self.tokens.typography.docx_sans;
        RunFonts::new().ascii(family).hi_ansi(family).cs(family)
    }

    pub fn mono(&self) -> RunFonts {
        let family = &self.tokens.typography.docx_mono;
        RunFonts::new().ascii(family).hi_ansi(family).cs(family)
    }
}

/// Apply page geometry, default fonts and paragraph styles. Returns the
/// document plus the usable text width, which the caller threads into [`Ctx`].
pub fn document(branding: &BrandingContext) -> (Docx, u32) {
    let tokens = &branding.tokens;
    let typography = &tokens.typography;

    let (width, height) = match paper_twips(&branding.page_size) {
        Some(size) => size,
        None => {
            // Not fatal: Typst accepts far more paper names than Word does, so
            // the PDF still honours the configured size.
            tracing::warn!(
                paper = %branding.page_size,
                "DOCX does not know this paper size; falling back to A4"
            );
            paper_twips("a4").unwrap_or((11906, 16838))
        }
    };
    let margin = length_twips(&branding.margin).unwrap_or_else(|| {
        tracing::warn!(
            margin = %branding.margin,
            "DOCX cannot parse this margin; falling back to 2cm"
        );
        1134
    });
    // Keep at least a token text column even if someone configures absurd
    // margins, so the table grid never goes negative.
    let usable = width.saturating_sub(margin * 2).max(1440);

    let family = &typography.docx_sans;
    let docx = Docx::new()
        .default_fonts(RunFonts::new().ascii(family).hi_ansi(family).cs(family))
        .default_size(half_points(typography.base_pt))
        .default_line_spacing(
            LineSpacing::new().line((typography.line_height * 240.0).round() as i32),
        )
        .page_size(width, height)
        .page_margin(
            PageMargin::new()
                .top(margin as i32)
                .bottom(margin as i32)
                .left(margin as i32)
                .right(margin as i32),
        )
        // The cover gets no header or footer of its own.
        .title_pg();

    (heading_styles(docx, tokens), usable)
}

fn heading_styles(docx: Docx, tokens: &ThemeTokens) -> Docx {
    let typography = &tokens.typography;
    let sizes = [typography.h1_pt, typography.h2_pt, typography.h3_pt];
    let colors = [
        &tokens.palette.primary,
        &tokens.palette.primary_dark,
        &tokens.palette.ink,
    ];
    let mut docx = docx;
    for (index, (size, color)) in sizes.iter().zip(colors).enumerate() {
        let level = index + 1;
        let family = &typography.docx_sans;
        docx = docx.add_style(
            Style::new(format!("Heading{level}"), StyleType::Paragraph)
                .name(format!("Heading {level}"))
                .based_on("Normal")
                .next("Normal")
                // The outline level is what populates Word's navigation pane
                // and lets the TOC field find these headings.
                .outline_lvl(index)
                .q_format(true)
                .size(half_points(*size))
                .bold()
                .color(hex(color))
                .fonts(RunFonts::new().ascii(family).hi_ansi(family).cs(family))
                .line_spacing(
                    LineSpacing::new()
                        .before(320 - (index as u32 * 80))
                        .after(140),
                ),
        );
    }
    docx
}

// ------------------------------------------------------------ paragraphs ----

pub fn heading(level: usize, text: &str) -> Paragraph {
    Paragraph::new()
        .style(&format!("Heading{level}"))
        .add_run(Run::new().add_text(text))
}

pub fn body(text: &str) -> Paragraph {
    Paragraph::new().add_run(Run::new().add_text(text))
}

pub fn muted(ctx: &Ctx, text: &str) -> Paragraph {
    Paragraph::new().add_run(
        Run::new()
            .add_text(text)
            .size(half_points(ctx.tokens.typography.small_pt))
            .color(hex(&ctx.tokens.palette.muted)),
    )
}

/// Word breaks lines at spaces and hyphens but not at `/`, so an ARM id would
/// otherwise widen its column until the table runs off the page. Zero-width
/// spaces give it somewhere to break.
pub fn wrappable(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        out.push(ch);
        if matches!(ch, '/' | '-' | '.' | '_') {
            out.push('\u{200B}');
        }
    }
    out
}

// ---------------------------------------------------------------- tables ----

fn borders(ctx: &Ctx) -> TableBorders {
    let color = hex(&ctx.tokens.palette.rule);
    let size = eighths(ctx.tokens.layout.rule_pt);
    let edge = |position| {
        TableBorder::new(position)
            .border_type(BorderType::Single)
            .size(size)
            .color(color.clone())
    };
    match ctx.tokens.layout.table {
        TableStyle::SolidHeader => TableBorders::with_empty()
            .set(edge(TableBorderPosition::Top))
            .set(edge(TableBorderPosition::Bottom))
            .set(edge(TableBorderPosition::Left))
            .set(edge(TableBorderPosition::Right))
            .set(edge(TableBorderPosition::InsideH))
            .set(edge(TableBorderPosition::InsideV)),
        // Hairline and banded rule horizontally only; vertical rules add noise
        // to dense inventory tables without helping the reader.
        TableStyle::Hairline | TableStyle::Banded => TableBorders::with_empty()
            .set(edge(TableBorderPosition::Top))
            .set(edge(TableBorderPosition::Bottom))
            .set(edge(TableBorderPosition::InsideH)),
    }
}

/// Distribute the text width across columns by how much content each holds.
/// Equal shares would starve a `name` column next to a one-word `location`.
fn column_widths(headers: &[String], rows: &[Vec<String>], usable: u32) -> Vec<u32> {
    let weights: Vec<f32> = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            let longest = rows
                .iter()
                .filter_map(|row| row.get(index))
                .map(|value| value.chars().count())
                .max()
                .unwrap_or(0);
            // Clamped so one enormous value cannot collapse every other column.
            longest.max(header.chars().count()).clamp(8, 60) as f32
        })
        .collect();

    let total: f32 = weights.iter().sum();
    let mut widths: Vec<u32> = weights
        .iter()
        .map(|w| ((w / total) * usable as f32).round() as u32)
        .collect();
    // The grid must sum to exactly the text width or Word re-derives it, so
    // the last column absorbs the rounding remainder.
    let sum: u32 = widths.iter().sum();
    if let Some(last) = widths.last_mut() {
        *last = last.saturating_add(usable).saturating_sub(sum).max(1);
    }
    widths
}

fn header_cell(ctx: &Ctx, label: &str, width: u32) -> TableCell {
    let tokens = ctx.tokens;
    let (fill, color) = match tokens.layout.table {
        TableStyle::SolidHeader => (
            Some(hex(&tokens.palette.primary)),
            hex(&tokens.palette.on_primary),
        ),
        TableStyle::Banded => (
            Some(hex(&tokens.palette.primary_tint)),
            hex(&tokens.palette.ink),
        ),
        TableStyle::Hairline => (None, hex(&tokens.palette.ink)),
    };
    let mut cell = TableCell::new()
        .width(width as usize, WidthType::Dxa)
        .vertical_align(VAlignType::Center)
        .add_paragraph(
            Paragraph::new().add_run(
                Run::new()
                    .add_text(label)
                    .size(half_points(tokens.typography.table_header_pt))
                    .bold()
                    .color(color)
                    .fonts(ctx.sans()),
            ),
        );
    if let Some(fill) = fill {
        cell = cell.shading(Shading::new().shd_type(ShdType::Clear).fill(fill));
    }
    cell
}

fn body_cell(ctx: &Ctx, value: &str, width: u32, zebra: bool, mono: bool) -> TableCell {
    let tokens = ctx.tokens;
    let mut run = Run::new()
        .add_text(wrappable(value))
        .size(half_points(tokens.typography.table_pt));
    run = if mono {
        run.fonts(ctx.mono())
    } else {
        run.fonts(ctx.sans())
    };
    let mut cell = TableCell::new()
        .width(width as usize, WidthType::Dxa)
        .add_paragraph(Paragraph::new().add_run(run));
    if zebra {
        cell = cell.shading(
            Shading::new()
                .shd_type(ShdType::Clear)
                .fill(hex(&tokens.palette.zebra)),
        );
    }
    cell
}

/// Shared table shell: fixed layout, an explicit grid summing to the text
/// width, and optional zebra striping.
///
/// Header rows do not repeat across page breaks: docx-rs 0.4 has no `tblHeader`
/// support, so the best available is `cant_split`, which at least keeps the
/// header from being torn in half. The PDF does repeat its headers.
fn shell(ctx: &Ctx, widths: &[u32], rows: Vec<TableRow>) -> Table {
    Table::new(rows)
        .layout(TableLayoutType::Fixed)
        .width(ctx.usable_twips as usize, WidthType::Dxa)
        .set_grid(widths.iter().map(|w| *w as usize).collect())
        .set_borders(borders(ctx))
        .margins(TableCellMargins::new().margin(40, 80, 40, 80))
}

/// A data table. `mono_columns` names columns rendered in the monospace face
/// (resource ids and check names).
pub fn data_table(
    ctx: &Ctx,
    headers: &[String],
    rows: &[Vec<String>],
    mono_columns: &[usize],
) -> Table {
    let widths = column_widths(headers, rows, ctx.usable_twips);
    let zebra = ctx.tokens.layout.zebra_rows;

    let mut table_rows = vec![
        TableRow::new(
            headers
                .iter()
                .enumerate()
                .map(|(i, h)| header_cell(ctx, h, widths[i]))
                .collect(),
        )
        .cant_split(),
    ];
    for (index, row) in rows.iter().enumerate() {
        table_rows.push(TableRow::new(
            (0..headers.len())
                .map(|i| {
                    body_cell(
                        ctx,
                        row.get(i).map(String::as_str).unwrap_or(""),
                        widths[i],
                        zebra && index % 2 == 1,
                        mono_columns.contains(&i),
                    )
                })
                .collect(),
        ));
    }
    shell(ctx, &widths, table_rows)
}

/// The findings table: same shell, but the severity cell carries the severity
/// colours from the theme.
pub fn findings_table(ctx: &Ctx, rows: &[[String; 4]]) -> Table {
    let headers: Vec<String> = ["Severity", "Title", "Category", "Check"]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
    let plain: Vec<Vec<String>> = rows.iter().map(|r| r.to_vec()).collect();
    let widths = column_widths(&headers, &plain, ctx.usable_twips);

    let mut table_rows = vec![
        TableRow::new(
            headers
                .iter()
                .enumerate()
                .map(|(i, h)| header_cell(ctx, h, widths[i]))
                .collect(),
        )
        .cant_split(),
    ];
    for (index, row) in rows.iter().enumerate() {
        let zebra = ctx.tokens.layout.zebra_rows && index % 2 == 1;
        let severity = &row[0];
        let colors = ctx.tokens.palette.severity.level(severity);
        let severity_cell = TableCell::new()
            .width(widths[0] as usize, WidthType::Dxa)
            .shading(
                Shading::new()
                    .shd_type(ShdType::Clear)
                    .fill(hex(colors.map_or(&ctx.tokens.palette.zebra, |c| &c.fill))),
            )
            .add_paragraph(
                Paragraph::new().add_run(
                    Run::new()
                        .add_text(severity)
                        .size(half_points(ctx.tokens.typography.table_pt))
                        .bold()
                        .color(hex(colors.map_or(&ctx.tokens.palette.ink, |c| &c.text)))
                        .fonts(ctx.sans()),
                ),
            );
        table_rows.push(TableRow::new(vec![
            severity_cell,
            body_cell(ctx, &row[1], widths[1], zebra, false),
            body_cell(ctx, &row[2], widths[2], zebra, false),
            body_cell(ctx, &row[3], widths[3], zebra, true),
        ]));
    }
    shell(ctx, &widths, table_rows)
}

/// Resource-type chapter opener: the type's icon beside the heading, over a
/// rule — the DOCX counterpart of `type-chapter` in `templates/typst/theme.typ`.
///
/// A borderless two-cell table rather than an inline image, because Word sits
/// an inline image on the text baseline and docx-rs exposes no `w:position` to
/// offset it; a taller-than-text icon therefore floats above the words. Cell
/// centring is the only vertical alignment available, and it is what the PDF's
/// `grid(align: horizon)` does anyway.
///
/// The heading paragraph keeps its `Heading1` style inside the cell, so the TOC
/// field and Word's navigation pane still find it.
pub fn type_heading(ctx: &Ctx, icon: Option<Run>, display: &str) -> Table {
    let icon_pt = ctx.tokens.typography.h1_pt * ICON_SCALE;
    let gap_twips = 140;
    let icon_col = match icon {
        Some(_) => (icon_pt * TWIPS_PER_POINT).round() as u32 + gap_twips,
        None => 0,
    };
    let text_col = ctx.usable_twips.saturating_sub(icon_col).max(1440);
    let widths: Vec<u32> = if icon_col == 0 {
        vec![text_col]
    } else {
        vec![icon_col, text_col]
    };

    let heading_cell = TableCell::new()
        .width(text_col as usize, WidthType::Dxa)
        .vertical_align(VAlignType::Center)
        .add_paragraph(
            Paragraph::new()
                .style("Heading1")
                .add_run(Run::new().add_text(display.to_uppercase())),
        );
    let mut cells = Vec::new();
    if let Some(icon) = icon {
        cells.push(
            TableCell::new()
                .width(icon_col as usize, WidthType::Dxa)
                .vertical_align(VAlignType::Center)
                .add_paragraph(Paragraph::new().add_run(icon)),
        );
    }
    cells.push(heading_cell);

    Table::new(vec![TableRow::new(cells)])
        .layout(TableLayoutType::Fixed)
        .width(ctx.usable_twips as usize, WidthType::Dxa)
        .set_grid(widths.iter().map(|w| *w as usize).collect())
        .set_borders(
            TableBorders::with_empty().set(
                TableBorder::new(TableBorderPosition::Bottom)
                    .border_type(BorderType::Single)
                    .size(eighths(1.5))
                    .color(hex(&ctx.tokens.palette.primary)),
            ),
        )
        // No left inset, so the icon lines up with the page's text margin.
        .margins(TableCellMargins::new().margin(0, 0, 60, 0))
}

/// Icon size relative to the level-1 heading, shared by the PDF and DOCX.
pub const ICON_SCALE: f32 = 1.4;

/// Name plate above each resource's detail, matching the PDF: a filled band
/// (or an underline for hairline themes) carrying the resource name.
pub fn resource_plate(ctx: &Ctx, name: &str) -> Table {
    let tokens = ctx.tokens;
    let hairline = matches!(tokens.layout.table, TableStyle::Hairline);
    let widths = [ctx.usable_twips];

    let mut cell = TableCell::new()
        .width(ctx.usable_twips as usize, WidthType::Dxa)
        .add_paragraph(
            Paragraph::new().add_run(
                Run::new()
                    .add_text(name.to_uppercase())
                    .size(half_points(tokens.typography.h3_pt))
                    .bold()
                    .color(hex(if hairline {
                        &tokens.palette.primary
                    } else {
                        &tokens.palette.on_primary
                    }))
                    .fonts(ctx.sans()),
            ),
        );
    if !hairline {
        cell = cell.shading(
            Shading::new()
                .shd_type(ShdType::Clear)
                .fill(hex(&tokens.palette.primary)),
        );
    }

    let borders = if hairline {
        TableBorders::with_empty().set(
            TableBorder::new(TableBorderPosition::Bottom)
                .border_type(BorderType::Single)
                .size(eighths(1.0))
                .color(hex(&tokens.palette.primary)),
        )
    } else {
        TableBorders::with_empty()
    };

    Table::new(vec![TableRow::new(vec![cell])])
        .layout(TableLayoutType::Fixed)
        .width(ctx.usable_twips as usize, WidthType::Dxa)
        .set_grid(widths.iter().map(|w| *w as usize).collect())
        .set_borders(borders)
        .margins(TableCellMargins::new().margin(60, 120, 60, 120))
}

/// Small labelled rule introducing a sub-block (Settings, Findings, Related).
pub fn sub_label(ctx: &Ctx, title: &str) -> Paragraph {
    Paragraph::new()
        .add_run(
            Run::new()
                .add_text(title)
                .size(half_points(ctx.tokens.typography.small_pt))
                .bold()
                .color(hex(&ctx.tokens.palette.primary_dark))
                .fonts(ctx.sans()),
        )
        .line_spacing(LineSpacing::new().before(200).after(60))
}

/// Two-column key/value table for a resource's settings.
pub fn settings_table(ctx: &Ctx, settings: &[(String, String)]) -> Table {
    let key_width = ctx.usable_twips / 3;
    let widths = vec![key_width, ctx.usable_twips - key_width];
    let zebra = ctx.tokens.layout.zebra_rows;

    let rows = settings
        .iter()
        .enumerate()
        .map(|(index, (key, value))| {
            let striped = zebra && index % 2 == 1;
            TableRow::new(vec![
                {
                    let mut cell = TableCell::new()
                        .width(widths[0] as usize, WidthType::Dxa)
                        .add_paragraph(
                            Paragraph::new().add_run(
                                Run::new()
                                    .add_text(key)
                                    .size(half_points(ctx.tokens.typography.table_pt))
                                    .bold()
                                    .fonts(ctx.sans()),
                            ),
                        );
                    if striped {
                        cell = cell.shading(
                            Shading::new()
                                .shd_type(ShdType::Clear)
                                .fill(hex(&ctx.tokens.palette.zebra)),
                        );
                    }
                    cell
                },
                body_cell(ctx, value, widths[1], striped, false),
            ])
        })
        .collect();

    shell(ctx, &widths, rows)
}

/// A finding attached to a resource: severity tag in its own tinted cell,
/// title alongside.
pub fn callout(ctx: &Ctx, severity: &str, title: &str) -> Table {
    let tag_width = ctx.usable_twips / 8;
    let widths = [tag_width, ctx.usable_twips - tag_width];
    let colors = ctx.tokens.palette.severity.level(severity);
    let fill = hex(colors.map_or(&ctx.tokens.palette.zebra, |c| &c.fill));

    let shaded =
        |cell: TableCell| cell.shading(Shading::new().shd_type(ShdType::Clear).fill(fill.clone()));
    let row = TableRow::new(vec![
        shaded(
            TableCell::new()
                .width(widths[0] as usize, WidthType::Dxa)
                .add_paragraph(
                    Paragraph::new().add_run(
                        Run::new()
                            .add_text(severity.to_uppercase())
                            .size(half_points(ctx.tokens.typography.table_pt))
                            .bold()
                            .color(hex(colors.map_or(&ctx.tokens.palette.ink, |c| &c.text)))
                            .fonts(ctx.sans()),
                    ),
                ),
        ),
        shaded(
            TableCell::new()
                .width(widths[1] as usize, WidthType::Dxa)
                .add_paragraph(
                    Paragraph::new().add_run(
                        Run::new()
                            .add_text(title)
                            .size(half_points(ctx.tokens.typography.table_pt))
                            .fonts(ctx.sans()),
                    ),
                ),
        ),
    ]);

    Table::new(vec![row])
        .layout(TableLayoutType::Fixed)
        .width(ctx.usable_twips as usize, WidthType::Dxa)
        .set_grid(widths.iter().map(|w| *w as usize).collect())
        .set_borders(TableBorders::with_empty())
        .margins(TableCellMargins::new().margin(50, 100, 50, 100))
}

/// The executive-summary statistics, laid out per the theme's `stat` strategy:
/// a single-row table of tinted or outlined cards, or plain label/value text.
pub fn stat_row(ctx: &Ctx, stats: &[(String, String)]) -> Vec<Paragraph> {
    stats
        .iter()
        .map(|(value, label)| {
            Paragraph::new()
                .add_run(
                    Run::new()
                        .add_text(value)
                        .size(half_points(ctx.tokens.typography.stat_value_pt))
                        .bold()
                        .color(hex(&ctx.tokens.palette.primary_dark))
                        .fonts(ctx.sans()),
                )
                .add_run(Run::new().add_break(docx_rs::BreakType::TextWrapping))
                .add_run(
                    Run::new()
                        .add_text(label)
                        .size(half_points(ctx.tokens.typography.stat_label_pt))
                        .color(hex(&ctx.tokens.palette.muted))
                        .fonts(ctx.sans()),
                )
                .align(AlignmentType::Left)
        })
        .collect()
}

/// Wrap the stat paragraphs into the themed container.
pub fn stat_table(ctx: &Ctx, stats: &[(String, String)]) -> Table {
    let count = stats.len().max(1);
    let width = ctx.usable_twips / count as u32;
    let mut widths: Vec<u32> = (0..count).map(|_| width).collect();
    // Same rule as the data tables: the grid has to add up to the declared
    // table width, so the last column takes the integer-division remainder.
    if let Some(last) = widths.last_mut() {
        *last += ctx.usable_twips - width * count as u32;
    }
    let fill = match ctx.tokens.layout.stat {
        crate::report::theme::StatStyle::Card => Some(hex(&ctx.tokens.palette.primary_tint)),
        _ => None,
    };

    let cells = stat_row(ctx, stats)
        .into_iter()
        .enumerate()
        .map(|(index, paragraph)| {
            let mut cell = TableCell::new()
                .width(widths[index] as usize, WidthType::Dxa)
                .add_paragraph(paragraph);
            if let Some(fill) = &fill {
                cell = cell.shading(Shading::new().shd_type(ShdType::Clear).fill(fill.clone()));
            }
            cell
        })
        .collect();

    let mut table = Table::new(vec![TableRow::new(cells)])
        .layout(TableLayoutType::Fixed)
        .width(ctx.usable_twips as usize, WidthType::Dxa)
        .set_grid(widths.iter().map(|w| *w as usize).collect())
        .margins(TableCellMargins::new().margin(80, 100, 80, 100));

    table = match ctx.tokens.layout.stat {
        crate::report::theme::StatStyle::Bare => table.set_borders(TableBorders::with_empty()),
        crate::report::theme::StatStyle::Card => table.set_borders(
            TableBorders::with_empty().set(
                TableBorder::new(TableBorderPosition::Top)
                    .border_type(BorderType::Single)
                    .size(eighths(2.0))
                    .color(hex(&ctx.tokens.palette.accent)),
            ),
        ),
        crate::report::theme::StatStyle::Outline => {
            let color = hex(&ctx.tokens.palette.rule);
            let size = eighths(ctx.tokens.layout.rule_pt);
            let edge = |position| {
                TableBorder::new(position)
                    .border_type(BorderType::Single)
                    .size(size)
                    .color(color.clone())
            };
            table.set_borders(
                TableBorders::with_empty()
                    .set(edge(TableBorderPosition::Top))
                    .set(edge(TableBorderPosition::Bottom))
                    .set(edge(TableBorderPosition::Left))
                    .set(edge(TableBorderPosition::Right))
                    .set(edge(TableBorderPosition::InsideV)),
            )
        }
    };
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_length_twips_parses_every_supported_unit() {
        assert_eq!(length_twips("1in"), Some(1440));
        assert_eq!(length_twips("2cm"), Some(1134));
        assert_eq!(length_twips("25.4mm"), Some(1440));
        assert_eq!(length_twips("36pt"), Some(720));
        assert_eq!(length_twips(" 1 in "), Some(1440));
    }

    #[test]
    fn unit_length_twips_rejects_unusable_values() {
        assert_eq!(length_twips("2em"), None);
        assert_eq!(length_twips("wide"), None);
        assert_eq!(length_twips("-3cm"), None);
        assert_eq!(length_twips("2"), None);
    }

    #[test]
    fn unit_paper_twips_accepts_typst_paper_names() {
        assert_eq!(paper_twips("a4"), Some((11906, 16838)));
        assert_eq!(paper_twips("US-Letter"), Some((12240, 15840)));
        assert_eq!(paper_twips("presentation-16-9"), None);
    }

    #[test]
    fn unit_column_widths_sum_to_the_text_width() {
        let headers = vec!["name".to_owned(), "id".to_owned(), "loc".to_owned()];
        let rows = vec![vec![
            "a-very-long-resource-name-here".to_owned(),
            "x".to_owned(),
            "uksouth".to_owned(),
        ]];

        let widths = column_widths(&headers, &rows, 9000);

        assert_eq!(widths.iter().sum::<u32>(), 9000);
        assert!(widths[0] > widths[1], "wider content should win more space");
    }

    #[test]
    fn unit_column_widths_handle_a_single_column() {
        let widths = column_widths(&["only".to_owned()], &[], 9000);

        assert_eq!(widths, vec![9000]);
    }

    /// 914400 EMU to the inch, 72 points to the inch. Getting this wrong by the
    /// twips factor renders icons at 1/20th size, which is how it shipped once.
    #[test]
    fn unit_pt_to_emu_matches_the_ooxml_definition_of_a_point() {
        assert_eq!(pt_to_emu(72.0), 914_400);
        assert_eq!(pt_to_emu(25.2), 320_040);
        assert!(pt_to_emu(0.0) >= 1, "a zero-sized image is not renderable");
    }

    #[test]
    fn unit_twips_to_emu_matches_the_ooxml_definition_of_an_inch() {
        assert_eq!(twips_to_emu(1440), 914_400);
        // A4 text width at 2cm margins, i.e. what a full-width diagram gets.
        assert_eq!(twips_to_emu(11906 - 2268), 6_120_130);
    }

    #[test]
    fn unit_wrappable_offers_break_points_inside_arm_ids() {
        let wrapped = wrappable("/subscriptions/abc/resourceGroups/rg-app");

        assert!(wrapped.contains('\u{200B}'));
        assert_eq!(
            wrapped.replace('\u{200B}', ""),
            "/subscriptions/abc/resourceGroups/rg-app"
        );
    }
}
