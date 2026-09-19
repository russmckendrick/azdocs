//! Canonical fixture estate shared by report/diagram golden tests: two
//! subscriptions, peered hub/spoke VNets, a VM with NIC + public IP, storage
//! with findings, a private endpoint, a SQL server with a database child, a
//! web app on its plan with a user-assigned identity, a Log Analytics
//! workspace, an AVD host pool, and a detached NSG.

use azdocs::collect::{audit, extractors, ingest};
use azdocs::querypack::QueryPack;
use azdocs::store::Store;
use serde_json::{Value, json};

pub fn seed_estate(store: &Store) -> String {
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
            json!({"id": "/subscriptions/sub-dev/resourceGroups/rg-dev", "name": "rg-dev", "subscriptionId": "sub-dev", "location": "ukwest"}),
        ],
    );
    ingest_rows("all_resources", &estate_resources());
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
    ingest_rows(
        "storage_public_blob_access",
        &[
            json!({"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Storage/storageAccounts/stprodapp01",
                 "name": "stprodapp01", "resourceGroup": "rg-app", "subscriptionId": "sub-prod",
                 "summary": "stprodapp01 allows public blob access"}),
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

    // Same post-pass the collect runner performs.
    let resources = store.resources(&snapshot.id).unwrap();
    let edges: Vec<_> = resources.iter().flat_map(extractors::extract).collect();
    store.insert_edges(&snapshot.id, &edges).unwrap();
    let tag_findings = audit::missing_required_tags(&resources, &["env".to_owned()]);
    store.insert_findings(&snapshot.id, &tag_findings).unwrap();
    store
        .set_snapshot_status(&snapshot.id, azdocs::model::SnapshotStatus::Complete)
        .unwrap();
    snapshot.id
}

fn estate_resources() -> Vec<Value> {
    vec![
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
                                    "networkSecurityGroup": {"id": "/subscriptions/sub-prod/resourceGroups/rg-network/providers/Microsoft.Network/networkSecurityGroups/nsg-app"}}}
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
                     "name": "app", "properties": {"addressPrefix": "10.1.0.0/24"}}
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
                "hardwareProfile": {"vmSize": "Standard_B2s"},
                "storageProfile": {"osDisk": {"managedDisk": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Compute/disks/vm-app-01-os"}}},
                "networkProfile": {"networkInterfaces": [{"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/networkInterfaces/vm-app-01-nic"}]}
            }
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Compute/disks/vm-app-01-os",
            "name": "vm-app-01-os", "type": "microsoft.compute/disks", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod",
            "properties": {"diskSizeGB": 64, "diskState": "Attached", "encryption": {"type": "EncryptionAtRestWithPlatformKey"}}
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/networkInterfaces/vm-app-01-nic",
            "name": "vm-app-01-nic", "type": "microsoft.network/networkinterfaces", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod",
            "properties": {
                "virtualMachine": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Compute/virtualMachines/vm-app-01"},
                "ipConfigurations": [{"properties": {
                    "subnet": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/virtualNetworks/vnet-app/subnets/app"},
                    "publicIPAddress": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/publicIPAddresses/vm-app-01-pip"}
                }}]
            }
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/publicIPAddresses/vm-app-01-pip",
            "name": "vm-app-01-pip", "type": "microsoft.network/publicipaddresses", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod",
            "properties": {"ipAddress": "20.0.0.10",
                           "ipConfiguration": {"id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Network/networkInterfaces/vm-app-01-nic/ipConfigurations/ipconfig1"}}
        }),
        json!({
            "id": "/subscriptions/sub-prod/resourceGroups/rg-app/providers/Microsoft.Storage/storageAccounts/stprodapp01",
            "name": "stprodapp01", "type": "microsoft.storage/storageaccounts", "location": "uksouth",
            "resourceGroup": "rg-app", "subscriptionId": "sub-prod", "kind": "StorageV2",
            "properties": {"allowBlobPublicAccess": true, "supportsHttpsTrafficOnly": true}
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
    ]
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
