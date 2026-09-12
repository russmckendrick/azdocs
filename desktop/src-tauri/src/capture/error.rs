//! Operational errors are distinct from the labels used to present outcomes.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    #[error("A capture or collection is already running")]
    Busy,
    #[error("Unsupported website URL")]
    UnsupportedUrl,
    #[error("URL is not a discovered, enabled endpoint")]
    UndiscoveredUrl,
    #[error("Capture cancelled")]
    Cancelled,
    #[error("Website capture exceeded 30 seconds")]
    Timeout,
    #[error("Capture window did not close")]
    WindowCleanup,
    #[error("Empty native screenshot")]
    EmptyImage,
    #[error("No saved screenshot")]
    NoSavedImage,
    #[error("{renderer}: {detail}")]
    Native {
        renderer: &'static str,
        detail: String,
    },
}
