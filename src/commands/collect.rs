use std::sync::Arc;

use anyhow::bail;
use comfy_table::{Table, presets};

use crate::cli::{CollectArgs, FailOn};
use crate::config::Config;
use crate::labels::{Labels, fill};
use crate::model::SnapshotStatus;
use crate::querypack::{QueryDef, QueryPack};
use crate::store::Store;

/// What a collect would run, decided from config and flags alone. Shared by
/// the real run and `--dry-run` so the two can never disagree.
#[derive(Debug)]
pub struct CollectPlan {
    pub queries: Vec<QueryDef>,
    pub subscriptions: Vec<String>,
    pub concurrency: usize,
}

pub fn plan(config: &Config, args: &CollectArgs) -> anyhow::Result<CollectPlan> {
    let pack = QueryPack::load()?;
    let queries: Vec<_> = pack
        .select(&args.categories, &args.queries, &args.skip_queries)
        .into_iter()
        .cloned()
        .collect();
    if queries.is_empty() {
        bail!(
            "no queries selected — check --categories/--queries filters against `azdocs query list`"
        );
    }
    let subscriptions = if args.subscriptions.is_empty() {
        config.collect.subscriptions.clone()
    } else {
        args.subscriptions.clone()
    };
    Ok(CollectPlan {
        queries,
        subscriptions,
        concurrency: args.concurrency.unwrap_or(config.collect.concurrency),
    })
}

/// Print the plan without opening the store, acquiring a token or touching
/// Azure Resource Graph.
pub fn dry_run(config: &Config, args: &CollectArgs, labels: &Labels) -> anyhow::Result<()> {
    let words = &labels.cli.collect;
    let plan = plan(config, args)?;
    println!(
        "{}",
        fill(
            &words.dry_run_header,
            &[
                ("queries", &plan.queries.len()),
                ("scope", &scope_words(&plan.subscriptions, labels)),
                ("concurrency", &plan.concurrency),
            ]
        )
    );
    let columns = &words.dry_run_columns;
    let mut table = Table::new();
    table.load_style(presets::UTF8_BORDERS_ONLY);
    table.set_header([
        columns.name.as_str(),
        columns.category.as_str(),
        columns.kind.as_str(),
        columns.severity.as_str(),
    ]);
    for def in &plan.queries {
        table.add_row([
            def.name.clone(),
            def.category.clone(),
            format!("{:?}", def.kind).to_lowercase(),
            def.severity
                .map_or(String::new(), |s| s.as_str().to_owned()),
        ]);
    }
    println!("{table}");
    for subscription in &plan.subscriptions {
        println!("  {subscription}");
    }
    Ok(())
}

/// The exit-code policy. Called after the snapshot line has been printed so
/// a pipeline still learns the id of what was written.
pub fn enforce_fail_on(status: SnapshotStatus, policy: FailOn) -> anyhow::Result<()> {
    let fails = match policy {
        FailOn::Failed => matches!(status, SnapshotStatus::Failed | SnapshotStatus::Cancelled),
        FailOn::Partial => !matches!(status, SnapshotStatus::Complete),
    };
    if fails {
        bail!(
            "collection finished with status `{}` (--fail-on {})",
            status.as_str(),
            match policy {
                FailOn::Failed => "failed",
                FailOn::Partial => "partial",
            }
        );
    }
    Ok(())
}

fn scope_words(subscriptions: &[String], labels: &Labels) -> String {
    let words = &labels.cli.collect;
    if subscriptions.is_empty() {
        words.all_subscriptions.clone()
    } else {
        fill(
            &words.some_subscriptions,
            &[("count", &subscriptions.len())],
        )
    }
}

pub async fn run(
    config: &Config,
    store: &Store,
    args: &CollectArgs,
    labels: &Labels,
) -> anyhow::Result<()> {
    let words = &labels.cli.collect;
    let credentials = config.credentials()?;
    let CollectPlan {
        queries,
        subscriptions,
        concurrency,
    } = plan(config, args)?;

    let provider = super::token_provider(config)?;
    let check = crate::auth::diagnostics::inspect(
        super::http_client(config),
        &provider,
        &subscriptions,
        config.cloud,
    )
    .await?;
    if !args.quiet {
        super::check::print_permissions(&check, labels);
    }
    // An empty `complete` snapshot would be worse than no snapshot: it would
    // become `latest` and describe an estate of nothing.
    if check.has_no_subscriptions() {
        bail!("{}", words.no_subscriptions);
    }
    let client = Arc::new(super::arg_client(config, provider));

    if !args.quiet {
        println!(
            "{}",
            fill(
                &words.collecting,
                &[
                    ("queries", &queries.len()),
                    ("scope", &scope_words(&subscriptions, labels)),
                    ("concurrency", &concurrency),
                ]
            )
        );
    }

    let summary = crate::collect::run(
        store,
        client,
        crate::collect::CollectRequest {
            tenant_id: credentials.tenant_id,
            queries,
            subscriptions,
            concurrency,
            notes: args.notes.clone(),
            required_tags: config.audit.required_tags.clone(),
            quiet: args.quiet,
        },
    )
    .await?;

    println!(
        "{}",
        fill(
            &words.snapshot_written,
            &[
                ("id", &summary.snapshot_id),
                ("status", &summary.status.as_str()),
                ("queries", &summary.queries_run),
                ("failed", &summary.queries_failed),
                ("rows", &summary.rows_ingested),
            ]
        )
    );
    match summary.status {
        SnapshotStatus::Partial => println!(
            "{}",
            fill(&words.partial_hint, &[("id", &summary.snapshot_id)])
        ),
        SnapshotStatus::Failed => println!(
            "{}",
            fill(&words.failed_hint, &[("id", &summary.snapshot_id)])
        ),
        _ => {}
    }
    enforce_fail_on(summary.status, args.fail_on)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_enforce_fail_on_rejects_failed_when_policy_is_failed() {
        assert!(enforce_fail_on(SnapshotStatus::Failed, FailOn::Failed).is_err());
        assert!(enforce_fail_on(SnapshotStatus::Cancelled, FailOn::Failed).is_err());
    }

    #[test]
    fn unit_enforce_fail_on_accepts_partial_when_policy_is_failed() {
        assert!(enforce_fail_on(SnapshotStatus::Partial, FailOn::Failed).is_ok());
        assert!(enforce_fail_on(SnapshotStatus::Complete, FailOn::Failed).is_ok());
    }

    #[test]
    fn unit_enforce_fail_on_rejects_partial_when_policy_is_partial() {
        assert!(enforce_fail_on(SnapshotStatus::Partial, FailOn::Partial).is_err());
        assert!(enforce_fail_on(SnapshotStatus::Complete, FailOn::Partial).is_ok());
    }

    #[test]
    fn unit_plan_lists_selected_queries_without_a_store() {
        let config = Config::default();
        let args = crate::cli::CollectArgs {
            subscriptions: vec!["11111111-1111-4111-8111-111111111111".into()],
            categories: vec!["inventory".into()],
            queries: vec![],
            skip_queries: vec!["all_resources".into()],
            notes: None,
            concurrency: Some(3),
            dry_run: true,
            quiet: false,
            fail_on: FailOn::Failed,
        };

        let plan = plan(&config, &args).unwrap();

        assert!(plan.queries.iter().all(|q| q.category == "inventory"));
        assert!(plan.queries.iter().all(|q| q.name != "all_resources"));
        assert_eq!(plan.subscriptions.len(), 1);
        assert_eq!(plan.concurrency, 3);
    }

    #[test]
    fn unit_plan_fails_when_filters_select_nothing() {
        let config = Config::default();
        let args = crate::cli::CollectArgs {
            subscriptions: vec![],
            categories: vec!["no-such-category".into()],
            queries: vec![],
            skip_queries: vec![],
            notes: None,
            concurrency: None,
            dry_run: false,
            quiet: false,
            fail_on: FailOn::Failed,
        };

        assert!(plan(&config, &args).is_err());
    }
}
