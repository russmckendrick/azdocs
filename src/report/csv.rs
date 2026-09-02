use std::path::Path;

use anyhow::Context;

use crate::labels::Labels;
use crate::model::{Finding, Resource};

/// `inventory.csv`: one row per resource, tags as compact JSON.
pub fn write_inventory(
    resources: &[Resource],
    labels: &Labels,
    out_path: &Path,
) -> anyhow::Result<()> {
    let mut writer = csv::Writer::from_path(out_path)
        .with_context(|| format!("writing {}", out_path.display()))?;
    let columns = &labels.report.csv.inventory_columns;
    writer.write_record([
        columns.id.as_str(),
        columns.name.as_str(),
        columns.r#type.as_str(),
        columns.kind.as_str(),
        columns.location.as_str(),
        columns.resource_group.as_str(),
        columns.subscription_id.as_str(),
        columns.tags.as_str(),
    ])?;
    for r in resources {
        writer.write_record([
            r.display_id.as_str(),
            r.name.as_str(),
            r.azure_type.as_str(),
            r.kind.as_deref().unwrap_or(""),
            r.location.as_deref().unwrap_or(""),
            r.resource_group.as_deref().unwrap_or(""),
            r.subscription_id.as_str(),
            &r.tags.as_ref().map(ToString::to_string).unwrap_or_default(),
        ])?;
    }
    writer.flush()?;
    Ok(())
}

/// `findings.csv`, ordered high severity first (store order).
pub fn write_findings(
    findings: &[Finding],
    labels: &Labels,
    out_path: &Path,
) -> anyhow::Result<()> {
    let mut writer = csv::Writer::from_path(out_path)
        .with_context(|| format!("writing {}", out_path.display()))?;
    let columns = &labels.report.csv.findings_columns;
    writer.write_record([
        columns.severity.as_str(),
        columns.category.as_str(),
        columns.check.as_str(),
        columns.title.as_str(),
        columns.resource_id.as_str(),
    ])?;
    for f in findings {
        writer.write_record([
            f.severity.as_str(),
            f.category.as_str(),
            f.query_name.as_str(),
            f.title.as_str(),
            f.resource_id.as_deref().unwrap_or(""),
        ])?;
    }
    writer.flush()?;
    Ok(())
}
