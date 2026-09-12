//! Print stored metadata in its structural groups, without repeating long
//! parent paths in every field label or reducing the saved values.
use super::{Block, MetadataGroup, TableLink};
use crate::{
    labels::Labels,
    model::{Resource, azure_values},
};
use serde_json::Value;

pub(super) fn identity<'a>(
    resource: &'a Resource,
    subscription: &'a str,
    labels: &'a Labels,
    blocks: &mut Vec<Block<'a>>,
) {
    let columns = &labels.common.columns;
    let words = &labels.report.assessment;
    let mut rows = vec![
        (columns.subscription.clone(), subscription.to_owned()),
        (columns.azure_type.clone(), resource.azure_type.clone()),
    ];
    for (key, value) in [
        (&columns.resource_group, &resource.resource_group),
        (&columns.location, &resource.location),
        (&columns.kind, &resource.kind),
    ] {
        if let Some(value) = value {
            rows.push((
                key.clone(),
                if key == &columns.location {
                    azure_values::display_location(value).into_owned()
                } else {
                    value.clone()
                },
            ));
        }
    }
    rows.push((
        words.reference_resource_id.clone(),
        resource.display_id.clone(),
    ));
    blocks.push(Block::Metadata {
        groups: vec![group(&words.reference_identity, rows)],
        keep_together: true,
    });
}

pub(super) fn configuration<'a>(
    resource: &'a Resource,
    labels: &'a Labels,
    blocks: &mut Vec<Block<'a>>,
) {
    let words = &labels.report.assessment;
    let mut groups = Vec::new();
    for (title, value) in [
        (&labels.common.columns.tags, &resource.tags),
        (&words.reference_sku, &resource.sku),
        (&words.reference_auth_identity, &resource.identity),
        (&words.reference_properties, &resource.properties),
    ] {
        if let Some(value) = value {
            walk(title, value, Vec::new(), labels, &mut groups);
        }
    }
    if !groups.is_empty() {
        blocks.push(Block::Metadata {
            groups,
            keep_together: false,
        });
    }
}

fn is_container(value: &Value) -> bool {
    matches!(value, Value::Object(map) if !map.is_empty())
        || matches!(value, Value::Array(items) if !items.is_empty())
}

fn stored_value(value: &Value) -> String {
    // Preserve empty containers and explicit nulls as evidence too.
    value
        .as_str()
        .map(str::to_owned)
        .unwrap_or_else(|| value.to_string())
}

fn walk(
    title: &str,
    value: &Value,
    mut rows: Vec<(String, String)>,
    labels: &Labels,
    groups: &mut Vec<MetadataGroup>,
) {
    let words = &labels.report.assessment;
    let children: Vec<(String, &Value)> = match value {
        Value::Object(map) if !map.is_empty() => map
            .iter()
            .map(|(key, value)| (key.clone(), value))
            .collect(),
        Value::Array(items) if !items.is_empty() => items
            .iter()
            .enumerate()
            .map(|(index, value)| (format!("[{index}]"), value))
            .collect(),
        _ => {
            rows.push((words.reference_record_value.clone(), stored_value(value)));
            groups.push(group(title, rows));
            return;
        }
    };
    let mut nested = Vec::new();
    for (key, value) in children {
        if key.starts_with('/')
            || key.starts_with("https://")
            || key.starts_with("http://")
            || key.chars().count() > 64
        {
            // Azure uses full ARM IDs as object keys for assigned identities.
            // Keep those IDs in the value column instead of the narrow label.
            nested.push((
                title.to_owned(),
                value,
                vec![(words.reference_record_key.clone(), key)],
            ));
        } else if is_container(value) {
            nested.push((format!("{title} / {key}"), value, Vec::new()));
        } else {
            rows.push((key, stored_value(value)));
        }
    }
    if !rows.is_empty() {
        groups.push(group(title, rows));
    }
    for (title, value, rows) in nested {
        walk(&title, value, rows, labels, groups);
    }
}

fn group(title: &str, rows: Vec<(String, String)>) -> MetadataGroup {
    let links = rows
        .iter()
        .enumerate()
        .filter(|(_, (_, value))| value.starts_with("https://") || value.starts_with("http://"))
        .map(|(row, (_, value))| TableLink {
            row,
            column: 1,
            target: value.clone(),
            external: true,
        })
        .collect();
    MetadataGroup {
        title: title.to_owned(),
        rows: rows
            .into_iter()
            .map(|(key, value)| vec![key, value])
            .collect(),
        links,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn metadata_preserves_nested_arrays_long_values_and_dynamic_identity_keys() {
        let labels = Labels::default();
        let long_url = format!("https://example.com/?state={}", "abc".repeat(500));
        let key = "/subscriptions/sub/resourceGroups/group/providers/Microsoft.ManagedIdentity/userAssignedIdentities/identity";
        let value = json!({"empty": [], "missing": null, "redirect": long_url, "rules": [{"name":"first", "enabled":false}, {"name":"second"}], "identities": {key: {}}});
        let mut groups = Vec::new();
        walk("properties", &value, Vec::new(), &labels, &mut groups);
        let values: Vec<_> = groups
            .iter()
            .flat_map(|group| group.rows.iter().flatten())
            .map(String::as_str)
            .collect();
        for expected in [
            long_url.as_str(),
            key,
            "[]",
            "{}",
            "null",
            "first",
            "second",
            "false",
        ] {
            assert!(values.contains(&expected), "lost {expected}");
        }
        assert!(
            groups
                .iter()
                .any(|group| group.title == "properties / rules / [1]")
        );
        assert!(groups.iter().any(|group| {
            group
                .links
                .iter()
                .any(|link| link.target == long_url && link.external)
        }));
        assert!(groups.iter().all(|group| {
            group
                .rows
                .iter()
                .all(|row| !row[0].starts_with("/subscriptions/"))
        }));
    }
}
