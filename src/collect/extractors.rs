//! Derive relationship edges by walking the properties JSON of collected
//! resources. Runs offline as a post-pass — no extra API calls.

use serde_json::Value;

use crate::model::{Edge, EdgeKind, Resource, normalize_arm_id};

/// All edges derivable from one resource: the per-type handler plus the
/// generic passes (child→parent containment, user-assigned identities) that
/// apply to every resource.
pub fn extract(resource: &Resource) -> Vec<Edge> {
    let mut edges = match resource.azure_type.as_str() {
        "microsoft.network/virtualnetworks" => vnet_edges(resource),
        "microsoft.network/networkinterfaces" => nic_edges(resource),
        "microsoft.compute/virtualmachines" => vm_edges(resource),
        "microsoft.compute/virtualmachinescalesets" => vmss_edges(resource),
        "microsoft.network/privateendpoints" => private_endpoint_edges(resource),
        "microsoft.network/publicipaddresses" => public_ip_edges(resource),
        "microsoft.network/privatednszones/virtualnetworklinks" => dns_link_edges(resource),
        "microsoft.network/loadbalancers" => load_balancer_edges(resource),
        "microsoft.network/applicationgateways" => app_gateway_edges(resource),
        "microsoft.web/sites" => site_edges(resource),
        "microsoft.containerservice/managedclusters" => aks_edges(resource),
        "microsoft.storage/storageaccounts" | "microsoft.keyvault/vaults" => {
            network_acl_edges(resource)
        }
        "microsoft.network/bastionhosts" => bastion_edges(resource),
        "microsoft.sqlvirtualmachine/sqlvirtualmachines" => sql_vm_edges(resource),
        "microsoft.insights/datacollectionrules" => data_collection_rule_edges(resource),
        "microsoft.operationsmanagement/solutions" | "microsoft.insights/components" => {
            workspace_link_edges(resource)
        }
        "microsoft.insights/scheduledqueryrules" | "microsoft.insights/metricalerts" => {
            scope_edges(resource)
        }
        "microsoft.eventgrid/systemtopics" => system_topic_edges(resource),
        _ => Vec::new(),
    };
    edges.extend(parent_edge(resource));
    edges.extend(identity_edges(resource));
    edges
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

/// A public IP's ipConfiguration id nests under the resource that holds it:
/// `.../networkInterfaces/<nic>/ipConfigurations/<name>` for NICs, but load
/// balancers and application gateways use `/frontendIPConfigurations/` — a
/// different segment, so both must be stripped to reach the owner.
fn public_ip_edges(pip: &Resource) -> Vec<Edge> {
    let props = properties(pip);
    let Some(ip_config) = id_at(props, &["ipConfiguration", "id"]) else {
        return Vec::new();
    };
    let owner = ip_config_owner(ip_config);
    vec![edge(&pip.display_id, owner, EdgeKind::AttachedTo, None)]
}

/// Strip the ip-configuration suffix from a nested config id to reach the
/// owning resource. Case-insensitive because ARM ids come back in any casing.
fn ip_config_owner(ip_config: &str) -> &str {
    let lower = ip_config.to_lowercase();
    for marker in [
        "/frontendipconfigurations/",
        "/bastionhostipconfigurations/",
        "/ipconfigurations/",
    ] {
        if let Some(pos) = lower.find(marker) {
            return &ip_config[..pos];
        }
    }
    ip_config
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

/// App Service: the plan it runs on and its VNet-integration subnet.
fn site_edges(site: &Resource) -> Vec<Edge> {
    let props = properties(site);
    let mut edges = Vec::new();
    if let Some(plan_id) = id_at(props, &["serverFarmId"]) {
        edges.push(edge(&site.display_id, plan_id, EdgeKind::RunsOn, None));
    }
    if let Some(subnet_id) = id_at(props, &["virtualNetworkSubnetId"]) {
        edges.push(edge(
            &site.display_id,
            subnet_id,
            EdgeKind::NicInSubnet,
            Some(serde_json::json!({ "via": "vnetIntegration" })),
        ));
    }
    edges
}

/// AKS: node-pool subnets and the addon Log Analytics workspace.
fn aks_edges(cluster: &Resource) -> Vec<Edge> {
    let props = properties(cluster);
    let mut edges = Vec::new();
    let mut seen_subnets = std::collections::BTreeSet::new();
    for pool in props
        .get("agentPoolProfiles")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(subnet_id) = id_at(pool, &["vnetSubnetID"])
            && seen_subnets.insert(normalize_arm_id(subnet_id))
        {
            edges.push(edge(
                &cluster.display_id,
                subnet_id,
                EdgeKind::NicInSubnet,
                None,
            ));
        }
    }
    if let Some(workspace) = id_at(
        props,
        &[
            "addonProfiles",
            "omsagent",
            "config",
            "logAnalyticsWorkspaceResourceID",
        ],
    ) {
        edges.push(edge(&cluster.display_id, workspace, EdgeKind::LogsTo, None));
    }
    edges
}

/// VM scale set: the subnets its NIC configurations place instances in.
fn vmss_edges(vmss: &Resource) -> Vec<Edge> {
    let props = properties(vmss);
    let mut edges = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for nic_config in props
        .get("virtualMachineProfile")
        .and_then(|p| p.get("networkProfile"))
        .and_then(|p| p.get("networkInterfaceConfigurations"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for ip_config in nic_config
            .get("properties")
            .and_then(|p| p.get("ipConfigurations"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(subnet_id) = id_at(ip_config, &["properties", "subnet", "id"])
                && seen.insert(normalize_arm_id(subnet_id))
            {
                edges.push(edge(
                    &vmss.display_id,
                    subnet_id,
                    EdgeKind::NicInSubnet,
                    None,
                ));
            }
        }
    }
    edges
}

/// Load balancer: backend-pool NICs and frontend public IPs.
fn load_balancer_edges(lb: &Resource) -> Vec<Edge> {
    let props = properties(lb);
    let mut edges = Vec::new();
    let mut seen_nics = std::collections::BTreeSet::new();
    for pool in props
        .get("backendAddressPools")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        for backend in pool
            .get("properties")
            .and_then(|p| p.get("backendIPConfigurations"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(config_id) = id_at(backend, &["id"]) {
                let nic_id = ip_config_owner(config_id);
                if seen_nics.insert(normalize_arm_id(nic_id)) {
                    edges.push(edge(nic_id, &lb.display_id, EdgeKind::AttachedTo, None));
                }
            }
        }
    }
    for frontend in props
        .get("frontendIPConfigurations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(pip_id) = id_at(frontend, &["properties", "publicIPAddress", "id"]) {
            edges.push(edge(pip_id, &lb.display_id, EdgeKind::AttachedTo, None));
        }
    }
    edges
}

/// Application gateway: the gateway subnet and frontend public IPs.
fn app_gateway_edges(agw: &Resource) -> Vec<Edge> {
    let props = properties(agw);
    let mut edges = Vec::new();
    let mut seen_subnets = std::collections::BTreeSet::new();
    for gateway_ip in props
        .get("gatewayIPConfigurations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(subnet_id) = id_at(gateway_ip, &["properties", "subnet", "id"])
            && seen_subnets.insert(normalize_arm_id(subnet_id))
        {
            edges.push(edge(
                &agw.display_id,
                subnet_id,
                EdgeKind::NicInSubnet,
                None,
            ));
        }
    }
    for frontend in props
        .get("frontendIPConfigurations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(pip_id) = id_at(frontend, &["properties", "publicIPAddress", "id"]) {
            edges.push(edge(pip_id, &agw.display_id, EdgeKind::AttachedTo, None));
        }
    }
    edges
}

/// Firewalled PaaS services (storage, key vault): the subnets their network
/// ACLs admit — the closest thing they expose to a network location.
fn network_acl_edges(service: &Resource) -> Vec<Edge> {
    let props = properties(service);
    let mut edges = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for rule in props
        .get("networkAcls")
        .and_then(|acls| acls.get("virtualNetworkRules"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(subnet_id) = id_at(rule, &["id"])
            && seen.insert(normalize_arm_id(subnet_id))
        {
            edges.push(edge(
                &service.display_id,
                subnet_id,
                EdgeKind::InVnet,
                Some(serde_json::json!({ "via": "networkAcl" })),
            ));
        }
    }
    edges
}

/// Bastion host: the subnet it serves from and its frontend public IP.
fn bastion_edges(bastion: &Resource) -> Vec<Edge> {
    let props = properties(bastion);
    let mut edges = Vec::new();
    for ip_config in props
        .get("ipConfigurations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(subnet_id) = id_at(ip_config, &["properties", "subnet", "id"]) {
            edges.push(edge(
                &bastion.display_id,
                subnet_id,
                EdgeKind::NicInSubnet,
                None,
            ));
        }
        if let Some(pip_id) = id_at(ip_config, &["properties", "publicIPAddress", "id"]) {
            edges.push(edge(
                pip_id,
                &bastion.display_id,
                EdgeKind::AttachedTo,
                None,
            ));
        }
    }
    edges
}

/// SQL VM registration: points at the VM it manages.
fn sql_vm_edges(sql_vm: &Resource) -> Vec<Edge> {
    let props = properties(sql_vm);
    let Some(vm_id) = id_at(props, &["virtualMachineResourceId"]) else {
        return Vec::new();
    };
    vec![edge(&sql_vm.display_id, vm_id, EdgeKind::RunsOn, None)]
}

/// Data collection rule: the Log Analytics workspaces it delivers to.
fn data_collection_rule_edges(rule: &Resource) -> Vec<Edge> {
    let props = properties(rule);
    let mut edges = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for destination in props
        .get("destinations")
        .and_then(|d| d.get("logAnalytics"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(workspace) = id_at(destination, &["workspaceResourceId"])
            && seen.insert(normalize_arm_id(workspace))
        {
            edges.push(edge(&rule.display_id, workspace, EdgeKind::LogsTo, None));
        }
    }
    edges
}

/// Solutions and Application Insights components: the workspace they live on.
/// App Insights spells the key `WorkspaceResourceId`; solutions use lowercase.
fn workspace_link_edges(resource: &Resource) -> Vec<Edge> {
    let props = properties(resource);
    let workspace =
        id_at(props, &["workspaceResourceId"]).or_else(|| id_at(props, &["WorkspaceResourceId"]));
    let Some(workspace) = workspace else {
        return Vec::new();
    };
    vec![edge(
        &resource.display_id,
        workspace,
        EdgeKind::LogsTo,
        None,
    )]
}

/// Alert rules: the resources their scopes watch.
fn scope_edges(rule: &Resource) -> Vec<Edge> {
    let props = properties(rule);
    let mut edges = Vec::new();
    for scope in props
        .get("scopes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(target) = scope.as_str().filter(|s| s.contains("/providers/")) {
            edges.push(edge(&rule.display_id, target, EdgeKind::Monitors, None));
        }
    }
    edges
}

/// Event Grid system topic: the resource whose events it surfaces.
fn system_topic_edges(topic: &Resource) -> Vec<Edge> {
    let props = properties(topic);
    let Some(source) = id_at(props, &["source"]) else {
        return Vec::new();
    };
    vec![edge(&topic.display_id, source, EdgeKind::Monitors, None)]
}

/// Child resources (`provider/parent/child` types, e.g. SQL databases) link to
/// the parent their ARM id nests under — one rule that connects every child
/// type without per-type handlers.
fn parent_edge(resource: &Resource) -> Option<Edge> {
    if resource.azure_type.matches('/').count() < 2 {
        return None;
    }
    let trimmed = resource
        .display_id
        .rsplitn(3, '/')
        .nth(2)
        .filter(|parent| parent.to_lowercase().contains("/providers/"))?;
    Some(edge(
        &resource.display_id,
        trimmed,
        EdgeKind::AttachedTo,
        None,
    ))
}

/// User-assigned managed identities referenced by any resource.
fn identity_edges(resource: &Resource) -> Vec<Edge> {
    let mut edges = Vec::new();
    for identity_id in resource
        .identity
        .as_ref()
        .and_then(|identity| identity.get("userAssignedIdentities"))
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|map| map.keys())
    {
        if identity_id.to_lowercase().contains("/providers/") {
            edges.push(edge(
                &resource.display_id,
                identity_id,
                EdgeKind::UsesIdentity,
                None,
            ));
        }
    }
    edges
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
            "microsoft.compute/virtualmachinescalesets",
            "microsoft.network/privateendpoints",
            "microsoft.network/publicipaddresses",
            "microsoft.network/privatednszones/virtualnetworklinks",
            "microsoft.network/loadbalancers",
            "microsoft.network/applicationgateways",
            "microsoft.web/sites",
            "microsoft.containerservice/managedclusters",
            "microsoft.storage/storageaccounts",
            "microsoft.keyvault/vaults",
            "microsoft.network/bastionhosts",
            "microsoft.insights/datacollectionrules",
            "microsoft.insights/components",
            "microsoft.insights/scheduledqueryrules",
            "microsoft.operationsmanagement/solutions",
            "microsoft.eventgrid/systemtopics",
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

    #[test]
    fn public_ip_edges_strip_frontend_ip_configuration_suffix_for_load_balancers() {
        let pip = resource(
            "microsoft.network/publicipaddresses",
            "/s1/rg/pip-lb",
            json!({"ipConfiguration": {"id": "/s1/rg/providers/Microsoft.Network/loadBalancers/lb-1/frontendIPConfigurations/fe1"}}),
        );

        let edges = extract(&pip);

        assert_eq!(
            edges[0].target_id,
            "/s1/rg/providers/microsoft.network/loadbalancers/lb-1"
        );
    }

    #[test]
    fn site_edges_link_plan_and_integration_subnet() {
        let site = resource(
            "microsoft.web/sites",
            "/s1/rg/providers/Microsoft.Web/sites/app-1",
            json!({
                "serverFarmId": "/s1/rg/providers/Microsoft.Web/serverfarms/Plan-1",
                "virtualNetworkSubnetId": "/s1/rg/providers/Microsoft.Network/virtualNetworks/vnet/subnets/integration"
            }),
        );

        let edges = extract(&site);

        assert_eq!(
            edges
                .iter()
                .map(|e| (e.kind, e.target_id.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (
                    EdgeKind::RunsOn,
                    "/s1/rg/providers/microsoft.web/serverfarms/plan-1"
                ),
                (
                    EdgeKind::NicInSubnet,
                    "/s1/rg/providers/microsoft.network/virtualnetworks/vnet/subnets/integration"
                ),
            ]
        );
    }

    #[test]
    fn aks_edges_dedupe_pool_subnets_and_link_the_workspace() {
        let cluster = resource(
            "microsoft.containerservice/managedclusters",
            "/s1/rg/providers/Microsoft.ContainerService/managedClusters/aks-1",
            json!({
                "agentPoolProfiles": [
                    {"vnetSubnetID": "/s1/rg/providers/Microsoft.Network/virtualNetworks/vnet/subnets/aks"},
                    {"vnetSubnetID": "/s1/rg/providers/Microsoft.Network/virtualNetworks/vnet/subnets/AKS"}
                ],
                "addonProfiles": {"omsagent": {"config": {"logAnalyticsWorkspaceResourceID": "/s1/rg/providers/Microsoft.OperationalInsights/workspaces/law-1"}}}
            }),
        );

        let edges = extract(&cluster);

        let kinds: Vec<_> = edges.iter().map(|e| e.kind).collect();
        assert_eq!(kinds, vec![EdgeKind::NicInSubnet, EdgeKind::LogsTo]);
    }

    #[test]
    fn vmss_edges_place_nic_configurations_in_subnets() {
        let vmss = resource(
            "microsoft.compute/virtualmachinescalesets",
            "/s1/rg/providers/Microsoft.Compute/virtualMachineScaleSets/vmss-1",
            json!({
                "virtualMachineProfile": {"networkProfile": {"networkInterfaceConfigurations": [
                    {"properties": {"ipConfigurations": [{"properties": {"subnet": {"id": "/s1/rg/providers/Microsoft.Network/virtualNetworks/vnet/subnets/pool"}}}]}}
                ]}}
            }),
        );

        let edges = extract(&vmss);

        assert_eq!(
            (edges[0].kind, edges[0].target_id.as_str()),
            (
                EdgeKind::NicInSubnet,
                "/s1/rg/providers/microsoft.network/virtualnetworks/vnet/subnets/pool"
            )
        );
    }

    #[test]
    fn load_balancer_edges_reach_backend_nics_and_frontend_pips() {
        let lb = resource(
            "microsoft.network/loadbalancers",
            "/s1/rg/providers/Microsoft.Network/loadBalancers/lb-1",
            json!({
                "backendAddressPools": [{"properties": {"backendIPConfigurations": [
                    {"id": "/s1/rg/providers/Microsoft.Network/networkInterfaces/nic-1/ipConfigurations/ipconfig1"}
                ]}}],
                "frontendIPConfigurations": [{"properties": {"publicIPAddress": {"id": "/s1/rg/providers/Microsoft.Network/publicIPAddresses/pip-1"}}}]
            }),
        );

        let edges = extract(&lb);

        assert_eq!(
            edges
                .iter()
                .map(|e| e.source_id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "/s1/rg/providers/microsoft.network/networkinterfaces/nic-1",
                "/s1/rg/providers/microsoft.network/publicipaddresses/pip-1",
            ]
        );
    }

    #[test]
    fn app_gateway_edges_link_subnet_and_frontend_pip() {
        let agw = resource(
            "microsoft.network/applicationgateways",
            "/s1/rg/providers/Microsoft.Network/applicationGateways/agw-1",
            json!({
                "gatewayIPConfigurations": [{"properties": {"subnet": {"id": "/s1/rg/providers/Microsoft.Network/virtualNetworks/vnet/subnets/agw"}}}],
                "frontendIPConfigurations": [{"properties": {"publicIPAddress": {"id": "/s1/rg/providers/Microsoft.Network/publicIPAddresses/pip-agw"}}}]
            }),
        );

        let edges = extract(&agw);

        let kinds: Vec<_> = edges.iter().map(|e| e.kind).collect();
        assert_eq!(kinds, vec![EdgeKind::NicInSubnet, EdgeKind::AttachedTo]);
    }

    #[test]
    fn network_acl_edges_admit_subnets_once_each() {
        let storage = resource(
            "microsoft.storage/storageaccounts",
            "/s1/rg/providers/Microsoft.Storage/storageAccounts/st1",
            json!({
                "networkAcls": {"virtualNetworkRules": [
                    {"id": "/s1/rg/providers/Microsoft.Network/virtualNetworks/vnet/subnets/app"},
                    {"id": "/s1/rg/providers/Microsoft.Network/virtualNetworks/vnet/subnets/APP"}
                ]}
            }),
        );

        let edges = extract(&storage);

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::InVnet);
    }

    #[test]
    fn parent_edge_links_child_resource_types_to_their_parent() {
        let db = resource(
            "microsoft.sql/servers/databases",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Sql/servers/SQL-1/databases/app-db",
            json!({}),
        );

        let edges = extract(&db);

        assert_eq!(
            (edges[0].kind, edges[0].target_id.as_str()),
            (
                EdgeKind::AttachedTo,
                "/subscriptions/s1/resourcegroups/rg/providers/microsoft.sql/servers/sql-1"
            )
        );
    }

    #[test]
    fn parent_edge_skips_top_level_types_and_short_ids() {
        let vm = resource(
            "microsoft.compute/virtualmachines",
            "/s1/rg/vm-1",
            json!({}),
        );
        assert!(extract(&vm).is_empty());

        let stub = resource("microsoft.sql/servers/databases", "/s1/db", json!({}));
        assert!(
            extract(&stub).is_empty(),
            "no /providers/ prefix to trim to"
        );
    }

    #[test]
    fn identity_edges_link_user_assigned_identities_from_any_type() {
        let mut app = resource(
            "microsoft.web/sites",
            "/s1/rg/providers/Microsoft.Web/sites/app-1",
            json!({}),
        );
        app.identity = Some(json!({
            "type": "UserAssigned",
            "userAssignedIdentities": {
                "/s1/rg/providers/Microsoft.ManagedIdentity/userAssignedIdentities/ID-1": {}
            }
        }));

        let edges = extract(&app);

        assert_eq!(
            (edges[0].kind, edges[0].target_id.as_str()),
            (
                EdgeKind::UsesIdentity,
                "/s1/rg/providers/microsoft.managedidentity/userassignedidentities/id-1"
            )
        );
    }

    #[test]
    fn bastion_edges_link_subnet_and_frontend_pip() {
        let bastion = resource(
            "microsoft.network/bastionhosts",
            "/s1/rg/providers/Microsoft.Network/bastionHosts/bast-1",
            json!({
                "ipConfigurations": [{"properties": {
                    "subnet": {"id": "/s1/rg/providers/Microsoft.Network/virtualNetworks/vnet/subnets/AzureBastionSubnet"},
                    "publicIPAddress": {"id": "/s1/rg/providers/Microsoft.Network/publicIPAddresses/pip-bast"}
                }}]
            }),
        );

        let edges = extract(&bastion);

        let kinds: Vec<_> = edges.iter().map(|e| e.kind).collect();
        assert_eq!(kinds, vec![EdgeKind::NicInSubnet, EdgeKind::AttachedTo]);
    }

    #[test]
    fn public_ip_edges_strip_bastion_host_ip_configuration_suffix() {
        let pip = resource(
            "microsoft.network/publicipaddresses",
            "/s1/rg/pip-bast",
            json!({"ipConfiguration": {"id": "/s1/rg/providers/Microsoft.Network/bastionHosts/bast-1/bastionHostIpConfigurations/IpConf"}}),
        );

        let edges = extract(&pip);

        assert_eq!(
            edges[0].target_id,
            "/s1/rg/providers/microsoft.network/bastionhosts/bast-1"
        );
    }

    #[test]
    fn sql_vm_edges_point_at_the_registered_vm() {
        let sql_vm = resource(
            "microsoft.sqlvirtualmachine/sqlvirtualmachines",
            "/s1/rg/providers/Microsoft.SqlVirtualMachine/sqlVirtualMachines/vm-sql-1",
            json!({"virtualMachineResourceId": "/s1/rg/providers/Microsoft.Compute/virtualMachines/VM-SQL-1"}),
        );

        let edges = extract(&sql_vm);

        assert_eq!(
            (edges[0].kind, edges[0].target_id.as_str()),
            (
                EdgeKind::RunsOn,
                "/s1/rg/providers/microsoft.compute/virtualmachines/vm-sql-1"
            )
        );
    }

    #[test]
    fn data_collection_rule_edges_dedupe_workspaces() {
        let rule = resource(
            "microsoft.insights/datacollectionrules",
            "/s1/rg/providers/Microsoft.Insights/dataCollectionRules/dcr-1",
            json!({"destinations": {"logAnalytics": [
                {"workspaceResourceId": "/s1/rg/providers/Microsoft.OperationalInsights/workspaces/law-1"},
                {"workspaceResourceId": "/s1/rg/providers/Microsoft.OperationalInsights/workspaces/LAW-1"}
            ]}}),
        );

        let edges = extract(&rule);

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::LogsTo);
    }

    #[test]
    fn workspace_link_edges_accept_both_key_spellings() {
        let solution = resource(
            "microsoft.operationsmanagement/solutions",
            "/s1/rg/providers/Microsoft.OperationsManagement/solutions/sol-1",
            json!({"workspaceResourceId": "/s1/rg/providers/Microsoft.OperationalInsights/workspaces/law-1"}),
        );
        let component = resource(
            "microsoft.insights/components",
            "/s1/rg/providers/Microsoft.Insights/components/appi-1",
            json!({"WorkspaceResourceId": "/s1/rg/providers/Microsoft.OperationalInsights/workspaces/law-1"}),
        );

        assert_eq!(extract(&solution)[0].kind, EdgeKind::LogsTo);
        assert_eq!(extract(&component)[0].kind, EdgeKind::LogsTo);
    }

    #[test]
    fn scope_edges_monitor_each_arm_scope_and_skip_subscription_scopes() {
        let rule = resource(
            "microsoft.insights/scheduledqueryrules",
            "/s1/rg/providers/Microsoft.Insights/scheduledQueryRules/alert-1",
            json!({"scopes": [
                "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.OperationalInsights/workspaces/law-1",
                "/subscriptions/s1"
            ]}),
        );

        let edges = extract(&rule);

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::Monitors);
    }

    #[test]
    fn system_topic_edges_monitor_the_source_resource() {
        let topic = resource(
            "microsoft.eventgrid/systemtopics",
            "/s1/rg/providers/Microsoft.EventGrid/systemTopics/topic-1",
            json!({"source": "/s1/rg/providers/Microsoft.Storage/storageAccounts/st1"}),
        );

        let edges = extract(&topic);

        assert_eq!(
            (edges[0].kind, edges[0].target_id.as_str()),
            (
                EdgeKind::Monitors,
                "/s1/rg/providers/microsoft.storage/storageaccounts/st1"
            )
        );
    }
}
