use serde_json::Value;

use crate::error::StoreError;
use crate::model::{Finding, Resource, ResourceGroup, Severity, Subscription, normalize_arm_id};
use crate::querypack::{QueryDef, QueryKind};
use crate::store::Store;

/// Route one query's rows to the right table(s). The three core inventory
/// queries load typed tables; other inventory queries keep their raw rows for
/// report use; finding queries become findings.
pub fn ingest(
    store: &Store,
    snapshot_id: &str,
    def: &QueryDef,
    rows: &[Value],
) -> Result<(), StoreError> {
    match (def.kind, def.name.as_str()) {
        (QueryKind::Inventory, "all_resources") => {
            let resources: Vec<_> = rows.iter().filter_map(resource_from_row).collect();
            store.insert_resources(snapshot_id, &resources)
        }
        (QueryKind::Inventory, "subscriptions") => {
            let subscriptions: Vec<_> = rows.iter().filter_map(subscription_from_row).collect();
            store.insert_subscriptions(snapshot_id, &subscriptions)
        }
        (QueryKind::Inventory, "resource_groups") => {
            let groups: Vec<_> = rows.iter().filter_map(resource_group_from_row).collect();
            store.insert_resource_groups(snapshot_id, &groups)
        }
        (QueryKind::Inventory, _) => store.insert_query_results(snapshot_id, &def.name, rows),
        (QueryKind::Finding, _) => {
            let findings: Vec<_> = rows.iter().map(|row| finding_from_row(def, row)).collect();
            store.insert_findings(snapshot_id, &findings)
        }
    }
}

fn str_field(row: &Value, field: &str) -> Option<String> {
    row.get(field)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// ARG returns `{}` for absent tags/sku/identity on some types; treat empty
/// objects and nulls alike as absent.
fn value_field(row: &Value, field: &str) -> Option<Value> {
    match row.get(field) {
        None | Some(Value::Null) => None,
        Some(Value::Object(map)) if map.is_empty() => None,
        Some(other) => Some(other.clone()),
    }
}

pub fn resource_from_row(row: &Value) -> Option<Resource> {
    let display_id = str_field(row, "id")?;
    Some(Resource {
        id: normalize_arm_id(&display_id),
        display_id,
        name: str_field(row, "name")?,
        azure_type: str_field(row, "type")?.to_lowercase(),
        kind: str_field(row, "kind"),
        location: str_field(row, "location"),
        resource_group: str_field(row, "resourceGroup").map(|rg| rg.to_lowercase()),
        subscription_id: str_field(row, "subscriptionId")?,
        tags: value_field(row, "tags"),
        sku: value_field(row, "sku"),
        identity: value_field(row, "identity"),
        properties: value_field(row, "properties"),
    })
}

pub fn subscription_from_row(row: &Value) -> Option<Subscription> {
    Some(Subscription {
        subscription_id: str_field(row, "subscriptionId")?,
        display_name: str_field(row, "name")?,
        state: str_field(row, "state"),
        tags: value_field(row, "tags"),
    })
}

pub fn resource_group_from_row(row: &Value) -> Option<ResourceGroup> {
    let id = str_field(row, "id")?;
    Some(ResourceGroup {
        id: normalize_arm_id(&id),
        name: str_field(row, "name")?,
        subscription_id: str_field(row, "subscriptionId")?,
        location: str_field(row, "location"),
        tags: value_field(row, "tags"),
    })
}

pub fn finding_from_row(def: &QueryDef, row: &Value) -> Finding {
    let title_field = def.title_field.as_deref();
    let title = title_field
        .and_then(|field| str_field(row, field))
        .or_else(|| str_field(row, "name"))
        .or_else(|| str_field(row, "id"))
        .unwrap_or_else(|| def.name.clone());
    Finding {
        query_name: def.name.clone(),
        category: def.category.clone(),
        severity: def.severity.unwrap_or(Severity::Info),
        resource_id: str_field(row, "id").map(|id| normalize_arm_id(&id)),
        title,
        detail: Some(row.clone()),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn resource_from_row_lowercases_join_keys_and_keeps_display_id() {
        let row = json!({
            "id": "/subscriptions/S1/resourceGroups/RG-App/providers/Microsoft.Compute/virtualMachines/VM1",
            "name": "VM1",
            "type": "Microsoft.Compute/virtualMachines",
            "location": "uksouth",
            "resourceGroup": "RG-App",
            "subscriptionId": "S1",
            "tags": {"env": "prod"},
            "properties": {"hardwareProfile": {"vmSize": "Standard_B2s"}}
        });

        let resource = resource_from_row(&row).unwrap();

        assert_eq!(
            (
                resource.id.contains("rg-app"),
                resource.display_id.contains("RG-App"),
                resource.azure_type.as_str(),
                resource.resource_group.as_deref(),
            ),
            (
                true,
                true,
                "microsoft.compute/virtualmachines",
                Some("rg-app")
            )
        );
    }

    #[test]
    fn resource_from_row_skips_rows_without_id() {
        let row = json!({"name": "orphan", "type": "x", "subscriptionId": "s"});

        assert_eq!(resource_from_row(&row), None);
    }

    #[test]
    fn value_field_treats_empty_object_as_absent() {
        let row = json!({"tags": {}});

        assert_eq!(value_field(&row, "tags"), None);
    }

    #[test]
    fn finding_from_row_prefers_title_field() {
        let def = QueryDef::parse(
            r#"
name = "nsg_open"
category = "security"
kind = "finding"
description = "d"
severity = "high"
title_field = "ruleName"
kql = "resources"
"#,
            "test.toml",
        )
        .unwrap();
        let row = json!({"id": "/X/Y", "name": "nsg1", "ruleName": "allow-all"});

        let finding = finding_from_row(&def, &row);

        assert_eq!(
            (finding.title.as_str(), finding.resource_id.as_deref()),
            ("allow-all", Some("/x/y"))
        );
    }
}
