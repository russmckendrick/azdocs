use std::path::Path;

use anyhow::Context;

use crate::model::{Finding, Resource};

/// `inventory.csv`: one row per resource, tags as compact JSON.
pub fn write_inventory(resources: &[Resource], out_path: &Path) -> anyhow::Result<()> {
    let mut writer = csv::Writer::from_path(out_path)
        .with_context(|| format!("writing {}", out_path.display()))?;
    writer.write_record([
        "id",
        "name",
        "type",
        "kind",
        "location",
        "resource_group",
        "subscription_id",
        "tags",
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
pub fn write_findings(findings: &[Finding], out_path: &Path) -> anyhow::Result<()> {
    let mut writer = csv::Writer::from_path(out_path)
        .with_context(|| format!("writing {}", out_path.display()))?;
    writer.write_record(["severity", "category", "check", "title", "resource_id"])?;
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
