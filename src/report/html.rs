use std::path::Path;

use anyhow::Context;
use minijinja::context;
use minijinja::value::ViaDeserialize;
use serde_json::Value;

use super::branding::BrandingContext;
use super::{ReportContext, cell_to_string};
use crate::diagram::assets::{DiagramAsset, DiagramAssetKind};

/// Render the single self-contained HTML report. Overview diagrams are
/// inlined as SVG so the file stays one file.
pub fn write(
    report: &ReportContext,
    branding: &BrandingContext,
    diagrams: &[DiagramAsset],
    out_path: &Path,
) -> anyhow::Result<()> {
    let html = render(report, branding, diagrams)?;
    std::fs::write(out_path, html).with_context(|| format!("writing {}", out_path.display()))?;
    Ok(())
}

#[derive(serde::Serialize)]
struct InlineDiagram<'a> {
    title: &'a str,
    svg: &'a str,
}

/// Render the report HTML to a string (write's testable core).
pub fn render(
    report: &ReportContext,
    branding: &BrandingContext,
    diagrams: &[DiagramAsset],
) -> anyhow::Result<String> {
    let mut env = super::markdown::environment(&branding.labels);
    env.add_filter("html_cell", html_cell);
    env.add_template(
        "report",
        include_str!("../../templates/html/report.html.j2"),
    )?;
    let inline: Vec<InlineDiagram<'_>> = diagrams
        .iter()
        .filter(|asset| {
            matches!(
                asset.kind,
                DiagramAssetKind::Hierarchy | DiagramAssetKind::Network
            )
        })
        .map(|asset| InlineDiagram {
            title: &asset.title,
            svg: &asset.svg,
        })
        .collect();
    let html = env.get_template("report")?.render(context! {
        diagrams => inline,
        branding => minijinja::Value::from_serialize(branding),
        labels => minijinja::Value::from_serialize(&branding.labels),
        tag_audit => super::governance::TAG_AUDIT,
        posture_tables => report.posture.tables(&branding.labels),
        provenance_records => super::provenance::records(&report.analysis.query_runs, &branding.labels),
        website_evidence => report.websites.html(&branding.labels, None, None),
        ..minijinja::Value::from_serialize(report)
    })?;
    Ok(html)
}

/// Pull one column out of a row for a table cell (minijinja auto-escapes it).
fn html_cell(row: ViaDeserialize<Value>, column: &str) -> String {
    cell_to_string(row.get(column))
}
