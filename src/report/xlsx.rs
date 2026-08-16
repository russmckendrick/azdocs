use std::path::Path;

use anyhow::Context;
use rust_xlsxwriter::{Color, Format, Workbook, Worksheet};

use super::{ReportContext, cell_to_string};
use crate::model::Resource;

/// One workbook: Summary, Inventory (autofilter), Findings (severity colors),
/// and one sheet per inventory category.
pub fn write(
    report: &ReportContext,
    resources: &[Resource],
    out_path: &Path,
) -> anyhow::Result<()> {
    let mut workbook = Workbook::new();
    let header = Format::new().set_bold();

    summary_sheet(
        workbook.add_worksheet().set_name("Summary")?,
        report,
        &header,
    )?;
    inventory_sheet(
        workbook.add_worksheet().set_name("Inventory")?,
        resources,
        &header,
    )?;
    findings_sheet(
        workbook.add_worksheet().set_name("Findings")?,
        report,
        &header,
    )?;
    for category in &report.categories {
        // Suffix avoids case-insensitive collisions with the fixed sheets
        // (e.g. a category literally named "inventory"); names cap at 31 chars.
        let name: String = format!("{} queries", category.name)
            .chars()
            .take(31)
            .collect();
        category_sheet(workbook.add_worksheet().set_name(name)?, category, &header)?;
    }

    workbook
        .save(out_path)
        .with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}

fn summary_sheet(
    sheet: &mut Worksheet,
    report: &ReportContext,
    header: &Format,
) -> anyhow::Result<()> {
    let rows: Vec<(&str, String)> = vec![
        ("Snapshot", report.snapshot_id.clone()),
        ("Collected", report.created_at.clone()),
        ("Tenant", report.tenant_id.clone()),
        ("Status", report.status.clone()),
        ("Subscriptions", report.totals.subscriptions.to_string()),
        ("Resource groups", report.totals.resource_groups.to_string()),
        ("Resources", report.totals.resources.to_string()),
        ("Findings", report.totals.findings.to_string()),
        ("High findings", report.severity_counts.high.to_string()),
        ("Medium findings", report.severity_counts.medium.to_string()),
        ("Low findings", report.severity_counts.low.to_string()),
        ("Info findings", report.severity_counts.info.to_string()),
        ("Tag coverage %", report.tag_coverage.percent.to_string()),
    ];
    for (row, (label, value)) in rows.iter().enumerate() {
        sheet.write_with_format(row as u32, 0, *label, header)?;
        sheet.write(row as u32, 1, value)?;
    }

    sheet.write_with_format(15, 0, "Type", header)?;
    sheet.write_with_format(15, 1, "Count", header)?;
    for (offset, tc) in report.type_counts.iter().enumerate() {
        let row = 16 + offset as u32;
        sheet.write(row, 0, &tc.display)?;
        sheet.write(row, 1, tc.count as u32)?;
    }
    sheet.set_column_width(0, 24)?;
    sheet.set_column_width(1, 40)?;
    Ok(())
}

fn inventory_sheet(
    sheet: &mut Worksheet,
    resources: &[Resource],
    header: &Format,
) -> anyhow::Result<()> {
    let columns = [
        "name",
        "type",
        "kind",
        "location",
        "resource group",
        "subscription",
        "tags",
        "id",
    ];
    for (col, name) in columns.iter().enumerate() {
        sheet.write_with_format(0, col as u16, *name, header)?;
    }
    for (offset, r) in resources.iter().enumerate() {
        let row = 1 + offset as u32;
        sheet.write(row, 0, &r.name)?;
        sheet.write(row, 1, &r.azure_type)?;
        sheet.write(row, 2, r.kind.as_deref().unwrap_or(""))?;
        sheet.write(row, 3, r.location.as_deref().unwrap_or(""))?;
        sheet.write(row, 4, r.resource_group.as_deref().unwrap_or(""))?;
        sheet.write(row, 5, &r.subscription_id)?;
        sheet.write(
            row,
            6,
            r.tags.as_ref().map(ToString::to_string).unwrap_or_default(),
        )?;
        sheet.write(row, 7, &r.display_id)?;
    }
    sheet.autofilter(0, 0, resources.len() as u32, (columns.len() - 1) as u16)?;
    sheet.set_freeze_panes(1, 0)?;
    sheet.set_column_width(0, 32)?;
    sheet.set_column_width(1, 36)?;
    sheet.set_column_width(7, 60)?;
    Ok(())
}

fn findings_sheet(
    sheet: &mut Worksheet,
    report: &ReportContext,
    header: &Format,
) -> anyhow::Result<()> {
    let severity_formats = [
        (
            "high",
            Format::new().set_background_color(Color::RGB(0xF8CECC)),
        ),
        (
            "medium",
            Format::new().set_background_color(Color::RGB(0xFFE6CC)),
        ),
        (
            "low",
            Format::new().set_background_color(Color::RGB(0xFFF2CC)),
        ),
        (
            "info",
            Format::new().set_background_color(Color::RGB(0xDAE8FC)),
        ),
    ];
    for (col, name) in ["severity", "category", "check", "title", "resource"]
        .iter()
        .enumerate()
    {
        sheet.write_with_format(0, col as u16, *name, header)?;
    }
    for (offset, f) in report.findings.iter().enumerate() {
        let row = 1 + offset as u32;
        let format = severity_formats
            .iter()
            .find(|(name, _)| *name == f.severity)
            .map(|(_, format)| format);
        match format {
            Some(format) => sheet.write_with_format(row, 0, &f.severity, format)?,
            None => sheet.write(row, 0, &f.severity)?,
        };
        sheet.write(row, 1, &f.category)?;
        sheet.write(row, 2, &f.query_name)?;
        sheet.write(row, 3, &f.title)?;
        sheet.write(row, 4, f.resource_id.as_deref().unwrap_or(""))?;
    }
    sheet.autofilter(0, 0, report.findings.len() as u32, 4)?;
    sheet.set_freeze_panes(1, 0)?;
    sheet.set_column_width(3, 60)?;
    sheet.set_column_width(4, 60)?;
    Ok(())
}

fn category_sheet(
    sheet: &mut Worksheet,
    category: &super::Category,
    header: &Format,
) -> anyhow::Result<()> {
    let mut row: u32 = 0;
    for query in &category.queries {
        sheet.write_with_format(row, 0, &query.name, header)?;
        row += 1;
        for (col, name) in query.columns.iter().enumerate() {
            sheet.write_with_format(row, col as u16, name.as_str(), header)?;
        }
        row += 1;
        for data_row in &query.rows {
            for (col, name) in query.columns.iter().enumerate() {
                sheet.write(row, col as u16, cell_to_string(data_row.get(name)))?;
            }
            row += 1;
        }
        row += 1;
    }
    Ok(())
}
