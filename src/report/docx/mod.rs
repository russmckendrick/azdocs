//! Native DOCX report via `docx-rs`: cover, TOC field (marked for refresh when
//! Word lays out the document), executive summary, estate overview, prioritised
//! findings, resource index, Azure hierarchy, evidence appendix and a branded
//! footer. No external binaries.
//!
//! Every visual decision comes from the resolved theme, so the DOCX and the
//! PDF are the same document in two containers. Two things they cannot share:
//! Word resolves fonts by name on the reader's machine (hence the separate
//! `docx_serif`/`docx_sans`/`docx_mono` theme keys), and docx-rs 0.4 cannot
//! repeat a table header across a page break.

mod sections;
mod style;

use std::io::Cursor;
use std::path::Path;

use anyhow::Context;
use docx_rs::TableOfContents;

use super::ReportContext;
use super::branding::BrandingContext;
use super::document::PrintDocument;
use crate::diagram::assets::DiagramAsset;

/// Render the DOCX report and write it to `out_path`.
pub fn write(
    report: &ReportContext,
    branding: &BrandingContext,
    diagrams: &[DiagramAsset],
    out_path: &Path,
) -> anyhow::Result<()> {
    let bytes = render(report, branding, diagrams)?;
    std::fs::write(out_path, bytes).with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}

/// Render the DOCX report to bytes.
pub fn render(
    report: &ReportContext,
    branding: &BrandingContext,
    diagrams: &[DiagramAsset],
) -> anyhow::Result<Vec<u8>> {
    let document = PrintDocument::build(report, branding, diagrams);
    let (mut docx, usable_twips, usable_height_twips) = style::document(branding);
    let ctx = style::Ctx {
        tokens: &branding.tokens,
        labels: &branding.labels,
        usable_twips,
        usable_height_twips,
    };

    docx = docx.footer(sections::footer(branding, &ctx));
    if branding.tokens.layout.running_header {
        docx = docx.header(sections::header(branding, &ctx));
    }
    docx = sections::cover(docx, &ctx, &document.cover, branding);
    docx = docx.add_table_of_contents(
        TableOfContents::new()
            .heading_styles_range(1, document.toc_depth as usize)
            .alias(&branding.labels.report.toc_title)
            .auto()
            .dirty(),
    );
    docx = docx.add_paragraph(sections::page_break());
    docx = sections::render(docx, &ctx, &document.blocks, diagrams);

    let mut package = docx.build();
    // Word owns the final pagination, so cached TOC page references cannot be
    // correct at generation time. docx-rs has no update-fields setting yet;
    // add the standard OOXML request before packing the native package.
    let settings =
        std::str::from_utf8(&package.settings).context("reading generated DOCX settings XML")?;
    let closing_tag = "</w:settings>";
    if !settings.contains(closing_tag) {
        anyhow::bail!("generated DOCX settings XML has no closing settings tag");
    }
    package.settings = settings
        .replacen(
            closing_tag,
            "<w:updateFields w:val=\"true\" /></w:settings>",
            1,
        )
        .into_bytes();

    let mut cursor = Cursor::new(Vec::new());
    package.pack(&mut cursor).context("packing DOCX archive")?;
    Ok(cursor.into_inner())
}
