use anyhow::bail;
use comfy_table::{Table, presets};

use crate::cli::{DiffFormat, SnapshotsCommand};
use crate::labels::{Labels, SnapshotsLabels, fill};
use crate::store::Store;

pub fn run(store: &Store, command: &SnapshotsCommand, labels: &Labels) -> anyhow::Result<()> {
    let words = &labels.cli.snapshots;
    match command {
        SnapshotsCommand::List => list(store, words),
        SnapshotsCommand::Show { snapshot } => show(store, snapshot, words),
        SnapshotsCommand::Diff { a, b, format } => diff(store, a, b, *format, words),
        SnapshotsCommand::Prune {
            keep,
            older_than,
            yes,
        } => prune(store, *keep, *older_than, *yes, words),
    }
}

fn list(store: &Store, words: &SnapshotsLabels) -> anyhow::Result<()> {
    let snapshots = store.list_snapshots()?;
    if snapshots.is_empty() {
        println!("{}", words.none_yet);
        return Ok(());
    }
    let columns = &words.columns;
    let mut table = Table::new();
    table.load_style(presets::UTF8_BORDERS_ONLY);
    table.set_header([
        columns.id.as_str(),
        columns.created.as_str(),
        columns.status.as_str(),
        columns.subscriptions.as_str(),
        columns.resources.as_str(),
        columns.findings.as_str(),
    ]);
    for entry in snapshots {
        table.add_row([
            entry.snapshot.id.chars().take(8).collect::<String>(),
            entry
                .snapshot
                .created_at
                .format("%Y-%m-%d %H:%M")
                .to_string(),
            entry.snapshot.status.as_str().to_owned(),
            entry.subscriptions.to_string(),
            entry.resources.to_string(),
            entry.findings.to_string(),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn show(store: &Store, reference: &str, words: &SnapshotsLabels) -> anyhow::Result<()> {
    let id = store.resolve_snapshot(reference)?;
    let snapshot = store.get_snapshot(&id)?;
    let line = |template: &str, value: &dyn std::fmt::Display| {
        println!("{}", fill(template, &[("value", value)]));
    };
    line(&words.snapshot, &snapshot.id);
    line(&words.created, &snapshot.created_at.to_rfc3339());
    line(&words.tenant, &snapshot.tenant_id);
    line(&words.status, &snapshot.status.as_str());
    line(&words.tool, &snapshot.tool_version);
    if let Some(notes) = &snapshot.notes {
        line(&words.notes, notes);
    }

    let runs = store.query_runs(&id)?;
    if runs.is_empty() {
        return Ok(());
    }
    let columns = &words.run_columns;
    let mut table = Table::new();
    table.load_style(presets::UTF8_BORDERS_ONLY);
    table.set_header([
        columns.query.as_str(),
        columns.category.as_str(),
        columns.rows.as_str(),
        columns.ms.as_str(),
        columns.error.as_str(),
    ]);
    for run in runs {
        table.add_row([
            run.query_name,
            run.category,
            run.row_count.map_or(String::from("-"), |v| v.to_string()),
            run.duration_ms.map_or(String::from("-"), |v| v.to_string()),
            run.error.unwrap_or_default(),
        ]);
    }
    println!("\n{table}");
    Ok(())
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
                        ("a", &&id_a[..8.min(id_a.len())]),
                        ("b", &&id_b[..8.min(id_b.len())]),
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

fn prune(
    store: &Store,
    keep: Option<usize>,
    older_than: Option<u32>,
    yes: bool,
    words: &SnapshotsLabels,
) -> anyhow::Result<()> {
    store.require_tenant()?;
    if keep.is_none() && older_than.is_none() {
        bail!("pass --keep <n> and/or --older-than <days>");
    }
    let snapshots = store.list_snapshots()?;
    let now = chrono::Utc::now();
    let mut doomed = Vec::new();
    for (index, entry) in snapshots.iter().enumerate() {
        let too_many = keep.is_some_and(|k| index >= k);
        let too_old = older_than.is_some_and(|days| {
            now - entry.snapshot.created_at > chrono::Duration::days(i64::from(days))
        });
        if too_many || too_old {
            doomed.push(entry.snapshot.id.clone());
        }
    }
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
                .map(|id| &id[..8.min(id.len())])
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    for id in &doomed {
        store.delete_snapshot(id)?;
    }
    println!("{}", fill(&words.deleted, &[("count", &doomed.len())]));
    Ok(())
}
