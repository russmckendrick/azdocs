use std::path::Path;

use anyhow::Context;

use super::{ReportContext, cell_to_string};
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

/// Every recorded query as `queries/<category>/<name>.csv`, the governance
/// analysis as three files and each operational evidence table under
/// `posture/`. Returns how many files were written so the CLI can say so.
/// The flat exports carry the full tables; only the printed formats cap rows.
pub fn write_tables(
    report: &ReportContext,
    labels: &Labels,
    out_root: &Path,
) -> anyhow::Result<usize> {
    let mut written = 0;
    for category in &report.categories {
        let dir = out_root.join("queries").join(&category.name);
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        for query in &category.queries {
            let path = dir.join(format!("{}.csv", query.name));
            let mut writer = csv::Writer::from_path(&path)
                .with_context(|| format!("writing {}", path.display()))?;
            writer.write_record(&query.columns)?;
            for row in &query.rows {
                writer.write_record(
                    query
                        .columns
                        .iter()
                        .map(|column| cell_to_string(row.get(column))),
                )?;
            }
            writer.flush()?;
            written += 1;
        }
    }

    let columns = &labels.common.columns;
    let governance = &report.governance;
    let mut writer = open(&out_root.join("governance-keys.csv"))?;
    writer.write_record([&columns.tag_key, &columns.resources, &columns.share])?;
    for key in &governance.top_keys {
        writer.write_record([&key.key, &key.count.to_string(), &key.percent.to_string()])?;
    }
    writer.flush()?;
    written += 1;

    let verdict = &labels.common.verdict;
    let mut writer = open(&out_root.join("governance-subscriptions.csv"))?;
    writer.write_record([
        &columns.subscription,
        &columns.tag_coverage,
        &columns.status,
    ])?;
    for subscription in &governance.subscriptions {
        writer.write_record([
            &subscription.display_name,
            &subscription.percent.to_string(),
            if subscription.healthy {
                &verdict.healthy
            } else {
                &verdict.below_threshold
            },
        ])?;
    }
    writer.flush()?;
    written += 1;

    let mut writer = open(&out_root.join("governance-groups.csv"))?;
    writer.write_record([
        &columns.resource_group,
        &columns.subscription,
        &columns.resources,
        &columns.non_compliant,
        &columns.missed_tags,
        &columns.status,
    ])?;
    for group in &governance.worst_groups {
        writer.write_record([
            &group.name,
            &group.subscription_name,
            &group.resources.to_string(),
            &group.non_compliant.to_string(),
            &group.missed_tags.join(", "),
            if group.flagged {
                &verdict.called_out
            } else {
                &verdict.none
            },
        ])?;
    }
    writer.flush()?;
    written += 1;

    let posture = out_root.join("posture");
    std::fs::create_dir_all(&posture).with_context(|| format!("creating {}", posture.display()))?;
    for evidence in report.posture.tables(labels) {
        let mut writer = open(&posture.join(format!("{}.csv", evidence.key)))?;
        writer.write_record(&evidence.columns)?;
        for row in &evidence.rows {
            writer.write_record(row)?;
        }
        writer.flush()?;
        written += 1;
    }
    Ok(written)
}

fn open(path: &Path) -> anyhow::Result<csv::Writer<std::fs::File>> {
    csv::Writer::from_path(path).with_context(|| format!("writing {}", path.display()))
}
