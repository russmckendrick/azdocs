//! The collection contract travels with the snapshot, not today's query pack.

use serde::{Deserialize, Serialize};

use super::Severity;

/// ARG scope semantics, including assignments inherited from management groups.
/// https://learn.microsoft.com/azure/governance/resource-graph/concepts/query-language#query-scope
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum AuthorizationScope {
    #[default]
    AtScopeAndBelow,
    AtScopeAndAbove,
    AtScopeAboveAndBelow,
    AtScopeExact,
}

impl AuthorizationScope {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AtScopeAndBelow => "AtScopeAndBelow",
            Self::AtScopeAndAbove => "AtScopeAndAbove",
            Self::AtScopeAboveAndBelow => "AtScopeAboveAndBelow",
            Self::AtScopeExact => "AtScopeExact",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QuerySource {
    pub urls: Vec<String>,
    /// Upstream commit/tag when known; a review date is not an upstream version.
    pub revision: Option<String>,
    pub reviewed_on: String,
}

/// A review threshold, not a provider retention period or a backup RPO promise.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceFreshness {
    pub timestamp_field: String,
    pub max_age_hours: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryProvenance {
    pub kql: String,
    pub kql_sha256: String,
    pub category: String,
    pub description: String,
    pub kind: String,
    /// Finding interpretation is versioned alongside the text that produced it.
    pub severity: Option<Severity>,
    pub title_field: Option<String>,
    pub resource_id_field: Option<String>,
    pub source: Option<QuerySource>,
    pub freshness: Option<EvidenceFreshness>,
    pub authorization_scope: AuthorizationScope,
    /// Empty means the ARG request used all subscriptions visible to the credential.
    pub subscriptions: Vec<String>,
}
