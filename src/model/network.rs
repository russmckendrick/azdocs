//! Reading network shape out of a VNet resource's `properties` blob.
//!
//! ARG returns a VNet's subnets inline rather than as separate rows, so both the
//! print diagrams (`diagram/graph.rs`) and the desktop topology builder have to
//! dig the same fields out of the same untyped JSON. They had two independent
//! copies of that knowledge — same paths, same fallbacks, character for
//! character in places — differing only in the nodes they built afterwards.
//! This module owns the parsing; callers keep their own node types.

use serde_json::Value;

use super::Resource;

/// A subnet as declared inside its VNet's `properties.subnets`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubnetInfo {
    /// Lowercased ARM id — the join key, per the project-wide rule.
    pub id: String,
    /// `name` from the payload, else the last segment of the id with its
    /// original casing preserved for display.
    pub name: String,
    /// CIDR from `properties.addressPrefix`, when the payload carries one.
    pub address_prefix: Option<String>,
}

/// Address prefixes declared on a VNet, in payload order.
///
/// Empty when the resource is not a VNet, carries no `properties`, or the shape
/// is not what ARG normally returns — callers decide whether to join them for a
/// label or take the first.
pub fn vnet_address_prefixes(vnet: &Resource) -> Vec<String> {
    vnet.properties
        .as_ref()
        .and_then(|p| p.get("addressSpace"))
        .and_then(|space| space.get("addressPrefixes"))
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// Subnets declared inline on a VNet, in payload order.
///
/// Entries without an `id` are skipped: the id is the join key every caller
/// needs, and a subnet without one cannot be connected to anything.
pub fn vnet_subnets(vnet: &Resource) -> Vec<SubnetInfo> {
    let Some(subnets) = vnet
        .properties
        .as_ref()
        .and_then(|p| p.get("subnets"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    subnets
        .iter()
        .filter_map(|subnet| {
            let raw_id = subnet.get("id").and_then(Value::as_str)?;
            let name = subnet
                .get("name")
                .and_then(Value::as_str)
                // From the original id, not the lowercased one: a fallback name
                // is still display text, and display keeps its casing.
                .or_else(|| raw_id.rsplit('/').next())
                .unwrap_or("subnet")
                .to_owned();
            let address_prefix = subnet
                .pointer("/properties/addressPrefix")
                .and_then(Value::as_str)
                .map(str::to_owned);
            Some(SubnetInfo {
                id: raw_id.to_lowercase(),
                name,
                address_prefix,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn vnet(properties: Value) -> Resource {
        Resource {
            id: "/subscriptions/s/resourcegroups/rg/providers/microsoft.network/virtualnetworks/v"
                .to_owned(),
            display_id:
                "/subscriptions/S/resourceGroups/rg/providers/Microsoft.Network/virtualNetworks/v"
                    .to_owned(),
            name: "v".to_owned(),
            azure_type: "microsoft.network/virtualnetworks".to_owned(),
            kind: None,
            location: None,
            resource_group: Some("rg".to_owned()),
            subscription_id: "s".to_owned(),
            tags: None,
            sku: None,
            identity: None,
            properties: Some(properties),
        }
    }

    #[test]
    fn unit_reads_every_address_prefix_when_the_vnet_declares_several() {
        let resource = vnet(json!({
            "addressSpace": { "addressPrefixes": ["10.0.0.0/16", "10.1.0.0/16"] }
        }));
        assert_eq!(
            vnet_address_prefixes(&resource),
            vec!["10.0.0.0/16".to_owned(), "10.1.0.0/16".to_owned()]
        );
    }

    #[test]
    fn unit_returns_no_prefixes_when_properties_are_missing_or_misshapen() {
        let mut resource = vnet(json!({}));
        assert!(vnet_address_prefixes(&resource).is_empty());
        resource.properties = None;
        assert!(vnet_address_prefixes(&resource).is_empty());
        resource.properties = Some(json!({ "addressSpace": { "addressPrefixes": "10.0.0.0/16" } }));
        assert!(vnet_address_prefixes(&resource).is_empty());
    }

    #[test]
    fn unit_lowercases_the_subnet_id_when_arg_returns_mixed_casing() {
        let resource = vnet(json!({
            "subnets": [{
                "id": "/subscriptions/S/resourceGroups/RG/providers/Microsoft.Network/virtualNetworks/V/subnets/App",
                "name": "app",
                "properties": { "addressPrefix": "10.0.1.0/24" }
            }]
        }));
        let subnets = vnet_subnets(&resource);
        assert_eq!(subnets.len(), 1);
        assert_eq!(
            subnets[0].id,
            "/subscriptions/s/resourcegroups/rg/providers/microsoft.network/virtualnetworks/v/subnets/app"
        );
        assert_eq!(subnets[0].address_prefix.as_deref(), Some("10.0.1.0/24"));
    }

    #[test]
    fn unit_falls_back_to_the_id_segment_with_original_casing_when_name_is_absent() {
        let resource = vnet(json!({
            "subnets": [{ "id": "/subscriptions/s/.../subnets/AppTier" }]
        }));
        let subnets = vnet_subnets(&resource);
        assert_eq!(subnets[0].name, "AppTier", "display text keeps its casing");
        assert!(subnets[0].address_prefix.is_none());
    }

    #[test]
    fn unit_skips_subnets_without_an_id_when_parsing() {
        let resource = vnet(json!({
            "subnets": [{ "name": "orphan" }, { "id": "/a/subnets/kept" }]
        }));
        let subnets = vnet_subnets(&resource);
        assert_eq!(subnets.len(), 1);
        assert_eq!(subnets[0].name, "kept");
    }

    #[test]
    fn unit_returns_no_subnets_when_the_vnet_declares_none() {
        assert!(vnet_subnets(&vnet(json!({}))).is_empty());
    }
}
