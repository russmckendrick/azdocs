use std::sync::{Arc, Mutex};
use tauri::WebviewWindow;
use tokio::sync::oneshot;

pub type NavigationError = Arc<Mutex<Option<String>>>;
pub type ImageSender = oneshot::Sender<Result<Vec<u8>, String>>;

#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod platform;
#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod platform;
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod platform;

pub fn configure(window: &WebviewWindow, error: NavigationError) -> Result<(), String> {
    let label = window.label().to_owned();
    window
        .with_webview(move |view| platform::configure(view, error, label))
        .map_err(|e| e.to_string())
}

pub async fn screenshot(window: &WebviewWindow) -> Result<Vec<u8>, String> {
    let (tx, rx) = oneshot::channel();
    window
        .with_webview(move |view| platform::screenshot(view, tx))
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}

pub fn cleanup(label: &str) {
    platform::cleanup(label);
}

#[cfg(target_os = "macos")]
pub async fn evaluate(window: &WebviewWindow, js: &str) -> Result<serde_json::Value, String> {
    let (tx, rx) = oneshot::channel();
    let js = js.to_owned();
    window
        .with_webview(move |view| platform::evaluate(view, &js, tx))
        .map_err(|e| e.to_string())?;
    let result = rx
        .await
        .map_err(|e| format!("WebKit evaluation callback closed: {e}"))??;
    serde_json::from_str(&result).map_err(|e| e.to_string())
}

pub fn renderer() -> &'static str {
    platform::RENDERER
}

fn failure(detail: impl Into<String>) -> String {
    super::CaptureError::Native {
        renderer: renderer(),
        detail: detail.into(),
    }
    .to_string()
}
