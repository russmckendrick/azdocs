use anyhow::bail;
use chrono::{DateTime, Utc};
use comfy_table::{Table, presets};

use crate::cli::{DiffFormat, OutputFormat, SnapshotsCommand};
use crate::labels::{Labels, SnapshotsLabels, fill};
use crate::model::SnapshotStatus;
use crate::model::diff::SnapshotChanges;
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
    let changes = store.snapshot_changes(&id_a, &id_b)?;
    print!("{}", render_diff(&changes, format, words)?);
    Ok(())
}

/// One renderer for every diff format so the three never disagree on what
/// counts as a change. `table` is for a terminal, `md` pastes into a review,
/// `json` is the full structure.
pub fn render_diff(
    changes: &SnapshotChanges,
    format: DiffFormat,
    words: &SnapshotsLabels,
) -> anyhow::Result<String> {
    let d = &words.diff;
    if format == DiffFormat::Json {
        return Ok(format!("{}\n", serde_json::to_string_pretty(changes)?));
    }
    let md = format == DiffFormat::Md;
    let mut out = String::new();
    let heading = |out: &mut String, text: &str| {
        if md {
            out.push_str(&format!("\n## {text}\n\n"));
        } else {
            out.push_str(&format!("\n{text}\n"));
        }
    };
    out.push_str(&fill(
        &d.summary,
        &[
            ("a", &short(&changes.base.id)),
            ("b", &short(&changes.target.id)),
            ("added", &changes.resources.added.len()),
            ("removed", &changes.resources.removed.len()),
            ("changed", &changes.resources.changed.len()),
            ("fields", &changes.field_changes()),
            ("new_findings", &changes.findings.added.len()),
            ("resolved", &changes.findings.resolved.len()),
        ],
    ));
    out.push('\n');
    if changes.is_empty() {
        out.push_str(&d.no_changes);
        out.push('\n');
        return Ok(out);
    }

    if !changes.resources.added.is_empty()
        || !changes.resources.removed.is_empty()
        || !changes.resources.changed.is_empty()
    {
        heading(&mut out, &d.section_resources);
        let mut table = Table::new();
        table.load_style(if md {
            presets::ASCII_MARKDOWN
        } else {
            presets::UTF8_BORDERS_ONLY
        });
        table.set_header([
            d.columns.change.as_str(),
            d.columns.resource.as_str(),
            d.columns.field.as_str(),
            d.columns.before.as_str(),
            d.columns.after.as_str(),
        ]);
        for r in &changes.resources.added {
            table.add_row([
                d.added.clone(),
                r.display_id.clone(),
                String::new(),
                String::new(),
                String::new(),
            ]);
        }
        for r in &changes.resources.removed {
            table.add_row([
                d.removed.clone(),
                r.display_id.clone(),
                String::new(),
                String::new(),
                String::new(),
            ]);
        }
        for change in &changes.resources.changed {
            for field in &change.fields {
                let name = if field.path.is_empty() {
                    field.field.as_str().to_owned()
                } else {
                    format!("{}.{}", field.field.as_str(), field.path)
                };
                table.add_row([
                    d.changed.clone(),
                    change.resource.display_id.clone(),
                    name,
                    cell(field.before.as_ref()),
                    cell(field.after.as_ref()),
                ]);
            }
        }
        out.push_str(&table.to_string());
        out.push('\n');
    }

    if !changes.findings.added.is_empty() || !changes.findings.resolved.is_empty() {
        heading(&mut out, &d.section_findings);
        let mut table = Table::new();
        table.load_style(if md {
            presets::ASCII_MARKDOWN
        } else {
            presets::UTF8_BORDERS_ONLY
        });
        table.set_header([
            d.columns.change.as_str(),
            d.columns.severity.as_str(),
            d.columns.check.as_str(),
            d.columns.resource.as_str(),
            d.columns.title.as_str(),
        ]);
        for f in &changes.findings.added {
            table.add_row([
                d.new_finding.clone(),
                f.severity.as_str().to_owned(),
                f.query_name.clone(),
                f.resource_id.clone().unwrap_or_default(),
                f.title.clone(),
            ]);
        }
        for f in &changes.findings.resolved {
            table.add_row([
                d.resolved.clone(),
                f.severity.as_str().to_owned(),
                f.query_name.clone(),
                f.resource_id.clone().unwrap_or_default(),
                f.title.clone(),
            ]);
        }
        out.push_str(&table.to_string());
        out.push('\n');
    }

    if !changes.edges.added.is_empty() || !changes.edges.removed.is_empty() {
        heading(&mut out, &d.section_edges);
        for e in &changes.edges.added {
            out.push_str(&format!(
                "+ {} -[{}]-> {}\n",
                e.source_id, e.kind, e.target_id
            ));
        }
        for e in &changes.edges.removed {
            out.push_str(&format!(
                "- {} -[{}]-> {}\n",
                e.source_id, e.kind, e.target_id
            ));
        }
    }

    if !changes.subscriptions.added.is_empty()
        || !changes.subscriptions.removed.is_empty()
        || !changes.resource_groups.added.is_empty()
        || !changes.resource_groups.removed.is_empty()
    {
        heading(&mut out, &d.section_scope);
        for id in &changes.subscriptions.added {
            out.push_str(&format!("+ {id}\n"));
        }
        for id in &changes.subscriptions.removed {
            out.push_str(&format!("- {id}\n"));
        }
        for id in &changes.resource_groups.added {
            out.push_str(&format!("+ {id}\n"));
        }
        for id in &changes.resource_groups.removed {
            out.push_str(&format!("- {id}\n"));
        }
    }
    Ok(out)
}

fn cell(value: Option<&serde_json::Value>) -> String {
    match value {
        None => String::new(),
        Some(serde_json::Value::String(s)) => truncate(s, 120),
        Some(other) => truncate(&other.to_string(), 120),
    }
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        text.to_owned()
    } else {
        let mut cut: String = text.chars().take(max - 1).collect();
        cut.push('…');
        cut
    }
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
    fn unit_diff_markdown_lists_field_changes_and_findings() {
        use crate::model::diff::*;
        let words = crate::labels::Labels::default().cli.snapshots;
        let snapshot_ref = |id: &str| SnapshotRef {
            id: id.into(),
            created_at: "2026-01-01T00:00:00Z".into(),
            status: "complete".into(),
        };
        let changes = SnapshotChanges {
            base: snapshot_ref("aaaaaaaa-1"),
            target: snapshot_ref("bbbbbbbb-2"),
            resources: ResourceChanges {
                added: vec![],
                removed: vec![],
                changed: vec![ResourceChange {
                    resource: ResourceRef {
                        id: "/r".into(),
                        display_id: "/R".into(),
                        name: "r".into(),
                        azure_type: "t".into(),
                        subscription_id: "s".into(),
                        resource_group: None,
                    },
                    fields: vec![FieldChange {
                        field: DiffField::Tags,
                        path: "env".into(),
                        before: Some("dev".into()),
                        after: Some("prod".into()),
                    }],
                }],
            },
            findings: FindingChanges {
                added: vec![FindingRef {
                    query_name: "nsg_open".into(),
                    category: "security".into(),
                    severity: crate::model::Severity::High,
                    resource_id: Some("/r".into()),
                    title: "open".into(),
                }],
                resolved: vec![],
            },
            edges: EdgeChanges::default(),
            subscriptions: SetChange::default(),
            resource_groups: SetChange::default(),
            counts: CountDelta::default(),
        };

        let md = render_diff(&changes, DiffFormat::Md, &words).unwrap();
        let table = render_diff(&changes, DiffFormat::Table, &words).unwrap();
        let json = render_diff(&changes, DiffFormat::Json, &words).unwrap();

        assert!(
            md.contains("## ") && md.contains("tags.env") && md.contains("| prod"),
            "{md}"
        );
        assert!(
            table.contains("nsg_open") && !table.contains("## "),
            "{table}"
        );
        assert!(serde_json::from_str::<SnapshotChanges>(&json).is_ok());
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
