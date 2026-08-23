use std::path::{Path, PathBuf};
use std::sync::Arc;

use azdocs::arg::ArgClient;
use azdocs::collect::CollectRequest;
use azdocs::config::{Config, default_config_path};
use azdocs::querypack::QueryPack;
use azdocs::report::ReportContext;
use azdocs::store::Store;
use tauri::State;
use tauri::ipc::Channel;

use crate::AppState;
use crate::dto::{
    AppBootstrap, CollectRequestDto, CollectResultDto, CollectionEvent, EstateSnapshot,
    SnapshotComparison, SnapshotSummary,
};
use crate::error::AppError;

fn database_path(state: &State<'_, AppState>) -> Result<PathBuf, AppError> {
    state
        .database_path
        .read()
        .map(|path| path.clone())
        .map_err(|error| AppError::State(error.to_string()))
}

fn set_database_path(state: &State<'_, AppState>, path: PathBuf) -> Result<(), AppError> {
    let mut current = state
        .database_path
        .write()
        .map_err(|error| AppError::State(error.to_string()))?;
    *current = path;
    Ok(())
}

fn bootstrap_for(path: &Path) -> Result<AppBootstrap, AppError> {
    let store = Store::open(path)?;
    let snapshots: Vec<SnapshotSummary> = store
        .list_snapshots()?
        .into_iter()
        .map(Into::into)
        .collect();
    let latest_snapshot_id = snapshots.first().map(|snapshot| snapshot.id.clone());
    let (config, source) =
        Config::load_with_source(None).map_err(|error| AppError::Config(error.to_string()))?;
    let has_credentials = config.credentials().is_ok();
    let config_path = source
        .clone()
        .unwrap_or_else(default_config_path)
        .display()
        .to_string();

    Ok(AppBootstrap {
        database_path: path.display().to_string(),
        config_path,
        config_found: source.is_some(),
        has_credentials,
        snapshots,
        latest_snapshot_id,
    })
}

#[tauri::command]
pub fn bootstrap(state: State<'_, AppState>) -> Result<AppBootstrap, AppError> {
    bootstrap_for(&database_path(&state)?)
}

#[tauri::command]
pub fn open_database(path: String, state: State<'_, AppState>) -> Result<AppBootstrap, AppError> {
    let path = PathBuf::from(&path);
    if !path.is_file() {
        return Err(AppError::InvalidDatabase(path.display().to_string()));
    }
    let bootstrap = bootstrap_for(&path)?;
    set_database_path(&state, path)?;
    Ok(bootstrap)
}

fn comparison(
    store: &Store,
    base_snapshot_id: String,
    target_snapshot_id: String,
) -> Result<SnapshotComparison, AppError> {
    let diff = store.diff_snapshots(&base_snapshot_id, &target_snapshot_id)?;
    Ok(SnapshotComparison::from_diff(
        base_snapshot_id,
        target_snapshot_id,
        diff,
    ))
}

#[tauri::command]
pub fn compare_snapshots(
    base_snapshot_id: String,
    target_snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<SnapshotComparison, AppError> {
    let store = Store::open(&database_path(&state)?)?;
    comparison(&store, base_snapshot_id, target_snapshot_id)
}

#[tauri::command]
pub fn load_snapshot(
    snapshot_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<EstateSnapshot, AppError> {
    let store = Store::open(&database_path(&state)?)?;
    let snapshot_id = store.resolve_snapshot(snapshot_id.as_deref().unwrap_or("latest"))?;
    let context = ReportContext::build(&store, &snapshot_id)?;
    let subscriptions = store.subscriptions(&snapshot_id)?;
    let resource_groups = store.resource_groups(&snapshot_id)?;
    let resources = store.resources(&snapshot_id)?;
    let findings = store.findings(&snapshot_id)?;
    let edges = store.edges(&snapshot_id)?;
    let query_runs = store.query_runs(&snapshot_id)?;

    let snapshots = store.list_snapshots()?;
    let previous_diff = snapshots
        .iter()
        .position(|entry| entry.snapshot.id == snapshot_id)
        .and_then(|index| snapshots.get(index + 1))
        .map(|previous| comparison(&store, previous.snapshot.id.clone(), snapshot_id.clone()))
        .transpose()?;

    Ok(EstateSnapshot::build(
        context,
        subscriptions,
        resource_groups,
        resources,
        findings,
        edges,
        query_runs,
        previous_diff,
    ))
}

#[tauri::command]
pub async fn collect_snapshot(
    request: CollectRequestDto,
    on_event: Channel<CollectionEvent>,
    state: State<'_, AppState>,
) -> Result<CollectResultDto, AppError> {
    let path = database_path(&state)?;
    let failure_channel = on_event.clone();
    let result: Result<CollectResultDto, AppError> =
        tauri::async_runtime::spawn_blocking(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|error| AppError::Collection(error.to_string()))?;
            runtime.block_on(async move {
                let _ = on_event.send(CollectionEvent::Phase {
                    message: "Loading query pack and credentials".to_owned(),
                });
                let config =
                    Config::load(None).map_err(|error| AppError::Config(error.to_string()))?;
                let credentials = config
                    .credentials()
                    .map_err(|error| AppError::Config(error.to_string()))?;
                let pack =
                    QueryPack::load().map_err(|error| AppError::Collection(error.to_string()))?;
                let queries = pack.all().into_iter().cloned().collect::<Vec<_>>();
                let _ = on_event.send(CollectionEvent::Phase {
                    message: format!("Running {} read-only Azure queries", queries.len()),
                });
                let provider = azdocs::commands::token_provider(&config)
                    .map_err(|error| AppError::Collection(error.to_string()))?;
                let client = Arc::new(ArgClient::new(azdocs::commands::http_client(), provider));
                let store = Store::open(&path)?;
                let subscriptions = if request.subscriptions.is_empty() {
                    config.collect.subscriptions.clone()
                } else {
                    request.subscriptions
                };
                let summary = azdocs::collect::run(
                    &store,
                    client,
                    CollectRequest {
                        tenant_id: credentials.tenant_id,
                        queries,
                        subscriptions,
                        concurrency: config.collect.concurrency,
                        notes: request.notes,
                        required_tags: config.audit.required_tags,
                        quiet: true,
                    },
                )
                .await
                .map_err(|error| AppError::Collection(error.to_string()))?;
                let _ = on_event.send(CollectionEvent::Complete {
                    snapshot_id: summary.snapshot_id.clone(),
                });
                Ok(CollectResultDto {
                    snapshot_id: summary.snapshot_id,
                    status: summary.status.as_str().to_owned(),
                    queries_run: summary.queries_run,
                    queries_failed: summary.queries_failed,
                    rows_ingested: summary.rows_ingested,
                })
            })
        })
        .await
        .map_err(|error| AppError::Collection(error.to_string()))?;

    if let Err(error) = &result {
        let _ = failure_channel.send(CollectionEvent::Failed {
            message: error.to_string(),
        });
    }
    result
}
