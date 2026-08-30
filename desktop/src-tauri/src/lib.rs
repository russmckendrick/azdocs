// Codegen for desktop/src/generated.ts; only ever exercised by `cargo test`.
#[cfg(test)]
mod bindings;
mod commands;
mod dto;
mod error;
mod groups;
pub mod topology;

use std::path::PathBuf;
use std::sync::RwLock;

use azdocs::config::Config;

pub struct AppState {
    database_path: RwLock<PathBuf>,
}

impl AppState {
    fn new(database_path: PathBuf) -> Self {
        Self {
            database_path: RwLock::new(database_path),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let database_path = match Config::load(None) {
        Ok(config) => config.storage.db_path,
        Err(error) => {
            eprintln!("azdocs configuration could not be loaded: {error}");
            azdocs::config::default_db_path()
        }
    };

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::new(database_path))
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::open_database,
            commands::load_snapshot,
            commands::topology_graph,
            commands::compare_snapshots,
            commands::collect_snapshot,
            commands::query_pack_metadata,
            commands::query_rows,
            commands::export_snapshot,
        ])
        .run(tauri::generate_context!());

    if let Err(error) = app {
        eprintln!("azdocs desktop failed to start: {error}");
        std::process::exit(1);
    }
}
