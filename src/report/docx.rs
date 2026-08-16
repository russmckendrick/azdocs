//! Native DOCX report via `docx-rs`: cover, TOC field (Word offers to update
//! it on open), executive summary, findings with severity shading, capped
//! per-category tables, subscriptions, and a branded footer. No external
//! binaries; diagrams are not embedded in v1.

use std::io::Cursor;
use std::path::Path;

use anyhow::Context;
use docx_rs::{
    AlignmentType, BreakType, Docx, FieldCharType, Footer, InstrPAGE, InstrText, Paragraph, Run,
    Shading, ShdType, Style, StyleType, Table, TableCell, TableOfContents, TableRow,
};

use super::branding::BrandingContext;
use super::{ReportContext, cell_to_string, page_columns};

/// Fill colors for severity cells (same palette as the XLSX emitter).
const SEVERITY_FILLS: [(&str, &str); 4] = [
    ("high", "F8CECC"),
    ("medium", "FFE6CC"),
    ("low", "FFF2CC"),
    ("info", "DAE8FC"),
];

/// Render the DOCX report and write it to `out_path`.
pub fn write(
    report: &ReportContext,
    branding: &BrandingContext,
    out_path: &Path,
) -> anyhow::Result<()> {
    let bytes = render(report, branding)?;
    std::fs::write(out_path, bytes).with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}

/// Render the DOCX report to bytes.
pub fn render(report: &ReportContext, branding: &BrandingContext) -> anyhow::Result<Vec<u8>> {
    let primary = hex(&branding.primary_color);
    let mut docx = Docx::new()
        .add_style(heading_style("Heading1", "Heading 1", 32, &primary))
        .add_style(heading_style("Heading2", "Heading 2", 26, &primary))
        .footer(footer(branding));

    docx = cover(docx, report, branding, &primary);
    docx = docx.add_table_of_contents(
        TableOfContents::new()
            .heading_styles_range(1, 2)
            .alias("Contents")
            .auto(),
    );
    docx = docx.add_paragraph(page_break());
    docx = summary(docx, report);
    docx = findings(docx, report);
    docx = categories(docx, report);
    docx = subscriptions(docx, report);

    let mut cursor = Cursor::new(Vec::new());
    docx.build()
        .pack(&mut cursor)
        .context("packing DOCX archive")?;
    Ok(cursor.into_inner())
}

/// Strip the leading `#` for OOXML color attributes.
fn hex(color: &str) -> String {
    color.trim_start_matches('#').to_owned()
}

fn heading_style(id: &str, name: &str, half_points: usize, color: &str) -> Style {
    Style::new(id, StyleType::Paragraph)
        .name(name)
        .size(half_points)
        .bold()
        .color(color)
}

fn heading1(text: &str) -> Paragraph {
    Paragraph::new()
        .style("Heading1")
        .add_run(Run::new().add_text(text))
}

fn heading2(text: &str) -> Paragraph {
    Paragraph::new()
        .style("Heading2")
        .add_run(Run::new().add_text(text))
}

fn body_text(text: &str) -> Paragraph {
    Paragraph::new().add_run(Run::new().add_text(text))
}

fn page_break() -> Paragraph {
    Paragraph::new().add_run(Run::new().add_break(BreakType::Page))
}

fn footer(branding: &BrandingContext) -> Footer {
    let mut text = branding.footer.clone();
    if !branding.company.is_empty() {
        text.push_str(" · ");
        text.push_str(&branding.company);
    }
    text.push_str(" · Page ");
    Footer::new().add_paragraph(
        Paragraph::new()
            .align(AlignmentType::Center)
            .add_run(Run::new().add_text(text).size(16))
            .add_run(Run::new().add_field_char(FieldCharType::Begin, false))
            .add_run(Run::new().add_instr_text(InstrText::PAGE(InstrPAGE::new())))
            .add_run(Run::new().add_field_char(FieldCharType::End, false)),
    )
}

fn cover(
    mut docx: Docx,
    report: &ReportContext,
    branding: &BrandingContext,
    primary: &str,
) -> Docx {
    if !branding.company.is_empty() {
        docx = docx.add_paragraph(
            Paragraph::new()
                .align(AlignmentType::Center)
                .add_run(Run::new().add_text(&branding.company).size(28)),
        );
    }
    docx = docx.add_paragraph(
        Paragraph::new().align(AlignmentType::Center).add_run(
            Run::new()
                .add_text(&branding.title)
                .size(56)
                .bold()
                .color(primary),
        ),
    );
    if !branding.subtitle.is_empty() {
        docx = docx.add_paragraph(
            Paragraph::new()
                .align(AlignmentType::Center)
                .add_run(Run::new().add_text(&branding.subtitle).size(28)),
        );
    }
    docx = docx.add_paragraph(Paragraph::new().align(AlignmentType::Center).add_run(
        Run::new().add_text(format!(
            "Tenant {} · snapshot {} · collected {} · status {}",
            report.tenant_id, report.snapshot_id, report.created_at, report.status
        )),
    ));
    docx.add_paragraph(page_break())
}

fn summary(mut docx: Docx, report: &ReportContext) -> Docx {
    docx = docx.add_paragraph(heading1("Executive Summary"));
    let rows: Vec<(&str, String)> = vec![
        ("Subscriptions", report.totals.subscriptions.to_string()),
        ("Resource groups", report.totals.resource_groups.to_string()),
        ("Resources", report.totals.resources.to_string()),
        ("Findings", report.totals.findings.to_string()),
        ("High severity", report.severity_counts.high.to_string()),
        ("Medium severity", report.severity_counts.medium.to_string()),
        ("Low severity", report.severity_counts.low.to_string()),
        ("Info severity", report.severity_counts.info.to_string()),
        ("Tag coverage", format!("{}%", report.tag_coverage.percent)),
    ];
    let table_rows = rows
        .into_iter()
        .map(|(label, value)| {
            TableRow::new(vec![
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(label).bold())),
                TableCell::new().add_paragraph(body_text(&value)),
            ])
        })
        .collect();
    docx = docx.add_table(Table::new(table_rows));

    docx = docx.add_paragraph(heading2("Resources by type"));
    let mut type_rows = vec![header_row(&["Type", "Azure type", "Count"])];
    for tc in &report.type_counts {
        type_rows.push(TableRow::new(vec![
            text_cell(&tc.display),
            text_cell(&tc.azure_type),
            text_cell(&tc.count.to_string()),
        ]));
    }
    docx.add_table(Table::new(type_rows))
}

fn findings(mut docx: Docx, report: &ReportContext) -> Docx {
    docx = docx.add_paragraph(heading1("Findings"));
    if report.findings.is_empty() {
        return docx.add_paragraph(body_text("No findings."));
    }
    let mut rows = vec![header_row(&["Severity", "Title", "Category", "Check"])];
    for finding in &report.findings {
        let fill = SEVERITY_FILLS
            .iter()
            .find(|(name, _)| *name == finding.severity)
            .map(|(_, fill)| *fill);
        let severity_cell = match fill {
            Some(fill) => TableCell::new()
                .shading(Shading::new().shd_type(ShdType::Clear).fill(fill))
                .add_paragraph(body_text(&finding.severity)),
            None => text_cell(&finding.severity),
        };
        rows.push(TableRow::new(vec![
            severity_cell,
            text_cell(&finding.title),
            text_cell(&finding.category),
            text_cell(&finding.query_name),
        ]));
    }
    docx.add_table(Table::new(rows))
}

fn categories(mut docx: Docx, report: &ReportContext) -> Docx {
    for category in &report.categories {
        docx = docx.add_paragraph(heading1(&capitalise(&category.name)));
        for query in &category.queries {
            docx = docx.add_paragraph(heading2(&query.name));
            if !query.description.is_empty() {
                docx = docx.add_paragraph(body_text(&query.description));
            }
            let columns = page_columns(&query.columns);
            let mut rows = vec![header_row(&columns)];
            for data_row in &query.rows {
                rows.push(TableRow::new(
                    columns
                        .iter()
                        .map(|c| text_cell(&cell_to_string(data_row.get(*c))))
                        .collect(),
                ));
            }
            docx = docx.add_table(Table::new(rows));
        }
    }
    docx
}

fn subscriptions(mut docx: Docx, report: &ReportContext) -> Docx {
    docx = docx.add_paragraph(heading1("Subscriptions"));
    for sub in &report.subscriptions {
        docx = docx.add_paragraph(heading2(&sub.display_name));
        docx = docx.add_paragraph(body_text(&format!(
            "{} · {} resources",
            sub.subscription_id, sub.resource_count
        )));
        for rg in &sub.resource_groups {
            if rg.resources.is_empty() {
                continue;
            }
            let label = match &rg.location {
                Some(location) => format!("{} ({location})", rg.name),
                None => rg.name.clone(),
            };
            docx = docx.add_paragraph(Paragraph::new().add_run(Run::new().add_text(label).bold()));
            let mut rows = vec![header_row(&["Name", "Type", "Location", "Tags"])];
            for resource in &rg.resources {
                rows.push(TableRow::new(vec![
                    text_cell(&resource.name),
                    text_cell(&resource.display_type),
                    text_cell(resource.location.as_deref().unwrap_or("")),
                    text_cell(resource.tags.as_deref().unwrap_or("")),
                ]));
            }
            docx = docx.add_table(Table::new(rows));
        }
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

fn text_cell(text: &str) -> TableCell {
    TableCell::new().add_paragraph(body_text(text))
}

/// Bold header row.
fn header_row(labels: &[&str]) -> TableRow {
    TableRow::new(
        labels
            .iter()
            .map(|label| {
                TableCell::new()
                    .add_paragraph(Paragraph::new().add_run(Run::new().add_text(*label).bold()))
            })
            .collect(),
    )
}
