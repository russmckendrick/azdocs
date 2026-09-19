//! Canonical fixture estate shared by report/diagram golden tests: two
//! subscriptions, peered hub/spoke VNets, a VM with NIC + public IP, storage
//! with findings, a private endpoint, a SQL server with a database child, a
//! web app on its plan with a user-assigned identity, a Log Analytics
//! workspace, an AVD host pool, and a detached NSG.

use azdocs::collect::{audit, extractors, ingest};
use azdocs::querypack::QueryPack;
use azdocs::store::Store;
use serde_json::{Value, json};

/// Which collection the fixture describes. `Older` is the same estate one
/// collect earlier: no private endpoint or SQL server yet, a smaller VM, a
/// storage account that still blocked public blobs, and a test disk that
/// has since been deleted. Seeding it before `Current` gives the goldens a
/// real "changes since the previous snapshot".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vintage {
    Older,
    Current,
}

#[allow(dead_code)]
pub fn seed_estate(store: &Store) -> String {
    seed_estate_at(store, Vintage::Current)
}

/// The older collection, then the current one; returns the current id.
#[allow(dead_code)]
pub fn seed_history(store: &Store) -> String {
    seed_estate_at(store, Vintage::Older);
    seed_estate_at(store, Vintage::Current)
}

#[allow(dead_code)]
pub fn seed_estate_at(store: &Store, vintage: Vintage) -> String {
    let snapshot = store
        .create_snapshot("fixture-tenant", Some("golden fixture"))
        .unwrap();
    let pack = QueryPack::builtin().unwrap();
    let ingest_rows = |name: &str, rows: &[Value]| {
        ingest::ingest(store, &snapshot.id, pack.get(name).unwrap(), rows).unwrap();
    };

    ingest_rows(
        "subscriptions",
        &[
            json!({"subscriptionId": "sub-prod", "name": "Production", "state": "Enabled"}),
            json!({"subscriptionId": "sub-dev", "name": "Development", "state": "Enabled"}),
        ],
    );
    ingest_rows(
        "resource_groups",
        &[
            json!({"id": "/subscriptions/sub-prod/resourceGroups/rg-network", "name": "rg-network", "subscriptionId": "sub-prod", "location": "uksouth"}),
            json!({"id": "/subscriptions/sub-prod/resourceGroups/rg-app", "name": "rg-app", "subscriptionId": "sub-prod", "location": "uksouth", "tags": {"env": "prod"}}),
            json!({"id": "/subscriptions/sub-prod/resourceGroups/MC_rg-app_aks-prod_uksouth", "name": "MC_rg-app_aks-prod_uksouth", "subscriptionId": "sub-prod", "location": "uksouth"}),
            json!({"id": "/subscriptions/sub-dev/resourceGroups/rg-dev", "name": "rg-dev", "subscriptionId": "sub-dev", "location": "ukwest"}),
        ],
    );
    ingest_rows("all_resources", &estate_resources(vintage));
    seed_microsoft_evidence(store, &snapshot.id);
    seed_operational_evidence(store, &snapshot.id);
    ingest_rows(
        "virtual_networks",
        &[
            json!({"id": "/subscriptions/sub-prod/resourcegroups/rg-network/providers/microsoft.network/virtualnetworks/vnet-hub",
                   "name": "vnet-hub", "location": "uksouth", "resourceGroup": "rg-network", "subscriptionId": "sub-prod",
                   "addressPrefixes": ["10.0.0.0/16"], "dnsServers": [], "subnetCount": 2}),
            json!({"id": "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.network/virtualnetworks/vnet-app",
                   "name": "vnet-app", "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod",
                   "addressPrefixes": ["10.1.0.0/16"], "dnsServers": [], "subnetCount": 1}),
        ],
    );
    ingest_rows(
        "resource_type_counts",
        &[
            json!({"type": "microsoft.network/virtualnetworks", "resourceCount": 2}),
            json!({"type": "microsoft.compute/virtualmachines", "resourceCount": 1}),
        ],
    );
    ingest_rows(
        "log_analytics_workspaces",
        &[
            json!({"id": "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.operationalinsights/workspaces/law-prod",
                   "name": "law-prod", "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod",
                   "skuName": "PerGB2018", "retentionInDays": 30,
                   "publicNetworkAccessForIngestion": "Enabled", "publicNetworkAccessForQuery": "Enabled"}),
        ],
    );
    ingest_rows(
        "avd_host_pools",
        &[
            json!({"id": "/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.desktopvirtualization/hostpools/hp-prod",
                   "name": "hp-prod", "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod",
                   "hostPoolType": "Pooled", "loadBalancerType": "BreadthFirst", "maxSessionLimit": 10,
                   "preferredAppGroupType": "Desktop"}),
        ],
    );
    if vintage == Vintage::Current {
        ingest_rows(
            "storage_public_blob_access",
            &[
                json!({"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Storage/storageAccounts/stprodapp01",
                     "name": "stprodapp01", "resourceGroup": "rg-app", "subscriptionId": "sub-prod",
                     "summary": "stprodapp01 allows public blob access"}),
            ],
        );
    } else {
        ingest_rows(
            "orphaned_resources",
            &[
                json!({"id": "/subscriptions/sub-dev/resourceGroups/rg-dev/providers/Microsoft.Compute/disks/disk-old-test",
                     "name": "disk-old-test", "resourceGroup": "rg-dev", "subscriptionId": "sub-dev",
                     "summary": "disk-old-test is an unattached managed disk"}),
            ],
        );
    }
    for (query, row) in finding_rows() {
        ingest_rows(query, &[row]);
    }
    ingest_rows(
        "backup_protected_items",
        &[
            json!({"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.RecoveryServices/vaults/rsv-prod/backupFabrics/Azure/protectionContainers/iaasvmcontainer;iaasvmcontainerv2;rg-app;vm-app-01/protectedItems/vm;iaasvmcontainerv2;rg-app;vm-app-01",
                   "name": "vm;iaasvmcontainerv2;rg-app;vm-app-01", "subscriptionId": "sub-prod", "resourceGroup": "rg-app",
                   "vaultName": "rsv-prod", "friendlyName": "vm-app-01", "protectionState": "Protected",
                   "lastBackupStatus": "Completed", "lastBackupTime": "2026-09-12T02:00:00Z", "policyName": "DailyPolicy",
                   "resourceId": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Compute/virtualMachines/vm-app-01"}),
        ],
    );
    ingest_rows(
        "nsg_open_to_internet",
        &[
            json!({"id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/networkSecurityGroups/nsg-app",
                 "name": "nsg-app", "resourceGroup": "rg-network", "subscriptionId": "sub-prod",
                 "ruleName": "allow-ssh", "port": "22", "priority": 100,
                 "summary": "nsg-app: rule allow-ssh allows Internet -> port 22"}),
        ],
    );
    ingest_rows(
        "web_app_https_only_disabled",
        &[
            json!({"id": "/subscriptions/sub-dev/resourceGroups/rg-dev/providers/Microsoft.Web/sites/web-dev",
                 "name": "web-dev", "resourceGroup": "rg-dev", "subscriptionId": "sub-dev",
                 "summary": "web-dev does not enforce HTTPS-only traffic"}),
        ],
    );
    ingest_rows(
        "unassociated_nsgs",
        &[
            json!({"id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/networkSecurityGroups/nsg-unused",
                 "name": "nsg-unused", "resourceGroup": "rg-network", "subscriptionId": "sub-prod",
                 "summary": "nsg-unused is not associated with any subnet or NIC"}),
        ],
    );

    // Same post-passes the collect runner performs.
    let resources = store.resources(&snapshot.id).unwrap();
    let mut edges = extractors::extract_all(&resources);
    edges.extend(extractors::evidence_edges(store, &snapshot.id).unwrap());
    store.insert_edges(&snapshot.id, &edges).unwrap();
    let tag_findings = audit::missing_required_tags(&resources, &["env".to_owned()]);
    store.insert_findings(&snapshot.id, &tag_findings).unwrap();
    store
        .set_snapshot_status(&snapshot.id, azdocs::model::SnapshotStatus::Complete)
        .unwrap();
    snapshot.id
}

/// One row per finding query the pack gained in the query-pack expansion,
/// so every check the assessment explains has an occurrence to show.
fn finding_rows() -> Vec<(&'static str, Value)> {
    let prod = |group: &str, provider: &str, name: &str| {
        format!("/subscriptions/sub-prod/resourceGroups/{group}/providers/{provider}/{name}")
    };
    let dev = |provider: &str, name: &str| {
        format!("/subscriptions/sub-dev/resourceGroups/rg-dev/providers/{provider}/{name}")
    };
    vec![
        (
            "nsg_management_ports_open",
            json!({"id": prod("rg-network", "Microsoft.Network/networkSecurityGroups", "nsg-app"), "name": "nsg-app", "resourceGroup": "rg-network", "subscriptionId": "sub-prod", "ruleName": "allow-ssh", "protocol": "Tcp", "summary": "nsg-app: rule allow-ssh opens management port 22 to the Internet"}),
        ),
        (
            "storage_public_network_access",
            json!({"id": prod("rg-app", "Microsoft.Storage/storageAccounts", "stprodapp01"), "name": "stprodapp01", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "summary": "stprodapp01 allows public network access"}),
        ),
        (
            "storage_shared_key_access",
            json!({"id": prod("rg-app", "Microsoft.Storage/storageAccounts", "stprodapp01"), "name": "stprodapp01", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "summary": "stprodapp01 allows shared key access"}),
        ),
        (
            "sql_weak_tls",
            json!({"id": prod("rg-app", "Microsoft.Sql/servers", "sql-prod"), "name": "sql-prod", "type": "microsoft.sql/servers", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "minimalTlsVersion": "1.0", "summary": "sql-prod accepts TLS 1.0"}),
        ),
        (
            "sql_entra_only_auth_off",
            json!({"id": prod("rg-app", "Microsoft.Sql/servers", "sql-prod"), "name": "sql-prod", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "summary": "sql-prod does not require Entra-only authentication"}),
        ),
        (
            "redis_insecure_transport",
            json!({"id": dev("Microsoft.Cache/Redis", "redis-dev"), "name": "redis-dev", "resourceGroup": "rg-dev", "subscriptionId": "sub-dev", "nonSslPort": true, "minTlsVersion": "1.0", "summary": "redis-dev accepts non-SSL connections"}),
        ),
        (
            "local_auth_enabled",
            json!({"id": prod("rg-app", "Microsoft.CognitiveServices/accounts", "oai-prod"), "name": "oai-prod", "type": "microsoft.cognitiveservices/accounts", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "summary": "oai-prod accepts local (key) authentication"}),
        ),
        (
            "aks_local_accounts_enabled",
            json!({"id": prod("rg-app", "Microsoft.ContainerService/managedClusters", "aks-prod"), "name": "aks-prod", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "summary": "aks-prod allows local Kubernetes accounts"}),
        ),
        (
            "certificates_expiring",
            json!({"id": dev("Microsoft.Web/certificates", "web-dev-cert"), "name": "web-dev-cert", "resourceGroup": "rg-dev", "subscriptionId": "sub-dev", "expirationDate": "2026-10-01T00:00:00Z", "summary": "web-dev-cert expires within 30 days"}),
        ),
        (
            "defender_unhealthy_high",
            json!({"id": format!("{}/providers/Microsoft.Security/assessments/4fb67663-9ab9-475d-b026-8c544cced439", prod("rg-app", "Microsoft.Compute/virtualMachines", "vm-app-01")), "name": "Machines should have vulnerability findings resolved", "subscriptionId": "sub-prod", "resourceGroup": "rg-app", "resourceId": prod("rg-app", "Microsoft.Compute/virtualMachines", "vm-app-01"), "summary": "vm-app-01: high severity Defender assessment is unhealthy"}),
        ),
        (
            "defender_plan_off",
            json!({"id": "/subscriptions/sub-prod/providers/Microsoft.Security/pricings/VirtualMachines", "name": "VirtualMachines", "subscriptionId": "sub-prod", "resourceGroup": "", "resourceId": "/subscriptions/sub-prod", "summary": "Defender for Servers is off in sub-prod"}),
        ),
        (
            "basic_sku_public_ips_and_lbs",
            json!({"id": prod("rg-app", "Microsoft.Network/publicIPAddresses", "vm-app-01-pip"), "name": "vm-app-01-pip", "type": "microsoft.network/publicipaddresses", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "summary": "vm-app-01-pip uses the retiring Basic SKU"}),
        ),
        (
            "private_dns_zones_unlinked",
            json!({"id": prod("rg-network", "Microsoft.Network/privateDnsZones", "privatelink.blob.core.windows.net"), "name": "privatelink.blob.core.windows.net", "resourceGroup": "rg-network", "subscriptionId": "sub-prod", "summary": "privatelink.blob.core.windows.net is linked to no virtual network"}),
        ),
        (
            "orphaned_snapshots",
            json!({"id": dev("Microsoft.Compute/snapshots", "web-dev-snap"), "name": "web-dev-snap", "resourceGroup": "rg-dev", "subscriptionId": "sub-dev", "sourceId": dev("Microsoft.Compute/disks", "disk-old-test"), "summary": "web-dev-snap's source disk no longer exists"}),
        ),
        (
            "route_tables_without_subnets",
            json!({"id": prod("rg-network", "Microsoft.Network/routeTables", "rt-unused"), "name": "rt-unused", "resourceGroup": "rg-network", "subscriptionId": "sub-prod", "summary": "rt-unused is associated with no subnet"}),
        ),
        (
            "unused_user_assigned_identities",
            json!({"id": dev("Microsoft.ManagedIdentity/userAssignedIdentities", "id-unused"), "name": "id-unused", "resourceGroup": "rg-dev", "subscriptionId": "sub-dev", "summary": "id-unused is assigned to no resource"}),
        ),
    ]
}

fn estate_resources(vintage: Vintage) -> Vec<Value> {
    let older = vintage == Vintage::Older;
    let mut resources = vec![
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/virtualNetworks/vnet-hub",
            "name": "vnet-hub", "type": "microsoft.network/virtualnetworks", "location": "uksouth",
            "resourceGroup": "rg-network", "subscriptionId": "sub-prod", "tags": {"env": "prod"},
            "properties": {
                "addressSpace": {"addressPrefixes": ["10.0.0.0/16"]},
                "subnets": [
                    {"id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/virtualNetworks/vnet-hub/subnets/gateway",
                     "name": "gateway", "properties": {"addressPrefix": "10.0.0.0/24"}},
                    {"id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/virtualNetworks/vnet-hub/subnets/shared",
                     "name": "shared",
                     "properties": {"addressPrefix": "10.0.1.0/24",
                                    "networkSecurityGroup": {"id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/networkSecurityGroups/nsg-app"},
                                    "routeTable": {"id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/routeTables/rt-shared"},
                                    "natGateway": {"id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/natGateways/natgw-hub"}}},
                    {"id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/virtualNetworks/vnet-hub/subnets/AzureFirewallSubnet",
                     "name": "AzureFirewallSubnet", "properties": {"addressPrefix": "10.0.2.0/24"}}
                ],
                "virtualNetworkPeerings": [
                    {"properties": {"remoteVirtualNetwork": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/virtualNetworks/vnet-app"},
                                    "peeringState": "Connected"}}
                ]
            }
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/virtualNetworks/vnet-app",
            "name": "vnet-app", "type": "microsoft.network/virtualnetworks", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": {"env": "prod"},
            "properties": {
                "addressSpace": {"addressPrefixes": ["10.1.0.0/16"]},
                "subnets": [
                    {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/virtualNetworks/vnet-app/subnets/app",
                     "name": "app", "properties": {"addressPrefix": "10.1.0.0/24"}},
                    {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/virtualNetworks/vnet-app/subnets/agw",
                     "name": "agw", "properties": {"addressPrefix": "10.1.1.0/24"}},
                    {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/virtualNetworks/vnet-app/subnets/aks",
                     "name": "aks", "properties": {"addressPrefix": "10.1.2.0/23"}},
                    {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/virtualNetworks/vnet-app/subnets/db",
                     "name": "db", "properties": {"addressPrefix": "10.1.4.0/24", "delegations": [{"name": "postgres", "properties": {"serviceName": "Microsoft.DBforPostgreSQL/flexibleServers"}}]}},
                    {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/virtualNetworks/vnet-app/subnets/cae",
                     "name": "cae", "properties": {"addressPrefix": "10.1.8.0/21"}}
                ],
                "virtualNetworkPeerings": [
                    {"properties": {"remoteVirtualNetwork": {"id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/virtualNetworks/vnet-hub"},
                                    "peeringState": "Connected"}}
                ]
            }
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/networkSecurityGroups/nsg-app",
            "name": "nsg-app", "type": "microsoft.network/networksecuritygroups", "location": "uksouth",
            "resourceGroup": "rg-network", "subscriptionId": "sub-prod", "tags": {"env": "prod"},
            "properties": {"securityRules": [{"name": "allow-ssh"}]}
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/networkSecurityGroups/nsg-unused",
            "name": "nsg-unused", "type": "microsoft.network/networksecuritygroups", "location": "uksouth",
            "resourceGroup": "rg-network", "subscriptionId": "sub-prod", "tags": {"env": "prod"},
            "properties": {"securityRules": []}
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.OperationalInsights/workspaces/law-prod",
            "name": "law-prod", "type": "microsoft.operationalinsights/workspaces", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": {"env": "prod"},
            "properties": {"sku": {"name": "PerGB2018"}, "retentionInDays": 30}
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.DesktopVirtualization/hostPools/hp-prod",
            "name": "hp-prod", "type": "microsoft.desktopvirtualization/hostpools", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": {"env": "prod"},
            "properties": {"hostPoolType": "Pooled", "loadBalancerType": "BreadthFirst", "maxSessionLimit": 10}
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Compute/virtualMachines/vm-app-01",
            "name": "vm-app-01", "type": "microsoft.compute/virtualmachines", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": {"env": "prod"},
            "properties": {
                "hardwareProfile": {"vmSize": if older { "Standard_B1s" } else { "Standard_B2s" }},
                "availabilitySet": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Compute/availabilitySets/avset-app"},
                "storageProfile": {"osDisk": {"managedDisk": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Compute/disks/vm-app-01-os"}}},
                "networkProfile": {"networkInterfaces": [{"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/networkInterfaces/vm-app-01-nic"}]}
            }
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Compute/disks/vm-app-01-os",
            "name": "vm-app-01-os", "type": "microsoft.compute/disks", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod",
            "properties": {"diskSizeGB": 64, "diskState": "Attached",
                           "encryption": {"type": "EncryptionAtRestWithCustomerKey",
                                          "diskEncryptionSetId": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Compute/diskEncryptionSets/des-prod"}}
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/networkInterfaces/vm-app-01-nic",
            "name": "vm-app-01-nic", "type": "microsoft.network/networkinterfaces", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod",
            "properties": {
                "virtualMachine": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Compute/virtualMachines/vm-app-01"},
                "ipConfigurations": [{"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/networkInterfaces/vm-app-01-nic/ipConfigurations/ipconfig1",
                  "properties": {
                    "subnet": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/virtualNetworks/vnet-app/subnets/app"},
                    "publicIPAddress": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/publicIPAddresses/vm-app-01-pip"},
                    "applicationSecurityGroups": [{"id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/applicationSecurityGroups/asg-web"}]
                }}]
            }
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/publicIPAddresses/vm-app-01-pip",
            "name": "vm-app-01-pip", "type": "microsoft.network/publicipaddresses", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "sku": {"name": "Basic"},
            "properties": {"ipAddress": "20.0.0.10",
                           "ipConfiguration": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/networkInterfaces/vm-app-01-nic/ipConfigurations/ipconfig1"}}
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Storage/storageAccounts/stprodapp01",
            "name": "stprodapp01", "type": "microsoft.storage/storageaccounts", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "kind": "StorageV2",
            "properties": {"allowBlobPublicAccess": !older, "supportsHttpsTrafficOnly": true,
                           "minimumTlsVersion": "TLS1_2", "allowSharedKeyAccess": true,
                           "networkAcls": {"defaultAction": "Deny",
                                           "virtualNetworkRules": [{"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/virtualNetworks/vnet-app/subnets/app", "action": "Allow"}]}}
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/privateEndpoints/pe-sql",
            "name": "pe-sql", "type": "microsoft.network/privateendpoints", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod",
            "properties": {
                "subnet": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/virtualNetworks/vnet-app/subnets/app"},
                "networkInterfaces": [{"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/networkInterfaces/pe-sql.nic.4f2a"}],
                "privateLinkServiceConnections": [{"properties": {"privateLinkServiceId": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Sql/servers/sql-prod"}}]
            }
        }),
        // Azure's own plumbing for `pe-sql`: its own ARG row, named after the
        // endpoint, with no `virtualMachine` of its own. It folds into the
        // endpoint, and the goldens are here to show that it does.
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/networkInterfaces/pe-sql.nic.4f2a",
            "name": "pe-sql.nic.4f2a", "type": "microsoft.network/networkinterfaces", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod",
            "properties": {
                "ipConfigurations": [{"properties": {
                    "subnet": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/virtualNetworks/vnet-app/subnets/app"}
                }}]
            }
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Sql/servers/sql-prod",
            "name": "sql-prod", "type": "microsoft.sql/servers", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": {"env": "prod"},
            "properties": {"publicNetworkAccess": "Disabled", "version": "12.0"}
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Sql/servers/sql-prod/databases/app-db",
            "name": "app-db", "type": "microsoft.sql/servers/databases", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": {"env": "prod"},
            "properties": {"status": "Online", "currentServiceObjectiveName": "S0"}
        }),
        json!({
            "id": "/subscriptions/sub-dev/resourceGroups/rg-dev/providers/Microsoft.Web/serverfarms/asp-dev",
            "name": "asp-dev", "type": "microsoft.web/serverfarms", "location": "ukwest",
            "resourceGroup": "rg-dev", "subscriptionId": "sub-dev",
            "sku": {"name": "B1", "tier": "Basic"}, "properties": {"numberOfSites": 1}
        }),
        json!({
            "id": "/subscriptions/sub-dev/resourceGroups/rg-dev/providers/Microsoft.Web/sites/web-dev",
            "name": "web-dev", "type": "microsoft.web/sites", "location": "ukwest", "kind": "app,linux",
            "resourceGroup": "rg-dev", "subscriptionId": "sub-dev",
            "identity": {"type": "UserAssigned",
                         "userAssignedIdentities": {"/subscriptions/sub-dev/resourceGroups/rg-dev/providers/Microsoft.ManagedIdentity/userAssignedIdentities/id-web-dev": {}}},
            "properties": {"state": "Running", "httpsOnly": false, "defaultHostName": "web-dev.azurewebsites.net",
                           "serverFarmId": "/subscriptions/sub-dev/resourceGroups/rg-dev/providers/Microsoft.Web/serverfarms/asp-dev"}
        }),
        json!({
            "id": "/subscriptions/sub-dev/resourceGroups/rg-dev/providers/Microsoft.ManagedIdentity/userAssignedIdentities/id-web-dev",
            "name": "id-web-dev", "type": "microsoft.managedidentity/userassignedidentities", "location": "ukwest",
            "resourceGroup": "rg-dev", "subscriptionId": "sub-dev",
            "properties": {"clientId": "00000000-0000-0000-0000-000000000000"}
        }),
    ];
    resources.extend(edge_services());
    // The private-link pair and the SQL server arrived after the older
    // collection; the old test disk left before the current one.
    if older {
        resources.retain(|r| {
            !["pe-sql", "pe-sql.nic.4f2a", "sql-prod", "app-db"]
                .contains(&r["name"].as_str().unwrap_or_default())
        });
        resources.push(json!({
            "id": "/subscriptions/sub-dev/resourceGroups/rg-dev/providers/Microsoft.Compute/disks/disk-old-test",
            "name": "disk-old-test", "type": "microsoft.compute/disks", "location": "ukwest",
            "resourceGroup": "rg-dev", "subscriptionId": "sub-dev",
            "properties": {"diskSizeGB": 32, "diskState": "Unattached"}
        }));
    }
    resources
}

/// The edge services and platform plumbing the query-pack expansion covers,
/// shaped so every `EdgeKind` the extractors know has at least one producer
/// (`fixture_exercises_every_edge_kind` holds the fixture to that).
fn edge_services() -> Vec<Value> {
    let net = |provider: &str, name: &str| {
        format!("/subscriptions/sub-prod/resourceGroups/rg-network/providers/{provider}/{name}")
    };
    let app = |provider: &str, name: &str| {
        format!("/subscriptions/sub-prod/resourceGroups/rg-app/providers/{provider}/{name}")
    };
    let dev = |provider: &str, name: &str| {
        format!("/subscriptions/sub-dev/resourceGroups/rg-dev/providers/{provider}/{name}")
    };
    let hub_subnet = |name: &str| {
        net(
            "Microsoft.Network/virtualNetworks",
            &format!("vnet-hub/subnets/{name}"),
        )
    };
    let app_subnet = |name: &str| {
        app(
            "Microsoft.Network/virtualNetworks",
            &format!("vnet-app/subnets/{name}"),
        )
    };
    let law = app("Microsoft.OperationalInsights/workspaces", "law-prod");
    let vm = app("Microsoft.Compute/virtualMachines", "vm-app-01");
    let prod_tags = json!({"env": "prod"});
    let network = |name: &str, azure_type: &str, properties: Value| {
        json!({
            "id": net(azure_type, name), "name": name,
            "type": format!("microsoft.network/{}", azure_type.rsplit('/').next().unwrap_or_default().to_lowercase()),
            "location": "uksouth", "resourceGroup": "rg-network", "subscriptionId": "sub-prod", "tags": prod_tags,
            "properties": properties
        })
    };
    let mut resources = vec![
        network(
            "natgw-hub",
            "Microsoft.Network/natGateways",
            json!({
                "subnets": [{"id": hub_subnet("shared")}],
                "publicIpAddresses": [{"id": net("Microsoft.Network/publicIPAddresses", "pip-natgw")}],
                "idleTimeoutInMinutes": 4
            }),
        ),
        network(
            "pip-natgw",
            "Microsoft.Network/publicIPAddresses",
            json!({"ipAddress": "20.0.0.11", "publicIPAllocationMethod": "Static"}),
        ),
        network(
            "fw-hub",
            "Microsoft.Network/azureFirewalls",
            json!({
                "ipConfigurations": [{"name": "fw-ip", "properties": {
                    "subnet": {"id": hub_subnet("AzureFirewallSubnet")},
                    "publicIPAddress": {"id": net("Microsoft.Network/publicIPAddresses", "pip-fw")}}}],
                "firewallPolicy": {"id": net("Microsoft.Network/firewallPolicies", "fwpol-hub")},
                "threatIntelMode": "Alert"
            }),
        ),
        network(
            "pip-fw",
            "Microsoft.Network/publicIPAddresses",
            json!({"ipAddress": "20.0.0.12", "publicIPAllocationMethod": "Static"}),
        ),
        network(
            "fwpol-hub",
            "Microsoft.Network/firewallPolicies",
            json!({"threatIntelMode": "Alert", "sku": {"tier": "Standard"}}),
        ),
        network(
            "vgw-hub",
            "Microsoft.Network/virtualNetworkGateways",
            json!({
                "gatewayType": "Vpn", "vpnType": "RouteBased",
                "ipConfigurations": [{"name": "default", "properties": {
                    "subnet": {"id": hub_subnet("gateway")},
                    "publicIPAddress": {"id": net("Microsoft.Network/publicIPAddresses", "pip-vgw")}}}]
            }),
        ),
        network(
            "pip-vgw",
            "Microsoft.Network/publicIPAddresses",
            json!({"ipAddress": "20.0.0.13", "publicIPAllocationMethod": "Static"}),
        ),
        network(
            "lng-onprem",
            "Microsoft.Network/localNetworkGateways",
            json!({
                "gatewayIpAddress": "203.0.113.10",
                "localNetworkAddressSpace": {"addressPrefixes": ["192.168.0.0/16"]}
            }),
        ),
        network(
            "cn-onprem",
            "Microsoft.Network/connections",
            json!({
                "connectionType": "IPsec", "connectionStatus": "Connected",
                "virtualNetworkGateway1": {"id": net("Microsoft.Network/virtualNetworkGateways", "vgw-hub")},
                "localNetworkGateway2": {"id": net("Microsoft.Network/localNetworkGateways", "lng-onprem")}
            }),
        ),
        network(
            "privatelink.database.windows.net",
            "Microsoft.Network/privateDnsZones",
            json!({"numberOfVirtualNetworkLinks": 1, "numberOfRecordSets": 2}),
        ),
        json!({
            "id": net("Microsoft.Network/privateDnsZones", "privatelink.database.windows.net/virtualNetworkLinks/link-hub"),
            "name": "link-hub", "type": "microsoft.network/privatednszones/virtualnetworklinks", "location": "global",
            "resourceGroup": "rg-network", "subscriptionId": "sub-prod", "tags": prod_tags,
            "properties": {"virtualNetwork": {"id": net("Microsoft.Network/virtualNetworks", "vnet-hub")}, "registrationEnabled": false, "virtualNetworkLinkState": "Completed"}
        }),
        network(
            "privatelink.blob.core.windows.net",
            "Microsoft.Network/privateDnsZones",
            json!({"numberOfVirtualNetworkLinks": 0, "numberOfRecordSets": 1}),
        ),
        network(
            "asg-web",
            "Microsoft.Network/applicationSecurityGroups",
            json!({}),
        ),
        network(
            "rt-shared",
            "Microsoft.Network/routeTables",
            json!({
                "disableBgpRoutePropagation": true,
                "routes": [{"name": "default", "properties": {"addressPrefix": "0.0.0.0/0", "nextHopType": "VirtualAppliance", "nextHopIpAddress": "10.0.2.4"}}]
            }),
        ),
        network(
            "rt-unused",
            "Microsoft.Network/routeTables",
            json!({"routes": []}),
        ),
        json!({
            "id": app("Microsoft.KeyVault/vaults", "kv-prod"), "name": "kv-prod", "type": "microsoft.keyvault/vaults",
            "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": prod_tags,
            "properties": {"enableSoftDelete": true, "enablePurgeProtection": true, "publicNetworkAccess": "Disabled",
                           "networkAcls": {"defaultAction": "Deny", "bypass": "AzureServices",
                                           "virtualNetworkRules": [{"id": app_subnet("app")}]}}
        }),
        json!({
            "id": app("Microsoft.Compute/diskEncryptionSets", "des-prod"), "name": "des-prod", "type": "microsoft.compute/diskencryptionsets",
            "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": prod_tags,
            "identity": {"type": "SystemAssigned"},
            "properties": {"encryptionType": "EncryptionAtRestWithCustomerKey",
                           "activeKey": {"sourceVault": {"id": app("Microsoft.KeyVault/vaults", "kv-prod")}, "keyUrl": "https://kv-prod.vault.azure.net/keys/disk/1"}}
        }),
        json!({
            "id": app("Microsoft.Insights/dataCollectionRules", "dcr-prod"), "name": "dcr-prod", "type": "microsoft.insights/datacollectionrules",
            "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": prod_tags,
            "properties": {"destinations": {"logAnalytics": [{"name": "law", "workspaceResourceId": law}]},
                           "dataFlows": [{"streams": ["Microsoft-Syslog"], "destinations": ["law"]}]}
        }),
        json!({
            "id": app("Microsoft.Insights/components", "appi-prod"), "name": "appi-prod", "type": "microsoft.insights/components",
            "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "kind": "web", "tags": prod_tags,
            "properties": {"Application_Type": "web", "WorkspaceResourceId": law, "RetentionInDays": 90}
        }),
        json!({
            "id": app("Microsoft.Insights/metricAlerts", "alert-cpu"), "name": "alert-cpu", "type": "microsoft.insights/metricalerts",
            "location": "global", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": prod_tags,
            "properties": {"scopes": [vm], "severity": 2, "enabled": true, "evaluationFrequency": "PT5M"}
        }),
        json!({
            "id": app("Microsoft.Network/applicationGateways", "agw-prod"), "name": "agw-prod", "type": "microsoft.network/applicationgateways",
            "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": prod_tags,
            "sku": {"name": "WAF_v2", "tier": "WAF_v2"},
            "properties": {
                "operationalState": "Running",
                "gatewayIPConfigurations": [{"name": "gw", "properties": {"subnet": {"id": app_subnet("agw")}}}],
                "frontendIPConfigurations": [{"name": "public", "properties": {"publicIPAddress": {"id": app("Microsoft.Network/publicIPAddresses", "pip-agw")}}}],
                "backendAddressPools": [{"name": "web", "properties": {"backendIPConfigurations": [{"id": app("Microsoft.Network/networkInterfaces", "vm-app-01-nic/ipConfigurations/ipconfig1")}]}}],
                "firewallPolicy": {"id": app("Microsoft.Network/ApplicationGatewayWebApplicationFirewallPolicies", "wafpol-prod")}
            }
        }),
        json!({
            "id": app("Microsoft.Network/publicIPAddresses", "pip-agw"), "name": "pip-agw", "type": "microsoft.network/publicipaddresses",
            "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "sku": {"name": "Standard"},
            "properties": {"ipAddress": "20.0.0.20", "publicIPAllocationMethod": "Static"}
        }),
        json!({
            "id": app("Microsoft.Network/ApplicationGatewayWebApplicationFirewallPolicies", "wafpol-prod"), "name": "wafpol-prod",
            "type": "microsoft.network/applicationgatewaywebapplicationfirewallpolicies",
            "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": prod_tags,
            "properties": {"policySettings": {"mode": "Prevention", "state": "Enabled"}}
        }),
        json!({
            "id": app("Microsoft.Compute/availabilitySets", "avset-app"), "name": "avset-app", "type": "microsoft.compute/availabilitysets",
            "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": prod_tags,
            "sku": {"name": "Aligned"}, "properties": {"platformFaultDomainCount": 2, "platformUpdateDomainCount": 5}
        }),
        json!({
            "id": app("Microsoft.ContainerService/managedClusters", "aks-prod"), "name": "aks-prod", "type": "microsoft.containerservice/managedclusters",
            "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": prod_tags,
            "identity": {"type": "SystemAssigned"},
            "properties": {
                "kubernetesVersion": "1.30.4", "nodeResourceGroup": "MC_rg-app_aks-prod_uksouth",
                "enableRBAC": true, "disableLocalAccounts": false,
                "agentPoolProfiles": [{"name": "nodepool1", "count": 2, "vmSize": "Standard_D2s_v5", "mode": "System", "vnetSubnetID": app_subnet("aks")}],
                "addonProfiles": {"omsagent": {"enabled": true, "config": {"logAnalyticsWorkspaceResourceID": law}}},
                "apiServerAccessProfile": {"enablePrivateCluster": false}
            }
        }),
        json!({
            "id": app("Microsoft.DBforPostgreSQL/flexibleServers", "psql-prod"), "name": "psql-prod", "type": "microsoft.dbforpostgresql/flexibleservers",
            "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": prod_tags,
            "sku": {"name": "Standard_B1ms", "tier": "Burstable"},
            "properties": {"version": "16", "state": "Ready",
                           "network": {"delegatedSubnetResourceId": app_subnet("db"), "publicNetworkAccess": "Disabled"},
                           "highAvailability": {"mode": "Disabled"}, "backup": {"backupRetentionDays": 7, "geoRedundantBackup": "Disabled"}}
        }),
        json!({
            "id": app("Microsoft.App/managedEnvironments", "cae-prod"), "name": "cae-prod", "type": "microsoft.app/managedenvironments",
            "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": prod_tags,
            "properties": {"vnetConfiguration": {"infrastructureSubnetId": app_subnet("cae"), "internal": true},
                           "appLogsConfiguration": {"destination": "log-analytics"}}
        }),
        json!({
            "id": app("Microsoft.App/containerApps", "ca-api"), "name": "ca-api", "type": "microsoft.app/containerapps",
            "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": prod_tags,
            "properties": {"managedEnvironmentId": app("Microsoft.App/managedEnvironments", "cae-prod"),
                           "configuration": {"ingress": {"external": false, "targetPort": 8080}}, "runningStatus": "Running"}
        }),
        json!({
            "id": app("Microsoft.RecoveryServices/vaults", "rsv-prod"), "name": "rsv-prod", "type": "microsoft.recoveryservices/vaults",
            "location": "uksouth", "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "tags": prod_tags,
            "sku": {"name": "RS0", "tier": "Standard"}, "properties": {"provisioningState": "Succeeded"}
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/MC_rg-app_aks-prod_uksouth/providers/Microsoft.Compute/virtualMachineScaleSets/aks-nodepool1-12345678-vmss",
            "name": "aks-nodepool1-12345678-vmss", "type": "microsoft.compute/virtualmachinescalesets",
            "location": "uksouth", "resourceGroup": "MC_rg-app_aks-prod_uksouth", "subscriptionId": "sub-prod",
            "tags": {"aks-managed-clusterName": "aks-prod", "aks-managed-poolName": "nodepool1"},
            "sku": {"name": "Standard_D2s_v5", "capacity": 2},
            "properties": {"orchestrationMode": "Uniform",
                           "virtualMachineProfile": {"networkProfile": {"networkInterfaceConfigurations": [{"name": "aks-nodepool1", "properties": {
                               "primary": true,
                               "ipConfigurations": [{"name": "ipconfig1", "properties": {"subnet": {"id": app_subnet("aks")}}}]}}]}}}
        }),
        json!({
            "id": dev("Microsoft.Cache/Redis", "redis-dev"), "name": "redis-dev", "type": "microsoft.cache/redis",
            "location": "ukwest", "resourceGroup": "rg-dev", "subscriptionId": "sub-dev",
            "sku": {"name": "Basic", "family": "C", "capacity": 0},
            "properties": {"enableNonSslPort": true, "minimumTlsVersion": "1.0", "redisVersion": "6.0"}
        }),
        json!({
            "id": dev("Microsoft.Web/certificates", "web-dev-cert"), "name": "web-dev-cert", "type": "microsoft.web/certificates",
            "location": "ukwest", "resourceGroup": "rg-dev", "subscriptionId": "sub-dev",
            "properties": {"subjectName": "web-dev.example.test", "expirationDate": "2026-10-01T00:00:00Z", "issuer": "Example CA"}
        }),
        json!({
            "id": dev("Microsoft.Compute/snapshots", "web-dev-snap"), "name": "web-dev-snap", "type": "microsoft.compute/snapshots",
            "location": "ukwest", "resourceGroup": "rg-dev", "subscriptionId": "sub-dev",
            "properties": {"diskSizeGB": 32, "creationData": {"createOption": "Copy", "sourceResourceId": dev("Microsoft.Compute/disks", "disk-old-test")}}
        }),
        json!({
            "id": dev("Microsoft.ManagedIdentity/userAssignedIdentities", "id-unused"), "name": "id-unused", "type": "microsoft.managedidentity/userassignedidentities",
            "location": "ukwest", "resourceGroup": "rg-dev", "subscriptionId": "sub-dev",
            "properties": {"clientId": "11111111-1111-1111-1111-111111111111"}
        }),
    ];
    resources.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    resources
}

/// Optional saved evidence for screenshot export tests. The source is entirely
/// local and deliberately simple so embedded PNG bytes stay deterministic.
#[allow(dead_code)]
pub fn seed_website_evidence(store: &Store, snapshot: &str) -> Vec<u8> {
    let mut image = image::RgbImage::from_pixel(1440, 900, image::Rgb([238, 243, 245]));
    for y in 0..120 {
        for x in 0..1440 {
            image.put_pixel(x, y, image::Rgb([23, 41, 54]));
        }
    }
    let mut bytes = std::io::Cursor::new(Vec::new());
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    let png = bytes.into_inner();
    let url = "https://web-dev.azurewebsites.net/";
    store
        .save_website_capture(
            snapshot,
            url,
            &azdocs::model::websites::CapturedWebsite {
                final_url: url.into(),
                captured_at: "2026-09-12T10:00:00Z".into(),
                renderer: "fixture".into(),
                png: png.clone(),
            },
        )
        .unwrap();
    png
}

/// Collection evidence shared by every export fixture; no extra inventory
/// resources or finding totals are introduced by these service records.
pub fn seed_microsoft_evidence(store: &Store, snapshot_id: &str) {
    let pack = QueryPack::builtin().unwrap();
    for (name, rows) in [
        (
            "advisor_cost_recommendations",
            vec![
                json!({"id":"/advisor/cost-1", "subscriptionId":"sub-prod", "resourceId":"/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.compute/virtualmachines/vm-web", "solution":"Review VM sizing", "currency":"GBP", "savingsPeriod":"Month", "savingsAmount":125.50, "annualSavingsAmount":null}),
            ],
        ),
        (
            "policy_states",
            vec![
                json!({"id":"/policy/state-1", "subscriptionId":"sub-prod", "resourceId":"/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.storage/storageaccounts/stprodapp01", "policyAssignmentId":"/subscriptions/sub-prod/providers/microsoft.authorization/policyassignments/baseline", "policyAssignmentName":"baseline", "policySetDefinitionId":"/providers/microsoft.authorization/policysetdefinitions/security-baseline", "complianceState":"Exempt", "evaluatedAt":"2026-09-01T12:00:00Z"}),
            ],
        ),
        (
            "policy_exemptions",
            vec![
                json!({"id":"/policy/exemption-1", "name":"migration", "displayName":"Migration exception", "subscriptionId":"sub-prod", "exemptionCategory":"Waiver", "expiresOn":"2099-01-01T00:00:00Z"}),
            ],
        ),
        (
            "defender_compliance_standards",
            vec![
                json!({"id":"/defender/standard-1", "subscriptionId":"sub-prod", "complianceStandard":"Azure-Security-Benchmark", "state":"Failed", "passedControls":12, "failedControls":2, "skippedControls":1, "unsupportedControls":3}),
            ],
        ),
        (
            "defender_compliance_controls",
            vec![
                json!({"id":"/defender/control-1", "subscriptionId":"sub-prod", "complianceStandard":"Azure-Security-Benchmark", "complianceControl":"NS-1", "state":"Unsupported"}),
            ],
        ),
        (
            "defender_compliance_assessments",
            vec![
                json!({"id":"/defender/assessment-1", "subscriptionId":"sub-prod", "complianceStandard":"Azure-Security-Benchmark", "complianceControl":"NS-1", "state":"Failed", "passedResources":12, "failedResources":2, "skippedResources":null}),
            ],
        ),
    ] {
        let def = pack.get(name).unwrap();
        ingest::ingest(store, snapshot_id, def, &rows).unwrap();
        store
            .record_query_run(
                snapshot_id,
                &azdocs::model::QueryRun {
                    provenance: None,
                    query_name: name.into(),
                    category: def.category.clone(),
                    row_count: Some(rows.len() as u64),
                    duration_ms: Some(10),
                    error: None,
                    rows_dropped: None,
                },
            )
            .unwrap();
    }
}

fn seed_operational_evidence(store: &Store, snapshot_id: &str) {
    let pack = QueryPack::builtin().unwrap();
    for (name, rows) in [
        (
            "policy_assignments",
            vec![
                json!({"id":"/subscriptions/sub-prod/providers/microsoft.authorization/policyassignments/baseline","displayName":"Production baseline","enforcementMode":"Default"}),
                json!({"id":"/subscriptions/sub-prod/providers/microsoft.authorization/policyassignments/not-evaluated","displayName":"Pending initiative","enforcementMode":"DoNotEnforce"}),
            ],
        ),
        (
            "patch_assessments",
            vec![
                json!({"id":"/patch/assessment-1","resourceId":"/subscriptions/sub-prod/resourcegroups/rg-app/providers/microsoft.compute/virtualmachines/vm-app-01","assessedAt":null,"osType":"Linux","status":"Succeeded","securityUpdates":2,"criticalUpdates":null}),
            ],
        ),
        (
            "backup_jobs",
            vec![
                json!({"id":"/backup/job-1","operation":"Backup","status":"CompletedWithWarnings"}),
                json!({"id":"/backup/job-2","operation":"Restore","status":"Failed"}),
            ],
        ),
        (
            "resource_changes",
            vec![
                json!({"id":"/changes/change-1","changeType":"Update","changedByType":"Application","changedBy":"fixture-deployment","clientType":"Azure Resource Manager"}),
            ],
        ),
    ] {
        let def = pack.get(name).unwrap();
        ingest::ingest(store, snapshot_id, def, &rows).unwrap();
        store
            .record_query_run(
                snapshot_id,
                &azdocs::model::QueryRun {
                    query_name: name.into(),
                    category: def.category.clone(),
                    row_count: Some(rows.len() as u64),
                    duration_ms: Some(10),
                    error: None,
                    provenance: Some(def.provenance(&["sub-prod".into()])),
                    rows_dropped: None,
                },
            )
            .unwrap();
    }
    store
        .record_query_run(
            snapshot_id,
            &azdocs::model::QueryRun {
                query_name: "all_resources".into(),
                category: "inventory".into(),
                row_count: Some(store.resources(snapshot_id).unwrap().len() as u64),
                duration_ms: Some(10),
                error: None,
                provenance: None,
                rows_dropped: None,
            },
        )
        .unwrap();
}
