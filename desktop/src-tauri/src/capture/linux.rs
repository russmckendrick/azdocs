use super::{ImageSender, NavigationError};
use webkit2gtk::{SnapshotOptions, SnapshotRegion, prelude::*};

pub const RENDERER: &str = "WebKitGTK";

pub fn configure(view: tauri::webview::PlatformWebview, error: NavigationError, _label: String) {
    let webview = view.inner();
    webview.connect_permission_request(|_, request| {
        request.deny();
        true
    });
    webview.connect_authenticate(|_, request| {
        request.cancel();
        true
    });
    let errors = error.clone();
    webview.connect_load_failed(move |_, _, _, failure| {
        if let Ok(mut error) = errors.lock() {
            *error = Some(failure.to_string());
        }
        true
    });
    webview.connect_load_failed_with_tls_errors(move |_, _, _, flags| {
        if let Ok(mut error) = error.lock() {
            *error = Some(super::failure(format!(
                "TLS certificate validation failed: {flags:?}"
            )));
        }
        true
    });
}

pub fn cleanup(_label: &str) {}

pub fn screenshot(view: tauri::webview::PlatformWebview, sender: ImageSender) {
    view.inner().snapshot(
        SnapshotRegion::Visible,
        SnapshotOptions::NONE,
        None::<&webkit2gtk::gio::Cancellable>,
        move |result| {
            let png = result.map_err(|e| e.to_string()).and_then(|surface| {
                let mut bytes = Vec::new();
                surface
                    .write_to_png(&mut bytes)
                    .map_err(|e| e.to_string())?;
                Ok(bytes)
            });
            let _ = sender.send(png);
        },
    );
}
