// Codegen for desktop/src/generated.ts; only ever exercised by `cargo test`.
#[cfg(test)]
mod bindings;
pub mod capture;
mod commands;
mod dto;
mod error;
mod groups;
mod labels;
pub mod topology;
mod websites;
pub use dto::WebsiteCaptureRequest;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use azdocs::config::Config;
use tauri::Manager as _;

use crate::labels::AppLabels;

pub struct AppState {
    database_path: RwLock<PathBuf>,
    captures: capture::CaptureControl,
    /// Resolved once at startup; editing an override file needs a restart.
    labels: Arc<AppLabels>,
}

impl AppState {
    fn new(database_path: PathBuf, labels: Arc<AppLabels>) -> Self {
        Self {
            database_path: RwLock::new(database_path),
            captures: capture::CaptureControl::default(),
            labels,
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let config = match Config::load(None) {
        Ok(config) => Some(config),
        Err(error) => {
            eprintln!("azdocs configuration could not be loaded: {error}");
            None
        }
    };
    let database_path = config
        .as_ref()
        .map(|config| config.storage.db_path.clone())
        .unwrap_or_else(azdocs::config::default_db_path);
    if let Ok(store) = azdocs::store::Store::open(&database_path) {
        let _ = store.recover_website_captures();
    }
    let labels = AppLabels::load(config.as_ref());
    let window_title = labels.desktop.app.window_title.clone();

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new(database_path, labels))
        .setup(move |app| {
            // tauri.conf.json carries the pre-render title; the labels file
            // owns the words, so the window is renamed once they are known.
            if let Some(window) = app.get_webview_window("main") {
                window.set_title(&window_title)?;
            }
            Ok(())
        })
        .invoke_handler({
            let handler: Box<dyn Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync> =
                Box::new(tauri::generate_handler![
                    commands::bootstrap,
                    commands::open_database,
                    commands::load_snapshot,
                    commands::topology_graph,
                    commands::compare_snapshots,
                    commands::collect_snapshot,
                    commands::query_pack_metadata,
                    commands::query_rows,
                    commands::export_snapshot,
                    websites::website_state,
                    websites::website_image,
                    websites::capture_websites,
                    websites::cancel_website_capture,
                    websites::save_website_image,
                ]);
            move |invoke: tauri::ipc::Invoke<tauri::Wry>| {
                if invoke.message.webview().label() != "main" {
                    invoke.resolver.reject("Untrusted capture webview");
                    return true;
                }
                handler(invoke)
            }
        })
        .run(tauri::generate_context!());

    if let Err(error) = app {
        eprintln!("azdocs desktop failed to start: {error}");
        std::process::exit(1);
    }
}
