use std::path::Path;

use anyhow::Context;
use rust_xlsxwriter::{Color, DocProperties, Format, FormatBorder, Workbook, Worksheet};

use super::branding::BrandingContext;
use super::theme::{TableStyle, ThemeTokens};
use super::{ReportContext, cell_to_string};
use crate::labels::{Labels, fill};
use crate::model::Resource;

/// Excel refuses any cell text past this many characters, and
/// `rust_xlsxwriter` fails the whole save rather than the one cell. Query
/// rows carry JSON bags and policy definitions that can run far past it, so
/// every data-driven string goes through [`cell_text`] before it is written.
const EXCEL_CELL_LIMIT: usize = 32_767;

/// A value bounded to what Excel will store; the snapshot keeps the rest.
fn cell_text(value: &str) -> std::borrow::Cow<'_, str> {
    if value.chars().count() <= EXCEL_CELL_LIMIT {
        std::borrow::Cow::Borrowed(value)
    } else {
        tracing::warn!(
            chars = value.chars().count(),
            limit = EXCEL_CELL_LIMIT,
            "cell text exceeds Excel's limit; truncated in the workbook"
        );
        std::borrow::Cow::Owned(crate::model::truncate(value, EXCEL_CELL_LIMIT))
    }
}

/// Document properties from the snapshot, never from the clock. The
/// creation stamp is the snapshot's collection time, so writing the same
/// snapshot twice gives the same bytes; a workbook that changed its own
/// timestamp on every export defeated any checksum a reader kept.
fn document_properties(report: &ReportContext, branding: &BrandingContext) -> DocProperties {
    let created = chrono::DateTime::parse_from_rfc3339(&report.created_at)
        .map(|stamp| stamp.with_timezone(&chrono::Utc))
        .unwrap_or_default();
    let mut properties = DocProperties::new()
        .set_title(&branding.title)
        .set_subject(&report.snapshot_id)
        .set_author("azdocs")
        .set_creation_datetime(&created);
    if !branding.company.is_empty() {
        properties = properties.set_company(&branding.company);
    }
    properties
}

/// `#rrggbb` from the theme as the packed integer `rust_xlsxwriter` wants.
/// The palette is validated on the way in, so a malformed value here would be
/// a bug rather than user input; fall back to black instead of panicking.
fn color(hex: &str) -> Color {
    Color::RGB(u32::from_str_radix(hex.trim_start_matches('#'), 16).unwrap_or(0))
}

/// Header format from the theme, matching the PDF and DOCX table headers.
fn header_format(tokens: &ThemeTokens) -> Format {
    let format = Format::new()
        .set_bold()
        .set_font_name(&tokens.typography.docx_sans);
    match tokens.layout.table {
        TableStyle::SolidHeader => format
            .set_background_color(color(&tokens.palette.primary))
            .set_font_color(color(&tokens.palette.on_primary)),
        TableStyle::Banded => format
            .set_background_color(color(&tokens.palette.primary_tint))
            .set_font_color(color(&tokens.palette.ink)),
        TableStyle::Hairline => format
            .set_font_color(color(&tokens.palette.ink))
            .set_border_bottom(FormatBorder::Thin)
            .set_border_bottom_color(color(&tokens.palette.rule)),
    }
}

/// One workbook: Summary, Inventory (autofilter), Findings (severity colors),
/// Governance, and one sheet per inventory category.
pub fn write(
    report: &ReportContext,
    branding: &BrandingContext,
    resources: &[Resource],
    out_path: &Path,
) -> anyhow::Result<()> {
    let mut workbook = Workbook::new();
    workbook.set_properties(&document_properties(report, branding));
    let tokens = &branding.tokens;
    let labels = &branding.labels;
    let words = &labels.report.xlsx;
    let header = header_format(tokens);

    summary_sheet(
        workbook.add_worksheet().set_name(&words.sheet_summary)?,
        report,
        branding,
        &header,
    )?;
    inventory_sheet(
        workbook.add_worksheet().set_name(&words.sheet_inventory)?,
        resources,
        labels,
        &header,
    )?;
    findings_sheet(
        workbook.add_worksheet().set_name(&words.sheet_findings)?,
        report,
        labels,
        tokens,
        &header,
    )?;
    governance_sheet(
        workbook.add_worksheet().set_name(&words.sheet_governance)?,
        report,
        labels,
        tokens,
        &header,
    )?;
    let sheet = workbook
        .add_worksheet()
        .set_name(&labels.report.posture.sheet)?;
    let mut row = 0;
    for evidence in report.posture.tables(labels) {
        sheet.merge_range(row, 0, row, 5, &evidence.title, &header)?;
        sheet.write(row + 1, 0, &evidence.status)?;
        sheet.merge_range(
            row + 2,
            0,
            row + 2,
            5,
            &evidence.note,
            &Format::new().set_text_wrap(),
        )?;
        sheet.set_row_height(row + 2, 48)?;
        row += 4;
        for (column, name) in evidence.columns.iter().enumerate() {
            sheet.write_string_with_format(row, column as u16, name, &header)?;
        }
        row += 1;
        for values in evidence.rows {
            for (column, value) in values.iter().enumerate() {
                sheet.write(row, column as u16, cell_text(value).as_ref())?;
            }
            row += 1;
        }
        row += 2;
    }
    sheet.set_column_width(0, 36)?;
    sheet.set_column_range_width(1, 5, 24)?;

    locations_sheet(
        workbook.add_worksheet().set_name(&words.sheet_locations)?,
        report,
        labels,
        &header,
    )?;
    let websites = report.websites.rows(labels);
    if !websites.is_empty() {
        websites_sheet(
            workbook.add_worksheet().set_name(&words.sheet_websites)?,
            &websites,
            labels,
            &header,
        )?;
    }
    if let Some(changes) = &report.changes {
        changes_sheet(
            workbook.add_worksheet().set_name(&words.sheet_changes)?,
            report,
            changes,
            labels,
            &header,
        )?;
        trend_sheet(
            workbook.add_worksheet().set_name(&words.sheet_trend)?,
            report,
            labels,
            &header,
        )?;
    }

    let records = super::provenance::records(&report.analysis.query_runs, labels);
    if !records.is_empty() {
        let values = &labels.report.posture.values;
        let sheet = workbook.add_worksheet().set_name(&values["provenance"])?;
        for (column, key) in ["query", "field", "value"].iter().enumerate() {
            sheet.write_with_format(0, column as u16, &values[*key], &header)?;
        }
        sheet.write_with_format(0, 3, &words.kql_column, &header)?;
        let mut row = 1;
        for record in records {
            // The query text sits beside the record's first field, so one
            // row per query carries it and the rest stay blank.
            let mut kql = Some(record.kql.as_str());
            for field in record.fields {
                sheet.write(row, 0, &record.name)?;
                sheet.write(row, 1, &field[0])?;
                sheet.write(row, 2, cell_text(&field[1]).as_ref())?;
                if let Some(text) = kql.take() {
                    sheet.write(row, 3, cell_text(text).as_ref())?;
                }
                row += 1;
            }
        }
        sheet.set_column_range_width(0, 1, 32)?;
        sheet.set_column_width(2, 100)?;
        sheet.set_column_width(3, 100)?;
    }

    for category in &report.categories {
        // Suffix avoids case-insensitive collisions with the fixed sheets
        // (e.g. a category literally named "inventory"); names cap at 31 chars.
        let name: String = fill(&words.queries_sheet, &[("category", &category.name)])
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
    branding: &BrandingContext,
    header: &Format,
) -> anyhow::Result<()> {
    // Branded title block above the figures, so a workbook mailed on its own
    // still says what it is and who it is for.
    let tokens = &branding.tokens;
    let title = Format::new()
        .set_bold()
        .set_font_size(16)
        .set_font_name(&tokens.typography.docx_serif)
        .set_font_color(color(&tokens.palette.primary));
    let subtitle = Format::new()
        .set_font_name(&tokens.typography.docx_sans)
        .set_font_color(color(&tokens.palette.muted));
    sheet.write_with_format(0, 0, &branding.title, &title)?;
    let mut caption = branding.company.clone();
    if !branding.subtitle.is_empty() {
        if !caption.is_empty() {
            caption.push_str(" · ");
        }
        caption.push_str(&branding.subtitle);
    }
    if !caption.is_empty() {
        sheet.write_with_format(1, 0, &caption, &subtitle)?;
    }
    const FIRST_ROW: u32 = 3;

    let labels = &branding.labels;
    let cover = &labels.common.cover;
    let columns = &labels.common.columns;
    let words = &labels.report.xlsx;
    let rows: Vec<(&str, String)> = vec![
        (&cover.snapshot, report.snapshot_id.clone()),
        (&cover.collected, report.created_at.clone()),
        (&cover.tenant, report.tenant_id.clone()),
        (&cover.status, report.status.clone()),
        (
            &columns.subscriptions,
            report.totals.subscriptions.to_string(),
        ),
        (
            &columns.resource_groups,
            report.totals.resource_groups.to_string(),
        ),
        (&columns.resources, report.totals.resources.to_string()),
        (&columns.findings, report.totals.findings.to_string()),
        (
            &words.high_findings,
            report.severity_counts.high.to_string(),
        ),
        (
            &words.medium_findings,
            report.severity_counts.medium.to_string(),
        ),
        (&words.low_findings, report.severity_counts.low.to_string()),
        (
            &words.info_findings,
            report.severity_counts.info.to_string(),
        ),
        (
            &words.tag_coverage_percent,
            report.tag_coverage.percent.to_string(),
        ),
    ];
    for (offset, (label, value)) in rows.iter().enumerate() {
        let row = FIRST_ROW + offset as u32;
        sheet.write_with_format(row, 0, *label, header)?;
        sheet.write(row, 1, value)?;
    }

    let types_row = FIRST_ROW + rows.len() as u32 + 2;
    sheet.write_with_format(types_row, 0, columns.r#type.as_str(), header)?;
    sheet.write_with_format(types_row, 1, columns.count.as_str(), header)?;
    for (offset, tc) in report.type_counts.iter().enumerate() {
        let row = types_row + 1 + offset as u32;
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
    labels: &Labels,
    header: &Format,
) -> anyhow::Result<()> {
    let names = &labels.report.xlsx.inventory_columns;
    let columns = [
        names.name.as_str(),
        names.r#type.as_str(),
        names.kind.as_str(),
        names.location.as_str(),
        names.resource_group.as_str(),
        names.subscription.as_str(),
        names.tags.as_str(),
        names.id.as_str(),
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
            cell_text(&r.tags.as_ref().map(ToString::to_string).unwrap_or_default()).as_ref(),
        )?;
        sheet.write(row, 7, &r.display_id)?;
    }
    sheet.autofilter(0, 0, resources.len() as u32, (columns.len() - 1) as u16)?;
    sheet.set_freeze_panes(1, 0)?;
    for (column, width) in [24.0, 42.0, 18.0, 14.0, 24.0, 20.0, 28.0, 64.0]
        .into_iter()
        .enumerate()
    {
        sheet.set_column_width(column as u16, width)?;
    }
    Ok(())
}

fn findings_sheet(
    sheet: &mut Worksheet,
    report: &ReportContext,
    labels: &Labels,
    tokens: &ThemeTokens,
    header: &Format,
) -> anyhow::Result<()> {
    // Same severity palette as every other format, straight from the theme.
    let severity = &tokens.palette.severity;
    let severity_formats = [
        ("high", &severity.high),
        ("medium", &severity.medium),
        ("low", &severity.low),
        ("info", &severity.info),
    ]
    .map(|(name, colors)| {
        (
            name,
            Format::new()
                .set_background_color(color(&colors.fill))
                .set_font_color(color(&colors.text)),
        )
    });
    let names = &labels.report.xlsx.findings_columns;
    for (col, name) in [
        names.severity.as_str(),
        names.category.as_str(),
        names.check.as_str(),
        names.title.as_str(),
        names.resource.as_str(),
    ]
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
    for (column, width) in [12.0, 16.0, 34.0, 60.0, 64.0].into_iter().enumerate() {
        sheet.set_column_width(column as u16, width)?;
    }
    Ok(())
}

/// The same three tables the report and the explorer draw, from the same
/// analysis: coverage by key, coverage by subscription, and the least
/// compliant resource groups. Verdicts arrive already decided
/// (`healthy`/`flagged`); this sheet only colours them.
fn governance_sheet(
    sheet: &mut Worksheet,
    report: &ReportContext,
    labels: &Labels,
    tokens: &ThemeTokens,
    header: &Format,
) -> anyhow::Result<()> {
    let governance = &report.governance;
    let columns = &labels.common.columns;
    let words = &labels.report.xlsx;
    let verdict = &labels.common.verdict;
    // A flagged group reads as a finding, so it uses the finding palette.
    let flagged = Format::new()
        .set_background_color(color(&tokens.palette.severity.high.fill))
        .set_font_color(color(&tokens.palette.severity.high.text));

    let mut row = table(
        sheet,
        0,
        &[
            &words.tag_coverage_percent,
            &columns.distinct_keys,
            &columns.non_compliant,
        ],
        header,
    )?;
    sheet.write(row, 0, report.tag_coverage.percent)?;
    sheet.write(row, 1, governance.distinct_keys as u32)?;
    sheet.write(row, 2, governance.non_compliant as u32)?;
    row += 2;

    row = table(
        sheet,
        row,
        &[
            &columns.tag_key,
            &columns.resources,
            &words.share_of_tagged_percent,
        ],
        header,
    )?;
    for key in &governance.top_keys {
        sheet.write(row, 0, &key.key)?;
        sheet.write(row, 1, key.count as u32)?;
        sheet.write(row, 2, key.percent)?;
        row += 1;
    }
    if governance.top_keys_total > governance.top_keys.len() {
        sheet.write(
            row,
            0,
            fill(
                &labels.common.governance.top_keys_note,
                &[
                    ("shown", &governance.top_keys.len()),
                    ("total", &governance.top_keys_total),
                ],
            ),
        )?;
        row += 1;
    }
    row += 1;

    row = table(
        sheet,
        row,
        &[
            &columns.subscription,
            &words.tag_coverage_percent,
            &columns.status,
        ],
        header,
    )?;
    for subscription in &governance.subscriptions {
        sheet.write(row, 0, &subscription.display_name)?;
        sheet.write(row, 1, subscription.percent)?;
        sheet.write(
            row,
            2,
            if subscription.healthy {
                &verdict.healthy
            } else {
                &verdict.below_threshold
            },
        )?;
        row += 1;
    }
    row += 1;

    row = table(
        sheet,
        row,
        &[
            &columns.resource_group,
            &columns.subscription,
            &columns.resources,
            &columns.non_compliant,
            &columns.missed_tags,
        ],
        header,
    )?;
    for group in &governance.worst_groups {
        sheet.write(row, 0, &group.name)?;
        sheet.write(row, 1, &group.subscription_name)?;
        sheet.write(row, 2, group.resources as u32)?;
        let count = group.non_compliant as u32;
        if group.flagged {
            sheet.write_with_format(row, 3, count, &flagged)?;
        } else {
            sheet.write(row, 3, count)?;
        }
        sheet.write(row, 4, group.missed_tags.join(", "))?;
        row += 1;
    }
    if governance.worst_groups_total > governance.worst_groups.len() {
        sheet.write(
            row,
            0,
            fill(
                &labels.common.governance.worst_groups_note,
                &[
                    ("shown", &governance.worst_groups.len()),
                    ("total", &governance.worst_groups_total),
                ],
            ),
        )?;
    }

    sheet.set_column_width(0, 28)?;
    sheet.set_column_width(1, 24)?;
    sheet.set_column_width(2, 18)?;
    sheet.set_column_width(3, 16)?;
    sheet.set_column_width(4, 32)?;
    Ok(())
}

fn locations_sheet(
    sheet: &mut Worksheet,
    report: &ReportContext,
    labels: &Labels,
    header: &Format,
) -> anyhow::Result<()> {
    let columns = &labels.common.columns;
    let start = table(
        sheet,
        0,
        &[&columns.location, &columns.kind, &columns.resources],
        header,
    )?;
    for (offset, location) in report.location_counts.iter().enumerate() {
        let row = start + offset as u32;
        sheet.write(row, 0, &location.display)?;
        sheet.write(row, 1, &location.name)?;
        sheet.write(row, 2, location.count as u32)?;
    }
    sheet.set_column_range_width(0, 1, 24)?;
    Ok(())
}

fn websites_sheet(
    sheet: &mut Worksheet,
    rows: &[super::websites::WebsiteRow],
    labels: &Labels,
    header: &Format,
) -> anyhow::Result<()> {
    let columns = &labels.common.columns;
    let words = &labels.common.websites;
    let start = table(
        sheet,
        0,
        &[
            &columns.resource,
            &words.website,
            &columns.status,
            &words.capture_time,
            &words.final_url,
            &words.failure_detail,
        ],
        header,
    )?;
    for (offset, website) in rows.iter().enumerate() {
        let row = start + offset as u32;
        sheet.write(row, 0, &website.resource_name)?;
        sheet.write(row, 1, cell_text(&website.target).as_ref())?;
        sheet.write(row, 2, &website.status)?;
        sheet.write(row, 3, website.captured_at.as_deref().unwrap_or(""))?;
        sheet.write(
            row,
            4,
            cell_text(website.final_url.as_deref().unwrap_or("")).as_ref(),
        )?;
        sheet.write(
            row,
            5,
            cell_text(website.error.as_deref().unwrap_or("")).as_ref(),
        )?;
    }
    sheet.set_column_width(0, 28)?;
    sheet.set_column_range_width(1, 5, 36)?;
    Ok(())
}

/// The full diff, uncapped: every changed field, finding, relationship and
/// scope change in blocks, the way the governance sheet is laid out.
fn changes_sheet(
    sheet: &mut Worksheet,
    report: &ReportContext,
    changes: &crate::model::diff::SnapshotChanges,
    labels: &Labels,
    header: &Format,
) -> anyhow::Result<()> {
    let columns = &labels.common.columns;
    let words = &labels.report.changes;
    let xlsx = &labels.report.xlsx;
    let assessment = &labels.report.assessment;
    let subscription = |id: &str| {
        report
            .analysis
            .subscriptions
            .get(id)
            .cloned()
            .unwrap_or_else(|| id.to_owned())
    };

    let mut row = table(
        sheet,
        0,
        &[
            &xlsx.side_column,
            &labels.common.cover.snapshot,
            &labels.common.cover.collected,
            &columns.status,
        ],
        header,
    )?;
    for (side, snapshot) in [
        (&words.removed_marker, &changes.base),
        (&words.added_marker, &changes.target),
    ] {
        sheet.write(row, 0, side)?;
        sheet.write(row, 1, &snapshot.id)?;
        sheet.write(row, 2, &snapshot.created_at)?;
        sheet.write(row, 3, &snapshot.status)?;
        row += 1;
    }
    row += 1;

    row = table(
        sheet,
        row,
        &[
            &xlsx.change_column,
            &columns.name,
            &columns.azure_type,
            &columns.resource_group,
            &columns.subscription,
            &columns.field,
            &columns.before,
            &columns.after,
        ],
        header,
    )?;
    let mut resource = |row: &mut u32,
                        marker: &str,
                        r: &crate::model::diff::ResourceRef,
                        field: [&str; 3]|
     -> anyhow::Result<()> {
        sheet.write(*row, 0, marker)?;
        sheet.write(*row, 1, &r.name)?;
        sheet.write(*row, 2, &r.azure_type)?;
        sheet.write(*row, 3, r.resource_group.as_deref().unwrap_or(""))?;
        sheet.write(*row, 4, subscription(&r.subscription_id))?;
        for (offset, value) in field.iter().enumerate() {
            sheet.write(*row, 5 + offset as u16, cell_text(value).as_ref())?;
        }
        *row += 1;
        Ok(())
    };
    for r in &changes.resources.added {
        resource(&mut row, &words.added_marker, r, ["", "", ""])?;
    }
    for r in &changes.resources.removed {
        resource(&mut row, &words.removed_marker, r, ["", "", ""])?;
    }
    for change in &changes.resources.changed {
        for f in &change.fields {
            let field = if f.path.is_empty() {
                f.field.as_str().to_owned()
            } else {
                format!("{}.{}", f.field.as_str(), f.path)
            };
            resource(
                &mut row,
                &words.changed,
                &change.resource,
                [
                    &field,
                    &cell_to_string(f.before.as_ref()),
                    &cell_to_string(f.after.as_ref()),
                ],
            )?;
        }
    }
    row += 1;

    row = table(
        sheet,
        row,
        &[
            &xlsx.change_column,
            &columns.severity,
            &columns.check,
            &columns.title,
            &columns.resource,
        ],
        header,
    )?;
    for (marker, findings) in [
        (&words.new_findings, &changes.findings.added),
        (&words.resolved_findings, &changes.findings.resolved),
    ] {
        for f in findings {
            sheet.write(row, 0, marker)?;
            sheet.write(row, 1, f.severity.as_str())?;
            sheet.write(row, 2, &f.query_name)?;
            sheet.write(row, 3, cell_text(&f.title).as_ref())?;
            sheet.write(row, 4, f.resource_id.as_deref().unwrap_or(""))?;
            row += 1;
        }
    }
    row += 1;

    row = table(
        sheet,
        row,
        &[
            &xlsx.change_column,
            &assessment.source,
            &assessment.relationship,
            &assessment.target,
        ],
        header,
    )?;
    for (marker, edges) in [
        (&words.added_marker, &changes.edges.added),
        (&words.removed_marker, &changes.edges.removed),
    ] {
        for e in edges {
            sheet.write(row, 0, marker)?;
            sheet.write(row, 1, &e.source_id)?;
            sheet.write(row, 2, &e.kind)?;
            sheet.write(row, 3, &e.target_id)?;
            row += 1;
        }
    }
    row += 1;

    row = table(
        sheet,
        row,
        &[&xlsx.change_column, &columns.kind, &columns.name],
        header,
    )?;
    for (marker, kind, ids) in [
        (
            &words.added_marker,
            &columns.subscription,
            &changes.subscriptions.added,
        ),
        (
            &words.removed_marker,
            &columns.subscription,
            &changes.subscriptions.removed,
        ),
        (
            &words.added_marker,
            &columns.resource_group,
            &changes.resource_groups.added,
        ),
        (
            &words.removed_marker,
            &columns.resource_group,
            &changes.resource_groups.removed,
        ),
    ] {
        for id in ids {
            sheet.write(row, 0, marker)?;
            sheet.write(row, 1, kind)?;
            sheet.write(row, 2, id)?;
            row += 1;
        }
    }

    sheet.set_column_width(0, 18)?;
    sheet.set_column_range_width(1, 4, 28)?;
    sheet.set_column_range_width(5, 7, 40)?;
    Ok(())
}

fn trend_sheet(
    sheet: &mut Worksheet,
    report: &ReportContext,
    labels: &Labels,
    header: &Format,
) -> anyhow::Result<()> {
    let columns = &labels.common.columns;
    let sev = &labels.common.severity;
    let start = table(
        sheet,
        0,
        &[
            &labels.common.cover.snapshot,
            &labels.common.cover.collected,
            &columns.status,
            &columns.subscriptions,
            &columns.resources,
            &columns.tagged,
            &columns.findings,
            &sev.high.label,
            &sev.medium.label,
            &sev.low.label,
            &sev.info.label,
            &labels.report.assessment.relationships,
        ],
        header,
    )?;
    for (offset, point) in report.trend.iter().enumerate() {
        let row = start + offset as u32;
        sheet.write(row, 0, &point.snapshot_id)?;
        sheet.write(row, 1, &point.created_at)?;
        sheet.write(row, 2, &point.status)?;
        for (offset, value) in [
            point.subscriptions,
            point.resources,
            point.tagged,
            point.findings,
            point.high,
            point.medium,
            point.low,
            point.info,
            point.edges,
        ]
        .into_iter()
        .enumerate()
        {
            sheet.write(row, 3 + offset as u16, value as u32)?;
        }
    }
    sheet.set_column_range_width(0, 1, 36)?;
    Ok(())
}

/// Write one header row and return the row the body starts on.
fn table(
    sheet: &mut Worksheet,
    row: u32,
    columns: &[&str],
    header: &Format,
) -> anyhow::Result<u32> {
    for (col, label) in columns.iter().enumerate() {
        sheet.write_with_format(row, col as u16, *label, header)?;
    }
    Ok(row + 1)
}

fn category_sheet(
    sheet: &mut Worksheet,
    category: &super::Category,
    header: &Format,
) -> anyhow::Result<()> {
    let mut row: u32 = 0;
    let column_count = category
        .queries
        .iter()
        .map(|query| query.columns.len())
        .max()
        .unwrap_or_default();
    let mut widths = vec![0usize; column_count];
    for query in &category.queries {
        sheet.write_with_format(row, 0, &query.name, header)?;
        if let Some(width) = widths.first_mut() {
            *width = (*width).max(display_width(&query.name));
        }
        row += 1;
        for (col, name) in query.columns.iter().enumerate() {
            sheet.write_with_format(row, col as u16, name.as_str(), header)?;
            widths[col] = widths[col].max(display_width(name));
        }
        row += 1;
        for data_row in &query.rows {
            for (col, name) in query.columns.iter().enumerate() {
                let value = cell_to_string(data_row.get(name));
                widths[col] = widths[col].max(display_width(&value));
                sheet.write(row, col as u16, cell_text(&value).as_ref())?;
            }
            row += 1;
        }
        row += 1;
    }
    for (column, width) in widths.into_iter().enumerate() {
        // IDs and JSON can be hundreds of characters. Give them a useful
        // inspection width without letting one value make the sheet unwieldy.
        let bounded = (width + 2).clamp(12, 48) as f64;
        sheet.set_column_width(column as u16, bounded)?;
    }
    Ok(())
}

fn display_width(value: &str) -> usize {
    value
        .lines()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_cell_text_keeps_short_values_and_bounds_long_ones() {
        assert_eq!(cell_text("short"), "short");
        let long = "x".repeat(EXCEL_CELL_LIMIT + 5);
        let bounded = cell_text(&long);
        assert_eq!(bounded.chars().count(), EXCEL_CELL_LIMIT);
        assert!(bounded.ends_with('…'));
    }
}
