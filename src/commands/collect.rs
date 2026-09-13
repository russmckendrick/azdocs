use std::sync::Arc;

use anyhow::bail;

use crate::arg::ArgClient;
use crate::cli::CollectArgs;
use crate::config::Config;
use crate::labels::{Labels, fill};
use crate::model::SnapshotStatus;
use crate::querypack::QueryPack;
use crate::store::Store;

pub async fn run(
    config: &Config,
    store: &Store,
    args: &CollectArgs,
    labels: &Labels,
) -> anyhow::Result<()> {
    let words = &labels.cli.collect;
    let credentials = config.credentials()?;
    let pack = QueryPack::load()?;
    let selected: Vec<_> = pack
        .select(&args.categories, &args.queries, &args.skip_queries)
        .into_iter()
        .cloned()
        .collect();
    if selected.is_empty() {
        bail!(
            "no queries selected — check --categories/--queries filters against `azdocs query list`"
        );
    }

    let subscriptions = if args.subscriptions.is_empty() {
        config.collect.subscriptions.clone()
    } else {
        args.subscriptions.clone()
    };
    let concurrency = args.concurrency.unwrap_or(config.collect.concurrency);

    let provider = super::token_provider(config)?;
    let check =
        crate::auth::diagnostics::inspect(super::http_client(), &provider, &subscriptions).await?;
    super::check::print_permissions(&check, labels);
    let client = Arc::new(ArgClient::new(super::http_client(), provider));

    let scope = if subscriptions.is_empty() {
        words.all_subscriptions.clone()
    } else {
        fill(
            &words.some_subscriptions,
            &[("count", &subscriptions.len())],
        )
    };
    println!(
        "{}",
        fill(
            &words.collecting,
            &[
                ("queries", &selected.len()),
                ("scope", &scope),
                ("concurrency", &concurrency),
            ]
        )
    );

    let summary = crate::collect::run(
        store,
        client,
        crate::collect::CollectRequest {
            tenant_id: credentials.tenant_id,
            queries: selected,
            subscriptions,
            concurrency,
            notes: args.notes.clone(),
            required_tags: config.audit.required_tags.clone(),
            quiet: false,
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
    if summary.status == SnapshotStatus::Partial {
        println!(
            "{}",
            fill(&words.partial_hint, &[("id", &summary.snapshot_id)])
        );
    }
    Ok(())
}
