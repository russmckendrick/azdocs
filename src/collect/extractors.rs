//! Derive relationship edges by walking the properties JSON of collected
//! resources. Runs offline as a post-pass — no extra API calls.

use serde_json::Value;

use crate::model::{Edge, EdgeKind, Resource, normalize_arm_id};

/// All edges derivable from one resource.
pub fn extract(resource: &Resource) -> Vec<Edge> {
    match resource.azure_type.as_str() {
        "microsoft.network/virtualnetworks" => vnet_edges(resource),
        "microsoft.network/networkinterfaces" => nic_edges(resource),
        "microsoft.compute/virtualmachines" => vm_edges(resource),
        "microsoft.network/privateendpoints" => private_endpoint_edges(resource),
        "microsoft.network/publicipaddresses" => public_ip_edges(resource),
        "microsoft.network/privatednszones/virtualnetworklinks" => dns_link_edges(resource),
        _ => Vec::new(),
    }
}

fn properties(resource: &Resource) -> &Value {
    resource.properties.as_ref().unwrap_or(&Value::Null)
}

fn id_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(key)?;
    }
    current.as_str().filter(|s| !s.is_empty())
}

fn edge(source: &str, target: &str, kind: EdgeKind, properties: Option<Value>) -> Edge {
    Edge {
        source_id: normalize_arm_id(source),
        target_id: normalize_arm_id(target),
        kind,
        properties,
    }
}

/// Subnets (and their NSG associations) plus peerings.
fn vnet_edges(vnet: &Resource) -> Vec<Edge> {
    let mut edges = Vec::new();
    let props = properties(vnet);

    for subnet in props
        .get("subnets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(subnet_id) = id_at(subnet, &["id"]) else {
            continue;
        };
        edges.push(edge(subnet_id, &vnet.display_id, EdgeKind::SubnetOf, None));
        if let Some(nsg_id) = id_at(subnet, &["properties", "networkSecurityGroup", "id"]) {
            edges.push(edge(subnet_id, nsg_id, EdgeKind::NsgAttached, None));
        }
        if let Some(route_table) = id_at(subnet, &["properties", "routeTable", "id"]) {
            edges.push(edge(subnet_id, route_table, EdgeKind::AttachedTo, None));
        }
    }

    for peering in props
        .get("virtualNetworkPeerings")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(remote) = id_at(peering, &["properties", "remoteVirtualNetwork", "id"]) else {
            continue;
        };
        let state = id_at(peering, &["properties", "peeringState"])
            .map(|s| serde_json::json!({ "state": s }));
        edges.push(edge(&vnet.display_id, remote, EdgeKind::PeeredWith, state));
    }
    edges
}

/// Subnet placement, NSG, attached VM, and public IPs.
fn nic_edges(nic: &Resource) -> Vec<Edge> {
    let mut edges = Vec::new();
    let props = properties(nic);

    if let Some(vm_id) = id_at(props, &["virtualMachine", "id"]) {
        edges.push(edge(&nic.display_id, vm_id, EdgeKind::AttachedTo, None));
    }
    if let Some(nsg_id) = id_at(props, &["networkSecurityGroup", "id"]) {
        edges.push(edge(&nic.display_id, nsg_id, EdgeKind::NsgAttached, None));
    }
    for ip_config in props
        .get("ipConfigurations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(subnet_id) = id_at(ip_config, &["properties", "subnet", "id"]) {
            edges.push(edge(
                &nic.display_id,
                subnet_id,
                EdgeKind::NicInSubnet,
                None,
            ));
        }
        if let Some(pip_id) = id_at(ip_config, &["properties", "publicIPAddress", "id"]) {
            edges.push(edge(pip_id, &nic.display_id, EdgeKind::AttachedTo, None));
        }
    }
    edges
}

fn vm_edges(vm: &Resource) -> Vec<Edge> {
    let props = properties(vm);
    let mut edges = Vec::new();
    for nic in props
        .get("networkProfile")
        .and_then(|np| np.get("networkInterfaces"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(nic_id) = id_at(nic, &["id"]) {
            edges.push(edge(nic_id, &vm.display_id, EdgeKind::AttachedTo, None));
        }
    }
    if let Some(os_disk) = id_at(props, &["storageProfile", "osDisk", "managedDisk", "id"]) {
        edges.push(edge(os_disk, &vm.display_id, EdgeKind::AttachedTo, None));
    }
    for data_disk in props
        .get("storageProfile")
        .and_then(|sp| sp.get("dataDisks"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(disk_id) = id_at(data_disk, &["managedDisk", "id"]) {
            edges.push(edge(disk_id, &vm.display_id, EdgeKind::AttachedTo, None));
        }
    }
    edges
}

/// The private-link target and the subnet the endpoint lives in.
fn private_endpoint_edges(pe: &Resource) -> Vec<Edge> {
    let mut edges = Vec::new();
    let props = properties(pe);

    if let Some(subnet_id) = id_at(props, &["subnet", "id"]) {
        edges.push(edge(&pe.display_id, subnet_id, EdgeKind::NicInSubnet, None));
    }
    for connection in props
        .get("privateLinkServiceConnections")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .chain(
            props
                .get("manualPrivateLinkServiceConnections")
                .and_then(Value::as_array)
                .into_iter()
                .flatten(),
        )
    {
        if let Some(target) = id_at(connection, &["properties", "privateLinkServiceId"]) {
            edges.push(edge(
                &pe.display_id,
                target,
                EdgeKind::PrivateEndpointFor,
                None,
            ));
        }
    }
    edges
}

/// A public IP's ipConfiguration id nests under the NIC (or LB frontend) that
/// holds it: `.../networkInterfaces/<nic>/ipConfigurations/<name>`.
fn public_ip_edges(pip: &Resource) -> Vec<Edge> {
    let props = properties(pip);
    let Some(ip_config) = id_at(props, &["ipConfiguration", "id"]) else {
        return Vec::new();
    };
    let owner = ip_config
        .split("/ipConfigurations/")
        .next()
        .unwrap_or(ip_config);
    vec![edge(&pip.display_id, owner, EdgeKind::AttachedTo, None)]
}

/// Zone link child resource: parent zone is the id up to the link segment.
fn dns_link_edges(link: &Resource) -> Vec<Edge> {
    let props = properties(link);
    let Some(vnet_id) = id_at(props, &["virtualNetwork", "id"]) else {
        return Vec::new();
    };
    let zone_id = link
        .display_id
        .split("/virtualNetworkLinks/")
        .next()
        .unwrap_or(&link.display_id);
    vec![edge(zone_id, vnet_id, EdgeKind::DnsLinked, None)]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn resource(azure_type: &str, id: &str, properties: Value) -> Resource {
        Resource {
            id: normalize_arm_id(id),
            display_id: id.to_owned(),
            name: id.rsplit('/').next().unwrap_or("r").to_owned(),
            azure_type: azure_type.to_owned(),
            kind: None,
            location: Some("uksouth".to_owned()),
            resource_group: Some("rg".to_owned()),
            subscription_id: "s1".to_owned(),
            tags: None,
            sku: None,
            identity: None,
            properties: Some(properties),
        }
    }

    #[test]
    fn vnet_edges_extract_subnets_nsgs_and_peerings() {
        let vnet = resource(
            "microsoft.network/virtualnetworks",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/virtualNetworks/VNet-Hub",
            json!({
                "subnets": [{
                    "id": "/subscriptions/s1/resourceGroups/RG/providers/Microsoft.Network/virtualNetworks/VNet-Hub/subnets/App",
                    "properties": {"networkSecurityGroup": {"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/networkSecurityGroups/NSG-1"}}
                }],
                "virtualNetworkPeerings": [{
                    "properties": {
                        "remoteVirtualNetwork": {"id": "/subscriptions/S2/resourceGroups/rg/providers/Microsoft.Network/virtualNetworks/vnet-spoke"},
                        "peeringState": "Connected"
                    }
                }]
            }),
        );

        let edges = extract(&vnet);

        let kinds: Vec<_> = edges.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![
                EdgeKind::SubnetOf,
                EdgeKind::NsgAttached,
                EdgeKind::PeeredWith
            ],
            "edges: {edges:?}"
        );
    }

    #[test]
    fn vnet_edges_lowercase_cross_subscription_peering_ids() {
        let vnet = resource(
            "microsoft.network/virtualnetworks",
            "/subscriptions/s1/rg/vnet-a",
            json!({
                "virtualNetworkPeerings": [{
                    "properties": {"remoteVirtualNetwork": {"id": "/Subscriptions/S2/ResourceGroups/RG/providers/Microsoft.Network/virtualNetworks/VNET-B"}}
                }]
            }),
        );

        let edges = extract(&vnet);

        assert_eq!(
            edges[0].target_id,
            "/subscriptions/s2/resourcegroups/rg/providers/microsoft.network/virtualnetworks/vnet-b"
        );
    }

    #[test]
    fn nic_edges_extract_vm_subnet_nsg_and_public_ip() {
        let nic = resource(
            "microsoft.network/networkinterfaces",
            "/s1/rg/nic-1",
            json!({
                "virtualMachine": {"id": "/s1/rg/vm-1"},
                "networkSecurityGroup": {"id": "/s1/rg/nsg-1"},
                "ipConfigurations": [{
                    "properties": {
                        "subnet": {"id": "/s1/rg/vnet/subnets/app"},
                        "publicIPAddress": {"id": "/s1/rg/pip-1"}
                    }
                }]
            }),
        );

        let edges = extract(&nic);

        assert_eq!(edges.len(), 4, "edges: {edges:?}");
    }

    #[test]
    fn extractors_survive_null_and_missing_properties() {
        for azure_type in [
            "microsoft.network/virtualnetworks",
            "microsoft.network/networkinterfaces",
            "microsoft.compute/disks",
            "microsoft.compute/virtualmachines",
            "microsoft.network/privateendpoints",
            "microsoft.network/publicipaddresses",
            "microsoft.network/privatednszones/virtualnetworklinks",
        ] {
            let mut r = resource(azure_type, "/s1/rg/x", json!({}));
            assert!(extract(&r).is_empty(), "{azure_type} with empty properties");
            r.properties = None;
            assert!(extract(&r).is_empty(), "{azure_type} with null properties");
            let r = resource(
                azure_type,
                "/s1/rg/x",
                json!({"subnets": null, "ipConfigurations": [{}], "virtualNetworkPeerings": [{"properties": {}}]}),
            );
            let _ = extract(&r);
        }
    }

    #[test]
    fn public_ip_edges_strip_ip_configuration_suffix() {
        let pip = resource(
            "microsoft.network/publicipaddresses",
            "/s1/rg/pip-1",
            json!({"ipConfiguration": {"id": "/s1/rg/providers/Microsoft.Network/networkInterfaces/nic-1/ipConfigurations/ipconfig1"}}),
        );

        let edges = extract(&pip);

        assert_eq!(
            edges[0].target_id,
            "/s1/rg/providers/microsoft.network/networkinterfaces/nic-1"
        );
    }

    #[test]
    fn dns_link_edges_join_zone_to_vnet() {
        let link = resource(
            "microsoft.network/privatednszones/virtualnetworklinks",
            "/s1/rg/providers/Microsoft.Network/privateDnsZones/zone1/virtualNetworkLinks/link1",
            json!({"virtualNetwork": {"id": "/s1/rg/vnet-1"}}),
        );

        let edges = extract(&link);

        assert_eq!(
            (edges[0].source_id.as_str(), edges[0].kind),
            (
                "/s1/rg/providers/microsoft.network/privatednszones/zone1",
                EdgeKind::DnsLinked
            )
        );
    }

    #[test]
    fn vm_edges_attach_os_and_data_disks() {
        let vm = resource(
            "microsoft.compute/virtualmachines",
            "/s1/rg/vm-1",
            json!({
                "storageProfile": {
                    "osDisk": {"managedDisk": {"id": "/s1/rg/disk-os"}},
                    "dataDisks": [{"managedDisk": {"id": "/s1/rg/disk-data"}}]
                }
            }),
        );

        let edges = extract(&vm);

        assert_eq!(edges.len(), 2);
    }
}
