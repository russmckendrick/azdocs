use anyhow::bail;
use chrono::{DateTime, Utc};
use comfy_table::{Table, presets};

use crate::cli::{DiffFormat, OutputFormat, SnapshotsCommand};
use crate::labels::{Labels, SnapshotsLabels, fill};
use crate::model::SnapshotStatus;
use crate::store::{SnapshotCounts, Store};

pub fn run(store: &Store, command: &SnapshotsCommand, labels: &Labels) -> anyhow::Result<()> {
    let words = &labels.cli.snapshots;
    match command {
        SnapshotsCommand::List { format } => {
            let entries = store.list_snapshots()?;
            print!("{}", render_list(&entries, *format, words)?);
            Ok(())
        }
        SnapshotsCommand::Show { snapshot, format } => {
            print!("{}", render_show(store, snapshot, *format, words)?);
            Ok(())
        }
        SnapshotsCommand::Diff { a, b, format } => diff(store, a, b, *format, words),
        SnapshotsCommand::Delete {
            snapshot,
            yes,
            force,
            vacuum,
        } => delete(store, snapshot, *yes, *force, *vacuum, words),
        SnapshotsCommand::Prune {
            keep,
            older_than,
            yes,
            vacuum,
        } => prune(store, *keep, *older_than, *yes, *vacuum, words),
        SnapshotsCommand::Verify { format } => verify(store, *format, words),
    }
}

pub fn render_list(
    entries: &[SnapshotCounts],
    format: OutputFormat,
    words: &SnapshotsLabels,
) -> anyhow::Result<String> {
    if format == OutputFormat::Json {
        return Ok(format!("{}\n", serde_json::to_string_pretty(entries)?));
    }
    if entries.is_empty() {
        return Ok(format!("{}\n", words.none_yet));
    }
    let columns = &words.columns;
    let mut table = Table::new();
    table.load_style(presets::UTF8_BORDERS_ONLY);
    table.set_header([
        columns.id.as_str(),
        columns.created.as_str(),
        columns.tenant.as_str(),
        columns.status.as_str(),
        columns.subscriptions.as_str(),
        columns.resources.as_str(),
        columns.findings.as_str(),
    ]);
    for entry in entries {
        table.add_row([
            short(&entry.snapshot.id),
            entry
                .snapshot
                .created_at
                .format("%Y-%m-%d %H:%M")
                .to_string(),
            short(&entry.snapshot.tenant_id),
            entry.snapshot.status.as_str().to_owned(),
            entry.subscriptions.to_string(),
            entry.resources.to_string(),
            entry.findings.to_string(),
        ]);
    }
    Ok(format!("{table}\n"))
}

pub fn render_show(
    store: &Store,
    reference: &str,
    format: OutputFormat,
    words: &SnapshotsLabels,
) -> anyhow::Result<String> {
    let id = store.resolve_snapshot(reference)?;
    let snapshot = store.get_snapshot(&id)?;
    let runs = store.query_runs(&id)?;
    let schema_version = store.schema_version()?;
    if format == OutputFormat::Json {
        return Ok(format!(
            "{}\n",
            serde_json::to_string_pretty(&serde_json::json!({
                "snapshot": snapshot,
                "schema_version": schema_version,
                "query_runs": runs,
            }))?
        ));
    }
    let mut out = String::new();
    let mut line = |template: &str, value: &dyn std::fmt::Display| {
        out.push_str(&fill(template, &[("value", value)]));
        out.push('\n');
    };
    line(&words.snapshot, &snapshot.id);
    line(&words.created, &snapshot.created_at.to_rfc3339());
    line(&words.tenant, &snapshot.tenant_id);
    line(&words.status, &snapshot.status.as_str());
    if let Some(at) = snapshot.interrupted_at {
        line(&words.interrupted, &at.to_rfc3339());
    }
    line(&words.tool, &snapshot.tool_version);
    line(&words.schema, &schema_version);
    if let Some(notes) = &snapshot.notes {
        line(&words.notes, notes);
    }
    if runs.is_empty() {
        return Ok(out);
    }
    let columns = &words.run_columns;
    let mut table = Table::new();
    table.load_style(presets::UTF8_BORDERS_ONLY);
    table.set_header([
        columns.query.as_str(),
        columns.category.as_str(),
        columns.rows.as_str(),
        columns.dropped.as_str(),
        columns.ms.as_str(),
        columns.error.as_str(),
    ]);
    for run in runs {
        table.add_row([
            run.query_name,
            run.category,
            run.row_count.map_or(String::from("-"), |v| v.to_string()),
            run.rows_dropped
                .map_or(String::from("-"), |v| v.to_string()),
            run.duration_ms.map_or(String::from("-"), |v| v.to_string()),
            run.error.unwrap_or_default(),
        ]);
    }
    out.push('\n');
    out.push_str(&table.to_string());
    out.push('\n');
    Ok(out)
}

fn short(id: &str) -> String {
    id.chars().take(8).collect()
}

fn diff(
    store: &Store,
    a: &str,
    b: &str,
    format: DiffFormat,
    words: &SnapshotsLabels,
) -> anyhow::Result<()> {
    let id_a = store.resolve_snapshot(a)?;
    let id_b = store.resolve_snapshot(b)?;
    let diff = store.diff_snapshots(&id_a, &id_b)?;

    match format {
        DiffFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "a": id_a, "b": id_b,
                    "added": diff.added, "removed": diff.removed, "changed": diff.changed,
                }))?
            );
        }
        DiffFormat::Md | DiffFormat::Table => {
            println!(
                "{}",
                fill(
                    &words.comparing,
                    &[
                        ("a", &short(&id_a)),
                        ("b", &short(&id_b)),
                        ("added", &diff.added.len()),
                        ("removed", &diff.removed.len()),
                        ("changed", &diff.changed.len()),
                    ]
                )
            );
            for (label, ids) in [
                ("+ ", &diff.added),
                ("- ", &diff.removed),
                ("~ ", &diff.changed),
            ] {
                for id in ids {
                    println!("{label}{id}");
                }
            }
        }
    }
    Ok(())
}

fn delete(
    store: &Store,
    reference: &str,
    yes: bool,
    force: bool,
    vacuum: bool,
    words: &SnapshotsLabels,
) -> anyhow::Result<()> {
    let id = store.resolve_snapshot(reference)?;
    let snapshot = store.get_snapshot(&id)?;
    if snapshot.status == SnapshotStatus::Running && !force {
        bail!(
            "snapshot {} is still marked running — pass --force if the collect is not alive",
            short(&id)
        );
    }
    if !yes {
        bail!(
            "would delete snapshot {} ({}) — pass --yes to confirm",
            short(&id),
            snapshot.status.as_str()
        );
    }
    store.delete_snapshot(&id)?;
    println!("{}", fill(&words.deleted, &[("count", &1)]));
    finish_vacuum(store, vacuum, words)
}

/// Which snapshots `prune` would remove. Running rows are excluded before
/// `--keep` counts, so a live collect neither counts as a kept snapshot nor
/// gets deleted from under its collector.
pub fn select_prunable(
    entries: &[SnapshotCounts],
    keep: Option<usize>,
    older_than: Option<u32>,
    now: DateTime<Utc>,
) -> Vec<String> {
    entries
        .iter()
        .filter(|entry| entry.snapshot.status != SnapshotStatus::Running)
        .enumerate()
        .filter(|(index, entry)| {
            let too_many = keep.is_some_and(|k| *index >= k);
            let too_old = older_than.is_some_and(|days| {
                now - entry.snapshot.created_at > chrono::Duration::days(i64::from(days))
            });
            too_many || too_old
        })
        .map(|(_, entry)| entry.snapshot.id.clone())
        .collect()
}

fn prune(
    store: &Store,
    keep: Option<usize>,
    older_than: Option<u32>,
    yes: bool,
    vacuum: bool,
    words: &SnapshotsLabels,
) -> anyhow::Result<()> {
    store.require_tenant()?;
    if keep.is_none() && older_than.is_none() {
        bail!("pass --keep <n> and/or --older-than <days>");
    }
    let doomed = select_prunable(&store.list_snapshots()?, keep, older_than, Utc::now());
    if doomed.is_empty() {
        println!("{}", words.nothing_to_prune);
        return Ok(());
    }
    if !yes {
        bail!(
            "would delete {} snapshot(s): {} — pass --yes to confirm",
            doomed.len(),
            doomed
                .iter()
                .map(|id| short(id))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    let deleted = store.delete_snapshots(&doomed)?;
    println!("{}", fill(&words.deleted, &[("count", &deleted)]));
    finish_vacuum(store, vacuum, words)
}

fn finish_vacuum(store: &Store, vacuum: bool, words: &SnapshotsLabels) -> anyhow::Result<()> {
    if vacuum {
        store.vacuum()?;
        println!("{}", words.vacuumed);
    }
    Ok(())
}

fn verify(store: &Store, format: OutputFormat, words: &SnapshotsLabels) -> anyhow::Result<()> {
    // The store was opened writable, so migrations and reconciliation have
    // already run; what is left is to check the file and report.
    let report = store.integrity_check()?;
    let schema_version = store.schema_version()?;
    let interrupted: Vec<String> = store
        .list_snapshots()?
        .into_iter()
        .filter(|entry| entry.snapshot.interrupted_at.is_some())
        .map(|entry| entry.snapshot.id)
        .collect();
    match format {
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "ok": report.is_ok(),
                "schema_version": schema_version,
                "problems": report.problems,
                "interrupted": interrupted,
            }))?
        ),
        OutputFormat::Table => {
            println!("{}", fill(&words.schema, &[("value", &schema_version)]));
            if report.is_ok() {
                println!("{}", words.verify_ok);
            } else {
                println!("{}", words.verify_problems);
                for problem in &report.problems {
                    println!("  {problem}");
                }
            }
            if !interrupted.is_empty() {
                println!(
                    "{}",
                    fill(&words.reconciled, &[("count", &interrupted.len())])
                );
                for id in &interrupted {
                    println!("  {id}");
                }
            }
        }
    }
    if report.is_ok() {
        Ok(())
    } else {
        bail!(
            "database integrity check reported {} problem(s)",
            report.problems.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Snapshot;

    fn entry(id: &str, created: &str, status: SnapshotStatus) -> SnapshotCounts {
        SnapshotCounts {
            snapshot: Snapshot {
                id: id.into(),
                created_at: created.parse().unwrap(),
                tenant_id: "tenant".into(),
                tool_version: "0".into(),
                status,
                notes: None,
                heartbeat_at: None,
                interrupted_at: None,
            },
            subscriptions: 1,
            resources: 2,
            findings: 3,
        }
    }

    #[test]
    fn unit_prune_excludes_running_when_counting_keep() {
        let entries = vec![
            entry("running", "2026-03-01T00:00:00Z", SnapshotStatus::Running),
            entry("newest", "2026-02-01T00:00:00Z", SnapshotStatus::Complete),
            entry("older", "2026-01-01T00:00:00Z", SnapshotStatus::Complete),
        ];

        let doomed = select_prunable(&entries, Some(1), None, Utc::now());

        assert_eq!(doomed, vec!["older".to_string()]);
    }

    #[test]
    fn unit_prune_selects_older_than_days() {
        let entries = vec![
            entry("recent", "2026-03-01T00:00:00Z", SnapshotStatus::Partial),
            entry("old", "2025-01-01T00:00:00Z", SnapshotStatus::Failed),
        ];
        let now = "2026-03-10T00:00:00Z".parse().unwrap();

        let doomed = select_prunable(&entries, None, Some(30), now);

        assert_eq!(doomed, vec!["old".to_string()]);
    }

    #[test]
    fn unit_snapshots_list_json_is_an_array_with_counts() {
        let words = crate::labels::Labels::default().cli.snapshots;
        let entries = vec![entry(
            "abc",
            "2026-01-01T00:00:00Z",
            SnapshotStatus::Complete,
        )];

        let json = render_list(&entries, OutputFormat::Json, &words).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed[0]["resources"], 2);
        assert_eq!(parsed[0]["snapshot"]["status"], "complete");
    }

    #[test]
    fn unit_snapshots_list_table_has_tenant_column() {
        let words = crate::labels::Labels::default().cli.snapshots;
        let entries = vec![entry(
            "abc",
            "2026-01-01T00:00:00Z",
            SnapshotStatus::Complete,
        )];

        let table = render_list(&entries, OutputFormat::Table, &words).unwrap();

        assert!(table.contains("tenant"));
    }
}
