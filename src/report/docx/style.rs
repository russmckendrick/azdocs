//! Document setup and the themed building blocks the DOCX sections assemble.
//!
//! Everything visual comes from [`ThemeTokens`]; this module only implements
//! the closed set of layout strategies, exactly like `templates/typst/theme.typ`
//! does for the PDF.

use docx_rs::{
    AbstractNumbering, AlignmentType, BorderType, BreakType, Docx, IndentLevel, Level, LevelJc,
    LevelText, LineSpacing, NumberFormat, Numbering, NumberingId, PageMargin, Paragraph,
    ParagraphBorder, ParagraphBorderPosition, ParagraphBorders, Run, RunFonts, Shading, ShdType,
    SpecialIndentType, Start, Style, StyleType, Tab, TabValueType, Table, TableBorder,
    TableBorderPosition, TableBorders, TableCell, TableCellMargins, TableLayoutType, TableRow,
    VAlignType, WidthType,
};

use crate::report::branding::BrandingContext;
use crate::report::document::{Fact, MetadataGroup, ResourceIndexItem, TableLink};
use crate::report::theme::{TableStyle, ThemeTokens};

/// Twips (twentieths of a point) per inch — the unit Word measures pages in.
const TWIPS_PER_INCH: f32 = 1440.0;
/// EMU per twip, for image sizing.
pub const EMU_PER_TWIP: u32 = 635;

/// Space reserved below a page-filling table for Word's mandatory trailing
/// paragraph. The paragraph itself is collapsed by `sections::page_break`.
pub const TABLE_TRAILING_PARAGRAPH_TWIPS: u32 = 120;

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
    pub table_grid: bool,
    pub tokens: &'a ThemeTokens,
    /// The resolved wording, so cover metadata and every other word the
    /// renderer writes itself comes from the same file as the document body.
    pub labels: &'a crate::labels::Labels,
    pub usable_twips: u32,
    pub usable_height_twips: u32,
    /// The physical sheet, for artwork anchored to the page rather than the
    /// text column.
    pub page_twips: (u32, u32),
}

/// Sheet and text-column sizes from [`document`], in twips.
pub struct Geometry {
    pub usable: u32,
    pub usable_height: u32,
    pub page: (u32, u32),
}

impl<'a> Ctx<'a> {
    pub fn serif(&self) -> RunFonts {
        let family = &self.tokens.typography.docx_serif;
        RunFonts::new().ascii(family).hi_ansi(family).cs(family)
    }

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
/// document plus its geometry, which the caller threads into [`Ctx`].
pub fn document(branding: &BrandingContext) -> (Docx, Geometry) {
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
    let usable_height = height.saturating_sub(margin * 2).max(1440);

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

    let docx = heading_styles(docx, tokens).add_style(
        Style::new("AzdocsTableHeader", StyleType::Paragraph)
            .name("Table header")
            .based_on("Normal"),
    );
    let docx = docx
        .add_abstract_numbering(
            AbstractNumbering::new(BULLET_NUMBERING_ID).add_level(Level::new(
                0,
                Start::new(1),
                NumberFormat::new("bullet"),
                LevelText::new("•"),
                LevelJc::new("left"),
            )),
        )
        .add_numbering(Numbering::new(BULLET_NUMBERING_ID, BULLET_NUMBERING_ID));
    let docx = if tokens.layout.heading_numbering {
        heading_numbering(docx)
    } else {
        docx
    };
    (
        docx,
        Geometry {
            usable,
            usable_height,
            page: (width, height),
        },
    )
}

const HEADING_NUMBERING_ID: usize = 42;
pub const BULLET_NUMBERING_ID: usize = 43;

fn heading_numbering(docx: Docx) -> Docx {
    let numbering = AbstractNumbering::new(HEADING_NUMBERING_ID)
        .add_level(
            Level::new(
                0,
                Start::new(1),
                NumberFormat::new("decimal"),
                LevelText::new("%1"),
                LevelJc::new("left"),
            )
            .paragraph_style("Heading1"),
        )
        .add_level(
            Level::new(
                1,
                Start::new(1),
                NumberFormat::new("decimal"),
                LevelText::new("%1.%2"),
                LevelJc::new("left"),
            )
            .paragraph_style("Heading2"),
        )
        .add_level(
            Level::new(
                2,
                Start::new(1),
                NumberFormat::new("decimal"),
                LevelText::new("%1.%2.%3"),
                LevelJc::new("left"),
            )
            .paragraph_style("Heading3"),
        );
    docx.add_abstract_numbering(numbering)
        .add_numbering(Numbering::new(HEADING_NUMBERING_ID, HEADING_NUMBERING_ID))
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
        let family = &typography.docx_serif;
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
                .color(hex(color))
                .fonts(RunFonts::new().ascii(family).hi_ansi(family).cs(family))
                .line_spacing(
                    LineSpacing::new()
                        .before([380, 280, 220][index])
                        .after([200, 150, 110][index]),
                ),
        );
    }
    for level in 1..=2 {
        docx = docx.add_style(
            Style::new(format!("ToC{level}"), StyleType::Paragraph)
                .name(format!("toc {level}"))
                .based_on("Normal")
                .next("Normal")
                .size(half_points(typography.table_pt))
                .fonts(
                    RunFonts::new()
                        .ascii(&typography.docx_sans)
                        .hi_ansi(&typography.docx_sans),
                )
                .line_spacing(LineSpacing::new().line(252).before(0).after(40)),
        );
    }
    docx.add_style(
        Style::new("Caption", StyleType::Paragraph)
            .name("Caption")
            .based_on("Normal")
            .next("Normal")
            .size(half_points(typography.small_pt))
            .italic()
            .color(hex(&tokens.palette.muted))
            .fonts(
                RunFonts::new()
                    .ascii(&typography.docx_sans)
                    .hi_ansi(&typography.docx_sans)
                    .cs(&typography.docx_sans),
            ),
    )
}

// ------------------------------------------------------------ paragraphs ----

pub fn heading(ctx: &Ctx, level: usize, text: &str) -> Paragraph {
    let paragraph = Paragraph::new()
        .style(&format!("Heading{level}"))
        .keep_next(true)
        .keep_lines(true)
        .add_run(Run::new().add_text(text));
    if ctx.tokens.layout.heading_numbering {
        paragraph.numbering(
            NumberingId::new(HEADING_NUMBERING_ID),
            IndentLevel::new(level.saturating_sub(1)),
        )
    } else {
        paragraph
    }
}

pub fn empty_state(ctx: &Ctx, text: &str) -> Paragraph {
    Paragraph::new()
        .line_spacing(LineSpacing::new().after(140))
        .add_run(
            Run::new()
                .add_text(text)
                .size(half_points(ctx.tokens.typography.small_pt))
                .italic()
                .color(hex(&ctx.tokens.palette.muted))
                .fonts(ctx.sans()),
        )
}

pub fn caption(ctx: &Ctx, text: &str) -> Paragraph {
    Paragraph::new()
        .style("Caption")
        .align(AlignmentType::Center)
        .line_spacing(LineSpacing::new().before(60).after(180))
        .add_run(
            Run::new()
                .add_text(text)
                .size(half_points(ctx.tokens.typography.small_pt))
                .italic()
                .color(hex(&ctx.tokens.palette.muted))
                .fonts(ctx.sans()),
        )
}

pub fn divider(ctx: &Ctx, title: &str) -> Table {
    let cell = TableCell::new()
        .width(ctx.usable_twips as usize, WidthType::Dxa)
        .vertical_align(VAlignType::Center)
        .add_paragraph(heading(ctx, 1, title));
    let row = TableRow::new(vec![cell])
        .row_height(
            ctx.usable_height_twips
                .saturating_sub(TABLE_TRAILING_PARAGRAPH_TWIPS) as f32,
        )
        .height_rule(docx_rs::HeightRule::Exact);
    let rule = |position| {
        TableBorder::new(position)
            .border_type(BorderType::Single)
            .size(eighths(1.5))
            .color(hex(&ctx.tokens.palette.primary))
    };
    Table::new(vec![row])
        .layout(TableLayoutType::Fixed)
        .width(ctx.usable_twips as usize, WidthType::Dxa)
        .set_grid(vec![ctx.usable_twips as usize])
        .set_borders(
            TableBorders::with_empty()
                .set(rule(TableBorderPosition::Top))
                .set(rule(TableBorderPosition::Bottom)),
        )
        .margins(TableCellMargins::new().margin(120, 160, 120, 160))
}

/// Word breaks lines at spaces and hyphens but not at `/`, so an ARM id would
/// otherwise widen its column until the table runs off the page. Zero-width
/// spaces give it somewhere to break.
pub fn wrappable(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut span = 0;
    for ch in value.chars() {
        out.push(ch);
        span += 1;
        if ch.is_whitespace() {
            span = 0;
        }
        if matches!(ch, '/' | '-' | '.' | '_') || span >= 12 {
            out.push('\u{200B}');
            span = 0;
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
    match if ctx.table_grid {
        TableStyle::SolidHeader
    } else {
        ctx.tokens.layout.table
    } {
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
fn column_widths(weights: &[u32], usable: u32) -> Vec<u32> {
    let total = weights.iter().sum::<u32>().max(1) as f32;
    let mut widths: Vec<u32> = weights
        .iter()
        .map(|w| ((*w as f32 / total) * usable as f32).round() as u32)
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
            Paragraph::new()
                .style("AzdocsTableHeader")
                .keep_next(true)
                .line_spacing(LineSpacing::new().line(252).before(0).after(0))
                .add_run(
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

/// One proportional, icon-marked link treatment for tables and source notes.
pub fn external_link(ctx: &Ctx, url: &str, display: &str, size: f32) -> docx_rs::Hyperlink {
    let mut link = docx_rs::Hyperlink::new(url, docx_rs::HyperlinkType::External);
    if let Ok(png) = crate::diagram::png::from_svg_transparent(
        &crate::report::document::external_link_svg(&ctx.tokens.palette.accent),
        2.0,
    ) {
        link = link.add_run(
            Run::new().add_image(docx_rs::Pic::new(&png).size(pt_to_emu(size), pt_to_emu(size))),
        );
    }
    link.add_run(
        Run::new()
            .add_text(format!("  {}", wrappable(display)))
            .fonts(ctx.sans())
            .size(half_points(size))
            .color(hex(&ctx.tokens.palette.accent)),
    )
}

fn body_cell(
    ctx: &Ctx,
    value: &str,
    width: u32,
    zebra: bool,
    mono: bool,
    target: Option<&TableLink>,
    keep_next: bool,
) -> TableCell {
    let tokens = ctx.tokens;
    let mut run = Run::new()
        .add_text(wrappable(value))
        .size(half_points(tokens.typography.table_pt));
    run = if mono {
        run.fonts(ctx.mono())
    } else {
        run.fonts(ctx.sans())
    };
    let paragraph = Paragraph::new()
        .keep_next(keep_next)
        .line_spacing(LineSpacing::new().line(252).before(0).after(0));
    let paragraph = if let Some(target) = target {
        let paragraph = if target.external {
            let inset = ((tokens.typography.table_pt + 5.0) * TWIPS_PER_POINT).round() as i32;
            paragraph.indent(
                Some(inset),
                Some(SpecialIndentType::Hanging(inset)),
                None,
                None,
            )
        } else {
            paragraph
        };
        paragraph.add_hyperlink(if target.external {
            external_link(ctx, &target.target, value, tokens.typography.table_pt)
        } else {
            docx_rs::Hyperlink::new(&target.target, docx_rs::HyperlinkType::Anchor)
                .add_run(run.color(hex(&tokens.palette.accent)))
        })
    } else {
        paragraph.add_run(run)
    };
    let mut cell = TableCell::new()
        .width(width as usize, WidthType::Dxa)
        .add_paragraph(paragraph);
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
/// Header repetition is injected as OOXML after docx-rs serialises the table.
fn shell(ctx: &Ctx, widths: &[u32], rows: Vec<TableRow>) -> Table {
    let inset = (ctx.tokens.layout.table_inset_pt * TWIPS_PER_POINT).round() as usize;
    Table::new(rows)
        .layout(TableLayoutType::Fixed)
        .width(ctx.usable_twips as usize, WidthType::Dxa)
        .set_grid(widths.iter().map(|w| *w as usize).collect())
        .set_borders(borders(ctx))
        .margins(TableCellMargins::new().margin(inset / 2, inset, inset / 2, inset))
}

/// A data table. `mono_columns` names columns rendered in the monospace face
/// (resource ids and check names).
pub fn data_table(
    ctx: &Ctx,
    headers: &[String],
    rows: &[Vec<String>],
    mono_columns: &[usize],
    weights: &[u32],
    links: &[TableLink],
    keep_together: bool,
) -> Table {
    let widths = column_widths(weights, ctx.usable_twips);
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
                        links
                            .iter()
                            .find(|link| link.row == index && link.column == i),
                        keep_together && index + 1 < rows.len(),
                    )
                })
                .collect(),
        ));
    }
    shell(ctx, &widths, table_rows)
}

/// Group headings share one full-width grid with their metadata fields.
pub fn metadata_table(ctx: &Ctx, groups: &[MetadataGroup], keep_together: bool) -> Table {
    let widths = column_widths(&[1, 2], ctx.usable_twips);
    let mut rows = Vec::new();
    for (group_index, group) in groups.iter().enumerate() {
        rows.push(
            TableRow::new(vec![
                TableCell::new()
                    .grid_span(2)
                    .width(ctx.usable_twips as usize, WidthType::Dxa)
                    .add_paragraph(
                        Paragraph::new()
                            .keep_next(true)
                            .keep_lines(true)
                            .line_spacing(LineSpacing::new().before(60).after(60))
                            .add_run(
                                Run::new()
                                    .add_text(wrappable(&group.title))
                                    .bold()
                                    .fonts(ctx.sans())
                                    .size(half_points(ctx.tokens.typography.table_header_pt)),
                            ),
                    ),
            ])
            .cant_split(),
        );
        for (index, row) in group.rows.iter().enumerate() {
            let keep_next =
                keep_together && (group_index + 1 < groups.len() || index + 1 < group.rows.len());
            rows.push(TableRow::new(
                (0..2)
                    .map(|column| {
                        body_cell(
                            ctx,
                            &row[column],
                            widths[column],
                            false,
                            false,
                            group
                                .links
                                .iter()
                                .find(|link| link.row == index && link.column == column),
                            keep_next,
                        )
                    })
                    .collect(),
            ));
        }
    }
    shell(ctx, &widths, rows)
}

/// Heading with a modest Azure type icon beside it.
///
/// A borderless two-cell table rather than an inline image, because Word sits
/// an inline image on the text baseline and docx-rs exposes no `w:position` to
/// offset it; a taller-than-text icon therefore floats above the words. Cell
/// centring is the only vertical alignment available, and it is what the PDF's
/// `grid(align: horizon)` does anyway.
///
/// The heading paragraph keeps its heading style inside the cell, so the TOC
/// field and Word's navigation pane still find it.
pub fn icon_heading(ctx: &Ctx, level: usize, icon: Option<Run>, display: &str) -> Table {
    let heading_pt = if level == 1 {
        ctx.tokens.typography.h1_pt
    } else {
        ctx.tokens.typography.h2_pt
    };
    let icon_pt = heading_pt * ICON_SCALE;
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
        .add_paragraph(heading(ctx, level, display));
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
        .set_borders(TableBorders::with_empty())
        // No left inset, so the icon lines up with the page's text margin.
        .margins(TableCellMargins::new().margin(0, 0, 60, 0))
}

/// Icon size relative to the level-1 heading, shared by the PDF and DOCX.
pub const ICON_SCALE: f32 = 1.0;

/// Name plate above each resource's detail. A paragraph band keeps the scan
/// marker without turning every resource into another table.
pub fn resource_plate(ctx: &Ctx, name: &str, subtitle: &str, icon: Option<Run>) -> Paragraph {
    let mut paragraph = Paragraph::new()
        .keep_next(true)
        .keep_lines(true)
        .line_spacing(LineSpacing::new().before(240).after(100));
    if let Some(icon) = icon {
        paragraph = paragraph.add_run(icon).add_run(Run::new().add_text("  "));
    }
    paragraph
        .add_run(
            Run::new()
                .add_text(wrappable(name))
                .size(half_points(ctx.tokens.typography.h3_pt))
                .fonts(ctx.sans()),
        )
        .add_run(
            Run::new()
                .add_break(BreakType::TextWrapping)
                .add_text(subtitle)
                .size(half_points(ctx.tokens.typography.small_pt))
                .fonts(ctx.sans())
                .color(hex(&ctx.tokens.palette.muted)),
        )
}

/// Small labelled rule introducing a sub-block (Settings, Findings, Related).
pub fn sub_label(ctx: &Ctx, title: &str) -> Paragraph {
    Paragraph::new()
        .keep_next(true)
        .keep_lines(true)
        .add_run(
            Run::new()
                .add_text(title)
                .bold()
                .size(half_points(ctx.tokens.typography.base_pt))
                .color(hex(&ctx.tokens.palette.primary_dark))
                .fonts(ctx.sans()),
        )
        .line_spacing(LineSpacing::new().before(220).after(100))
}

/// Definition-list paragraphs for settings and compact evidence. Hanging
/// indents align wrapped values under one another without a table grid.
pub fn fact_list(ctx: &Ctx, items: &[Fact<'_>]) -> Vec<Paragraph> {
    let key_width = ctx.usable_twips / 3;
    let fact_pt = (ctx.tokens.typography.base_pt - 1.0).max(ctx.tokens.typography.table_pt);
    items
        .iter()
        .map(|item| {
            let value = Run::new()
                .add_text(wrappable(&item.value))
                .size(half_points(fact_pt));
            let value = if item.mono {
                value.fonts(ctx.mono())
            } else {
                value.fonts(ctx.sans())
            };
            Paragraph::new()
                .add_tab(Tab::new().val(TabValueType::Left).pos(key_width as usize))
                .indent(
                    Some(key_width as i32),
                    Some(SpecialIndentType::Hanging(key_width as i32)),
                    None,
                    None,
                )
                .keep_lines(true)
                .line_spacing(LineSpacing::new().before(30).after(90))
                .add_run(
                    Run::new()
                        .add_text(item.label.as_ref())
                        .size(half_points(fact_pt))
                        .color(hex(&ctx.tokens.palette.primary_dark))
                        .fonts(ctx.sans()),
                )
                .add_run(Run::new().add_tab())
                .add_run(value)
        })
        .collect()
}

/// A flowing resource index with the name first and its Azure context in a
/// quieter typographic register.
pub fn resource_index(ctx: &Ctx, items: &[ResourceIndexItem<'_>]) -> Vec<Paragraph> {
    items
        .iter()
        .map(|item| {
            let context = [
                item.subscription.as_ref(),
                item.resource_group.as_ref(),
                item.location.as_ref(),
            ]
            .into_iter()
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
            let run = Run::new()
                .add_text(item.name.as_ref())
                .size(half_points(ctx.tokens.typography.base_pt))
                .color(hex(if item.target.is_some() {
                    &ctx.tokens.palette.accent
                } else {
                    &ctx.tokens.palette.ink
                }))
                .fonts(ctx.sans());
            let paragraph = Paragraph::new()
                .keep_lines(true)
                .line_spacing(LineSpacing::new().before(30).after(110));
            let mut paragraph = if let Some(target) = &item.target {
                paragraph.add_hyperlink(
                    docx_rs::Hyperlink::new(target, docx_rs::HyperlinkType::Anchor).add_run(run),
                )
            } else {
                paragraph.add_run(run)
            };
            if !context.is_empty() {
                paragraph = paragraph.add_run(
                    Run::new()
                        .add_text(format!("  —  {context}"))
                        .size(half_points(ctx.tokens.typography.small_pt))
                        .color(hex(&ctx.tokens.palette.muted))
                        .fonts(ctx.sans()),
                );
            }
            paragraph
        })
        .collect()
}

/// A finding callout with a semantic severity border and tint. Keeping it as a
/// paragraph lets findings read as a sequence rather than another table.
pub fn callout(ctx: &Ctx, severity: &str, title: &str, detail: Option<&str>) -> Paragraph {
    let colors = ctx.tokens.palette.severity.level(severity);
    let fill = hex(colors.map_or(&ctx.tokens.palette.zebra, |c| &c.fill));
    let border = ParagraphBorder::new(ParagraphBorderPosition::Left)
        .val(BorderType::Single)
        .size(eighths(2.5))
        .space(5)
        .color(hex(colors.map_or(&ctx.tokens.palette.ink, |c| &c.text)));
    let mut paragraph = Paragraph::new()
        .keep_lines(true)
        .indent(Some(120), None, Some(120), None)
        .line_spacing(LineSpacing::new().before(70).after(130))
        .set_borders(ParagraphBorders::with_empty().set(border))
        .add_run(
            Run::new()
                .add_text(format!(" {} ", severity.to_uppercase()))
                .size(half_points(ctx.tokens.typography.small_pt))
                .bold()
                .color(hex(colors.map_or(&ctx.tokens.palette.ink, |c| &c.text)))
                .shading(Shading::new().shd_type(ShdType::Clear).fill(fill))
                .fonts(ctx.sans()),
        )
        .add_run(
            Run::new()
                .add_text(format!("  {title}"))
                .size(half_points(ctx.tokens.typography.base_pt))
                .fonts(ctx.sans()),
        );
    if let Some(detail) = detail {
        paragraph = paragraph.add_run(
            Run::new()
                .add_break(BreakType::TextWrapping)
                .add_text(detail)
                .size(half_points(ctx.tokens.typography.small_pt))
                .color(hex(&ctx.tokens.palette.muted))
                .fonts(ctx.sans()),
        );
    }
    paragraph
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
                        .color(hex(&ctx.tokens.palette.primary_dark))
                        .fonts(ctx.serif()),
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

    let inset = (ctx.tokens.layout.table_inset_pt * TWIPS_PER_POINT).round() as usize;
    let mut table = Table::new(vec![TableRow::new(cells)])
        .layout(TableLayoutType::Fixed)
        .width(ctx.usable_twips as usize, WidthType::Dxa)
        .set_grid(widths.iter().map(|w| *w as usize).collect())
        .margins(TableCellMargins::new().margin(inset, inset, inset, inset));

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
        let widths = column_widths(&[30, 10, 10], 9000);

        assert_eq!(widths.iter().sum::<u32>(), 9000);
        assert!(widths[0] > widths[1], "wider content should win more space");
    }

    #[test]
    fn unit_column_widths_handle_a_single_column() {
        let widths = column_widths(&[10], 9000);

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
