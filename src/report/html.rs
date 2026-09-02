use std::path::Path;

use anyhow::Context;
use minijinja::context;
use minijinja::value::ViaDeserialize;
use serde_json::Value;

use super::branding::BrandingContext;
use super::{ReportContext, cell_to_string};

/// Render the single self-contained HTML report.
pub fn write(
    report: &ReportContext,
    branding: &BrandingContext,
    out_path: &Path,
) -> anyhow::Result<()> {
    let html = render(report, branding)?;
    std::fs::write(out_path, html).with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}

/// Render the report HTML to a string (write's testable core).
pub fn render(report: &ReportContext, branding: &BrandingContext) -> anyhow::Result<String> {
    let mut env = super::markdown::environment(&branding.labels);
    env.add_filter("html_cell", html_cell);
    env.add_template(
        "report",
        include_str!("../../templates/html/report.html.j2"),
    )?;
    let html = env.get_template("report")?.render(context! {
        branding => minijinja::Value::from_serialize(branding),
        labels => minijinja::Value::from_serialize(&branding.labels),
        tag_audit => super::governance::TAG_AUDIT,
        ..minijinja::Value::from_serialize(report)
    })?;
    Ok(html)
}

/// Pull one column out of a row for a table cell (minijinja auto-escapes it).
fn html_cell(row: ViaDeserialize<Value>, column: &str) -> String {
    cell_to_string(row.get(column))
}
