// Codegen for desktop/src/generated.ts; only ever exercised by `cargo test`.
#[cfg(test)]
mod bindings;
pub mod capture;
mod commands;
mod dto;
mod error;
mod groups;
mod labels;
mod settings;
pub mod topology;
mod websites;
pub use dto::WebsiteCaptureRequest;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use azdocs::config::Config;
use tauri::Manager as _;

use crate::labels::AppLabels;

pub struct AppState {
    session: RwLock<settings::Session>,
    pending_secrets: std::sync::Mutex<std::collections::BTreeMap<String, settings::PendingSecret>>,
    checks: Arc<RwLock<std::collections::BTreeMap<String, settings::SavedCheck>>>,
    captures: capture::BatchControl,
    exports: capture::BatchControl,
    /// Destinations this session has exported to; the reveal and open-folder
    /// commands refuse any path outside them.
    export_roots: std::sync::Mutex<std::collections::BTreeSet<PathBuf>>,
}

impl AppState {
    fn new(database_path: PathBuf, labels: Arc<AppLabels>) -> Self {
        Self {
            session: RwLock::new(settings::Session::initial(database_path, labels)),
            pending_secrets: std::sync::Mutex::new(std::collections::BTreeMap::new()),
            checks: Arc::new(RwLock::new(std::collections::BTreeMap::new())),
            captures: capture::BatchControl::default(),
            exports: capture::BatchControl::default(),
            export_roots: std::sync::Mutex::new(std::collections::BTreeSet::new()),
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
        // First, so a second launch is handed to this one before anything
        // else initialises: the existing window is focused and the new
        // process exits.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
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
                    settings::settings_load,
                    settings::settings_load_config,
                    settings::select_tenant,
                    settings::settings_save,
                    settings::settings_migrate,
                    settings::settings_test,
                    settings::settings_export,
                    settings::settings_discard_secrets,
                    commands::open_database,
                    commands::load_snapshot,
                    commands::topology_graph,
                    commands::compare_snapshots,
                    commands::collect_snapshot,
                    commands::cancel_collect,
                    commands::query_pack_metadata,
                    commands::query_rows,
                    commands::resource_query_rows,
                    commands::resource_detail,
                    commands::export_snapshot,
                    commands::cancel_export,
                    commands::copy_text,
                    commands::save_text_file,
                    commands::reveal_export_path,
                    commands::open_export_folder,
                    commands::open_docs,
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
