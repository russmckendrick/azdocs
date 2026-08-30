use std::path::{Path, PathBuf};
use std::sync::Arc;

use azdocs::arg::ArgClient;
use azdocs::cli::ArgValue;
use azdocs::cli::{DiagramArgs, DiagramFormat, DiagramType, ReportArgs, ReportFormat};
use azdocs::collect::CollectRequest;
use azdocs::config::{Config, default_config_path};
use azdocs::querypack::{QueryKind, QueryPack};
use azdocs::report::ReportContext;
use azdocs::report::theme::ThemePack;
use azdocs::store::Store;
use tauri::State;
use tauri::ipc::Channel;

use crate::AppState;
use crate::dto::{
    AppBootstrap, CollectRequestDto, CollectResultDto, CollectionEvent, EstateSnapshot,
    ExportEvent, ExportRequestDto, ExportResultDto, QueryDefDto, QueryRowsDto, SnapshotComparison,
    SnapshotSummary,
};
use crate::error::AppError;
use crate::topology::{self, TopologyGraphDto, TopologyRequest};

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
    let theme_pack = match ThemePack::load() {
        Ok(pack) => pack,
        Err(_) => ThemePack::builtin().map_err(|error| AppError::Config(error.to_string()))?,
    };
    let mut report_themes: Vec<String> =
        theme_pack.names().into_iter().map(str::to_owned).collect();
    if !report_themes.contains(&config.branding.theme) {
        report_themes.push(config.branding.theme.clone());
        report_themes.sort();
    }
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
        required_tags: config.audit.required_tags.clone(),
        report_theme: config.branding.theme,
        report_themes,
        snapshots,
        latest_snapshot_id,
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
        })
        .collect())
}

#[tauri::command]
pub fn query_rows(
    snapshot_id: Option<String>,
    query_name: String,
    state: State<'_, AppState>,
) -> Result<QueryRowsDto, AppError> {
    let store = Store::open(&database_path(&state)?)?;
    let snapshot_id = store.resolve_snapshot(snapshot_id.as_deref().unwrap_or("latest"))?;
    let rows = store.query_results(&snapshot_id, &query_name)?;
    Ok(QueryRowsDto {
        query_name,
        columns: azdocs::model::rows::columns(&rows),
        rows,
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
pub fn topology_graph(
    request: TopologyRequest,
    state: State<'_, AppState>,
) -> Result<TopologyGraphDto, AppError> {
    let store = Store::open(&database_path(&state)?)?;
    let snapshot_id = store.resolve_snapshot(request.snapshot_id.as_deref().unwrap_or("latest"))?;
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
        },
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

fn export_reports(
    request: ExportRequestDto,
    destination: &Path,
    store: &Store,
    on_event: &Channel<ExportEvent>,
) -> Result<Vec<PathBuf>, AppError> {
    let mut formats = Vec::new();
    for value in &request.formats {
        let format = report_format(value)?;
        if !formats.contains(&format) {
            formats.push(format);
        }
    }
    if formats.is_empty() {
        return Err(AppError::Export(
            "select at least one report format".to_owned(),
        ));
    }

    let _ = on_event.send(ExportEvent::Phase {
        message: format!("Composing {} report format(s)", formats.len()),
    });
    let (config, source) =
        Config::load_with_source(None).map_err(|error| AppError::Config(error.to_string()))?;
    let config_dir = source.as_deref().and_then(Path::parent);
    let args = ReportArgs {
        snapshot: request.snapshot_id,
        format: formats[0],
        theme: request.theme,
        out: Some(destination.to_path_buf()),
    };
    azdocs::commands::report::run_selected_with_outputs(&config, config_dir, store, &args, &formats)
        .map_err(|error| AppError::Export(error.to_string()))
}

fn export_diagrams(
    request: ExportRequestDto,
    destination: &Path,
    store: &Store,
    on_event: &Channel<ExportEvent>,
) -> Result<Vec<PathBuf>, AppError> {
    let kind = request
        .diagram_type
        .as_deref()
        .ok_or_else(|| AppError::Export("select a diagram type".to_owned()))
        .and_then(diagram_type)?;
    let mut formats = Vec::new();
    for value in &request.formats {
        let format = diagram_format(value)?;
        if !formats.contains(&format) {
            formats.push(format);
        }
    }
    if formats.is_empty() {
        return Err(AppError::Export(
            "select at least one diagram format".to_owned(),
        ));
    }
    if let Some(reason) = formats
        .iter()
        .find_map(|format| kind.unsupported_reason(*format))
    {
        return Err(AppError::Export(reason.to_owned()));
    }

    let mut outputs = Vec::new();
    for format in formats {
        let _ = on_event.send(ExportEvent::Phase {
            message: format!(
                "Rendering {} as {}",
                kind.slug(),
                format.extension().unwrap_or("out")
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
            azdocs::commands::diagram::run_with_outputs(store, &args)
                .map_err(|error| AppError::Export(error.to_string()))?,
        );
    }
    Ok(outputs)
}

#[tauri::command]
pub async fn export_snapshot(
    request: ExportRequestDto,
    on_event: Channel<ExportEvent>,
    state: State<'_, AppState>,
) -> Result<ExportResultDto, AppError> {
    let database = database_path(&state)?;
    let failure_channel = on_event.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let destination = PathBuf::from(&request.destination);
        if request.destination.trim().is_empty() {
            return Err(AppError::Export("choose an output directory".to_owned()));
        }
        if destination.exists() && !destination.is_dir() {
            return Err(AppError::Export(format!(
                "output destination is not a directory: {}",
                destination.display()
            )));
        }
        let store = Store::open(&database)?;
        store.resolve_snapshot(&request.snapshot_id)?;
        let export_kind = request.export_kind.clone();
        let mut outputs = match export_kind.as_str() {
            "reports" => export_reports(request, &destination, &store, &on_event)?,
            "diagrams" => export_diagrams(request, &destination, &store, &on_event)?,
            other => {
                return Err(AppError::Export(format!(
                    "unsupported export kind `{other}`"
                )));
            }
        };
        outputs.sort();
        outputs.dedup();
        let output_count = outputs.len();
        let _ = on_event.send(ExportEvent::Complete { output_count });
        Ok(ExportResultDto {
            destination: destination.display().to_string(),
            outputs: outputs
                .into_iter()
                .map(|path| path.display().to_string())
                .collect(),
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
