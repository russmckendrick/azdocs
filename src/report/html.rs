use std::path::Path;

use anyhow::Context;
use minijinja::value::ViaDeserialize;
use serde_json::Value;

use super::{ReportContext, cell_to_string};

/// Render the single self-contained HTML report.
pub fn write(report: &ReportContext, out_path: &Path) -> anyhow::Result<()> {
    let mut env = super::markdown::environment();
    env.add_filter("html_cell", html_cell);
    env.add_template(
        "report",
        include_str!("../../templates/html/report.html.j2"),
    )?;
    let html = env
        .get_template("report")?
        .render(minijinja::Value::from_serialize(report))?;
    std::fs::write(out_path, html).with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}

/// Pull one column out of a row for a table cell (minijinja auto-escapes it).
fn html_cell(row: ViaDeserialize<Value>, column: &str) -> String {
    cell_to_string(row.get(column))
}
