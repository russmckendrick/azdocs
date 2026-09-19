//! Exhaustive DOCX rendering of the semantic print-document block stream.

use std::collections::HashMap;

use docx_rs::{
    AlignmentType, BorderType, BreakType, Docx, FieldCharType, Footer, Header, HeightRule,
    IndentLevel, InstrNUMPAGES, InstrPAGE, InstrText, LineSpacing, LineSpacingType, NumberingId,
    Paragraph, Pic, Run, Shading, ShdType, SpecialIndentType, Table, TableBorder,
    TableBorderPosition, TableBorders, TableCell, TableCellMargins, TableLayoutType, TableRow,
    VAlignType, WidthType,
};

use super::style::{self, Ctx, half_points, hex, pt_to_emu, twips_to_emu};
use crate::diagram::assets::DiagramAsset;
use crate::mark;
use crate::report::branding::BrandingContext;
use crate::report::document::{Block, Cover, ParagraphStyle, TableKind, TextRun, TextStyle};
use crate::report::theme::CoverStyle;

pub fn header(branding: &BrandingContext, ctx: &Ctx) -> Header {
    let color = hex(&ctx.tokens.palette.muted);
    let size = half_points(ctx.tokens.typography.stat_label_pt);
    let left = Paragraph::new()
        .add_run(Run::new().add_field_char(FieldCharType::Begin, false))
        .add_run(Run::new().add_instr_text(InstrText::Unsupported(
            " STYLEREF \"Heading 1\" ".to_owned(),
        )))
        .add_run(Run::new().add_field_char(FieldCharType::End, false));
    let right = Paragraph::new().align(AlignmentType::Right).add_run(
        Run::new()
            .add_text(&branding.title)
            .size(size)
            .color(color)
            .fonts(ctx.sans()),
    );
    Header::new().add_table(chrome_table(ctx, left, right, true))
}

pub fn footer(branding: &BrandingContext, ctx: &Ctx) -> Footer {
    let mut left_text = branding.footer.clone();
    if !branding.company.is_empty() {
        left_text.push_str(" · ");
        left_text.push_str(&branding.company);
    }
    let size = half_points(ctx.tokens.typography.stat_label_pt);
    let color = hex(&ctx.tokens.palette.muted);
    let left = Paragraph::new().add_run(
        Run::new()
            .add_text(left_text)
            .size(size)
            .color(color.clone())
            .fonts(ctx.sans()),
    );
    let right = Paragraph::new()
        .align(AlignmentType::Right)
        .add_run(Run::new().add_field_char(FieldCharType::Begin, false))
        .add_run(Run::new().add_instr_text(InstrText::PAGE(InstrPAGE::new())))
        .add_run(Run::new().add_field_char(FieldCharType::End, false))
        .add_run(
            Run::new()
                .add_text(" / ")
                .size(size)
                .color(color)
                .fonts(ctx.sans()),
        )
        .add_run(Run::new().add_field_char(FieldCharType::Begin, false))
        .add_run(Run::new().add_instr_text(InstrText::NUMPAGES(InstrNUMPAGES::new())))
        .add_run(Run::new().add_field_char(FieldCharType::End, false));
    Footer::new().add_table(chrome_table(ctx, left, right, false))
}

fn chrome_table(ctx: &Ctx, left: Paragraph, right: Paragraph, top_rule: bool) -> Table {
    let left_width = ctx.usable_twips * 2 / 3;
    let widths = [left_width, ctx.usable_twips - left_width];
    let border_position = if top_rule {
        TableBorderPosition::Bottom
    } else {
        TableBorderPosition::Top
    };
    Table::new(vec![TableRow::new(vec![
        TableCell::new()
            .width(widths[0] as usize, WidthType::Dxa)
            .add_paragraph(left),
        TableCell::new()
            .width(widths[1] as usize, WidthType::Dxa)
            .add_paragraph(right),
    ])])
    .layout(TableLayoutType::Fixed)
    .width(ctx.usable_twips as usize, WidthType::Dxa)
    .set_grid(widths.iter().map(|width| *width as usize).collect())
    .set_borders(
        TableBorders::with_empty().set(
            TableBorder::new(border_position)
                .border_type(BorderType::Single)
                .size((ctx.tokens.layout.rule_pt * 8.0).round().max(1.0) as usize)
                .color(hex(&ctx.tokens.palette.rule)),
        ),
    )
    .margins(TableCellMargins::new().margin(30, 0, 30, 0))
}

pub fn cover(mut docx: Docx, ctx: &Ctx, cover: &Cover<'_>, branding: &BrandingContext) -> Docx {
    let page_filling_table = matches!(ctx.tokens.layout.cover, CoverStyle::Block);
    match ctx.tokens.layout.cover {
        CoverStyle::Band => {
            docx = docx.add_table(cover_band(ctx));
            if let Some(mark) = product_mark_picture(ctx, cover, false) {
                docx = docx.add_paragraph(cover_identity(mark, AlignmentType::Left));
            }
            if let Some(logo) = logo_picture(branding, ctx) {
                docx = docx.add_paragraph(cover_identity(logo, AlignmentType::Left));
            }
            docx = add_cover_text(docx, ctx, cover, AlignmentType::Left, false);
        }
        CoverStyle::Editorial => {
            if let Some(mark) = product_mark_picture(ctx, cover, false) {
                docx = docx.add_paragraph(cover_identity(mark, AlignmentType::Center));
            }
            if let Some(logo) = logo_picture(branding, ctx) {
                docx = docx.add_paragraph(cover_identity(logo, AlignmentType::Center));
            }
            docx = add_cover_text(docx, ctx, cover, AlignmentType::Center, false);
        }
        CoverStyle::Block => {
            docx = docx.add_table(block_cover(
                ctx,
                cover,
                product_mark_picture(ctx, cover, true),
                logo_picture(branding, ctx),
            ));
        }
    }
    if page_filling_table {
        docx
    } else {
        docx.add_paragraph(page_break())
    }
}

fn cover_band(ctx: &Ctx) -> Table {
    let row = TableRow::new(vec![
        TableCell::new()
            .width(ctx.usable_twips as usize, WidthType::Dxa)
            .shading(
                Shading::new()
                    .shd_type(ShdType::Clear)
                    .fill(hex(&ctx.tokens.palette.band)),
            )
            .add_paragraph(Paragraph::new()),
    ])
    .row_height(ctx.tokens.layout.cover_band_pt * 20.0)
    .height_rule(HeightRule::Exact);
    Table::new(vec![row])
        .layout(TableLayoutType::Fixed)
        .width(ctx.usable_twips as usize, WidthType::Dxa)
        .set_grid(vec![ctx.usable_twips as usize])
        .set_borders(TableBorders::with_empty())
        .margins(TableCellMargins::new().margin(0, 0, 0, 0))
}

fn block_cover(
    ctx: &Ctx,
    cover: &Cover<'_>,
    product_mark: Option<Run>,
    logo: Option<Run>,
) -> Table {
    let mut cell = TableCell::new()
        .width(ctx.usable_twips as usize, WidthType::Dxa)
        .vertical_align(VAlignType::Center)
        .shading(
            Shading::new()
                .shd_type(ShdType::Clear)
                .fill(hex(&ctx.tokens.palette.band)),
        );
    if let Some(product_mark) = product_mark {
        cell = cell.add_paragraph(cover_identity(product_mark, AlignmentType::Center));
    }
    if let Some(logo) = logo {
        cell = cell.add_paragraph(cover_identity(logo, AlignmentType::Center));
    }
    cell = cell.add_paragraph(cover_title(ctx, cover, AlignmentType::Center, true));
    if !cover.subtitle.is_empty() {
        cell = cell.add_paragraph(cover_subtitle(
            ctx,
            &cover.subtitle,
            AlignmentType::Center,
            true,
        ));
    }
    if !cover.company.is_empty() {
        cell = cell.add_paragraph(cover_company(
            ctx,
            &cover.company,
            AlignmentType::Center,
            true,
        ));
    }
    cell = cell.add_paragraph(cover_metadata(ctx, cover, AlignmentType::Center, true));

    let row = TableRow::new(vec![cell])
        .row_height(
            ctx.usable_height_twips
                .saturating_sub(style::TABLE_TRAILING_PARAGRAPH_TWIPS) as f32,
        )
        .height_rule(HeightRule::Exact);
    Table::new(vec![row])
        .layout(TableLayoutType::Fixed)
        .width(ctx.usable_twips as usize, WidthType::Dxa)
        .set_grid(vec![ctx.usable_twips as usize])
        .set_borders(TableBorders::with_empty())
        .margins(TableCellMargins::new().margin(180, 300, 180, 300))
}

fn add_cover_text(
    mut docx: Docx,
    ctx: &Ctx,
    cover: &Cover<'_>,
    align: AlignmentType,
    reversed: bool,
) -> Docx {
    docx = docx.add_paragraph(cover_title(ctx, cover, align, reversed));
    if !cover.subtitle.is_empty() {
        docx = docx.add_paragraph(cover_subtitle(ctx, &cover.subtitle, align, reversed));
    }
    if !cover.company.is_empty() {
        docx = docx.add_paragraph(cover_company(ctx, &cover.company, align, reversed));
    }
    docx.add_paragraph(cover_metadata(ctx, cover, align, reversed))
}

fn cover_identity(run: Run, align: AlignmentType) -> Paragraph {
    Paragraph::new()
        .align(align)
        .line_spacing(LineSpacing::new().after(160))
        .add_run(run)
}

fn cover_title(ctx: &Ctx, cover: &Cover<'_>, align: AlignmentType, reversed: bool) -> Paragraph {
    Paragraph::new()
        .align(align)
        .line_spacing(LineSpacing::new().before(160).after(240))
        .add_run(
            Run::new()
                .add_text(cover.title.as_ref())
                .size(half_points(ctx.tokens.typography.title_pt))
                .bold()
                .color(hex(if reversed {
                    &ctx.tokens.palette.on_band
                } else {
                    &ctx.tokens.palette.primary
                }))
                .fonts(ctx.serif()),
        )
}

fn cover_subtitle(ctx: &Ctx, subtitle: &str, align: AlignmentType, reversed: bool) -> Paragraph {
    Paragraph::new()
        .align(align)
        .line_spacing(LineSpacing::new().after(140))
        .add_run(
            Run::new()
                .add_text(subtitle)
                .size(half_points(ctx.tokens.typography.subtitle_pt))
                .color(hex(if reversed {
                    &ctx.tokens.palette.on_band
                } else {
                    &ctx.tokens.palette.muted
                }))
                .fonts(ctx.sans()),
        )
}

fn cover_company(ctx: &Ctx, company: &str, align: AlignmentType, reversed: bool) -> Paragraph {
    Paragraph::new()
        .align(align)
        .line_spacing(LineSpacing::new().after(280))
        .add_run(
            Run::new()
                .add_text(company)
                .size(half_points(ctx.tokens.typography.subtitle_pt))
                .color(hex(if reversed {
                    &ctx.tokens.palette.on_band
                } else {
                    &ctx.tokens.palette.ink
                }))
                .fonts(ctx.sans()),
        )
}

fn cover_metadata(ctx: &Ctx, cover: &Cover<'_>, align: AlignmentType, reversed: bool) -> Paragraph {
    let color = hex(if reversed {
        &ctx.tokens.palette.on_band
    } else {
        &ctx.tokens.palette.muted
    });
    let size = half_points(ctx.tokens.typography.small_pt);
    let label = |text: &str| {
        Run::new()
            .add_text(text)
            .size(size)
            .color(color.clone())
            .fonts(ctx.sans())
    };
    let value = |text: &str| {
        Run::new()
            .add_text(style::wrappable(text))
            .size(size)
            .color(color.clone())
            .fonts(ctx.mono())
    };
    let words = &ctx.labels.common.cover;
    Paragraph::new()
        .align(align)
        .line_spacing(LineSpacing::new().before(160))
        .add_run(label(&format!("{} ", words.tenant)))
        .add_run(value(&cover.tenant))
        .add_run(label(&format!(" · {} ", words.snapshot)))
        .add_run(value(&cover.snapshot))
        .add_run(label(&format!(" · {} ", words.collected)))
        .add_run(label(&cover.collected))
        .add_run(label(&format!(" · {} ", words.status)))
        .add_run(label(&cover.status))
}

pub fn page_break() -> Paragraph {
    Paragraph::new()
        .line_spacing(
            LineSpacing::new()
                .line_rule(LineSpacingType::Exact)
                .line(20)
                .before(0)
                .after(0),
        )
        .add_run(Run::new().size(2).add_break(BreakType::Page))
}

pub fn render(mut docx: Docx, ctx: &Ctx, blocks: &[Block<'_>], assets: &[DiagramAsset]) -> Docx {
    let assets_by_slug: HashMap<&str, &DiagramAsset> = assets
        .iter()
        .map(|asset| (asset.slug.as_str(), asset))
        .collect();
    let mut first_chapter = true;

    for (block_index, block) in blocks.iter().enumerate() {
        match block {
            Block::Section {
                level,
                title,
                id,
                break_before,
            } => {
                let paragraph = style::heading(ctx, usize::from(*level), title)
                    .page_break_before(*break_before)
                    .add_bookmark_start(block_index + 1000, id)
                    .add_bookmark_end(block_index + 1000);
                docx = docx.add_paragraph(paragraph);
            }
            Block::CrossReference { target, title } => {
                let hyperlink = docx_rs::Hyperlink::new(target, docx_rs::HyperlinkType::Anchor)
                    .add_run(
                        Run::new()
                            .add_text(style::wrappable(title))
                            .fonts(ctx.sans())
                            .size(half_points(ctx.tokens.typography.small_pt))
                            .color(hex(&ctx.tokens.palette.accent)),
                    );
                docx = docx.add_paragraph(
                    Paragraph::new()
                        .add_hyperlink(hyperlink)
                        .line_spacing(LineSpacing::new().after(140)),
                );
            }
            Block::ExternalLink { title, url } => {
                docx = docx.add_paragraph(external_link_paragraph(ctx, url, Some(title)));
            }
            Block::Metadata {
                groups,
                keep_together,
            } => {
                docx = docx.add_table(style::metadata_table(ctx, groups, *keep_together));
            }
            Block::Chart { slug, svg, caption } => {
                let asset = DiagramAsset {
                    slug: slug.clone(),
                    svg: svg.clone(),
                    title: String::new(),
                    kind: crate::diagram::assets::DiagramAssetKind::Network,
                    mermaid: String::new(),
                    group_key: None,
                };
                if let Some(run) = diagram_run(ctx, &asset) {
                    docx = docx.add_paragraph(Paragraph::new().keep_next(true).add_run(run));
                    docx = docx.add_paragraph(style::caption(ctx, caption));
                }
            }
            Block::Chapter {
                title,
                break_before,
                divider,
            } => {
                docx = render_chapter(docx, ctx, title, *break_before, *divider, first_chapter);
                first_chapter = false;
            }
            Block::Heading { level, title, icon } => {
                if let Some(azure_type) = icon {
                    docx = docx.add_table(style::icon_heading(
                        ctx,
                        usize::from(*level),
                        icon_run(ctx, azure_type, usize::from(*level)),
                        title,
                    ));
                } else {
                    docx = docx.add_paragraph(style::heading(ctx, usize::from(*level), title));
                }
            }
            Block::Paragraph { style, runs } => {
                docx = docx.add_paragraph(rich_paragraph(ctx, *style, runs));
            }
            Block::BulletList { items } => {
                for item in items {
                    docx = docx.add_paragraph(
                        rich_paragraph(
                            ctx,
                            ParagraphStyle::Muted,
                            &[crate::report::document::normal(item.as_ref())],
                        )
                        .numbering(
                            NumberingId::new(style::BULLET_NUMBERING_ID),
                            IndentLevel::new(0),
                        )
                        .indent(Some(280), Some(SpecialIndentType::Hanging(160)), None, None)
                        .keep_lines(true),
                    );
                }
            }
            Block::Statistics { items } => {
                let stats: Vec<(String, String)> = items
                    .iter()
                    .map(|item| (item.value.to_string(), item.label.to_string()))
                    .collect();
                docx = docx.add_table(style::stat_table(ctx, &stats));
            }
            Block::Table {
                style: table_kind,
                keep_together,
                columns,
                rows,
                links,
            } => {
                let headers: Vec<String> = columns
                    .iter()
                    .map(|column| column.label.to_string())
                    .collect();
                let body: Vec<Vec<String>> = rows
                    .iter()
                    .map(|row| row.iter().map(ToString::to_string).collect())
                    .collect();
                let mono_columns: Vec<usize> = columns
                    .iter()
                    .enumerate()
                    .filter_map(|(index, column)| column.mono.then_some(index))
                    .collect();
                docx = match table_kind {
                    TableKind::Data => docx.add_table(style::data_table(
                        ctx,
                        &headers,
                        &body,
                        &mono_columns,
                        &columns
                            .iter()
                            .map(|column| column.weight)
                            .collect::<Vec<_>>(),
                        links,
                        *keep_together,
                    )),
                };
            }
            Block::Facts { items } => {
                for paragraph in style::fact_list(ctx, items) {
                    docx = docx.add_paragraph(paragraph);
                }
            }
            Block::ResourceIndex { items } => {
                for paragraph in style::resource_index(ctx, items) {
                    docx = docx.add_paragraph(paragraph);
                }
            }
            Block::ResourcePlate {
                id,
                name,
                icon,
                subtitle,
            } => {
                docx = docx.add_paragraph(
                    style::resource_plate(ctx, name, subtitle, icon_run(ctx, icon, 3))
                        .add_bookmark_start(block_index + 1000, id)
                        .add_bookmark_end(block_index + 1000),
                );
            }
            Block::SubLabel { title } => {
                docx = docx.add_paragraph(style::sub_label(ctx, title));
            }
            Block::Callout {
                severity,
                title,
                detail,
            } => {
                docx = docx.add_paragraph(style::callout(ctx, severity, title, detail.as_deref()));
            }
            Block::RasterImage { png, caption, .. } => {
                let Ok(decoded) = image::load_from_memory(png) else {
                    tracing::warn!("skipping corrupt website image");
                    continue;
                };
                let target_width = twips_to_emu(ctx.usable_twips);
                let target_height = (f64::from(target_width) * f64::from(decoded.height())
                    / f64::from(decoded.width()))
                .round() as u32;
                docx = docx.add_paragraph(
                    Paragraph::new()
                        .align(AlignmentType::Center)
                        .keep_next(true)
                        .add_run(
                            Run::new().add_image(Pic::new(png).size(target_width, target_height)),
                        ),
                );
                docx = docx.add_paragraph(style::caption(ctx, caption).align(AlignmentType::Left));
            }
            Block::Diagram { slug, caption } => {
                let Some(asset) = assets_by_slug.get(slug.as_ref()) else {
                    tracing::warn!(slug = %slug, "print document references a missing diagram");
                    continue;
                };
                if let Some(run) = diagram_run(ctx, asset) {
                    docx = docx.add_paragraph(
                        Paragraph::new()
                            .align(AlignmentType::Center)
                            .keep_next(caption.is_some())
                            .line_spacing(LineSpacing::new().after(100))
                            .add_run(run),
                    );
                    if let Some(caption) = caption {
                        docx = docx.add_paragraph(style::caption(ctx, caption));
                    }
                }
            }
            Block::EmptyState { text } => {
                docx = docx.add_paragraph(style::empty_state(ctx, text));
            }
        }
        // Word merges adjacent tables unless a paragraph separates them.
        // Keep each website's evidence visibly distinct, as it is in the PDF.
        if matches!(block, Block::Table { .. } | Block::Metadata { .. })
            && blocks
                .get(block_index + 1)
                .is_some_and(|next| matches!(next, Block::Table { .. } | Block::Metadata { .. }))
        {
            docx = docx.add_paragraph(
                Paragraph::new().line_spacing(
                    LineSpacing::new()
                        .before(0)
                        .after(140)
                        .line_rule(LineSpacingType::Exact)
                        .line(1),
                ),
            );
        }
    }
    docx
}

fn external_link_paragraph(ctx: &Ctx, url: &str, title: Option<&str>) -> Paragraph {
    let size = ctx.tokens.typography.small_pt;
    let mut paragraph = Paragraph::new().keep_lines(true).line_spacing(
        LineSpacing::new()
            .before(100)
            .after(140)
            .line_rule(LineSpacingType::AtLeast)
            .line((size * ctx.tokens.typography.line_height * 20.0).round() as i32),
    );
    if let Some(title) = title {
        paragraph = paragraph.add_run(
            Run::new()
                .add_text(title)
                .add_tab()
                .bold()
                .fonts(ctx.sans())
                .size(half_points(size))
                .color(hex(&ctx.tokens.palette.muted)),
        );
    }
    paragraph
        .add_tab(
            docx_rs::Tab::new()
                .val(docx_rs::TabValueType::Left)
                .pos(1520),
        )
        .indent(
            Some(1520),
            Some(SpecialIndentType::Hanging(1520)),
            None,
            None,
        )
        .add_hyperlink(style::external_link(ctx, url, url, size))
}

fn render_chapter(
    mut docx: Docx,
    ctx: &Ctx,
    title: &str,
    break_before: bool,
    divider: bool,
    first: bool,
) -> Docx {
    if ctx.tokens.layout.divider_pages && divider {
        if !first {
            docx = docx.add_paragraph(page_break());
        }
        docx.add_table(style::divider(ctx, title))
    } else {
        let heading = style::heading(ctx, 1, title).page_break_before(break_before);
        docx.add_paragraph(heading)
    }
}

fn rich_paragraph(ctx: &Ctx, paragraph_style: ParagraphStyle, runs: &[TextRun<'_>]) -> Paragraph {
    let size = match paragraph_style {
        ParagraphStyle::Body => ctx.tokens.typography.base_pt,
        ParagraphStyle::Muted => ctx.tokens.typography.small_pt,
    };
    let after = match paragraph_style {
        ParagraphStyle::Body => 180,
        ParagraphStyle::Muted => 140,
    };
    let mut paragraph = Paragraph::new().line_spacing(
        LineSpacing::new()
            // Word's automatic multiplier includes the font's extra leading.
            // Use the theme's point-based baseline distance, allowing Word to
            // expand a line for taller glyphs rather than clipping them.
            .line_rule(LineSpacingType::AtLeast)
            .line((size * ctx.tokens.typography.line_height * 20.0).round() as i32)
            .after(after),
    );
    for text_run in runs {
        let mut run = Run::new()
            .add_text(if text_run.style == TextStyle::Mono {
                style::wrappable(&text_run.text)
            } else {
                text_run.text.to_string()
            })
            .size(half_points(size))
            .color(hex(match paragraph_style {
                ParagraphStyle::Body => &ctx.tokens.palette.ink,
                ParagraphStyle::Muted => &ctx.tokens.palette.muted,
            }));
        run = match text_run.style {
            TextStyle::Normal => run.fonts(ctx.sans()),
            TextStyle::Mono => run.fonts(ctx.mono()),
        };
        paragraph = paragraph.add_run(run);
    }
    paragraph
}

fn logo_picture(branding: &BrandingContext, ctx: &Ctx) -> Option<Run> {
    let logo = branding.logo.as_ref()?;
    let png = if logo.extension == "svg" {
        let svg = std::str::from_utf8(&logo.bytes).ok()?;
        crate::diagram::png::from_svg(svg, crate::diagram::png::DEFAULT_SCALE)
            .map_err(|error| tracing::warn!(%error, "skipping unrenderable branding logo"))
            .ok()?
    } else {
        logo.bytes.clone()
    };
    let decoded = image::load_from_memory(&png)
        .map_err(|error| tracing::warn!(%error, "skipping undecodable branding logo"))
        .ok()?;
    let (width, height) = image::GenericImageView::dimensions(&decoded);
    if width == 0 || height == 0 {
        return None;
    }
    let max_width_emu = twips_to_emu(ctx.usable_twips / 3);
    let max_height_emu = twips_to_emu(1134);
    let scale = f64::min(
        f64::from(max_width_emu) / f64::from(width),
        f64::from(max_height_emu) / f64::from(height),
    );
    let emu = |value: u32| (f64::from(value) * scale).round().max(1.0) as u32;
    Some(Run::new().add_image(Pic::new(&png).size(emu(width), emu(height))))
}

fn product_mark_picture(ctx: &Ctx, cover: &Cover<'_>, on_dark: bool) -> Option<Run> {
    if !cover.product_mark {
        return None;
    }
    let svg = if on_dark {
        mark::on_dark_svg()
    } else {
        mark::primary_svg()
    };
    let svg = std::str::from_utf8(svg).ok()?;
    // LibreOffice loses subsequent cover text after a transparent image inside
    // an exactly sized, filled table cell. Give the dark-cover variant an
    // opaque matte matching the cell; the light covers can retain transparency.
    let dark_svg = on_dark.then(|| svg_on_background(svg, &hex(&ctx.tokens.palette.band)));
    let png = if let Some(dark_svg) = dark_svg {
        crate::diagram::png::from_svg(&dark_svg?, 0.25)
    } else {
        crate::diagram::png::from_svg_transparent(svg, 0.25)
    }
    .map_err(|error| tracing::warn!(%error, "skipping unrenderable azdocs product mark"))
    .ok()?;
    image::load_from_memory(&png)
        .map_err(|error| tracing::warn!(%error, "skipping undecodable azdocs product mark"))
        .ok()?;
    let side = pt_to_emu(48.0);
    Some(Run::new().add_image(Pic::new(&png).size(side, side)))
}

fn svg_on_background(svg: &str, fill: &str) -> Option<String> {
    let svg_start = svg.find("<svg")?;
    let tag_end = svg_start + svg[svg_start..].find('>')? + 1;
    let (head, tail) = svg.split_at(tag_end);
    Some(format!(
        "{head}<rect width=\"100%\" height=\"100%\" fill=\"#{fill}\"/>{tail}"
    ))
}

fn icon_run(ctx: &Ctx, azure_type: &str, level: usize) -> Option<Run> {
    let svg = String::from_utf8(crate::diagram::icons::svg_bytes(azure_type)).ok()?;
    let png = crate::diagram::png::from_svg(&svg, crate::diagram::png::DEFAULT_SCALE).ok()?;
    image::load_from_memory(&png).ok()?;
    let heading_size = match level {
        1 => ctx.tokens.typography.h1_pt,
        2 => ctx.tokens.typography.h2_pt,
        _ => ctx.tokens.typography.h3_pt,
    };
    let side = pt_to_emu(heading_size * style::ICON_SCALE);
    Some(Run::new().add_image(Pic::new(&png).size(side, side)))
}

fn diagram_run(ctx: &Ctx, asset: &DiagramAsset) -> Option<Run> {
    let png = crate::diagram::png::from_svg(&asset.svg, crate::diagram::png::DEFAULT_SCALE)
        .map_err(
            |error| tracing::warn!(slug = %asset.slug, %error, "skipping unrenderable diagram"),
        )
        .ok()?;
    let decoded = image::load_from_memory(&png).ok()?;
    let (width, height) = image::GenericImageView::dimensions(&decoded);
    if width == 0 || height == 0 {
        return None;
    }
    let target_width = twips_to_emu(ctx.usable_twips);
    let target_height =
        (f64::from(target_width) * f64::from(height) / f64::from(width)).round() as u32;
    Some(Run::new().add_image(Pic::new(&png).size(target_width, target_height)))
}
