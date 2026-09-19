//! Derive relationship edges by walking the properties JSON of collected
//! resources. Runs offline as a post-pass — no extra API calls.

use serde_json::Value;

use crate::model::{Edge, EdgeKind, Resource, azure_types, normalize_arm_id};

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
        "microsoft.network/natgateways" => nat_gateway_edges(resource),
        "microsoft.network/azurefirewalls" => firewall_edges(resource),
        "microsoft.network/virtualnetworkgateways" => vnet_gateway_edges(resource),
        "microsoft.network/connections" => connection_edges(resource),
        "microsoft.app/containerapps" => container_app_edges(resource),
        "microsoft.app/managedenvironments" => managed_environment_edges(resource),
        "microsoft.dbforpostgresql/flexibleservers" | "microsoft.dbformysql/flexibleservers" => {
            flexible_server_edges(resource)
        }
        "microsoft.compute/diskencryptionsets" => disk_encryption_set_edges(resource),
        "microsoft.compute/disks" => disk_edges(resource),
        _ => Vec::new(),
    };
    edges.extend(parent_edge(resource));
    edges.extend(identity_edges(resource));
    edges
}

/// Every edge for a snapshot: the per-resource pass plus the relationships
/// that need a second resource to resolve (a scale set names its AKS cluster
/// only by tag, and the cluster's node resource group by property).
pub fn extract_all(resources: &[Resource]) -> Vec<Edge> {
    let mut edges: Vec<Edge> = resources.iter().flat_map(extract).collect();
    edges.extend(cross_resource_edges(resources));
    edges
}

/// VMSS → AKS cluster. AKS tags every node-pool scale set with
/// `aks-managed-clusterName`; the cluster's `nodeResourceGroup` must also
/// match, so two clusters with the same name in one subscription cannot claim
/// each other's pools.
fn cross_resource_edges(resources: &[Resource]) -> Vec<Edge> {
    let clusters: std::collections::BTreeMap<(String, String, String), &Resource> = resources
        .iter()
        .filter(|r| r.azure_type == "microsoft.containerservice/managedclusters")
        .filter_map(|cluster| {
            let node_group = id_at(properties(cluster), &["nodeResourceGroup"])?;
            Some((
                (
                    cluster.subscription_id.to_ascii_lowercase(),
                    node_group.to_ascii_lowercase(),
                    cluster.name.to_ascii_lowercase(),
                ),
                cluster,
            ))
        })
        .collect();
    if clusters.is_empty() {
        return Vec::new();
    }
    resources
        .iter()
        .filter(|r| r.azure_type == "microsoft.compute/virtualmachinescalesets")
        .filter_map(|vmss| {
            let cluster_name = vmss
                .tags
                .as_ref()
                .and_then(|tags| tags.get("aks-managed-clusterName"))
                .and_then(Value::as_str)?;
            let group = vmss.resource_group.as_deref()?;
            let cluster = clusters.get(&(
                vmss.subscription_id.to_ascii_lowercase(),
                group.to_ascii_lowercase(),
                cluster_name.to_ascii_lowercase(),
            ))?;
            Some(edge(
                &vmss.display_id,
                &cluster.display_id,
                EdgeKind::AttachedTo,
                Some(serde_json::json!({ "via": "aks-managed-clusterName" })),
            ))
        })
        .collect()
}

/// Backup vault → protected VM, from stored `backup_protected_items` rows.
/// Protected items are `recoveryservicesresources` rows, never resources, so
/// this reads the evidence table rather than the resource list. Offline.
pub fn evidence_edges(
    store: &crate::store::Store,
    snapshot_id: &str,
) -> Result<Vec<Edge>, crate::error::StoreError> {
    let mut edges = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for row in store.query_results(snapshot_id, "backup_protected_items")? {
        let Some(item_id) = row.get("id").and_then(Value::as_str) else {
            continue;
        };
        let Some(target) = row
            .get("resourceId")
            .and_then(Value::as_str)
            .filter(|s| s.to_lowercase().contains("/providers/"))
        else {
            continue;
        };
        let lower = item_id.to_lowercase();
        let Some(pos) = lower.find("/backupfabrics/") else {
            continue;
        };
        let vault = &item_id[..pos];
        if seen.insert((normalize_arm_id(vault), normalize_arm_id(target))) {
            edges.push(edge(
                vault,
                target,
                EdgeKind::Monitors,
                Some(serde_json::json!({ "via": "backup" })),
            ));
        }
    }
    Ok(edges)
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
    let mut seen_asgs = std::collections::BTreeSet::new();

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
        for asg in ip_config
            .get("properties")
            .and_then(|p| p.get("applicationSecurityGroups"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            if let Some(asg_id) = id_at(asg, &["id"])
                && seen_asgs.insert(normalize_arm_id(asg_id))
            {
                edges.push(edge(&nic.display_id, asg_id, EdgeKind::NsgAttached, None));
            }
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
    if let Some(avset) = id_at(props, &["availabilitySet", "id"]) {
        edges.push(edge(&vm.display_id, avset, EdgeKind::AttachedTo, None));
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
    // Azure creates a NIC per endpoint and returns it as its own ARG row, but
    // nothing linked the two: the NIC's own extractor only looks for a
    // `virtualMachine`. Without this edge the pair drew as two unrelated tiles
    // that happened to share a subnet, which is most of what made a
    // private-endpoint subnet unreadable.
    for nic in props
        .get("networkInterfaces")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(nic_id) = id_at(nic, &["id"]) {
            edges.push(edge(nic_id, &pe.display_id, EdgeKind::AttachedTo, None));
        }
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
    let mut edges = Vec::new();
    if let Some(ip_config) = id_at(props, &["ipConfiguration", "id"]) {
        let owner = ip_config_owner(ip_config);
        edges.push(edge(&pip.display_id, owner, EdgeKind::AttachedTo, None));
    }
    if let Some(nat) = id_at(props, &["natGateway", "id"]) {
        edges.push(edge(&pip.display_id, nat, EdgeKind::AttachedTo, None));
    }
    edges
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
    // Case-insensitive like `ip_config_owner`: a lowercased id must still
    // yield the zone, not the link itself.
    let lower = link.display_id.to_lowercase();
    let zone_id = lower
        .find("/virtualnetworklinks/")
        .map_or(link.display_id.as_str(), |pos| &link.display_id[..pos]);
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
            for pool in [
                "loadBalancerBackendAddressPools",
                "applicationGatewayBackendAddressPools",
            ]
            .into_iter()
            .flat_map(|key| {
                ip_config
                    .get("properties")
                    .and_then(|p| p.get(key))
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
            }) {
                if let Some(pool_id) = id_at(pool, &["id"]) {
                    let owner = backend_pool_owner(pool_id);
                    if seen.insert(normalize_arm_id(owner)) {
                        edges.push(edge(&vmss.display_id, owner, EdgeKind::AttachedTo, None));
                    }
                }
            }
        }
    }
    edges
}

/// `.../loadBalancers/lb/backendAddressPools/pool` → the load balancer (or
/// application gateway); case-insensitive like every id split here.
fn backend_pool_owner(pool_id: &str) -> &str {
    let lower = pool_id.to_lowercase();
    lower
        .find("/backendaddresspools/")
        .map_or(pool_id, |pos| &pool_id[..pos])
}

/// A backend ip-configuration id belongs to a NIC, or to a scale-set
/// instance NIC (`.../virtualMachineScaleSets/x/virtualMachines/0/networkInterfaces/n/...`).
/// Instance NICs are never stored resources, so the edge targets the scale
/// set itself rather than dangling.
fn backend_owner(config_id: &str) -> &str {
    let lower = config_id.to_lowercase();
    if lower.contains("/virtualmachinescalesets/")
        && let Some(pos) = lower.find("/virtualmachines/")
    {
        return &config_id[..pos];
    }
    ip_config_owner(config_id)
}

/// Backend pools → the NICs (or scale sets) behind them. Shared by load
/// balancers and application gateways, which expose the same shape.
fn backend_pool_nic_edges(props: &Value, owner_id: &str) -> Vec<Edge> {
    let mut edges = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
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
                let nic_id = backend_owner(config_id);
                if seen.insert(normalize_arm_id(nic_id)) {
                    edges.push(edge(nic_id, owner_id, EdgeKind::AttachedTo, None));
                }
            }
        }
    }
    edges
}

/// Load balancer: backend-pool NICs and frontend public IPs.
fn load_balancer_edges(lb: &Resource) -> Vec<Edge> {
    let props = properties(lb);
    let mut edges = backend_pool_nic_edges(props, &lb.display_id);
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

/// Application gateway: the gateway subnet, frontend public IPs, backend
/// NICs and the WAF policy it applies.
fn app_gateway_edges(agw: &Resource) -> Vec<Edge> {
    let props = properties(agw);
    let mut edges = backend_pool_nic_edges(props, &agw.display_id);
    if let Some(policy) = id_at(props, &["firewallPolicy", "id"]) {
        edges.push(edge(&agw.display_id, policy, EdgeKind::DependsOn, None));
    }
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

/// NAT gateway: the subnets it serves and the public IPs it uses.
fn nat_gateway_edges(nat: &Resource) -> Vec<Edge> {
    let props = properties(nat);
    let mut edges = Vec::new();
    for subnet in props
        .get("subnets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(subnet_id) = id_at(subnet, &["id"]) {
            edges.push(edge(subnet_id, &nat.display_id, EdgeKind::AttachedTo, None));
        }
    }
    for pip in ["publicIpAddresses", "publicIpPrefixes"]
        .into_iter()
        .flat_map(|key| {
            props
                .get(key)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
        })
    {
        if let Some(pip_id) = id_at(pip, &["id"]) {
            edges.push(edge(pip_id, &nat.display_id, EdgeKind::AttachedTo, None));
        }
    }
    edges
}

/// Azure Firewall: its subnet, public IPs and the policy it enforces.
fn firewall_edges(firewall: &Resource) -> Vec<Edge> {
    let props = properties(firewall);
    let mut edges = ip_configuration_edges(props, &firewall.display_id);
    if let Some(policy) = id_at(props, &["firewallPolicy", "id"]) {
        edges.push(edge(
            &firewall.display_id,
            policy,
            EdgeKind::DependsOn,
            None,
        ));
    }
    edges
}

/// VPN / ExpressRoute gateway: the gateway subnet and its public IPs.
fn vnet_gateway_edges(gateway: &Resource) -> Vec<Edge> {
    ip_configuration_edges(properties(gateway), &gateway.display_id)
}

/// `ipConfigurations[].properties.{subnet,publicIPAddress}` as Bastion,
/// firewalls and gateways all expose it.
fn ip_configuration_edges(props: &Value, owner_id: &str) -> Vec<Edge> {
    let mut edges = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for ip_config in props
        .get("ipConfigurations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(subnet_id) = id_at(ip_config, &["properties", "subnet", "id"])
            && seen.insert(normalize_arm_id(subnet_id))
        {
            edges.push(edge(owner_id, subnet_id, EdgeKind::NicInSubnet, None));
        }
        if let Some(pip_id) = id_at(ip_config, &["properties", "publicIPAddress", "id"]) {
            edges.push(edge(pip_id, owner_id, EdgeKind::AttachedTo, None));
        }
    }
    edges
}

/// Network connection: both gateways, the local network gateway, or the
/// ExpressRoute circuit it joins.
fn connection_edges(connection: &Resource) -> Vec<Edge> {
    let props = properties(connection);
    [
        "virtualNetworkGateway1",
        "virtualNetworkGateway2",
        "localNetworkGateway2",
        "peer",
    ]
    .into_iter()
    .filter_map(|key| id_at(props, &[key, "id"]))
    .map(|target| edge(&connection.display_id, target, EdgeKind::AttachedTo, None))
    .collect()
}

/// Container App: the managed environment it runs in.
fn container_app_edges(app: &Resource) -> Vec<Edge> {
    let props = properties(app);
    let environment =
        id_at(props, &["managedEnvironmentId"]).or_else(|| id_at(props, &["environmentId"]));
    let Some(environment) = environment else {
        return Vec::new();
    };
    vec![edge(&app.display_id, environment, EdgeKind::RunsOn, None)]
}

/// Container Apps environment: its infrastructure subnet.
fn managed_environment_edges(environment: &Resource) -> Vec<Edge> {
    let props = properties(environment);
    let Some(subnet) = id_at(props, &["vnetConfiguration", "infrastructureSubnetId"]) else {
        return Vec::new();
    };
    vec![edge(
        &environment.display_id,
        subnet,
        EdgeKind::NicInSubnet,
        Some(serde_json::json!({ "via": "infrastructureSubnet" })),
    )]
}

/// PostgreSQL / MySQL flexible server: the delegated subnet.
fn flexible_server_edges(server: &Resource) -> Vec<Edge> {
    let props = properties(server);
    let Some(subnet) = id_at(props, &["network", "delegatedSubnetResourceId"]) else {
        return Vec::new();
    };
    vec![edge(
        &server.display_id,
        subnet,
        EdgeKind::NicInSubnet,
        Some(serde_json::json!({ "via": "delegatedSubnet" })),
    )]
}

/// Disk encryption set: the key vault holding its key.
fn disk_encryption_set_edges(des: &Resource) -> Vec<Edge> {
    let props = properties(des);
    let Some(vault) = id_at(props, &["activeKey", "sourceVault", "id"]) else {
        return Vec::new();
    };
    vec![edge(&des.display_id, vault, EdgeKind::DependsOn, None)]
}

/// Managed disk: the encryption set that protects it.
fn disk_edges(disk: &Resource) -> Vec<Edge> {
    let props = properties(disk);
    let Some(des) = id_at(props, &["encryption", "diskEncryptionSetId"]) else {
        return Vec::new();
    };
    vec![edge(&disk.display_id, des, EdgeKind::DependsOn, None)]
}

/// Child resources (`provider/parent/child` types, e.g. SQL databases) link to
/// the parent their ARM id nests under — one rule that connects every child
/// type without per-type handlers.
fn parent_edge(resource: &Resource) -> Option<Edge> {
    if !azure_types::is_child_type(&resource.azure_type) {
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

    fn kinds(edges: &[Edge]) -> Vec<(EdgeKind, String)> {
        edges
            .iter()
            .map(|e| (e.kind, e.target_id.clone()))
            .collect()
    }

    #[test]
    fn unit_nat_gateway_edges_attach_subnets_and_pips() {
        let nat = resource(
            "microsoft.network/natgateways",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/natGateways/nat",
            json!({
                "subnets": [{"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/virtualNetworks/v/subnets/app"}],
                "publicIpAddresses": [{"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/publicIPAddresses/PIP"}]
            }),
        );

        let edges = extract(&nat);

        assert_eq!(edges.len(), 2);
        assert!(edges.iter().all(|e| e.kind == EdgeKind::AttachedTo));
        assert!(edges.iter().all(|e| e.target_id == nat.id));
        assert!(
            edges
                .iter()
                .any(|e| e.source_id.ends_with("/publicipaddresses/pip"))
        );
    }

    #[test]
    fn unit_firewall_edges_place_in_subnet_and_depend_on_policy() {
        let firewall = resource(
            "microsoft.network/azurefirewalls",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/azureFirewalls/fw",
            json!({
                "ipConfigurations": [{"properties": {
                    "subnet": {"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/virtualNetworks/hub/subnets/AzureFirewallSubnet"},
                    "publicIPAddress": {"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/publicIPAddresses/fw-pip"}
                }}],
                "firewallPolicy": {"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/firewallPolicies/policy"}
            }),
        );

        let edges = extract(&firewall);

        let kinds: Vec<_> = edges.iter().map(|e| e.kind).collect();
        assert_eq!(
            kinds,
            vec![
                EdgeKind::NicInSubnet,
                EdgeKind::AttachedTo,
                EdgeKind::DependsOn
            ]
        );
    }

    #[test]
    fn unit_connection_edges_attach_both_gateways_and_lng() {
        let connection = resource(
            "microsoft.network/connections",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/connections/c",
            json!({
                "virtualNetworkGateway1": {"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/virtualNetworkGateways/gw"},
                "localNetworkGateway2": {"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/localNetworkGateways/onprem"}
            }),
        );

        let edges = extract(&connection);

        assert_eq!(edges.len(), 2);
        assert!(
            edges
                .iter()
                .all(|e| e.kind == EdgeKind::AttachedTo && e.source_id == connection.id)
        );
    }

    #[test]
    fn unit_container_app_runs_on_environment_and_environment_sits_in_subnet() {
        let app = resource(
            "microsoft.app/containerapps",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.App/containerApps/api",
            json!({"managedEnvironmentId": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.App/managedEnvironments/env"}),
        );
        let environment = resource(
            "microsoft.app/managedenvironments",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.App/managedEnvironments/env",
            json!({"vnetConfiguration": {"infrastructureSubnetId": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/virtualNetworks/v/subnets/aca"}}),
        );

        assert_eq!(extract(&app)[0].kind, EdgeKind::RunsOn);
        let env_edges = extract(&environment);
        assert_eq!(env_edges[0].kind, EdgeKind::NicInSubnet);
        assert_eq!(
            env_edges[0].properties,
            Some(json!({"via": "infrastructureSubnet"}))
        );
    }

    #[test]
    fn unit_flexible_server_sits_in_delegated_subnet() {
        let server = resource(
            "microsoft.dbforpostgresql/flexibleservers",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.DBforPostgreSQL/flexibleServers/pg",
            json!({"network": {"delegatedSubnetResourceId": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/virtualNetworks/v/subnets/pg"}}),
        );

        let edges = extract(&server);

        assert_eq!(kinds(&edges), vec![(EdgeKind::NicInSubnet, "/subscriptions/s1/resourcegroups/rg/providers/microsoft.network/virtualnetworks/v/subnets/pg".to_owned())]);
    }

    #[test]
    fn unit_vmss_edges_reach_backend_pools_and_aks_cluster_by_tag() {
        let mut vmss = resource(
            "microsoft.compute/virtualmachinescalesets",
            "/subscriptions/s1/resourceGroups/MC_rg_aks_uksouth/providers/Microsoft.Compute/virtualMachineScaleSets/aks-nodepool1",
            json!({"virtualMachineProfile": {"networkProfile": {"networkInterfaceConfigurations": [{"properties": {"ipConfigurations": [{"properties": {
                "subnet": {"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/virtualNetworks/v/subnets/aks"},
                "loadBalancerBackendAddressPools": [{"id": "/subscriptions/s1/resourceGroups/MC_rg_aks_uksouth/providers/Microsoft.Network/loadBalancers/kubernetes/backendAddressPools/pool"}]
            }}]}}]}}}),
        );
        vmss.resource_group = Some("mc_rg_aks_uksouth".into());
        vmss.tags = Some(json!({"aks-managed-clusterName": "aks"}));
        let cluster = resource(
            "microsoft.containerservice/managedclusters",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.ContainerService/managedClusters/aks",
            json!({"nodeResourceGroup": "MC_rg_aks_uksouth"}),
        );

        let edges = extract_all(&[cluster.clone(), vmss.clone()]);

        let targets: Vec<_> = edges
            .iter()
            .filter(|e| e.source_id == vmss.id)
            .map(|e| e.target_id.as_str())
            .collect();
        assert!(targets.contains(&"/subscriptions/s1/resourcegroups/mc_rg_aks_uksouth/providers/microsoft.network/loadbalancers/kubernetes"), "{targets:?}");
        assert!(targets.contains(&cluster.id.as_str()), "{targets:?}");
    }

    #[test]
    fn unit_nic_edges_attach_application_security_groups_once() {
        let nic = resource(
            "microsoft.network/networkinterfaces",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/networkInterfaces/nic",
            json!({"ipConfigurations": [
                {"properties": {"applicationSecurityGroups": [{"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/applicationSecurityGroups/web"}]}},
                {"properties": {"applicationSecurityGroups": [{"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/applicationSecurityGroups/WEB"}]}}
            ]}),
        );

        let edges = extract(&nic);

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::NsgAttached);
    }

    #[test]
    fn unit_disk_encryption_chain_uses_depends_on() {
        let des = resource(
            "microsoft.compute/diskencryptionsets",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Compute/diskEncryptionSets/des",
            json!({"activeKey": {"sourceVault": {"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.KeyVault/vaults/kv"}}}),
        );
        let disk = resource(
            "microsoft.compute/disks",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Compute/disks/os",
            json!({"encryption": {"diskEncryptionSetId": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Compute/diskEncryptionSets/des"}}),
        );

        assert_eq!(extract(&des)[0].kind, EdgeKind::DependsOn);
        assert_eq!(extract(&disk)[0].target_id, des.id);
    }

    #[test]
    fn unit_vm_edges_attach_availability_set() {
        let vm = resource(
            "microsoft.compute/virtualmachines",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Compute/virtualMachines/vm",
            json!({"availabilitySet": {"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Compute/availabilitySets/AS"}}),
        );

        let edges = extract(&vm);

        assert_eq!(
            edges[0].target_id,
            "/subscriptions/s1/resourcegroups/rg/providers/microsoft.compute/availabilitysets/as"
        );
    }

    #[test]
    fn unit_dns_link_edges_split_zone_when_link_segment_is_lowercase() {
        let mut link = resource(
            "microsoft.network/privatednszones/virtualnetworklinks",
            "/subscriptions/s1/resourcegroups/rg/providers/microsoft.network/privatednszones/zone/virtualnetworklinks/link",
            json!({"virtualNetwork": {"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/virtualNetworks/v"}}),
        );
        link.display_id = link.id.clone();

        let edges = extract(&link);

        let dns: Vec<_> = edges
            .iter()
            .filter(|e| e.kind == EdgeKind::DnsLinked)
            .collect();
        assert_eq!(
            dns[0].source_id,
            "/subscriptions/s1/resourcegroups/rg/providers/microsoft.network/privatednszones/zone"
        );
    }

    #[test]
    fn unit_load_balancer_edges_attach_scale_set_not_instance_nic() {
        let lb = resource(
            "microsoft.network/loadbalancers",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/loadBalancers/lb",
            json!({"backendAddressPools": [{"properties": {"backendIPConfigurations": [
                {"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Compute/virtualMachineScaleSets/vmss/virtualMachines/0/networkInterfaces/nic/ipConfigurations/ip"},
                {"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Compute/virtualMachineScaleSets/vmss/virtualMachines/1/networkInterfaces/nic/ipConfigurations/ip"}
            ]}}]}),
        );

        let edges = extract(&lb);

        assert_eq!(edges.len(), 1);
        assert_eq!(
            edges[0].source_id,
            "/subscriptions/s1/resourcegroups/rg/providers/microsoft.compute/virtualmachinescalesets/vmss"
        );
    }

    #[test]
    fn unit_app_gateway_edges_reach_backend_nics_and_waf_policy() {
        let agw = resource(
            "microsoft.network/applicationgateways",
            "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/applicationGateways/agw",
            json!({
                "backendAddressPools": [{"properties": {"backendIPConfigurations": [{"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/networkInterfaces/web-nic/ipConfigurations/ipconfig1"}]}}],
                "firewallPolicy": {"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Network/ApplicationGatewayWebApplicationFirewallPolicies/waf"}
            }),
        );

        let edges = extract(&agw);

        assert!(
            edges
                .iter()
                .any(|e| e.kind == EdgeKind::AttachedTo && e.source_id.ends_with("/web-nic"))
        );
        assert!(
            edges
                .iter()
                .any(|e| e.kind == EdgeKind::DependsOn && e.target_id.ends_with("/waf"))
        );
    }

    #[test]
    fn unit_evidence_edges_link_vault_to_protected_vm() {
        let store = crate::store::Store::open_in_memory().unwrap();
        let id = store.create_snapshot("t", None).unwrap().id;
        store
            .insert_query_results(
                &id,
                "backup_protected_items",
                &[
                    json!({"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.RecoveryServices/vaults/rsv/backupFabrics/Azure/protectionContainers/c/protectedItems/i",
                           "resourceId": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.Compute/virtualMachines/VM"}),
                    json!({"id": "/subscriptions/s1/resourceGroups/rg/providers/Microsoft.DataProtection/backupVaults/bv/backupInstances/x", "resourceId": "not-an-arm-id"}),
                ],
            )
            .unwrap();

        let edges = evidence_edges(&store, &id).unwrap();

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].kind, EdgeKind::Monitors);
        assert_eq!(
            edges[0].source_id,
            "/subscriptions/s1/resourcegroups/rg/providers/microsoft.recoveryservices/vaults/rsv"
        );
        assert_eq!(
            edges[0].target_id,
            "/subscriptions/s1/resourcegroups/rg/providers/microsoft.compute/virtualmachines/vm"
        );
    }
}
