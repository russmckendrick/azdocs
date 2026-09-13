//! Advisory inspection of active Azure RBAC grants. Never probes a write operation.
use super::TokenProvider;
use crate::arg::ArgClient;
use crate::error::{ArgError, AuthError};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;
use ts_rs::TS;

#[derive(Debug, thiserror::Error)]
pub enum DiagnosticError {
    #[error("{0}")]
    Auth(#[from] AuthError),
    #[error("subscription discovery failed: {0}")]
    Probe(#[from] ArgError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum PermissionVerdict {
    ReadOnly,
    BroaderGrants,
    UnableToVerify,
}

impl PermissionVerdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ReadOnly => "read_only",
            Self::BroaderGrants => "broader_grants",
            Self::UnableToVerify => "unable_to_verify",
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VisibleSubscription {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct PermissionGrant {
    pub scope: String,
    pub role: String,
    pub role_id: String,
    pub verdict: PermissionVerdict,
    pub actions: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AccessIssueKind {
    IdentityUnavailable,
    NoSubscriptions,
    AssignmentReadFailed,
    DefinitionReadFailed,
    ConditionalGrant,
    UnsupportedPermissions,
    NoAssignments,
    InaccessibleSubscription,
}

impl AccessIssueKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::IdentityUnavailable => "identity_unavailable",
            Self::NoSubscriptions => "no_subscriptions",
            Self::AssignmentReadFailed => "assignment_read_failed",
            Self::DefinitionReadFailed => "definition_read_failed",
            Self::ConditionalGrant => "conditional_grant",
            Self::UnsupportedPermissions => "unsupported_permissions",
            Self::NoAssignments => "no_assignments",
            Self::InaccessibleSubscription => "inaccessible_subscription",
        }
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct AccessIssue {
    pub kind: AccessIssueKind,
    pub scope: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionCheck {
    pub checked_at: String,
    pub subscriptions: Vec<VisibleSubscription>,
    pub inaccessible_subscriptions: Vec<String>,
    pub verdict: PermissionVerdict,
    pub grants: Vec<PermissionGrant>,
    pub issues: Vec<AccessIssue>,
}

const SUBSCRIPTIONS: &str = "resourcecontainers | where type =~ 'microsoft.resources/subscriptions' | project id, subscriptionId, name | order by id asc";

pub async fn inspect<P: TokenProvider>(
    http: reqwest::Client,
    provider: &P,
    configured: &[String],
) -> Result<ConnectionCheck, DiagnosticError> {
    inspect_at(http, provider, configured, "https://management.azure.com").await
}

pub async fn inspect_at<P: TokenProvider>(
    http: reqwest::Client,
    provider: &P,
    configured: &[String],
    endpoint: &str,
) -> Result<ConnectionCheck, DiagnosticError> {
    let token = provider.token().await?;
    let principal = token
        .split('.')
        .nth(1)
        .and_then(|part| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(part)
                .ok()
        })
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|claims| claims.get("oid").and_then(Value::as_str).map(str::to_owned))
        .filter(|id| uuid::Uuid::parse_str(id).is_ok());
    let client = ArgClient::with_endpoint(http.clone(), provider, endpoint);
    let outcome = client.query_all(SUBSCRIPTIONS, &[]).await?;
    let discovered_rows = outcome.rows.len();
    let mut subscriptions: Vec<_> = outcome
        .rows
        .iter()
        .filter_map(|row| {
            Some(VisibleSubscription {
                id: row
                    .get("subscriptionId")?
                    .as_str()
                    .filter(|id| uuid::Uuid::parse_str(id).is_ok())?
                    .to_ascii_lowercase(),
                name: row
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .into(),
            })
        })
        .collect();
    let incomplete_discovery = subscriptions.len() != discovered_rows;
    subscriptions.sort_by(|a, b| a.id.cmp(&b.id));
    subscriptions.dedup_by(|a, b| a.id == b.id);
    let mut result = ConnectionCheck {
        checked_at: chrono::Utc::now().to_rfc3339(),
        subscriptions,
        inaccessible_subscriptions: Vec::new(),
        verdict: PermissionVerdict::UnableToVerify,
        grants: Vec::new(),
        issues: Vec::new(),
    };
    if incomplete_discovery {
        result.issues.push(AccessIssue {
            kind: AccessIssueKind::AssignmentReadFailed,
            scope: String::new(),
        });
    }
    for id in configured {
        if !result
            .subscriptions
            .iter()
            .any(|s| s.id.eq_ignore_ascii_case(id))
        {
            result.inaccessible_subscriptions.push(id.clone());
            result.issues.push(AccessIssue {
                kind: AccessIssueKind::InaccessibleSubscription,
                scope: id.clone(),
            });
        }
    }
    if result.subscriptions.is_empty() {
        result.issues.push(AccessIssue {
            kind: AccessIssueKind::NoSubscriptions,
            scope: String::new(),
        });
    }
    let Some(principal) = principal else {
        result.issues.push(AccessIssue {
            kind: AccessIssueKind::IdentityUnavailable,
            scope: String::new(),
        });
        return Ok(result);
    };
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let mut definitions: BTreeMap<String, Value> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for sub in &result.subscriptions {
        let scope = format!("/subscriptions/{}", sub.id);
        let url = format!(
            "{endpoint}{scope}/providers/Microsoft.Authorization/roleAssignments?api-version=2022-04-01&$filter=assignedTo('{principal}')"
        );
        let assignments = match get_pages(&http, &token, &url, endpoint, deadline).await {
            Ok((value, complete)) => {
                if !complete {
                    result.issues.push(AccessIssue {
                        kind: AccessIssueKind::AssignmentReadFailed,
                        scope: scope.clone(),
                    });
                }
                value
            }
            Err(ArmReadError::Authentication) => {
                return Err(AuthError::Rejected {
                    status: 401,
                    detail: "permission inspection authentication failed".into(),
                }
                .into());
            }
            Err(ArmReadError::Incomplete) => {
                result.issues.push(AccessIssue {
                    kind: AccessIssueKind::AssignmentReadFailed,
                    scope,
                });
                continue;
            }
        };
        if assignments.is_empty() {
            result.issues.push(AccessIssue {
                kind: AccessIssueKind::NoAssignments,
                scope,
            });
        }
        for assignment in assignments {
            let Some(properties) = assignment.get("properties") else {
                result.issues.push(AccessIssue {
                    kind: AccessIssueKind::AssignmentReadFailed,
                    scope: sub.id.clone(),
                });
                continue;
            };
            let scope = properties
                .get("scope")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let role_id = properties
                .get("roleDefinitionId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_owned();
            let Some(id) = assignment
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
            else {
                result.issues.push(AccessIssue {
                    kind: AccessIssueKind::AssignmentReadFailed,
                    scope,
                });
                continue;
            };
            if !seen.insert(id.to_ascii_lowercase()) {
                continue;
            }
            if !role_id.starts_with('/')
                || role_id.contains("..")
                || role_id.contains('?')
                || scope.is_empty()
            {
                result.issues.push(AccessIssue {
                    kind: AccessIssueKind::DefinitionReadFailed,
                    scope,
                });
                continue;
            }
            if !definitions.contains_key(&role_id) {
                match get_json_before(
                    &http,
                    &token,
                    &format!("{endpoint}{role_id}?api-version=2022-04-01"),
                    endpoint,
                    deadline,
                )
                .await
                {
                    Ok(definition) => {
                        definitions.insert(role_id.clone(), definition);
                    }
                    Err(ArmReadError::Authentication) => {
                        return Err(AuthError::Rejected {
                            status: 401,
                            detail: "permission inspection authentication failed".into(),
                        }
                        .into());
                    }
                    Err(ArmReadError::Incomplete) => {
                        result.issues.push(AccessIssue {
                            kind: AccessIssueKind::DefinitionReadFailed,
                            scope,
                        });
                        continue;
                    }
                }
            }
            let Some(definition) = definitions.get(&role_id).and_then(|v| v.get("properties"))
            else {
                result.issues.push(AccessIssue {
                    kind: AccessIssueKind::DefinitionReadFailed,
                    scope,
                });
                continue;
            };
            let (verdict, actions) =
                classify_permissions(definition.get("permissions").unwrap_or(&Value::Null));
            if properties
                .get("condition")
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty())
            {
                result.issues.push(AccessIssue {
                    kind: AccessIssueKind::ConditionalGrant,
                    scope: scope.clone(),
                });
            }
            if verdict == PermissionVerdict::UnableToVerify {
                result.issues.push(AccessIssue {
                    kind: AccessIssueKind::UnsupportedPermissions,
                    scope: scope.clone(),
                });
            }
            result.grants.push(PermissionGrant {
                scope,
                role_id: role_id.clone(),
                role: definition
                    .get("roleName")
                    .and_then(Value::as_str)
                    .unwrap_or(&role_id)
                    .into(),
                verdict,
                actions,
            });
        }
    }
    result
        .grants
        .sort_by(|a, b| (&a.scope, &a.role_id).cmp(&(&b.scope, &b.role_id)));
    result.verdict = if result
        .grants
        .iter()
        .any(|g| g.verdict == PermissionVerdict::BroaderGrants)
    {
        PermissionVerdict::BroaderGrants
    } else if result.issues.is_empty() && !result.grants.is_empty() {
        PermissionVerdict::ReadOnly
    } else {
        PermissionVerdict::UnableToVerify
    };
    Ok(result)
}

/// Only prove a grant read-only when its entire remaining permission language
/// is read-only. Wildcard subtraction we cannot prove is explicitly unknown.
pub fn classify_permissions(blocks: &Value) -> (PermissionVerdict, Vec<String>) {
    let Some(blocks) = blocks.as_array().filter(|a| !a.is_empty()) else {
        return (PermissionVerdict::UnableToVerify, vec![]);
    };
    let mut broader = Vec::new();
    let mut unknown = false;
    for block in blocks {
        for (allowed, excluded) in [("actions", "notActions"), ("dataActions", "notDataActions")] {
            let actions = block.get(allowed).and_then(Value::as_array);
            let exclusions = block.get(excluded).and_then(Value::as_array);
            let (Some(actions), Some(exclusions)) = (actions, exclusions) else {
                unknown = true;
                continue;
            };
            if exclusions
                .iter()
                .any(|v| !v.as_str().is_some_and(supported_pattern))
            {
                unknown = true;
                continue;
            }
            for action in actions {
                let Some(action) = action.as_str() else {
                    unknown = true;
                    continue;
                };
                if !supported_pattern(action) {
                    unknown = true;
                    continue;
                }
                let lower = action.to_ascii_lowercase();
                if exclusions.iter().filter_map(Value::as_str).any(|e| {
                    e.eq_ignore_ascii_case(action)
                        || e == "*"
                        || (!action.contains('*') && glob_matches(e, action))
                }) {
                    continue;
                }
                if lower.ends_with("/read") {
                    continue;
                }
                if action.contains('*') && !exclusions.is_empty() {
                    let witnesses = [
                        "Microsoft.Compute/virtualMachines/write",
                        "Microsoft.Resources/deployments/write",
                        "Microsoft.Authorization/roleAssignments/write",
                        "Microsoft.Storage/storageAccounts/write",
                    ];
                    if witnesses.iter().any(|w| {
                        glob_matches(action, w)
                            && !exclusions
                                .iter()
                                .filter_map(Value::as_str)
                                .any(|e| glob_matches(e, w))
                    }) {
                        broader.push(action.into());
                    } else {
                        unknown = true;
                    }
                } else if action.contains('/') || action == "*" {
                    broader.push(action.into());
                } else {
                    unknown = true;
                }
            }
        }
    }
    broader.sort();
    broader.dedup();
    let verdict = if !broader.is_empty() {
        PermissionVerdict::BroaderGrants
    } else if unknown {
        PermissionVerdict::UnableToVerify
    } else {
        PermissionVerdict::ReadOnly
    };
    (verdict, broader)
}

fn supported_pattern(pattern: &str) -> bool {
    !pattern.is_empty()
        && pattern
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"./_*-".contains(&b))
}
fn glob_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.to_ascii_lowercase();
    let value = value.to_ascii_lowercase();
    let (p, v) = (pattern.as_bytes(), value.as_bytes());
    let (mut i, mut j, mut star, mut resume) = (0, 0, None, 0);
    while j < v.len() {
        if i < p.len() && p[i] == v[j] {
            i += 1;
            j += 1;
        } else if i < p.len() && p[i] == b'*' {
            star = Some(i);
            i += 1;
            resume = j;
        } else if let Some(index) = star {
            i = index + 1;
            resume += 1;
            j = resume;
        } else {
            return false;
        }
    }
    while i < p.len() && p[i] == b'*' {
        i += 1;
    }
    i == p.len()
}

enum ArmReadError {
    Authentication,
    Incomplete,
}
async fn get_json(
    http: &reqwest::Client,
    token: &str,
    url: &str,
    endpoint: &str,
) -> Result<Value, ArmReadError> {
    let parsed = reqwest::Url::parse(url).map_err(|_| ArmReadError::Incomplete)?;
    let base = reqwest::Url::parse(endpoint).map_err(|_| ArmReadError::Incomplete)?;
    if parsed.origin() != base.origin()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(ArmReadError::Incomplete);
    }
    for attempt in 0..3 {
        let response = http
            .get(parsed.clone())
            .bearer_auth(token)
            .timeout(Duration::from_secs(20))
            .send()
            .await
            .map_err(|_| ArmReadError::Incomplete)?;
        if response.status().as_u16() == 401 {
            return Err(ArmReadError::Authentication);
        }
        if response.status().as_u16() == 429 || response.status().is_server_error() {
            if attempt == 2 {
                return Err(ArmReadError::Incomplete);
            }
            let pause = response
                .headers()
                .get("retry-after")
                .and_then(|h| h.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1 << attempt)
                .min(5);
            tokio::time::sleep(Duration::from_secs(pause)).await;
            continue;
        }
        if !response.status().is_success() {
            return Err(ArmReadError::Incomplete);
        }
        return response.json().await.map_err(|_| ArmReadError::Incomplete);
    }
    Err(ArmReadError::Incomplete)
}
async fn get_json_before(
    http: &reqwest::Client,
    token: &str,
    url: &str,
    endpoint: &str,
    deadline: tokio::time::Instant,
) -> Result<Value, ArmReadError> {
    tokio::time::timeout_at(deadline, get_json(http, token, url, endpoint))
        .await
        .map_err(|_| ArmReadError::Incomplete)?
}
async fn get_pages(
    http: &reqwest::Client,
    token: &str,
    url: &str,
    endpoint: &str,
    deadline: tokio::time::Instant,
) -> Result<(Vec<Value>, bool), ArmReadError> {
    let mut next = Some(url.to_owned());
    let mut seen = BTreeSet::new();
    let mut rows = Vec::new();
    while let Some(url) = next {
        if !seen.insert(url.clone()) || seen.len() > 1000 {
            return Ok((rows, false));
        }
        let value = match get_json_before(http, token, &url, endpoint, deadline).await {
            Ok(value) => value,
            Err(ArmReadError::Authentication) => return Err(ArmReadError::Authentication),
            Err(ArmReadError::Incomplete) => return Ok((rows, false)),
        };
        let Some(values) = value.get("value").and_then(Value::as_array) else {
            return Ok((rows, false));
        };
        rows.extend(values.iter().cloned());
        next = match value.get("nextLink") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) if s.is_empty() => None,
            Some(Value::String(s)) => Some(s.clone()),
            _ => return Ok((rows, false)),
        };
    }
    Ok((rows, true))
}
