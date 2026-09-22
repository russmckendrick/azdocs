use crate::{AppState, commands::database_path, dto::*, error::AppError};
use base64::Engine as _;
use tauri::{State, ipc::Channel};
use tauri_plugin_dialog::DialogExt;

#[tauri::command]
pub async fn website_state(
    snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<WebsiteState, AppError> {
    let session = crate::settings::session(&state)?;
    crate::commands::blocking(session, move |session| {
        let store = crate::commands::open_store_for(&session)?;
        let id = store.resolve_snapshot(&snapshot_id)?;
        Ok(WebsiteState {
            endpoints: store
                .website_endpoints(&id)?
                .into_iter()
                .map(|e| WebsiteEndpointDto {
                    status: e.status.as_str().into(),
                    resource_id: e.resource_id,
                    resource_name: e.resource_name,
                    source: e.source,
                    hostname: e.hostname,
                    url: e.url,
                })
                .collect(),
            captures: store
                .website_captures(&id, false)?
                .into_iter()
                .map(|c| WebsiteCaptureDto {
                    status: c.status.as_str().into(),
                    url: c.url,
                    final_url: c.final_url,
                    captured_at: c.captured_at,
                    attempted_at: c.attempted_at,
                    error: c.error,
                    renderer: c.renderer,
                    width: c.width,
                    height: c.height,
                })
                .collect(),
            evidence_errors: store
                .website_evidence(&id)?
                .into_iter()
                .filter_map(|e| e.error.map(|error| format!("{}: {error}", e.resource_id)))
                .collect(),
        })
    })
    .await
}

#[tauri::command]
pub async fn website_image(
    snapshot_id: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, AppError> {
    let session = crate::settings::session(&state)?;
    crate::commands::blocking(session, move |session| {
        let store = crate::commands::open_store_for(&session)?;
        let id = store.resolve_snapshot(&snapshot_id)?;
        Ok(store.website_png(&id, &url)?.map(|bytes| {
            format!(
                "data:image/png;base64,{}",
                base64::engine::general_purpose::STANDARD.encode(bytes)
            )
        }))
    })
    .await
}

#[tauri::command]
pub async fn capture_websites(
    app: tauri::AppHandle,
    request: WebsiteCaptureRequest,
    on_event: Channel<WebsiteProgress>,
    state: State<'_, AppState>,
) -> Result<WebsiteBatchResult, AppError> {
    let lease = state.captures.begin()?;
    let path = database_path(&state)?;
    crate::commands::open_store(&state)?.resolve_snapshot(&request.snapshot_id)?;
    crate::capture::run(&app, &path, request, &lease, |progress| {
        let _ = on_event.send(progress);
    })
    .await
}

#[tauri::command]
pub fn cancel_website_capture(state: State<'_, AppState>) {
    state.captures.cancel();
}

#[tauri::command]
pub async fn save_website_image(
    app: tauri::AppHandle,
    snapshot_id: String,
    url: String,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    let bytes = {
        let store = crate::commands::open_store(&state)?;
        let id = store.resolve_snapshot(&snapshot_id)?;
        store.website_png(&id, &url)?.ok_or_else(|| {
            AppError::Capture(crate::capture::CaptureError::NoSavedImage.to_string())
        })?
    };
    let title = crate::settings::session(&state)?
        .labels
        .common
        .websites
        .save
        .clone();
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .file()
        .set_title(title)
        .set_file_name("website.png")
        .add_filter("PNG", &["png"])
        .save_file(move |path| {
            let _ = tx.send(path);
        });
    let Some(path) = rx.await.map_err(|e| AppError::Capture(e.to_string()))? else {
        return Ok(false);
    };
    let path = path
        .into_path()
        .map_err(|e| AppError::Capture(e.to_string()))?;
    std::fs::write(path, bytes).map_err(|e| AppError::Capture(e.to_string()))?;
    Ok(true)
}
