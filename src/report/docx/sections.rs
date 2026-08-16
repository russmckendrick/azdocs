//! The document's content, section by section. Layout decisions live in
//! `style.rs`; this module only decides what goes in and in what order.

use docx_rs::{
    AlignmentType, BreakType, Docx, FieldCharType, Footer, InstrNUMPAGES, InstrPAGE, InstrText,
    Paragraph, Pic, Run,
};

use super::style::{self, Ctx, EMU_PER_TWIP, half_points, hex};
use crate::diagram::assets::{DiagramAsset, DiagramAssetKind};
use crate::report::branding::BrandingContext;
use crate::report::theme::CoverStyle;
use crate::report::{ReportContext, cell_to_string};

pub fn footer(branding: &BrandingContext) -> Footer {
    let tokens = &branding.tokens;
    let mut text = branding.footer.clone();
    if !branding.company.is_empty() {
        text.push_str(" · ");
        text.push_str(&branding.company);
    }
    text.push_str(" · Page ");
    let small = half_points(tokens.typography.stat_label_pt);
    let color = hex(&tokens.palette.muted);
    let run = |body: &str| Run::new().add_text(body).size(small).color(color.clone());

    Footer::new().add_paragraph(
        Paragraph::new()
            .align(AlignmentType::Center)
            .add_run(run(&text))
            // PAGE and NUMPAGES are fields, so the count stays right after the
            // reader edits the document.
            .add_run(Run::new().add_field_char(FieldCharType::Begin, false))
            .add_run(Run::new().add_instr_text(InstrText::PAGE(InstrPAGE::new())))
            .add_run(Run::new().add_field_char(FieldCharType::End, false))
            .add_run(run(" of "))
            .add_run(Run::new().add_field_char(FieldCharType::Begin, false))
            .add_run(Run::new().add_instr_text(InstrText::NUMPAGES(InstrNUMPAGES::new())))
            .add_run(Run::new().add_field_char(FieldCharType::End, false)),
    )
}

pub fn cover(
    mut docx: Docx,
    ctx: &Ctx,
    report: &ReportContext,
    branding: &BrandingContext,
) -> Docx {
    let tokens = ctx.tokens;
    // Word has no full-bleed page fill, so the block cover degrades to the
    // same centred treatment as the editorial one; band keeps its left rail.
    let centred = matches!(tokens.layout.cover, CoverStyle::Editorial | CoverStyle::Block);
    let align = if centred {
        AlignmentType::Center
    } else {
        AlignmentType::Left
    };

    if let Some(logo) = logo_picture(branding, ctx) {
        docx = docx.add_paragraph(Paragraph::new().align(align).add_run(logo));
    }

    let title = if centred {
        branding.title.to_uppercase()
    } else {
        branding.title.clone()
    };
    docx = docx.add_paragraph(
        Paragraph::new().align(align).add_run(
            Run::new()
                .add_text(title)
                .size(half_points(tokens.typography.title_pt))
                .bold()
                .color(hex(&tokens.palette.primary))
                .fonts(ctx.sans()),
        ),
    );
    if !branding.subtitle.is_empty() {
        docx = docx.add_paragraph(
            Paragraph::new().align(align).add_run(
                Run::new()
                    .add_text(&branding.subtitle)
                    .size(half_points(tokens.typography.subtitle_pt))
                    .color(hex(&tokens.palette.muted))
                    .fonts(ctx.sans()),
            ),
        );
    }
    if !branding.company.is_empty() {
        docx = docx.add_paragraph(
            Paragraph::new().align(align).add_run(
                Run::new()
                    .add_text(&branding.company)
                    .size(half_points(tokens.typography.subtitle_pt))
                    .fonts(ctx.sans()),
            ),
        );
    }

    docx = docx.add_paragraph(Paragraph::new());
    docx = docx.add_paragraph(
        Paragraph::new().align(align).add_run(
            Run::new()
                .add_text(format!(
                    "Tenant {} · snapshot {} · collected {} · status {}",
                    report.tenant_id, report.snapshot_id, report.created_at, report.status
                ))
                .size(half_points(tokens.typography.small_pt))
                .color(hex(&tokens.palette.muted))
                .fonts(ctx.sans()),
        ),
    );
    docx.add_paragraph(page_break())
}

/// Decode the logo before handing it to `Pic`, which panics on an image it
/// cannot read; an unusable logo should cost the reader a logo, not the run.
/// SVG goes through the diagram rasteriser, since `image` cannot read it.
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
    let (width, height) = (
        image::GenericImageView::dimensions(&decoded).0,
        image::GenericImageView::dimensions(&decoded).1,
    );
    if width == 0 || height == 0 {
        return None;
    }

    // Cap the logo at a third of the text width and 2cm tall, preserving the
    // aspect ratio, so an oversized upload cannot swallow the cover.
    let max_width_emu = ctx.usable_twips / 3 * EMU_PER_TWIP;
    let max_height_emu = 1134 / 2 * EMU_PER_TWIP;
    let scale = f64::min(
        f64::from(max_width_emu) / f64::from(width),
        f64::from(max_height_emu) / f64::from(height),
    );
    let emu = |value: u32| (f64::from(value) * scale).round().max(1.0) as u32;

    Some(Run::new().add_image(Pic::new(&png).size(emu(width), emu(height))))
}

pub fn page_break() -> Paragraph {
    Paragraph::new().add_run(Run::new().add_break(BreakType::Page))
}

pub fn summary(mut docx: Docx, ctx: &Ctx, report: &ReportContext) -> Docx {
    docx = docx.add_paragraph(style::heading(1, "Executive Summary"));

    let stats = [
        (report.totals.subscriptions.to_string(), "Subscriptions"),
        (report.totals.resource_groups.to_string(), "Resource groups"),
        (report.totals.resources.to_string(), "Resources"),
        (report.totals.findings.to_string(), "Findings"),
        (
            format!("{}%", report.tag_coverage.percent),
            "Tag coverage",
        ),
    ]
    .map(|(value, label)| (value, label.to_owned()));
    docx = docx.add_table(style::stat_table(ctx, &stats));

    let severity = &report.severity_counts;
    docx = docx.add_paragraph(Paragraph::new());
    docx = docx.add_paragraph(style::body(&format!(
        "Findings by severity: {} high, {} medium, {} low, {} info. \
         {} resources are tagged and {} are untagged.",
        severity.high,
        severity.medium,
        severity.low,
        severity.info,
        report.tag_coverage.tagged,
        report.tag_coverage.untagged,
    )));

    docx = docx.add_paragraph(style::heading(2, "Resources by type"));
    let headers = ["Type", "Azure type", "Count"].map(str::to_owned);
    let rows: Vec<Vec<String>> = report
        .type_counts
        .iter()
        .map(|tc| {
            vec![
                tc.display.clone(),
                tc.azure_type.clone(),
                tc.count.to_string(),
            ]
        })
        .collect();
    docx.add_table(style::data_table(ctx, &headers, &rows, &[]))
}

pub fn findings(mut docx: Docx, ctx: &Ctx, report: &ReportContext) -> Docx {
    docx = docx.add_paragraph(style::heading(1, "Findings"));
    if report.findings.is_empty() {
        return docx.add_paragraph(style::body("No findings."));
    }
    let rows: Vec<[String; 4]> = report
        .findings
        .iter()
        .map(|finding| {
            [
                finding.severity.clone(),
                finding.title.clone(),
                finding.category.clone(),
                finding.query_name.clone(),
            ]
        })
        .collect();
    docx.add_table(style::findings_table(ctx, &rows))
}

pub fn categories(mut docx: Docx, ctx: &Ctx, report: &ReportContext) -> Docx {
    for category in &report.categories {
        docx = docx.add_paragraph(style::heading(1, &capitalise(&category.name)));
        for query in &category.queries {
            docx = docx.add_paragraph(style::heading(2, &query.name));
            if !query.description.is_empty() {
                docx = docx.add_paragraph(style::muted(ctx, &query.description));
            }
            // print_columns is computed once in ReportContext so the PDF trims
            // to exactly the same set.
            let headers = &query.print_columns;
            if headers.is_empty() {
                docx = docx.add_paragraph(style::muted(ctx, "No results."));
                continue;
            }
            let rows: Vec<Vec<String>> = query
                .rows
                .iter()
                .map(|row| {
                    headers
                        .iter()
                        .map(|column| cell_to_string(row.get(column)))
                        .collect()
                })
                .collect();
            docx = docx.add_table(style::data_table(ctx, headers, &rows, &[]));
        }
    }
    docx
}

pub fn subscriptions(mut docx: Docx, ctx: &Ctx, report: &ReportContext) -> Docx {
    docx = docx.add_paragraph(style::heading(1, "Subscriptions"));
    let headers = ["Name", "Type", "Location", "Tags"].map(str::to_owned);
    for sub in &report.subscriptions {
        docx = docx.add_paragraph(style::heading(2, &sub.display_name));
        docx = docx.add_paragraph(style::muted(
            ctx,
            &format!(
                "{} · {} resources",
                sub.subscription_id, sub.resource_count
            ),
        ));
        for rg in &sub.resource_groups {
            if rg.resources.is_empty() {
                continue;
            }
            let label = match &rg.location {
                Some(location) => format!("{} ({location})", rg.name),
                None => rg.name.clone(),
            };
            docx = docx.add_paragraph(style::heading(3, &label));
            let rows: Vec<Vec<String>> = rg
                .resources
                .iter()
                .map(|resource| {
                    vec![
                        resource.name.clone(),
                        resource.display_type.clone(),
                        resource.location.clone().unwrap_or_default(),
                        resource.tags.clone().unwrap_or_default(),
                    ]
                })
                .collect();
            docx = docx.add_table(style::data_table(ctx, &headers, &rows, &[]));
        }
    }
    docx
}

/// Overview diagrams, rasterised to PNG. Word cannot place an SVG, and the
/// estate diagrams are wide, so each is scaled to the full text width.
pub fn diagrams(mut docx: Docx, ctx: &Ctx, assets: &[DiagramAsset]) -> Docx {
    let embeds: Vec<&DiagramAsset> = assets
        .iter()
        .filter(|asset| {
            matches!(
                asset.kind,
                DiagramAssetKind::Hierarchy | DiagramAssetKind::Network
            )
        })
        .collect();
    if embeds.is_empty() {
        return docx;
    }

    docx = docx.add_paragraph(style::heading(1, "Diagrams"));
    for asset in embeds {
        let png = match crate::diagram::png::from_svg(&asset.svg, crate::diagram::png::DEFAULT_SCALE)
        {
            Ok(png) => png,
            Err(error) => {
                tracing::warn!(slug = %asset.slug, %error, "skipping unrenderable diagram");
                continue;
            }
        };
        let Ok(decoded) = image::load_from_memory(&png) else {
            tracing::warn!(slug = %asset.slug, "skipping undecodable diagram");
            continue;
        };
        let (width, height) = image::GenericImageView::dimensions(&decoded);
        if width == 0 || height == 0 {
            continue;
        }
        let target_width = ctx.usable_twips * EMU_PER_TWIP;
        let target_height =
            (f64::from(target_width) * f64::from(height) / f64::from(width)).round() as u32;

        docx = docx.add_paragraph(style::heading(2, &asset.title));
        docx = docx.add_paragraph(
            Paragraph::new()
                .align(AlignmentType::Center)
                .add_run(Run::new().add_image(Pic::new(&png).size(target_width, target_height))),
        );
    }
    docx
}

fn capitalise(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}
