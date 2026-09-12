pub mod audit;
pub mod extractors;
pub mod ingest;
pub mod websites;

use std::sync::Arc;
use std::time::Instant;

use indicatif::{ProgressBar, ProgressStyle};
use tokio::sync::{Semaphore, mpsc};

use crate::arg::ArgClient;
use crate::auth::TokenProvider;
use crate::model::{QueryRun, SnapshotStatus};
use crate::querypack::QueryDef;
use crate::store::Store;

/// Outcome of a collect run.
#[derive(Debug)]
pub struct CollectSummary {
    pub snapshot_id: String,
    pub status: SnapshotStatus,
    pub queries_run: usize,
    pub queries_failed: usize,
    pub rows_ingested: u64,
}

/// Everything a collect run needs beyond the store and client.
#[derive(Debug)]
pub struct CollectRequest {
    pub tenant_id: String,
    pub queries: Vec<QueryDef>,
    pub subscriptions: Vec<String>,
    pub concurrency: usize,
    pub notes: Option<String>,
    pub required_tags: Vec<String>,
    pub quiet: bool,
}

/// Runs the selected queries concurrently (bounded by `concurrency`), ingesting
/// each result into the store as it arrives. Individual query failures mark the
/// snapshot `partial` rather than aborting the run.
pub async fn run<P: TokenProvider + 'static>(
    store: &Store,
    client: Arc<ArgClient<P>>,
    request: CollectRequest,
) -> anyhow::Result<CollectSummary> {
    run_with_progress(store, client, request, |_| {}).await
}

#[derive(Debug, Clone)]
pub struct CollectProgress {
    pub completed: usize,
    pub total: usize,
    pub rows: u64,
    pub failed: usize,
    pub latest_query: Option<String>,
}

/// Emits progress only after a query's evidence and outcome have been stored.
pub async fn run_with_progress<P: TokenProvider + 'static>(
    store: &Store,
    client: Arc<ArgClient<P>>,
    request: CollectRequest,
    on_progress: impl Fn(CollectProgress),
) -> anyhow::Result<CollectSummary> {
    let CollectRequest {
        tenant_id,
        queries,
        subscriptions,
        concurrency,
        notes,
        required_tags,
        quiet,
    } = request;
    let snapshot = store.create_snapshot(&tenant_id, notes.as_deref())?;
    let total = queries.len();
    let progress = if quiet {
        ProgressBar::hidden()
    } else {
        ProgressBar::new(total as u64).with_style(
            ProgressStyle::with_template("{spinner} [{bar:30}] {pos}/{len} {msg}")
                .expect("static template is valid")
                .progress_chars("=> "),
        )
    };

    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let (sender, mut receiver) =
        mpsc::channel::<(QueryDef, Result<QueryPageData, String>, u64)>(16);

    for def in queries {
        let client = Arc::clone(&client);
        let semaphore = Arc::clone(&semaphore);
        let sender = sender.clone();
        let subscriptions = subscriptions.clone();
        tokio::spawn(async move {
            let _permit = semaphore
                .acquire_owned()
                .await
                .expect("semaphore not closed");
            let started = Instant::now();
            let result = client
                .query_all(&def.kql, &subscriptions)
                .await
                .map(|outcome| QueryPageData { rows: outcome.rows })
                .map_err(|err| err.to_string());
            let elapsed_ms = started.elapsed().as_millis() as u64;
            // Receiver dropping means the run was aborted; nothing to do.
            let _ = sender.send((def, result, elapsed_ms)).await;
        });
    }
    drop(sender);

    on_progress(CollectProgress {
        completed: 0,
        total,
        rows: 0,
        failed: 0,
        latest_query: None,
    });
    let mut completed = 0;
    let mut queries_failed = 0;
    let mut rows_ingested: u64 = 0;
    while let Some((def, result, duration_ms)) = receiver.recv().await {
        let run_record = match result {
            Ok(data) => {
                let count = data.rows.len() as u64;
                match ingest::ingest(store, &snapshot.id, &def, &data.rows) {
                    Ok(()) => {
                        rows_ingested += count;
                        progress.set_message(format!("{} ({count} rows)", def.name));
                        QueryRun {
                            query_name: def.name.clone(),
                            category: def.category.clone(),
                            row_count: Some(count),
                            duration_ms: Some(duration_ms),
                            error: None,
                        }
                    }
                    Err(err) => {
                        queries_failed += 1;
                        tracing::error!(query = %def.name, %err, "ingest failed");
                        query_failure(&def, duration_ms, err.to_string())
                    }
                }
            }
            Err(err) => {
                queries_failed += 1;
                tracing::error!(query = %def.name, %err, "query failed");
                progress.set_message(format!("{} FAILED", def.name));
                query_failure(&def, duration_ms, err)
            }
        };
        store.record_query_run(&snapshot.id, &run_record)?;
        progress.inc(1);
        completed += 1;
        on_progress(CollectProgress {
            completed,
            total,
            rows: rows_ingested,
            failed: queries_failed,
            latest_query: Some(def.description.clone()),
        });
    }
    progress.finish_and_clear();

    // Post-pass over stored resources: derive relationship edges and run the
    // config-driven audits.
    let resources = store.resources(&snapshot.id)?;
    let edges: Vec<_> = resources.iter().flat_map(extractors::extract).collect();
    store.insert_edges(&snapshot.id, &edges)?;
    let tag_findings = audit::missing_required_tags(&resources, &required_tags);
    store.insert_findings(&snapshot.id, &tag_findings)?;
    tracing::info!(
        edges = edges.len(),
        tag_findings = tag_findings.len(),
        "post-pass complete"
    );

    let status = if queries_failed == 0 {
        SnapshotStatus::Complete
    } else if queries_failed < total {
        SnapshotStatus::Partial
    } else {
        SnapshotStatus::Failed
    };
    store.set_snapshot_status(&snapshot.id, status)?;

    Ok(CollectSummary {
        snapshot_id: snapshot.id,
        status,
        queries_run: total,
        queries_failed,
        rows_ingested,
    })
}

struct QueryPageData {
    rows: Vec<serde_json::Value>,
}

fn query_failure(def: &QueryDef, duration_ms: u64, error: String) -> QueryRun {
    QueryRun {
        query_name: def.name.clone(),
        category: def.category.clone(),
        row_count: None,
        duration_ms: Some(duration_ms),
        error: Some(error),
    }
}
