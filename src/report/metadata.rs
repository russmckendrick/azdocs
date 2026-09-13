//! Selected operational settings for print. Raw definitions and provider
//! histories remain in the snapshot and data exports.
use std::sync::LazyLock;

use serde::Deserialize;
use serde_json::Value;

use super::{Block, MetadataGroup, TableLink};
use crate::{
    labels::Labels,
    model::{Resource, azure_values, truncate},
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FieldPack {
    fields: Vec<Field>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Field {
    label: String,
    paths: Vec<String>,
    #[serde(default)]
    count: bool,
}

static FIELDS: LazyLock<FieldPack> = LazyLock::new(|| {
    toml::from_str(include_str!("../../data/reference_fields.toml"))
        .expect("the embedded reference field pack is validated by unit tests")
});

pub(super) fn summary<'a>(resource: &Resource, labels: &Labels, blocks: &mut Vec<Block<'a>>) {
    let words = &labels.report.assessment;
    let mut rows = Vec::new();
    if let Some(location) = &resource.location {
        rows.push(vec![
            labels.common.columns.location.clone(),
            azure_values::display_location(location).into_owned(),
        ]);
    }
    if let Some(kind) = &resource.kind {
        rows.push(vec![
            labels.common.columns.kind.clone(),
            azure_values::display_kind(&resource.azure_type, kind).into_owned(),
        ]);
    }
    if let Some(sku) = &resource.sku {
        let values: Vec<_> = ["name", "tier", "capacity"]
            .into_iter()
            .filter_map(|key| sku.get(key).and_then(display_value))
            .collect();
        if !values.is_empty() {
            rows.push(vec![words.reference_sku.clone(), values.join(" · ")]);
        }
    }
    if let Some(identity) = resource
        .identity
        .as_ref()
        .and_then(|v| v.get("type"))
        .and_then(display_value)
    {
        rows.push(vec![words.reference_auth_identity.clone(), identity]);
    }
    if let Some(properties) = &resource.properties {
        for field in &FIELDS.fields {
            let value = field.paths.iter().find_map(|path| {
                let value = properties.pointer(path)?;
                if field.count {
                    value.as_array().map(|items| items.len().to_string())
                } else {
                    display_value(value)
                }
            });
            if let Some(value) = value {
                rows.push(vec![words.reference_fields[&field.label].clone(), value]);
            }
        }
    }
    if let Some(tags) = resource.tags.as_ref().and_then(Value::as_object) {
        let mut values: Vec<_> = tags
            .iter()
            .filter(|(key, _)| !key.starts_with("hidden-"))
            .filter_map(|(key, value)| display_value(value).map(|value| format!("{key}={value}")))
            .collect();
        values.sort();
        if !values.is_empty() {
            rows.push(vec![
                labels.common.columns.tags.clone(),
                truncate(&values.join("; "), 200),
            ]);
        }
    }
    if rows.is_empty() {
        return;
    }
    let links = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| standalone_url(&row[1]))
        .map(|(row, values)| TableLink {
            row,
            column: 1,
            target: values[1].clone(),
            external: true,
        })
        .collect();
    blocks.push(Block::Metadata {
        groups: vec![MetadataGroup {
            title: words.reference_properties.clone(),
            rows,
            links,
        }],
        keep_together: false,
    });
}

fn standalone_url(value: &str) -> bool {
    (value.starts_with("https://") || value.starts_with("http://"))
        && !value.chars().any(char::is_whitespace)
}

fn display_value(value: &Value) -> Option<String> {
    let text = match value {
        Value::String(text) if !text.is_empty() => text.clone(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(items)
            if !items.is_empty()
                && items
                    .iter()
                    .all(|item| !item.is_array() && !item.is_object()) =>
        {
            items
                .iter()
                .filter_map(display_value)
                .collect::<Vec<_>>()
                .join(", ")
        }
        _ => return None,
    };
    if text.is_empty() {
        return None;
    }
    Some(if standalone_url(&text) {
        text
    } else {
        truncate(&text, 160)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn unit_reference_field_pack_has_labels_unique_keys_and_valid_paths() {
        let labels = Labels::default();
        let mut seen = std::collections::BTreeSet::new();
        for field in &FIELDS.fields {
            assert!(seen.insert(&field.label));
            assert!(
                labels
                    .report
                    .assessment
                    .reference_fields
                    .contains_key(&field.label)
            );
            assert!(!field.paths.is_empty());
            assert!(field.paths.iter().all(|path| path.starts_with('/')));
        }
    }

    #[test]
    fn unit_summary_keeps_operational_settings_without_expanding_provider_catalogues() {
        let resource = Resource {
            id: "/vm".into(),
            display_id: "/VM".into(),
            name: "vm".into(),
            azure_type: "microsoft.compute/virtualmachines".into(),
            subscription_id: "sub".into(),
            kind: None,
            location: Some("uksouth".into()),
            resource_group: Some("rg".into()),
            tags: None,
            sku: None,
            identity: None,
            properties: Some(json!({
                "hardwareProfile": {"vmSize": "Standard_D4s_v5"},
                "storageProfile": {"osDisk": {"osType": "Linux"}, "dataDisks": [{}, {}]},
                "publicNetworkAccess": "Disabled",
                "callRateLimit": {"rules": vec![json!({"provider_internal": "noise"}); 2000]},
                "provisioningState": "Succeeded"
            })),
        };
        let mut blocks = Vec::new();
        summary(&resource, &Labels::default(), &mut blocks);
        let serialized = serde_json::to_string(&blocks).unwrap();
        for expected in ["Standard_D4s_v5", "Linux", "Disabled", "Succeeded"] {
            assert!(serialized.contains(expected));
        }
        assert!(!serialized.contains("provider_internal"));
        let Block::Metadata { groups, .. } = &blocks[0] else {
            panic!("settings table missing")
        };
        assert!(groups[0].rows.iter().any(|row| row == &["Data disks", "2"]));
        assert!(groups[0].rows.len() <= 8);
    }

    #[test]
    fn unit_display_value_preserves_false_zero_and_standalone_urls() {
        assert_eq!(display_value(&json!(false)).as_deref(), Some("false"));
        assert_eq!(display_value(&json!(0)).as_deref(), Some("0"));
        let url = format!("https://example.com/?state={}", "abc".repeat(500));
        assert_eq!(display_value(&json!(url)), Some(url));
        assert_eq!(display_value(&Value::Null), None);
        assert_eq!(display_value(&json!([null, ""])), None);
    }
}
