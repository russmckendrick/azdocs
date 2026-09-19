pub mod azure_types;
pub mod azure_values;
pub mod diff;
mod evidence;
pub use evidence::{AuthorizationScope, EvidenceFreshness, QueryProvenance, QuerySource};
pub mod network;
pub mod rows;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Snapshot {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub tenant_id: String,
    pub tool_version: String,
    pub status: SnapshotStatus,
    pub notes: Option<String>,
    /// Last time a running collector proved it was alive; None before the
    /// column existed or once the run finished.
    pub heartbeat_at: Option<DateTime<Utc>>,
    /// Set when a stale `running` row was reconciled to `failed` at open.
    pub interrupted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SnapshotStatus {
    Running,
    Complete,
    Partial,
    Failed,
    /// Stopped on request before every query ran. Real but incomplete
    /// evidence: selectable by id, never as the implicit `latest`.
    Cancelled,
}

impl SnapshotStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "running" => Some(Self::Running),
            "complete" => Some(Self::Complete),
            "partial" => Some(Self::Partial),
            "failed" => Some(Self::Failed),
            "cancelled" => Some(Self::Cancelled),
            _ => None,
        }
    }

    /// Whether a snapshot in this state is a usable, finished collection.
    /// Only these resolve as the implicit `latest` or as a diff baseline.
    pub fn is_usable(self) -> bool {
        matches!(self, Self::Complete | Self::Partial)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Subscription {
    pub subscription_id: String,
    pub display_name: String,
    pub state: Option<String>,
    pub tags: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceGroup {
    /// Lowercased full ARM id.
    pub id: String,
    pub name: String,
    pub subscription_id: String,
    pub location: Option<String>,
    pub tags: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Resource {
    /// Lowercased full ARM id — the join key everywhere.
    pub id: String,
    /// Original casing for display.
    pub display_id: String,
    pub name: String,
    /// Lowercased, e.g. `microsoft.network/virtualnetworks`.
    pub azure_type: String,
    pub kind: Option<String>,
    pub location: Option<String>,
    /// Lowercased resource group name.
    pub resource_group: Option<String>,
    pub subscription_id: String,
    pub tags: Option<Value>,
    pub sku: Option<Value>,
    pub identity: Option<Value>,
    pub properties: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    /// Lowercased ARM id.
    pub source_id: String,
    /// Lowercased ARM id.
    pub target_id: String,
    pub kind: EdgeKind,
    pub properties: Option<Value>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EdgeKind {
    SubnetOf,
    InVnet,
    PeeredWith,
    NsgAttached,
    NicInSubnet,
    AttachedTo,
    PrivateEndpointFor,
    DnsLinked,
    DependsOn,
    RunsOn,
    UsesIdentity,
    LogsTo,
    Monitors,
}

impl Edge {
    /// The endpoint that is not `resource_id`, or `None` when this edge does
    /// not touch that resource.
    ///
    /// Both the report's related list and the TUI's needed this; what they do
    /// with it differs (the TUI keeps direction, the report sorts and dedups),
    /// so only the lookup is shared.
    pub fn other_end(&self, resource_id: &str) -> Option<&str> {
        if self.source_id == resource_id {
            Some(&self.target_id)
        } else if self.target_id == resource_id {
            Some(&self.source_id)
        } else {
            None
        }
    }
}

impl EdgeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SubnetOf => "subnet_of",
            Self::InVnet => "in_vnet",
            Self::PeeredWith => "peered_with",
            Self::NsgAttached => "nsg_attached",
            Self::NicInSubnet => "nic_in_subnet",
            Self::AttachedTo => "attached_to",
            Self::PrivateEndpointFor => "private_endpoint_for",
            Self::DnsLinked => "dns_linked",
            Self::DependsOn => "depends_on",
            Self::RunsOn => "runs_on",
            Self::UsesIdentity => "uses_identity",
            Self::LogsTo => "logs_to",
            Self::Monitors => "monitors",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "subnet_of" => Some(Self::SubnetOf),
            "in_vnet" => Some(Self::InVnet),
            "peered_with" => Some(Self::PeeredWith),
            "nsg_attached" => Some(Self::NsgAttached),
            "nic_in_subnet" => Some(Self::NicInSubnet),
            "attached_to" => Some(Self::AttachedTo),
            "private_endpoint_for" => Some(Self::PrivateEndpointFor),
            "dns_linked" => Some(Self::DnsLinked),
            "depends_on" => Some(Self::DependsOn),
            "runs_on" => Some(Self::RunsOn),
            "uses_identity" => Some(Self::UsesIdentity),
            "logs_to" => Some(Self::LogsTo),
            "monitors" => Some(Self::Monitors),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    High,
    Medium,
    Low,
    Info,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Medium => "medium",
            Self::Low => "low",
            Self::Info => "info",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "high" => Some(Self::High),
            "medium" => Some(Self::Medium),
            "low" => Some(Self::Low),
            "info" => Some(Self::Info),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub query_name: String,
    pub category: String,
    pub severity: Severity,
    /// Lowercased ARM id; None for estate-level findings.
    pub resource_id: Option<String>,
    pub title: String,
    pub detail: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QueryRun {
    /// None on snapshots collected before provenance was recorded.
    pub provenance: Option<QueryProvenance>,
    pub query_name: String,
    pub category: String,
    pub row_count: Option<u64>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
    /// Rows ARG returned that ingest could not shape (missing id, name, type
    /// or subscription). None on snapshots recorded before this was counted.
    pub rows_dropped: Option<u64>,
}

/// Normalize an ARM id for joining: ARM ids are case-insensitive and ARG
/// returns inconsistent casing across queries.
pub fn normalize_arm_id(id: &str) -> String {
    id.to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_kind_round_trips_through_strings() {
        for kind in [
            EdgeKind::SubnetOf,
            EdgeKind::InVnet,
            EdgeKind::PeeredWith,
            EdgeKind::NsgAttached,
            EdgeKind::NicInSubnet,
            EdgeKind::AttachedTo,
            EdgeKind::PrivateEndpointFor,
            EdgeKind::DnsLinked,
            EdgeKind::DependsOn,
            EdgeKind::RunsOn,
            EdgeKind::UsesIdentity,
            EdgeKind::LogsTo,
            EdgeKind::Monitors,
        ] {
            assert_eq!(EdgeKind::parse(kind.as_str()), Some(kind));
        }
    }

    #[test]
    fn severity_orders_high_first() {
        assert!(Severity::High < Severity::Info);
    }
}

/// The last segment of an ARM id — the resource's own name.
///
/// Falls back to the whole id rather than an empty string, so a trailing slash
/// or a value that is not an ARM id still shows the reader something. Mirrors
/// `resourceName` in desktop/src/format.ts.
pub fn short_name(arm_id: &str) -> &str {
    arm_id
        .rsplit('/')
        .find(|segment| !segment.is_empty())
        .unwrap_or(arm_id)
}

/// Filesystem- and anchor-safe slug: ASCII alphanumerics lowercased, every
/// other run collapsed to a single dash, no leading or trailing dash.
///
/// One implementation because the same strings become both a Markdown filename
/// and a diagram slug, and a mismatch would break the link between a report
/// section and its picture. There were two algorithms doing this — verified
/// identical before merging.
pub fn slugify(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            out.push(character.to_ascii_lowercase());
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_owned()
}

/// Shorten to `max` characters, ending in an ellipsis when it had to cut.
///
/// Counts `char`s, not bytes, so a multi-byte name is measured as a reader
/// sees it and never split mid-character.
pub fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_owned();
    }
    let mut out: String = value.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod text_tests {
    use super::{slugify, truncate};

    #[test]
    fn unit_collapses_runs_and_trims_edges_when_slugifying() {
        assert_eq!(slugify("My Production (EU) Sub"), "my-production-eu-sub");
        assert_eq!(slugify("--leading"), "leading");
        assert_eq!(slugify("trailing--"), "trailing");
        assert_eq!(slugify("a  b"), "a-b");
        assert_eq!(slugify("---"), "");
    }

    #[test]
    fn unit_keeps_short_values_intact_when_truncating() {
        assert_eq!(truncate("short", 30), "short");
        assert_eq!(truncate("exactly-ten", 11), "exactly-ten");
    }

    #[test]
    fn unit_counts_characters_not_bytes_when_truncating() {
        // Five chars, ten bytes: it must not cut at byte five.
        assert_eq!(truncate("ünïcö", 5), "ünïcö");
        assert_eq!(truncate("ünïcödé", 5), "ünïc…");
    }
}

#[cfg(test)]
mod edge_tests {
    use super::{Edge, EdgeKind, short_name};

    fn edge(source: &str, target: &str) -> Edge {
        Edge {
            source_id: source.to_owned(),
            target_id: target.to_owned(),
            kind: EdgeKind::AttachedTo,
            properties: None,
        }
    }

    #[test]
    fn unit_returns_the_far_endpoint_when_the_edge_touches_the_resource() {
        let link = edge("/a/nic", "/a/vm");
        assert_eq!(link.other_end("/a/nic"), Some("/a/vm"));
        assert_eq!(link.other_end("/a/vm"), Some("/a/nic"));
    }

    #[test]
    fn unit_returns_nothing_when_the_edge_does_not_touch_the_resource() {
        assert_eq!(edge("/a/nic", "/a/vm").other_end("/a/disk"), None);
    }

    #[test]
    fn unit_takes_the_last_non_empty_segment_when_shortening_an_id() {
        assert_eq!(short_name("/subscriptions/s/providers/x/vm-1"), "vm-1");
        assert_eq!(short_name("/a/b/"), "b", "a trailing slash is skipped");
        assert_eq!(short_name("bare"), "bare");
        assert_eq!(short_name(""), "");
    }
}
pub mod websites;
