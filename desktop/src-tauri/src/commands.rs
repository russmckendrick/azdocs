use std::path::{Path, PathBuf};
use std::sync::Arc;

use azdocs::cli::ArgValue;
use azdocs::cli::{DiagramArgs, DiagramFormat, DiagramType, ReportArgs, ReportFormat};
use azdocs::collect::CollectRequest;
use azdocs::config::{Config, default_config_path};
use azdocs::labels::fill;
use azdocs::querypack::{QueryKind, QueryPack};
use azdocs::report::ReportContext;
use azdocs::store::Store;
use tauri::State;
use tauri::ipc::Channel;

use crate::AppState;
use crate::dto::{
    AppBootstrap, CollectRequestDto, CollectResultDto, CollectionEvent, EstateSnapshot,
    ExportEvent, ExportRequestDto, ExportResultDto, QueryDefDto, QueryRowsDto, ResourceDetailDto,
    SnapshotComparison, SnapshotSummary,
};
use crate::error::AppError;
use crate::labels::AppLabels;
use crate::settings::Session;
use crate::topology::{self, TopologyGraphDto, TopologyRequest};

/// Runs a store read on a blocking thread so the webview never waits on
/// SQLite. Every read command takes a clone of the session, opens its own
/// read-only store there, and returns the DTO; a 300-resource estate was
/// freezing the window for the length of `load_snapshot` before this.
pub(crate) async fn blocking<T, F>(session: Session, work: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(Session) -> Result<T, AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || work(session))
        .await
        .map_err(|error| AppError::State(error.to_string()))?
}

pub(crate) fn database_path(state: &State<'_, AppState>) -> Result<PathBuf, AppError> {
    Ok(crate::settings::session(state)?.database_path)
}

/// Read-only: every explorer, topology, comparison and export path reads
/// stored evidence and must never migrate or rewrite the database. Only the
/// start-up open, `open_database` and collection open it for writing.
pub(crate) fn open_store(state: &State<'_, AppState>) -> Result<Store, AppError> {
    open_store_for(&crate::settings::session(state)?)
}

pub(crate) fn open_store_for(session: &Session) -> Result<Store, AppError> {
    Ok(Store::open_read_only(&session.database_path)?.with_tenant(session.tenant_id.as_deref()))
}

pub(crate) fn bootstrap_for(session: &crate::settings::Session) -> Result<AppBootstrap, AppError> {
    let document = session.document();
    let config_error = document.as_ref().err().map(ToString::to_string);
    let config = document
        .as_ref()
        .ok()
        .and_then(|d| d.resolve(session.tenant_id.as_deref()).ok());
    let source = document
        .as_ref()
        .ok()
        .and_then(|d| d.source.clone())
        .or(session.config_path.clone());
    // A database that does not exist yet is an empty estate, not an error:
    // the first collect creates it.
    let store = match Store::open_read_only(&session.database_path) {
        Ok(store) => Some(store),
        Err(azdocs::error::StoreError::DatabaseMissing(_)) => None,
        Err(error) => return Err(error.into()),
    };
    let mut tenants: Vec<crate::settings::TenantSummary> = document
        .as_ref()
        .ok()
        .map(|d| {
            d.values
                .tenants
                .iter()
                .map(|(reference, p)| crate::settings::TenantSummary {
                    reference: reference.clone(),
                    name: p.name.clone(),
                    tenant_id: p.tenant_id.to_ascii_lowercase(),
                    configured: true,
                })
                .collect()
        })
        .unwrap_or_default();
    if tenants.is_empty()
        && let Some(id) = config.as_ref().and_then(|c| c.auth.tenant_id.clone())
    {
        tenants.push(crate::settings::TenantSummary {
            reference: id.clone(),
            name: id.clone(),
            tenant_id: id,
            configured: true,
        });
    }
    let stored_tenants = match &store {
        Some(store) => store.tenant_ids()?,
        None => Vec::new(),
    };
    for id in stored_tenants {
        if !tenants.iter().any(|p| p.tenant_id == id) {
            tenants.push(crate::settings::TenantSummary {
                reference: id.clone(),
                name: id.clone(),
                tenant_id: id,
                configured: false,
            });
        }
    }
    tenants.sort_by(|a, b| (&a.name, &a.tenant_id).cmp(&(&b.name, &b.tenant_id)));
    let store = store.map(|store| store.with_tenant(session.tenant_id.as_deref()));
    let snapshots: Vec<SnapshotSummary> = match (&store, session.tenant_id.is_some()) {
        (Some(store), true) => store
            .list_snapshots()?
            .into_iter()
            .map(Into::into)
            .collect(),
        _ => Vec::new(),
    };
    let latest_snapshot_id = match (&store, session.tenant_id.is_some()) {
        (Some(store), true) => store.resolve_snapshot("latest").ok(),
        _ => None,
    };
    let has_credentials = config.as_ref().is_some_and(|c| {
        c.auth.tenant_id.is_some()
            && c.auth.client_id.is_some()
            && (c.auth.secret_ref.is_some()
                || c.auth.secret_env.is_some()
                || c.auth.client_secret.is_some())
    });
    Ok(AppBootstrap {
        tenants,
        active_tenant_id: session.tenant_id.clone(),
        config_error,
        database_path: session.database_path.display().to_string(),
        config_path: source
            .as_deref()
            .unwrap_or(&default_config_path())
            .display()
            .to_string(),
        config_found: source.is_some(),
        has_credentials,
        required_tags: config.map(|c| c.audit.required_tags).unwrap_or_default(),
        snapshots,
        latest_snapshot_id,
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        labels: (*session.labels).clone(),
    })
}

#[tauri::command]
pub fn query_pack_metadata() -> Result<Vec<QueryDefDto>, AppError> {
    let pack = QueryPack::load().map_err(|error| AppError::Config(error.to_string()))?;
    Ok(pack
        .all()
        .into_iter()
        .map(|def| QueryDefDto {
            name: def.name.clone(),
            category: def.category.clone(),
            kind: match def.kind {
                QueryKind::Inventory => "inventory".to_owned(),
                QueryKind::Finding => "finding".to_owned(),
            },
            description: def.description.clone(),
            resource_types: def.resource_types.clone(),
            resource_column: def.resource_column().map(str::to_owned),
            resource_table: def.resource_table,
        })
        .collect())
}

/// Every inventory row that describes one resource, grouped by query in pack
/// order — the per-resource side of the join the Estate type tables make.
#[tauri::command]
pub async fn resource_query_rows(
    snapshot_id: String,
    resource_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<QueryRowsDto>, AppError> {
    let session = crate::settings::session(&state)?;
    let pack = QueryPack::load().map_err(|error| AppError::Config(error.to_string()))?;
    blocking(session, move |session| {
        let store = open_store_for(&session)?;
        let snapshot_id = store.resolve_snapshot(&snapshot_id)?;
        let mut out = Vec::new();
        for def in pack.all() {
            let Some(column) = def.resource_column() else {
                continue;
            };
            let rows =
                store.query_results_for_resource(&snapshot_id, &def.name, column, &resource_id)?;
            if !rows.is_empty() {
                out.push(QueryRowsDto {
                    query_name: def.name.clone(),
                    columns: azdocs::model::rows::columns(&rows),
                    rows,
                });
            }
        }
        Ok(out)
    })
    .await
}

#[tauri::command]
pub async fn query_rows(
    snapshot_id: Option<String>,
    query_name: String,
    state: State<'_, AppState>,
) -> Result<QueryRowsDto, AppError> {
    let session = crate::settings::session(&state)?;
    blocking(session, move |session| {
        let store = open_store_for(&session)?;
        let snapshot_id = store.resolve_snapshot(snapshot_id.as_deref().unwrap_or("latest"))?;
        let rows = store.query_results(&snapshot_id, &query_name)?;
        Ok(QueryRowsDto {
            query_name,
            columns: azdocs::model::rows::columns(&rows),
            rows,
        })
    })
    .await
}

/// The stored bags of one resource, read when its record is opened.
#[tauri::command]
pub async fn resource_detail(
    snapshot_id: String,
    resource_id: String,
    state: State<'_, AppState>,
) -> Result<Option<ResourceDetailDto>, AppError> {
    let session = crate::settings::session(&state)?;
    blocking(session, move |session| {
        let store = open_store_for(&session)?;
        let snapshot_id = store.resolve_snapshot(&snapshot_id)?;
        Ok(store
            .resource(&snapshot_id, &resource_id)?
            .map(ResourceDetailDto::from))
    })
    .await
}

#[tauri::command]
pub fn bootstrap(state: State<'_, AppState>) -> Result<AppBootstrap, AppError> {
    bootstrap_for(&crate::settings::session(&state)?)
}

#[tauri::command]
pub fn open_database(path: String, state: State<'_, AppState>) -> Result<AppBootstrap, AppError> {
    let _lease = state.captures.begin()?;
    let path = PathBuf::from(path);
    if !path.is_file() {
        return Err(AppError::InvalidDatabase(path.display().to_string()));
    }
    let store = Store::open(&path)?;
    store.recover_website_captures()?;
    let mut next = crate::settings::session(&state)?;
    next.database_path = path;
    let ids = store.tenant_ids()?;
    if !next.tenant_id.as_ref().is_some_and(|id| ids.contains(id)) {
        next.tenant_id = if ids.len() == 1 {
            ids.first().cloned()
        } else {
            None
        };
    }
    let bootstrap = bootstrap_for(&next)?;
    crate::settings::install(&state, next)?;
    Ok(bootstrap)
}

fn comparison(
    store: &Store,
    base_snapshot_id: String,
    target_snapshot_id: String,
) -> Result<SnapshotComparison, AppError> {
    Ok(SnapshotComparison::from(store.snapshot_changes(
        &base_snapshot_id,
        &target_snapshot_id,
    )?))
}

#[tauri::command]
pub async fn compare_snapshots(
    base_snapshot_id: String,
    target_snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<SnapshotComparison, AppError> {
    let session = crate::settings::session(&state)?;
    blocking(session, move |session| {
        let store = open_store_for(&session)?;
        comparison(&store, base_snapshot_id, target_snapshot_id)
    })
    .await
}

#[tauri::command]
pub async fn load_snapshot(
    snapshot_id: Option<String>,
    state: State<'_, AppState>,
) -> Result<EstateSnapshot, AppError> {
    let session = crate::settings::session(&state)?;
    blocking(session, move |session| {
        let store = open_store_for(&session)?;
        let snapshot_id = store.resolve_snapshot(snapshot_id.as_deref().unwrap_or("latest"))?;
        let context = ReportContext::build_for_desktop(&store, &snapshot_id)?;
        let subscriptions = store.subscriptions(&snapshot_id)?;
        let resource_groups = store.resource_groups(&snapshot_id)?;
        let resources = store.resources(&snapshot_id)?;
        let findings = store.findings(&snapshot_id)?;
        let edges = store.edges(&snapshot_id)?;
        let query_runs = store.query_runs(&snapshot_id)?;

        // The baseline is the previous *usable* snapshot; a failed or running
        // neighbour would make the whole estate look newly added. Only its id
        // travels here; the frontend asks for the comparison after first paint.
        let previous_snapshot_id = store.previous_snapshot(&snapshot_id)?.map(|s| s.id);

        let evidence_summaries = context
            .posture
            .tables_with_words(&session.labels.posture)
            .into_iter()
            .map(Into::into)
            .collect();
        let mut estate = EstateSnapshot::build(
            context,
            subscriptions,
            resource_groups,
            resources,
            findings,
            edges,
            query_runs,
            previous_snapshot_id,
            &session.labels.common.subscription_scope,
        );
        estate.evidence_summaries = evidence_summaries;
        Ok(estate)
    })
    .await
}

#[tauri::command]
pub async fn topology_graph(
    request: TopologyRequest,
    state: State<'_, AppState>,
) -> Result<TopologyGraphDto, AppError> {
    let session = crate::settings::session(&state)?;
    blocking(session, move |session| {
        let store = open_store_for(&session)?;
        let snapshot_id =
            store.resolve_snapshot(request.snapshot_id.as_deref().unwrap_or("latest"))?;
        let subscriptions = store.subscriptions(&snapshot_id)?;
        let resource_groups = store.resource_groups(&snapshot_id)?;
        let resources = store.resources(&snapshot_id)?;
        let edges = store.edges(&snapshot_id)?;
        let findings = store.findings(&snapshot_id)?;
        let mut finding_counts = std::collections::BTreeMap::new();
        for finding in &findings {
            if let Some(resource_id) = &finding.resource_id {
                *finding_counts.entry(resource_id.clone()).or_insert(0) += 1;
            }
        }
        Ok(topology::build(
            &request,
            &topology::TopologyInput {
                subscriptions: &subscriptions,
                resource_groups: &resource_groups,
                resources: &resources,
                edges: &edges,
                finding_counts: &finding_counts,
                labels: &session.labels,
            },
        ))
    })
    .await
}

/// Puts text on the system clipboard (an ARM id, a set of finding ids).
#[tauri::command]
pub fn copy_text(app: tauri::AppHandle, text: String) -> Result<(), AppError> {
    use tauri_plugin_clipboard_manager::ClipboardExt as _;
    app.clipboard()
        .write_text(text)
        .map_err(|error| AppError::State(error.to_string()))
}

/// Saves text the frontend composed (a findings CSV) where the user
/// chooses; the same native picker `save_website_image` uses.
#[tauri::command]
pub async fn save_text_file(
    app: tauri::AppHandle,
    suggested_name: String,
    contents: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    use tauri_plugin_dialog::DialogExt as _;
    let title = crate::settings::session(&state)?
        .labels
        .desktop
        .dialogs
        .save_file_title
        .clone();
    let extension = std::path::Path::new(&suggested_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("txt")
        .to_owned();
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title(title)
        .set_file_name(&suggested_name)
        .add_filter(extension.to_ascii_uppercase(), &[extension.as_str()])
        .save_file(move |path| {
            let _ = tx.send(path);
        });
    let Some(path) = rx.await.map_err(|e| AppError::State(e.to_string()))? else {
        return Ok(false);
    };
    let path = path
        .into_path()
        .map_err(|e| AppError::State(e.to_string()))?;
    std::fs::write(path, contents).map_err(|e| AppError::State(e.to_string()))?;
    Ok(true)
}

/// Only paths under a destination this session exported to may be opened
/// or revealed: the frontend names paths, but the backend decides which
/// ones are its own outputs rather than arbitrary files on the machine.
fn exported_path(state: &State<'_, AppState>, path: &str) -> Result<PathBuf, AppError> {
    let path = PathBuf::from(path);
    let roots = state
        .export_roots
        .lock()
        .map_err(|e| AppError::State(e.to_string()))?;
    if roots.iter().any(|root| path.starts_with(root)) && path.exists() {
        Ok(path)
    } else {
        Err(AppError::Export(format!(
            "{} is not an export of this session",
            path.display()
        )))
    }
}

/// Shows an exported file in the platform file manager.
#[tauri::command]
pub fn reveal_export_path(
    app: tauri::AppHandle,
    path: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    use tauri_plugin_opener::OpenerExt as _;
    let path = exported_path(&state, &path)?;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|error| AppError::Export(error.to_string()))
}

/// Opens an export destination folder in the platform file manager.
#[tauri::command]
pub fn open_export_folder(
    app: tauri::AppHandle,
    path: String,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    use tauri_plugin_opener::OpenerExt as _;
    let path = exported_path(&state, &path)?;
    let folder = if path.is_dir() {
        path
    } else {
        path.parent().map(Path::to_path_buf).unwrap_or(path)
    };
    app.opener()
        .open_path(folder.display().to_string(), None::<&str>)
        .map_err(|error| AppError::Export(error.to_string()))
}

/// The one external link the app opens: its own documentation.
pub const DOCS_URL: &str = "https://github.com/russmckendrick/azdocs/tree/main/docs";

#[tauri::command]
pub fn open_docs(app: tauri::AppHandle) -> Result<(), AppError> {
    use tauri_plugin_opener::OpenerExt as _;
    app.opener()
        .open_url(DOCS_URL, None::<&str>)
        .map_err(|error| AppError::Export(error.to_string()))
}

#[tauri::command]
pub async fn collect_snapshot(
    app: tauri::AppHandle,
    request: CollectRequestDto,
    on_event: Channel<CollectionEvent>,
    state: State<'_, AppState>,
) -> Result<CollectResultDto, AppError> {
    let lease = state.captures.begin()?;
    let cancel = azdocs::collect::CancelToken::from(lease.flag());
    let path = database_path(&state)?;
    let capture_path = path.clone();
    let capture_channel = on_event.clone();
    let completion_channel = on_event.clone();
    let session = crate::settings::session(&state)?;
    let labels = Arc::clone(&session.labels);
    let failure_channel = on_event.clone();
    let document = session.document()?;
    let revision = document.revision.clone();
    let config = document
        .resolve(session.tenant_id.as_deref())
        .map_err(crate::settings::config_error)?;
    let checks = Arc::clone(&state.checks);
    let result: Result<CollectResultDto, AppError> =
        tauri::async_runtime::spawn_blocking(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|error| AppError::Collection(error.to_string()))?;
            runtime.block_on(async move {
                let _ = on_event.send(CollectionEvent::Stage {
                    stage: crate::dto::CollectionStage::Inventory,
                });
                let phases = &labels.desktop.backend.phases;
                let _ = on_event.send(CollectionEvent::Phase {
                    message: phases.loading_pack.clone(),
                });
                let credentials = config
                    .credentials()
                    .map_err(|error| AppError::Config(error.to_string()))?;
                let pack =
                    QueryPack::load().map_err(|error| AppError::Collection(error.to_string()))?;
                let queries = pack.all().into_iter().cloned().collect::<Vec<_>>();
                let _ = on_event.send(CollectionEvent::Phase {
                    message: fill(&phases.running_queries, &[("count", &queries.len())]),
                });
                let provider = azdocs::commands::token_provider(&config)
                    .map_err(|error| AppError::Collection(error.to_string()))?;
                let subscriptions = if request.subscriptions.is_empty() {
                    config.collect.subscriptions.clone()
                } else {
                    request.subscriptions
                };
                let check = azdocs::auth::diagnostics::inspect(
                    azdocs::commands::http_client(&config),
                    &provider,
                    &subscriptions,
                    config.cloud,
                )
                .await
                .map_err(|e| AppError::Collection(e.to_string()))?;
                if let Ok(mut saved) = checks.write() {
                    saved.insert(
                        credentials.tenant_id.to_ascii_lowercase(),
                        crate::settings::SavedCheck {
                            revision: revision.clone(),
                            check: check.clone(),
                        },
                    );
                }
                let _ = on_event.send(CollectionEvent::Permissions { check });
                let client = Arc::new(azdocs::commands::arg_client(&config, provider));
                let store = Store::open(&path)?;
                let summary = azdocs::collect::run_with_progress(
                    &store,
                    client,
                    CollectRequest {
                        tenant_id: credentials.tenant_id,
                        queries,
                        subscriptions,
                        concurrency: config.collect.concurrency,
                        notes: request.notes,
                        audit: config.audit.clone(),
                        quiet: true,
                        cancel,
                    },
                    |progress| {
                        let _ = on_event.send(CollectionEvent::Queries {
                            progress: crate::dto::CollectionQueryProgress {
                                completed: progress.completed,
                                total: progress.total,
                                rows: progress.rows,
                                failed: progress.failed,
                                latest_query: progress.latest_query,
                            },
                        });
                    },
                )
                .await
                .map_err(|error| AppError::Collection(error.to_string()))?;
                let mut discovery_error = None;
                if summary.status.is_usable() {
                    let _ = on_event.send(CollectionEvent::Stage {
                        stage: crate::dto::CollectionStage::Discovery,
                    });
                    let discovery = async {
                        let resources = store.resources(&summary.snapshot_id)?;
                        let _ = on_event.send(CollectionEvent::Phase {
                            message: labels.common.websites.discovering.clone(),
                        });
                        let provider = azdocs::commands::token_provider(&config)
                            .map_err(|e| AppError::Config(e.to_string()))?;
                        let evidence = azdocs::collect::websites::WebsiteManagement::new(
                            provider,
                            config.cloud,
                        )
                        .map_err(|e| AppError::Collection(e.to_string()))?
                        .enrich(&resources)
                        .await;
                        let endpoints = azdocs::collect::websites::discover(&resources, &evidence);
                        store.save_website_inventory(
                            &summary.snapshot_id,
                            &endpoints,
                            &evidence,
                        )?;
                        Ok::<(), AppError>(())
                    }
                    .await;
                    discovery_error = discovery.err().map(|e| e.to_string());
                }
                Ok(CollectResultDto {
                    snapshot_id: summary.snapshot_id,
                    status: summary.status.as_str().to_owned(),
                    queries_run: summary.queries_run,
                    queries_failed: summary.queries_failed,
                    rows_ingested: summary.rows_ingested,
                    screenshots: discovery_error.map(|error| crate::dto::WebsiteBatchResult {
                        captured: 0,
                        failed: 0,
                        skipped: 0,
                        cancelled: false,
                        error: Some(error),
                    }),
                })
            })
        })
        .await
        .map_err(|error| AppError::Collection(error.to_string()))?;

    let mut result = result;
    if let Ok(summary) = &mut result {
        if summary.status == "cancelled" {
            let _ = completion_channel.send(CollectionEvent::Cancelled {
                snapshot_id: summary.snapshot_id.clone(),
            });
            return result;
        }
        if summary.status != "failed" {
            let _ = capture_channel.send(CollectionEvent::Stage {
                stage: crate::dto::CollectionStage::Capture,
            });
            let request = crate::dto::WebsiteCaptureRequest {
                snapshot_id: summary.snapshot_id.clone(),
                urls: vec![],
                retry_only: false,
            };
            let discovery_error = summary.screenshots.take().and_then(|s| s.error);
            summary.screenshots = Some(
                match crate::capture::run(&app, &capture_path, request, &lease, |progress| {
                    let _ = capture_channel.send(CollectionEvent::Screenshots { progress });
                })
                .await
                {
                    Ok(mut result) => {
                        if let Some(discovery_error) = discovery_error {
                            result.error = Some(match result.error {
                                Some(error) => format!("{discovery_error}; {error}"),
                                None => discovery_error,
                            });
                        }
                        result
                    }
                    Err(error) => crate::dto::WebsiteBatchResult {
                        captured: 0,
                        failed: 0,
                        skipped: 0,
                        cancelled: false,
                        error: Some(error.to_string()),
                    },
                },
            );
        }
        let _ = completion_channel.send(CollectionEvent::Complete {
            snapshot_id: summary.snapshot_id.clone(),
        });
    }
    if let Err(error) = &result {
        let _ = failure_channel.send(CollectionEvent::Failed {
            message: error.to_string(),
        });
    }
    result
}

/// Stops the running collection after its in-flight query (or the current
/// screenshot); the snapshot keeps what finished and is stored as cancelled.
#[tauri::command]
pub fn cancel_collect(state: State<'_, AppState>) {
    state.captures.cancel();
}

/// Stops the running export between formats; what was written stays.
#[tauri::command]
pub fn cancel_export(state: State<'_, AppState>) {
    state.exports.cancel();
}

/// Parse a value the frontend sent into the CLI enum it names.
///
/// clap already derives these kebab-case names, so asking it is the only way
/// the desktop and `azdocs diagram --type ...` cannot drift apart. The three
/// hand-written match tables this replaced had done exactly that job.
fn parse_value<T: ArgValue>(kind: &str, value: &str) -> Result<T, AppError> {
    T::from_str(value, true).map_err(|_| AppError::Export(format!("unsupported {kind} `{value}`")))
}

fn report_format(value: &str) -> Result<ReportFormat, AppError> {
    let format: ReportFormat = parse_value("report format", value)?;
    // `all` is a CLI convenience meaning "every format". The desktop sends an
    // explicit list, so accepting it here would silently widen the export.
    if format == ReportFormat::All {
        return Err(AppError::Export(
            "report format `all` must be expanded into explicit formats".to_owned(),
        ));
    }
    Ok(format)
}

fn diagram_type(value: &str) -> Result<DiagramType, AppError> {
    parse_value("diagram type", value)
}

fn diagram_format(value: &str) -> Result<DiagramFormat, AppError> {
    let format: DiagramFormat = parse_value("diagram format", value)?;
    // Same reasoning as `all` above: these name a set, and the desktop has to
    // know each concrete format to name its output file.
    if format.extension().is_none() {
        return Err(AppError::Export(format!(
            "diagram format `{value}` names a set; send the formats explicitly"
        )));
    }
    Ok(format)
}

/// Outputs written so far plus whether the run was stopped on request.
struct ExportOutcome {
    outputs: Vec<PathBuf>,
    cancelled: bool,
}

/// What every export family needs besides its request.
struct ExportEnv<'a> {
    destination: &'a Path,
    store: &'a Store,
    on_event: &'a Channel<ExportEvent>,
    labels: &'a AppLabels,
    config: &'a Config,
    config_dir: Option<&'a Path>,
    cancelled: &'a dyn Fn() -> bool,
}

fn export_reports(
    request: ExportRequestDto,
    env: &ExportEnv<'_>,
) -> Result<ExportOutcome, AppError> {
    let ExportEnv {
        destination,
        store,
        on_event,
        labels,
        config,
        config_dir,
        cancelled,
    } = *env;
    let mut formats = Vec::new();
    for value in &request.formats {
        let format = report_format(value)?;
        if !formats.contains(&format) {
            formats.push(format);
        }
    }
    if formats.is_empty() {
        return Err(AppError::Export(
            labels.desktop.backend.errors.no_report_format.clone(),
        ));
    }

    let _ = on_event.send(ExportEvent::Phase {
        message: fill(
            &labels.desktop.backend.phases.composing_reports,
            &[("count", &formats.len())],
        ),
    });
    let severity = request
        .min_severity
        .as_deref()
        .map(|value| parse_value::<azdocs::cli::SeverityArg>("severity", value))
        .transpose()?;
    let args = ReportArgs {
        include_reference: request.include_reference.unwrap_or(false),
        snapshot: request.snapshot_id,
        format: formats[0],
        theme: None,
        out: Some(destination.to_path_buf()),
        subscription: request.subscription_id.clone(),
        resource_group: request.resource_group.clone(),
        severity,
    };
    // One format per call so a cancel lands between documents rather than
    // after the whole set.
    let mut outputs = Vec::new();
    for format in formats {
        if cancelled() {
            return Ok(ExportOutcome {
                outputs,
                cancelled: true,
            });
        }
        outputs.extend(
            azdocs::commands::report::run_selected_with_progress(
                config,
                config_dir,
                store,
                &args,
                &[format],
                |path| {
                    let file = path.strip_prefix(destination).unwrap_or(path).display();
                    let _ = on_event.send(ExportEvent::Phase {
                        message: fill(
                            &labels.desktop.backend.phases.rendering_report,
                            &[("path", &file)],
                        ),
                    });
                },
            )
            .map_err(|error| AppError::Export(error.to_string()))?,
        );
    }
    Ok(ExportOutcome {
        outputs,
        cancelled: false,
    })
}

fn export_diagrams(
    request: ExportRequestDto,
    env: &ExportEnv<'_>,
) -> Result<ExportOutcome, AppError> {
    let ExportEnv {
        destination,
        store,
        on_event,
        labels,
        config,
        cancelled,
        ..
    } = *env;
    let errors = &labels.desktop.backend.errors;
    let kind = request
        .diagram_type
        .as_deref()
        .ok_or_else(|| AppError::Export(errors.no_diagram_type.clone()))
        .and_then(diagram_type)?;
    let mut formats = Vec::new();
    for value in &request.formats {
        let format = diagram_format(value)?;
        if !formats.contains(&format) {
            formats.push(format);
        }
    }
    if formats.is_empty() {
        return Err(AppError::Export(errors.no_diagram_format.clone()));
    }
    if let Some(reason) = formats
        .iter()
        .find_map(|format| kind.unsupported_reason(*format))
    {
        return Err(AppError::Export(reason.to_owned()));
    }

    // The diagram command prints through the CLI labels, so the desktop
    // resolves the same set the report path does: the config's choice, with
    // the built-ins when the config cannot be read.
    let cli_labels =
        azdocs::labels::resolve(&config.branding).map_err(|e| AppError::Config(e.to_string()))?;
    let mut outputs = Vec::new();
    for format in formats {
        if cancelled() {
            return Ok(ExportOutcome {
                outputs,
                cancelled: true,
            });
        }
        let _ = on_event.send(ExportEvent::Phase {
            message: fill(
                &labels.desktop.backend.phases.rendering_diagram,
                &[
                    ("kind", &kind.slug()),
                    ("format", &format.extension().unwrap_or("out")),
                ],
            ),
        });
        let args = DiagramArgs {
            snapshot: request.snapshot_id.clone(),
            diagram_type: kind,
            format,
            subscription: request.subscription_id.clone(),
            resource_group: request.resource_group.clone(),
            out: Some(azdocs::commands::diagram::default_output_path(
                destination,
                kind,
                format,
            )),
        };
        outputs.extend(
            azdocs::commands::diagram::run_with_outputs(store, &args, &cli_labels)
                .map_err(|error| AppError::Export(error.to_string()))?,
        );
    }
    Ok(ExportOutcome {
        outputs,
        cancelled: false,
    })
}

#[tauri::command]
pub async fn export_snapshot(
    request: ExportRequestDto,
    on_event: Channel<ExportEvent>,
    state: State<'_, AppState>,
) -> Result<ExportResultDto, AppError> {
    let lease = state.exports.begin()?;
    let cancel_flag = lease.flag();
    let session = crate::settings::session(&state)?;
    if !request.destination.trim().is_empty() {
        state
            .export_roots
            .lock()
            .map_err(|e| AppError::State(e.to_string()))?
            .insert(PathBuf::from(&request.destination));
    }
    let database = session.database_path.clone();
    let labels = Arc::clone(&session.labels);
    let document = session.document().or_else(|_| {
        azdocs::config::ConfigDocument::parse("", None).map_err(crate::settings::config_error)
    })?;
    let failure_channel = on_event.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let cancelled = move || cancel_flag.load(std::sync::atomic::Ordering::Relaxed);
        let errors = &labels.desktop.backend.errors;
        let destination = PathBuf::from(&request.destination);
        if request.destination.trim().is_empty() {
            return Err(AppError::Export(errors.no_destination.clone()));
        }
        if destination.exists() && !destination.is_dir() {
            return Err(AppError::Export(fill(
                &errors.destination_not_dir,
                &[("path", &destination.display())],
            )));
        }
        let store = Store::open_read_only(&database)?.with_tenant(session.tenant_id.as_deref());
        let id = store.resolve_snapshot(&request.snapshot_id)?;
        let config = document
            .for_snapshot(&store.get_snapshot(&id)?.tenant_id)
            .map_err(crate::settings::config_error)?;
        let export_kind = request.export_kind.clone();
        let env = ExportEnv {
            destination: &destination,
            store: &store,
            on_event: &on_event,
            labels: &labels,
            config: &config,
            config_dir: document.source.as_deref().and_then(Path::parent),
            cancelled: &cancelled,
        };
        let outcome = match export_kind.as_str() {
            "reports" => export_reports(request, &env)?,
            "diagrams" => export_diagrams(request, &env)?,
            other => {
                return Err(AppError::Export(fill(
                    &errors.unsupported_kind,
                    &[("kind", &other)],
                )));
            }
        };
        let mut outputs = outcome.outputs;
        outputs.sort();
        outputs.dedup();
        let output_count = outputs.len();
        let _ = on_event.send(if outcome.cancelled {
            ExportEvent::Cancelled { output_count }
        } else {
            ExportEvent::Complete { output_count }
        });
        Ok(ExportResultDto {
            destination: destination.display().to_string(),
            outputs: outputs
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
            cancelled: outcome.cancelled,
        })
    })
    .await
    .map_err(|error| AppError::Export(error.to_string()))?;

    if let Err(error) = &result {
        let _ = failure_channel.send(ExportEvent::Failed {
            message: error.to_string(),
        });
    }
    result
}

#[cfg(test)]
mod export_tests {
    use super::*;

    #[test]
    fn unit_parses_the_names_clap_accepts_when_reading_a_request() {
        assert_eq!(report_format("md").unwrap(), ReportFormat::Md);
        assert_eq!(report_format("DOCX").unwrap(), ReportFormat::Docx);
        assert_eq!(
            diagram_type("resource-groups").unwrap(),
            DiagramType::ResourceGroups
        );
        assert_eq!(diagram_format("mermaid").unwrap(), DiagramFormat::Mermaid);
    }

    #[test]
    fn unit_rejects_the_set_aliases_when_reading_a_request() {
        // clap accepts these on the command line as "every format"; the
        // desktop sends an explicit list, so honouring them here would widen
        // the export beyond what the user ticked.
        assert!(report_format("all").is_err());
        assert!(diagram_format("both").is_err());
        assert!(diagram_format("all").is_err());
    }

    #[test]
    fn unit_rejects_an_unknown_value_when_reading_a_request() {
        assert!(report_format("pptx").is_err());
        assert!(diagram_type("galaxy").is_err());
        assert!(diagram_format("bmp").is_err());
    }

    #[test]
    fn unit_gives_the_same_reason_the_cli_does_for_a_mermaid_workbook() {
        // Shared with `azdocs diagram --type workbook --format mermaid`, so the
        // two surfaces cannot explain the same restriction differently.
        let reason = DiagramType::Workbook
            .unsupported_reason(DiagramFormat::Mermaid)
            .expect("a Mermaid workbook is rejected");
        assert!(reason.contains("no sheet"));
        assert!(
            DiagramType::Workbook
                .unsupported_reason(DiagramFormat::Drawio)
                .is_none()
        );
        assert!(
            DiagramType::Network
                .unsupported_reason(DiagramFormat::Mermaid)
                .is_none()
        );
    }

    #[test]
    fn unit_names_the_mermaid_extension_mmd_not_mermaid() {
        // The command-line name and the file extension differ for exactly one
        // format, which is why `extension()` cannot come from clap.
        assert_eq!(DiagramFormat::Mermaid.extension(), Some("mmd"));
        assert_eq!(DiagramFormat::Both.extension(), None);
        assert_eq!(DiagramType::ResourceGroups.slug(), "resource-groups");
    }

    #[test]
    fn unit_single_diagram_targets_a_named_file_in_the_destination() {
        let path = azdocs::commands::diagram::default_output_path(
            Path::new("exports"),
            DiagramType::Network,
            DiagramFormat::Svg,
        );

        assert_eq!(path, Path::new("exports/azdocs-network.svg"));
    }

    #[test]
    fn unit_fan_out_diagrams_target_their_own_directory() {
        let path = azdocs::commands::diagram::default_output_path(
            Path::new("exports"),
            DiagramType::ResourceGroups,
            DiagramFormat::Png,
        );

        assert_eq!(path, Path::new("exports/diagrams/resource-groups"));
    }

    #[test]
    fn unit_workbook_targets_a_file_for_drawio_and_a_directory_for_rasters() {
        assert_eq!(
            azdocs::commands::diagram::default_output_path(
                Path::new("exports"),
                DiagramType::Workbook,
                DiagramFormat::Drawio,
            ),
            Path::new("exports/azdocs-workbook.drawio")
        );
        assert_eq!(
            azdocs::commands::diagram::default_output_path(
                Path::new("exports"),
                DiagramType::Workbook,
                DiagramFormat::Svg,
            ),
            Path::new("exports/diagrams/workbook")
        );
    }
}
