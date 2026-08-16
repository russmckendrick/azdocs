//! Native DOCX report via `docx-rs`: cover, TOC field (Word offers to update
//! it on open), executive summary, findings with severity shading, capped
//! per-category tables, subscriptions, rasterised overview diagrams and a
//! branded footer. No external binaries.
//!
//! Every visual decision comes from the resolved theme, so the DOCX and the
//! PDF are the same document in two containers. Two things they cannot share:
//! Word resolves fonts by name on the reader's machine (hence the separate
//! `docx_sans`/`docx_mono` theme keys), and docx-rs 0.4 cannot repeat a table
//! header across a page break.

mod sections;
mod style;

use std::io::Cursor;
use std::path::Path;

use anyhow::Context;
use docx_rs::TableOfContents;

use super::ReportContext;
use super::branding::BrandingContext;
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
    let (mut docx, usable_twips) = style::document(branding);
    let ctx = style::Ctx {
        tokens: &branding.tokens,
        usable_twips,
    };

    docx = docx.footer(sections::footer(branding));
    docx = sections::cover(docx, &ctx, report, branding);
    docx = docx.add_table_of_contents(
        TableOfContents::new()
            .heading_styles_range(1, 3)
            .alias("Contents")
            .auto(),
    );
    docx = docx.add_paragraph(sections::page_break());
    docx = sections::summary(docx, &ctx, report);
    docx = sections::findings(docx, &ctx, report);
    docx = sections::categories(docx, &ctx, report);
    docx = sections::type_index(docx, &ctx, report);
    docx = sections::estate(docx, &ctx, report, diagrams);
    docx = sections::diagrams(docx, &ctx, diagrams);

    let mut cursor = Cursor::new(Vec::new());
    docx.build()
        .pack(&mut cursor)
        .context("packing DOCX archive")?;
    Ok(cursor.into_inner())
}
