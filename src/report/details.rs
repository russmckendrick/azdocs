//! Per-resource detail pages: one page per resource group, one section per
//! resource with settings flattened from the stored properties bag, finding
//! callouts, and related-resource links — Cloudockit style, offline.

use serde::Serialize;
use serde_json::Value;

use crate::model::{Edge, Finding, Resource, azure_types, azure_values};

/// One `docs/resources/<sub>/<rg>` page.
#[derive(Debug, Serialize)]
pub struct ResourceGroupPage {
    /// Relative path without extension, e.g. `resources/production/rg-app`.
    pub path: String,
    pub subscription_name: String,
    pub subscription_slug: String,
    pub resource_group: String,
    pub location: Option<String>,
    /// `<subscription id>/<lowercased group name>` — the key a per-group
    /// diagram is filed under, so a section can find its own picture. Group
    /// names are not unique across subscriptions, so the name alone will not do.
    pub group_key: String,
    pub resources: Vec<ResourceDetail>,
}

/// Build the key a resource group and its diagram agree on.
pub fn group_key(subscription_id: &str, resource_group: &str) -> String {
    format!(
        "{}/{}",
        subscription_id.to_lowercase(),
        resource_group.to_lowercase()
    )
}

#[derive(Debug, Serialize)]
pub struct ResourceDetail {
    pub name: String,
    pub display_type: String,
    pub azure_type: String,
    pub arm_id: String,
    /// Where the resource lives, for the by-type sections, which lose the
    /// subscription/group context the by-group pages get from their heading.
    pub subscription_name: String,
    pub resource_group: Option<String>,
    pub location: Option<String>,
    pub settings: Vec<Setting>,
    pub findings: Vec<Callout>,
    pub related: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Setting {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Serialize)]
pub struct Callout {
    pub severity: String,
    pub title: String,
}

pub fn resource_detail(
    resource: &Resource,
    subscription_name: &str,
    findings: &[Finding],
    edges: &[Edge],
) -> ResourceDetail {
    ResourceDetail {
        name: resource.name.clone(),
        display_type: azure_types::display_name(&resource.azure_type).to_owned(),
        azure_type: resource.azure_type.clone(),
        arm_id: resource.display_id.clone(),
        subscription_name: subscription_name.to_owned(),
        resource_group: resource.resource_group.clone(),
        location: resource
            .location
            .as_deref()
            .map(|value| azure_values::display_location(value).into_owned()),
        settings: settings_rows(resource),
        findings: findings
            .iter()
            .filter(|f| f.resource_id.as_deref() == Some(&resource.id))
            .map(|f| Callout {
                severity: f.severity.as_str().to_owned(),
                title: f.title.clone(),
            })
            .collect(),
        related: related_names(resource, edges),
    }
}

/// Basics plus every scalar (and scalar-array) top-level property, in stored
/// order. Nested objects are skipped: the full bag stays available in the TUI.
fn settings_rows(resource: &Resource) -> Vec<Setting> {
    const MAX_ROWS: usize = 24;
    let mut rows = Vec::new();
    fn push(rows: &mut Vec<Setting>, key: &str, value: String) {
        const MAX_VALUE: usize = 120;
        if !value.is_empty() {
            rows.push(Setting {
                key: humanize_key(key),
                value: truncate(&value, MAX_VALUE),
            });
        }
    }

    if let Some(location) = &resource.location {
        push(
            &mut rows,
            "location",
            azure_values::display_location(location).into_owned(),
        );
    }
    if let Some(kind) = &resource.kind {
        push(
            &mut rows,
            "kind",
            azure_values::display_kind(&resource.azure_type, kind).into_owned(),
        );
    }
    if let Some(sku) = resource
        .sku
        .as_ref()
        .and_then(|s| s.get("name"))
        .and_then(Value::as_str)
    {
        push(&mut rows, "sku", sku.to_owned());
    }
    if let Some(identity) = resource
        .identity
        .as_ref()
        .and_then(|i| i.get("type"))
        .and_then(Value::as_str)
    {
        push(&mut rows, "identityType", identity.to_owned());
    }
    if let Some(tags) = resource.tags.as_ref().and_then(Value::as_object) {
        let rendered: Vec<String> = tags
            .iter()
            .map(|(k, v)| format!("{k}={}", v.as_str().unwrap_or_default()))
            .collect();
        push(&mut rows, "tags", rendered.join(", "));
    }

    for (key, value) in resource
        .properties
        .as_ref()
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
    {
        if rows.len() >= MAX_ROWS {
            break;
        }
        match value {
            Value::String(s) => push(&mut rows, key, s.clone()),
            Value::Bool(b) => push(&mut rows, key, b.to_string()),
            Value::Number(n) => push(&mut rows, key, n.to_string()),
            Value::Array(items) if items.iter().all(|i| !i.is_object() && !i.is_array()) => {
                let rendered: Vec<String> = items
                    .iter()
                    .map(|i| match i {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    })
                    .collect();
                push(&mut rows, key, rendered.join(", "));
            }
            _ => {}
        }
    }
    rows
}

fn related_names(resource: &Resource, edges: &[Edge]) -> Vec<String> {
    let mut related: Vec<String> = edges
        .iter()
        .filter(|e| e.source_id == resource.id || e.target_id == resource.id)
        .map(|e| {
            let other = if e.source_id == resource.id {
                &e.target_id
            } else {
                &e.source_id
            };
            let name = other.rsplit('/').next().unwrap_or(other);
            format!("{name} ({})", e.kind.as_str())
        })
        .collect();
    related.sort();
    related.dedup();
    related
}

/// `supportsHttpsTrafficOnly` -> `Supports Https Traffic Only`; consecutive
/// capitals stay together (`diskSizeGB` -> `Disk Size GB`).
fn humanize_key(key: &str) -> String {
    let mut out = String::with_capacity(key.len() + 4);
    let mut previous_lower_or_digit = false;
    for (index, c) in key.chars().enumerate() {
        if index == 0 {
            out.extend(c.to_uppercase());
        } else {
            if c.is_uppercase() && previous_lower_or_digit {
                out.push(' ');
            }
            out.push(c);
        }
        previous_lower_or_digit = c.is_lowercase() || c.is_ascii_digit();
    }
    out
}

fn truncate(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_owned();
    }
    let mut out: String = value.chars().take(max - 1).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::model::normalize_arm_id;

    fn storage_account() -> Resource {
        Resource {
            id: normalize_arm_id("/s1/rg/st1"),
            display_id: "/s1/rg/st1".to_owned(),
            name: "st1".to_owned(),
            azure_type: "microsoft.storage/storageaccounts".to_owned(),
            kind: Some("StorageV2".to_owned()),
            location: Some("uksouth".to_owned()),
            resource_group: Some("rg".to_owned()),
            subscription_id: "s1".to_owned(),
            tags: Some(json!({"env": "prod"})),
            sku: Some(json!({"name": "Standard_LRS"})),
            identity: None,
            properties: Some(json!({
                "supportsHttpsTrafficOnly": true,
                "allowBlobPublicAccess": false,
                "networkAcls": {"defaultAction": "Allow"},
                "privateEndpointConnections": []
            })),
        }
    }

    #[test]
    fn settings_rows_flatten_scalars_and_skip_nested_objects() {
        let rows = settings_rows(&storage_account());

        let keys: Vec<&str> = rows.iter().map(|s| s.key.as_str()).collect();
        assert!(
            keys.contains(&"Supports Https Traffic Only")
                && !keys.iter().any(|k| k.contains("Acls")),
            "keys: {keys:?}"
        );
    }

    #[test]
    fn humanize_key_splits_camel_case() {
        assert_eq!(
            humanize_key("allowBlobPublicAccess"),
            "Allow Blob Public Access"
        );
    }

    #[test]
    fn resource_detail_attaches_matching_findings() {
        let resource = storage_account();
        let findings = vec![Finding {
            query_name: "q".into(),
            category: "security".into(),
            severity: crate::model::Severity::High,
            resource_id: Some(resource.id.clone()),
            title: "st1 allows public blob access".into(),
            detail: None,
        }];

        let detail = resource_detail(&resource, "Production", &findings, &[]);

        assert_eq!(detail.findings.len(), 1);
    }
}
