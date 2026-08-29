pub mod azure_types;
pub mod azure_values;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub id: String,
    pub created_at: DateTime<Utc>,
    pub tenant_id: String,
    pub tool_version: String,
    pub status: SnapshotStatus,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotStatus {
    Running,
    Complete,
    Partial,
    Failed,
}

impl SnapshotStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Failed => "failed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "running" => Some(Self::Running),
            "complete" => Some(Self::Complete),
            "partial" => Some(Self::Partial),
            "failed" => Some(Self::Failed),
            _ => None,
        }
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

#[derive(Debug, Clone, PartialEq)]
pub struct QueryRun {
    pub query_name: String,
    pub category: String,
    pub row_count: Option<u64>,
    pub duration_ms: Option<u64>,
    pub error: Option<String>,
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
