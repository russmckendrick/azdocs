/// Human display names for common Azure resource types (keys lowercased).
/// Types not listed fall back to the last path segment of the type string.
const DISPLAY_NAMES: &[(&str, &str)] = &[
    ("microsoft.compute/virtualmachines", "Virtual Machine"),
    ("microsoft.compute/virtualmachinescalesets", "VM Scale Set"),
    ("microsoft.compute/disks", "Managed Disk"),
    ("microsoft.compute/snapshots", "Disk Snapshot"),
    ("microsoft.compute/images", "VM Image"),
    ("microsoft.compute/availabilitysets", "Availability Set"),
    ("microsoft.containerservice/managedclusters", "AKS Cluster"),
    ("microsoft.app/containerapps", "Container App"),
    (
        "microsoft.app/managedenvironments",
        "Container Apps Environment",
    ),
    (
        "microsoft.containerregistry/registries",
        "Container Registry",
    ),
    (
        "microsoft.containerinstance/containergroups",
        "Container Instance",
    ),
    ("microsoft.web/serverfarms", "App Service Plan"),
    ("microsoft.web/sites", "App Service / Function App"),
    ("microsoft.web/staticsites", "Static Web App"),
    ("microsoft.network/virtualnetworks", "Virtual Network"),
    (
        "microsoft.network/networksecuritygroups",
        "Network Security Group",
    ),
    ("microsoft.network/networkinterfaces", "Network Interface"),
    ("microsoft.network/publicipaddresses", "Public IP Address"),
    ("microsoft.network/loadbalancers", "Load Balancer"),
    (
        "microsoft.network/applicationgateways",
        "Application Gateway",
    ),
    ("microsoft.network/azurefirewalls", "Azure Firewall"),
    ("microsoft.network/firewallpolicies", "Firewall Policy"),
    (
        "microsoft.network/virtualnetworkgateways",
        "Virtual Network Gateway",
    ),
    (
        "microsoft.network/localnetworkgateways",
        "Local Network Gateway",
    ),
    ("microsoft.network/connections", "Gateway Connection"),
    (
        "microsoft.network/expressroutecircuits",
        "ExpressRoute Circuit",
    ),
    ("microsoft.network/privateendpoints", "Private Endpoint"),
    ("microsoft.network/privatednszones", "Private DNS Zone"),
    (
        "microsoft.network/privatednszones/virtualnetworklinks",
        "Private DNS VNet Link",
    ),
    ("microsoft.network/dnszones", "DNS Zone"),
    ("microsoft.network/routetables", "Route Table"),
    ("microsoft.network/natgateways", "NAT Gateway"),
    ("microsoft.network/bastionhosts", "Bastion Host"),
    (
        "microsoft.network/trafficmanagerprofiles",
        "Traffic Manager",
    ),
    ("microsoft.network/frontdoors", "Front Door"),
    ("microsoft.cdn/profiles", "CDN / Front Door Profile"),
    ("microsoft.storage/storageaccounts", "Storage Account"),
    (
        "microsoft.recoveryservices/vaults",
        "Recovery Services Vault",
    ),
    ("microsoft.dataprotection/backupvaults", "Backup Vault"),
    ("microsoft.sql/servers", "SQL Server"),
    ("microsoft.sql/servers/databases", "SQL Database"),
    ("microsoft.sql/managedinstances", "SQL Managed Instance"),
    ("microsoft.documentdb/databaseaccounts", "Cosmos DB Account"),
    (
        "microsoft.dbforpostgresql/flexibleservers",
        "PostgreSQL Flexible Server",
    ),
    (
        "microsoft.dbformysql/flexibleservers",
        "MySQL Flexible Server",
    ),
    ("microsoft.cache/redis", "Redis Cache"),
    ("microsoft.keyvault/vaults", "Key Vault"),
    (
        "microsoft.managedidentity/userassignedidentities",
        "Managed Identity",
    ),
    ("microsoft.insights/components", "Application Insights"),
    (
        "microsoft.operationalinsights/workspaces",
        "Log Analytics Workspace",
    ),
    ("microsoft.eventhub/namespaces", "Event Hubs Namespace"),
    ("microsoft.servicebus/namespaces", "Service Bus Namespace"),
    ("microsoft.eventgrid/topics", "Event Grid Topic"),
    ("microsoft.logic/workflows", "Logic App"),
    (
        "microsoft.automation/automationaccounts",
        "Automation Account",
    ),
    ("microsoft.apimanagement/service", "API Management"),
    ("microsoft.signalrservice/signalr", "SignalR Service"),
    (
        "microsoft.cognitiveservices/accounts",
        "Cognitive Services / OpenAI",
    ),
    (
        "microsoft.machinelearningservices/workspaces",
        "ML Workspace",
    ),
    ("microsoft.databricks/workspaces", "Databricks Workspace"),
    ("microsoft.datafactory/factories", "Data Factory"),
    ("microsoft.synapse/workspaces", "Synapse Workspace"),
];

/// Display name for a (lowercased) Azure resource type.
pub fn display_name(azure_type: &str) -> &str {
    DISPLAY_NAMES
        .iter()
        .find(|(key, _)| *key == azure_type)
        .map(|(_, name)| *name)
        .unwrap_or_else(|| {
            azure_type
                .rsplit('/')
                .next()
                .filter(|s| !s.is_empty())
                .unwrap_or(azure_type)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_name_maps_known_types() {
        assert_eq!(
            display_name("microsoft.network/virtualnetworks"),
            "Virtual Network"
        );
    }

    #[test]
    fn display_name_falls_back_to_last_segment() {
        assert_eq!(display_name("microsoft.custom/widgets"), "widgets");
    }
}
