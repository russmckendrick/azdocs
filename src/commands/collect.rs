use std::sync::Arc;

use anyhow::bail;

use crate::arg::ArgClient;
use crate::cli::CollectArgs;
use crate::config::Config;
use crate::model::SnapshotStatus;
use crate::querypack::QueryPack;
use crate::store::Store;

pub async fn run(config: &Config, store: &Store, args: &CollectArgs) -> anyhow::Result<()> {
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
    let client = Arc::new(ArgClient::new(super::http_client(), provider));

    println!(
        "Collecting {} queries across {} (concurrency {concurrency})...",
        selected.len(),
        if subscriptions.is_empty() {
            "all visible subscriptions".to_owned()
        } else {
            format!("{} subscriptions", subscriptions.len())
        },
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
            quiet: false,
        },
    )
    .await?;

    println!(
        "Snapshot {} — {} ({} queries, {} failed, {} rows)",
        summary.snapshot_id,
        summary.status.as_str(),
        summary.queries_run,
        summary.queries_failed,
        summary.rows_ingested,
    );
    if summary.status == SnapshotStatus::Partial {
        println!(
            "Some queries failed; see `azdocs snapshots show {}`",
            summary.snapshot_id
        );
    }
    Ok(())
}
