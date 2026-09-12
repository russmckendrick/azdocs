//! Serial native webview capture; no browser driver or Azure token enters here.
mod error;
mod native;
pub(crate) use error::CaptureError;

use std::collections::BTreeSet;
use std::io::Cursor;
use std::path::Path;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::Duration;

use azdocs::model::websites::{CaptureStatus, CapturedWebsite, EndpointStatus};
use azdocs::store::Store;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::dto::{WebsiteBatchResult, WebsiteCaptureRequest, WebsiteProgress};
use crate::error::AppError;

#[derive(Default)]
pub struct CaptureControl(Mutex<Option<Arc<AtomicBool>>>);

pub struct BatchLease<'a> {
    control: &'a CaptureControl,
    cancel: Arc<AtomicBool>,
}

impl CaptureControl {
    pub fn begin(&self) -> Result<BatchLease<'_>, AppError> {
        let mut active = self.0.lock().map_err(|e| AppError::State(e.to_string()))?;
        if active.is_some() {
            return Err(AppError::Capture(CaptureError::Busy.to_string()));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        *active = Some(Arc::clone(&cancel));
        Ok(BatchLease {
            control: self,
            cancel,
        })
    }
    pub fn cancel(&self) {
        if let Ok(active) = self.0.lock()
            && let Some(cancel) = active.as_ref()
        {
            cancel.store(true, Ordering::Relaxed);
        }
    }
    pub fn is_active(&self) -> bool {
        self.0.lock().map(|v| v.is_some()).unwrap_or(true)
    }
}

impl Drop for BatchLease<'_> {
    fn drop(&mut self) {
        if let Ok(mut active) = self.control.0.lock() {
            *active = None;
        }
    }
}

pub fn allowed_navigation(url: &str) -> bool {
    tauri::Url::parse(url).is_ok_and(|u| {
        matches!(u.scheme(), "http" | "https") && u.username().is_empty() && u.password().is_none()
    })
}

struct CaptureView(WebviewWindow);
impl Drop for CaptureView {
    fn drop(&mut self) {
        let _ = self.0.destroy();
    }
}

async fn cancelled(flag: &AtomicBool) {
    while !flag.load(Ordering::Relaxed) {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

#[cfg(not(target_os = "macos"))]
async fn evaluate(window: &WebviewWindow, js: &str) -> Result<serde_json::Value, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let tx = Mutex::new(Some(tx));
    window
        .eval_with_callback(js, move |result| {
            if let Ok(mut tx) = tx.lock()
                && let Some(tx) = tx.take()
            {
                let _ = tx.send(result);
            }
        })
        .map_err(|e| e.to_string())?;
    serde_json::from_str(&rx.await.map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
async fn evaluate(window: &WebviewWindow, js: &str) -> Result<serde_json::Value, String> {
    native::evaluate(window, js).await
}

async fn capture_page(
    app: &AppHandle,
    label: &str,
    url: &str,
    title: &str,
) -> Result<CapturedWebsite, String> {
    let parsed = tauri::Url::parse(url).map_err(|e| e.to_string())?;
    if !allowed_navigation(url) {
        return Err(CaptureError::UnsupportedUrl.to_string());
    }
    // Start blank so native policies are installed before any remote code runs.
    let window = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::External(tauri::Url::parse("about:blank").map_err(|e| e.to_string())?),
    )
    .title(title)
    .inner_size(1440.0, 900.0)
    .resizable(false)
    .focused(false)
    .visible(false)
    .skip_taskbar(true)
    .incognito(true)
    .on_navigation(|url| allowed_navigation(url.as_str()))
    .on_new_window(|_, _| tauri::webview::NewWindowResponse::Deny)
    .on_download(|_, _| false)
    .initialization_script_for_all_frames(include_str!("isolation.js"))
    .build()
    .map_err(|e| e.to_string())?;
    let cleanup_label = label.to_owned();
    window.on_window_event(move |event| {
        if matches!(event, tauri::WindowEvent::Destroyed) {
            native::cleanup(&cleanup_label);
        }
    });
    let view = CaptureView(window);
    let error = Arc::new(Mutex::new(None));
    native::configure(&view.0, Arc::clone(&error))?;
    // WebView2/WebKitGTK need a mapped view; never request keyboard focus.
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    view.0.show().map_err(|e| e.to_string())?;
    view.0.navigate(parsed).map_err(|e| e.to_string())?;
    loop {
        if let Some(error) = error.lock().map_err(|e| e.to_string())?.clone() {
            return Err(error);
        }
        let ready = evaluate(&view.0, "({ready:document.readyState==='complete'&&(!document.fonts||document.fonts.status==='loaded'),url:location.href})").await?;
        if ready["ready"] == true && ready["url"].as_str().is_some_and(allowed_navigation) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    tokio::time::sleep(Duration::from_secs(2)).await;
    if let Some(error) = error.lock().map_err(|e| e.to_string())?.clone() {
        return Err(error);
    }
    let final_url = view.0.url().map_err(|e| e.to_string())?.to_string();
    let bytes = native::screenshot(&view.0).await?;
    let image = image::load_from_memory(&bytes).map_err(|e| e.to_string())?;
    if image.width() == 0 || image.height() == 0 {
        return Err(CaptureError::EmptyImage.to_string());
    }
    let image = image.resize_exact(1440, 900, image::imageops::FilterType::Lanczos3);
    let mut png = Cursor::new(Vec::new());
    image
        .write_to(&mut png, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(CapturedWebsite {
        final_url,
        captured_at: chrono::Utc::now().to_rfc3339(),
        renderer: native::renderer().into(),
        png: png.into_inner(),
    })
}

pub async fn run(
    app: &AppHandle,
    path: &Path,
    request: WebsiteCaptureRequest,
    lease: &BatchLease<'_>,
    title: &str,
    on_progress: impl Fn(WebsiteProgress),
) -> Result<WebsiteBatchResult, AppError> {
    let (snapshot, urls, skipped) = {
        let store = Store::open(path)?;
        let snapshot = store.resolve_snapshot(&request.snapshot_id)?;
        let endpoints = store.website_endpoints(&snapshot)?;
        let prior = store.website_captures(&snapshot, false)?;
        let selected: BTreeSet<_> = request.urls.iter().map(String::as_str).collect();
        let ready: BTreeSet<_> = endpoints
            .iter()
            .filter(|e| e.status == EndpointStatus::Ready)
            .filter_map(|e| e.url.clone())
            .collect();
        if selected.iter().any(|url| !ready.contains(*url)) {
            return Err(AppError::Capture(CaptureError::UndiscoveredUrl.to_string()));
        }
        let urls: Vec<_> = ready
            .into_iter()
            .filter(|url| selected.is_empty() || selected.contains(url.as_str()))
            .filter(|url| {
                !request.retry_only
                    || !prior
                        .iter()
                        .any(|c| &c.url == url && c.status == CaptureStatus::Captured)
            })
            .collect();
        let skipped = if selected.is_empty() {
            endpoints
                .iter()
                .filter(|e| e.status != EndpointStatus::Ready)
                .count()
        } else {
            0
        };
        (snapshot, urls, skipped)
    };
    let mut result = WebsiteBatchResult {
        captured: 0,
        failed: 0,
        skipped,
        cancelled: false,
        error: None,
    };
    for (index, url) in urls.iter().enumerate() {
        if lease.cancel.load(Ordering::Relaxed) {
            result.cancelled = true;
            break;
        }
        Store::open(path)?.record_website_attempt(&snapshot, url, CaptureStatus::Running, None)?;
        on_progress(WebsiteProgress {
            completed: index,
            total: urls.len(),
            url: Some(url.clone()),
            captured: result.captured,
            failed: result.failed,
            cancelled: false,
        });
        // Destruction is asynchronous in Tauri. Unique identities also keep late
        // native callbacks from a cancelled page separate from the next page.
        static NEXT_VIEW: AtomicU64 = AtomicU64::new(1);
        let label = format!(
            "website-capture-{}",
            NEXT_VIEW.fetch_add(1, Ordering::Relaxed)
        );
        let outcome = tokio::select! {
            _ = cancelled(&lease.cancel) => { result.cancelled=true; Err(CaptureError::Cancelled.to_string()) }
            outcome = tokio::time::timeout(Duration::from_secs(30),capture_page(app,&label,url,title)) => outcome.unwrap_or_else(|_| Err(CaptureError::Timeout.to_string())),
        };
        // CaptureView's drop queues destruction even when timeout/select drops
        // the future. Wait for the manager to remove it before the next URL.
        let cleanup = tokio::time::timeout(Duration::from_secs(5), async {
            while app.get_webview_window(&label).is_some() {
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await;
        let store = Store::open(path)?;
        match outcome {
            Ok(image) => {
                store.save_website_capture(&snapshot, url, &image)?;
                result.captured += 1;
            }
            Err(error) => {
                store.record_website_attempt(
                    &snapshot,
                    url,
                    if result.cancelled {
                        CaptureStatus::Cancelled
                    } else {
                        CaptureStatus::Failed
                    },
                    Some(&error),
                )?;
                if !result.cancelled {
                    result.failed += 1;
                }
            }
        }
        if cleanup.is_err() {
            result.error = Some(CaptureError::WindowCleanup.to_string());
            break;
        }
        if result.cancelled {
            break;
        }
    }
    on_progress(WebsiteProgress {
        completed: result.captured + result.failed,
        total: urls.len(),
        url: None,
        captured: result.captured,
        failed: result.failed,
        cancelled: result.cancelled,
    });
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capture_lease_serializes_batches_and_cancels_without_poisoning_next_run() {
        let control = CaptureControl::default();
        let first = control.begin().unwrap();
        assert!(control.begin().is_err());
        control.cancel();
        assert!(first.cancel.load(Ordering::Relaxed));
        drop(first);
        assert!(!control.begin().unwrap().cancel.load(Ordering::Relaxed));
    }
    #[test]
    fn navigation_allows_http_sites_but_rejects_local_files_and_credentials() {
        assert!(allowed_navigation("https://internal.example/"));
        for url in [
            "file:///tmp/private",
            "tauri://localhost",
            "javascript:alert(1)",
            "https://user:secret@example.test/",
        ] {
            assert!(!allowed_navigation(url));
        }
    }
}
