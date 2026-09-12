//! Snapshot-scoped website evidence. Capture time is independent of audit time.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndpointStatus {
    Ready,
    Disabled,
    Wildcard,
    MissingHostname,
    InvalidHostname,
    MissingEvidence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebsiteEndpoint {
    pub resource_id: String,
    pub resource_name: String,
    pub source: String,
    pub hostname: Option<String>,
    pub url: Option<String>,
    pub status: EndpointStatus,
}

/// Raw management API rows are kept separately from ARG query runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebsiteEvidence {
    pub resource_id: String,
    pub kind: String,
    pub collected_at: String,
    pub rows: Vec<Value>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureStatus {
    Running,
    Captured,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebsiteCapture {
    pub url: String,
    pub final_url: Option<String>,
    pub captured_at: Option<String>,
    pub attempted_at: String,
    pub status: CaptureStatus,
    pub error: Option<String>,
    pub renderer: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(skip)]
    pub png: Vec<u8>,
}

#[derive(Debug)]
pub struct CapturedWebsite {
    pub final_url: String,
    pub captured_at: String,
    pub renderer: String,
    pub png: Vec<u8>,
}

impl EndpointStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Disabled => "disabled",
            Self::Wildcard => "wildcard",
            Self::MissingHostname => "missing_hostname",
            Self::InvalidHostname => "invalid_hostname",
            Self::MissingEvidence => "missing_evidence",
        }
    }
}
impl CaptureStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Captured => "captured",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Interrupted => "interrupted",
        }
    }
}
