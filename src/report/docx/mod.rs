//! Native DOCX report via `docx-rs`: cover, TOC field (marked for refresh when
//! Word lays out the document), shared estate assessment or technical reference
//! and a branded
//! footer. No external binaries.
//!
//! Every visual decision comes from the resolved theme, so the DOCX and the
//! PDF are the same document in two containers. Two things they cannot share:
//! Word resolves fonts by name on the reader's machine (hence the separate
//! `docx_serif`/`docx_sans`/`docx_mono` theme keys); repeating table headers are added through OOXML.

mod sections;
mod style;

use std::io::Cursor;
use std::path::Path;

use anyhow::Context;
use docx_rs::{LineSpacing, Paragraph, Run, TableOfContents};

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
    render_document(&document, branding, diagrams)
}

/// Render the optional companion without duplicating the main assessment.
pub fn render_reference(
    report: &ReportContext,
    branding: &BrandingContext,
    diagrams: &[DiagramAsset],
) -> anyhow::Result<Vec<u8>> {
    render_document(
        &PrintDocument::reference(report, branding, diagrams),
        branding,
        diagrams,
    )
}

fn render_document(
    document: &PrintDocument<'_>,
    branding: &BrandingContext,
    diagrams: &[DiagramAsset],
) -> anyhow::Result<Vec<u8>> {
    let render_branding = document.render_branding(branding);
    let branding = &render_branding;
    let (mut docx, geometry) = style::document(branding);
    let usable_twips = geometry.usable;
    let ctx = style::Ctx {
        tokens: &branding.tokens,
        labels: &branding.labels,
        usable_twips,
        usable_height_twips: geometry.usable_height,
        page_twips: geometry.page,
        table_grid: document.technical_reference && branding.tokens.layout.reference_table_borders,
    };

    docx = docx.footer(sections::footer(branding, &ctx));
    if branding.tokens.layout.running_header {
        docx = docx.header(sections::header(branding, &ctx));
    }
    docx = sections::cover(docx, &ctx, &document.cover, branding);
    let mut toc = TableOfContents::new()
        .heading_styles_range(1, document.toc_depth as usize)
        .alias(&branding.labels.report.toc_title)
        .hyperlink()
        .add_before_paragraph(
            Paragraph::new()
                .keep_next(true)
                .line_spacing(LineSpacing::new().after(200))
                .add_run(
                    Run::new()
                        .add_text(&branding.labels.report.toc_title)
                        .fonts(ctx.serif())
                        .size(style::half_points(ctx.tokens.typography.h1_pt)),
                ),
        )
        .auto()
        .dirty();
    // A number is only valid after Word has paginated the document.
    toc.page_ref_placeholder = Some(String::new());
    docx = docx.add_table_of_contents(toc);
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

    let document_xml =
        std::str::from_utf8(&package.document).context("reading generated DOCX document XML")?;
    // docx-rs places TOC tabs at 80,000 twips, far beyond A4. Keep
    // leaders inside the actual text column in Word and other readers.
    let document_xml =
        document_xml.replace("w:pos=\"80000\"", &format!("w:pos=\"{usable_twips}\""));
    // docx-rs writes every anchored picture in front of the text. The only
    // anchor the report emits is cover artwork, which belongs behind it;
    // everything else is inline and carries no behindDoc attribute.
    let document_xml = document_xml.replace("behindDoc=\"0\"", "behindDoc=\"1\"");
    package.document = repeat_table_headers(&document_xml).into_bytes();
    // docx-rs collects hyperlink relationships in a hash map, so their order
    // in the part changed from run to run while every id still resolved.
    // Sorting them is what makes two exports of one snapshot identical bytes.
    let document_rels = std::str::from_utf8(&package.document_rels)
        .context("reading generated DOCX relationships XML")?;
    package.document_rels = sort_relationships(document_rels).into_bytes();

    let mut cursor = Cursor::new(Vec::new());
    package.pack(&mut cursor).context("packing DOCX archive")?;
    Ok(cursor.into_inner())
}

/// The `<Relationship …/>` entries of a relationships part in id order.
/// Anything outside the entries (declaration, root element) is kept as is.
fn sort_relationships(xml: &str) -> String {
    let Some(open_end) = xml
        .find("<Relationships")
        .and_then(|start| xml[start..].find('>').map(|offset| start + offset + 1))
    else {
        return xml.to_owned();
    };
    let Some(close) = xml.rfind("</Relationships>") else {
        return xml.to_owned();
    };
    let body = &xml[open_end..close];
    let mut entries: Vec<&str> = body
        .split("<Relationship ")
        .filter(|entry| !entry.trim().is_empty())
        .collect();
    let id_of = |entry: &str| -> (usize, String) {
        let id = entry
            .split_once("Id=\"")
            .and_then(|(_, rest)| rest.split('"').next())
            .unwrap_or_default();
        let digits: String = id.chars().filter(char::is_ascii_digit).collect();
        (digits.parse().unwrap_or(0), id.to_owned())
    };
    entries.sort_by_key(|entry| id_of(entry));
    let mut out = String::with_capacity(xml.len());
    out.push_str(&xml[..open_end]);
    for entry in entries {
        out.push_str("<Relationship ");
        out.push_str(entry);
    }
    out.push_str(&xml[close..]);
    out
}

fn repeat_table_headers(xml: &str) -> String {
    // Mark only semantic data headers, not the first rows of cover/figure tables.
    xml.split("<w:tr>")
        .enumerate()
        .map(|(index, part)| {
            if index == 0 {
                return part.to_owned();
            }
            let end = part.find("</w:tr>").unwrap_or(part.len());
            let row = &part[..end];
            let body = if row.contains("w:val=\"AzdocsTableHeader\"") {
                part.replacen("<w:trPr>", "<w:trPr><w:tblHeader/>", 1)
            } else {
                part.to_owned()
            };
            format!("<w:tr>{body}")
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::sort_relationships;

    #[test]
    fn unit_sort_relationships_orders_entries_by_id_and_keeps_the_envelope() {
        let xml = concat!(
            "<?xml version=\"1.0\"?>",
            "<Relationships xmlns=\"x\">",
            "<Relationship Id=\"rIdHyperlink3\" Target=\"c\" />",
            "<Relationship Id=\"rId1\" Target=\"a\" />",
            "<Relationship Id=\"rIdHyperlink1\" Target=\"b\" />",
            "</Relationships>",
        );
        let sorted = sort_relationships(xml);
        let ids: Vec<&str> = sorted
            .split("Id=\"")
            .skip(1)
            .filter_map(|rest| rest.split('"').next())
            .collect();
        assert_eq!(ids, ["rId1", "rIdHyperlink1", "rIdHyperlink3"]);
        assert!(sorted.starts_with("<?xml") && sorted.ends_with("</Relationships>"));
    }
}
