//! Azure resource type → draw.io azure2 icon SVG path (category, file name).
//! Icons render via `image;...;image=img/lib/azure2/<category>/<Name>.svg`.

const ICONS: &[(&str, &str, &str)] = &[
    (
        "microsoft.compute/virtualmachines",
        "compute",
        "Virtual_Machine",
    ),
    (
        "microsoft.compute/virtualmachinescalesets",
        "compute",
        "VM_Scale_Sets",
    ),
    ("microsoft.compute/disks", "compute", "Disks"),
    (
        "microsoft.compute/availabilitysets",
        "compute",
        "Availability_Sets",
    ),
    (
        "microsoft.containerservice/managedclusters",
        "containers",
        "Kubernetes_Services",
    ),
    (
        "microsoft.app/containerapps",
        "containers",
        "Container_Instances",
    ),
    (
        "microsoft.containerregistry/registries",
        "containers",
        "Container_Registries",
    ),
    (
        "microsoft.containerinstance/containergroups",
        "containers",
        "Container_Instances",
    ),
    (
        "microsoft.web/serverfarms",
        "app_services",
        "App_Service_Plans",
    ),
    ("microsoft.web/sites", "app_services", "App_Services"),
    ("microsoft.web/staticsites", "app_services", "Static_Apps"),
    (
        "microsoft.network/virtualnetworks",
        "networking",
        "Virtual_Networks",
    ),
    (
        "microsoft.network/networksecuritygroups",
        "networking",
        "Network_Security_Groups",
    ),
    (
        "microsoft.network/networkinterfaces",
        "networking",
        "Network_Interfaces",
    ),
    (
        "microsoft.network/publicipaddresses",
        "networking",
        "Public_IP_Addresses",
    ),
    (
        "microsoft.network/loadbalancers",
        "networking",
        "Load_Balancers",
    ),
    (
        "microsoft.network/applicationgateways",
        "networking",
        "Application_Gateways",
    ),
    (
        "microsoft.network/azurefirewalls",
        "networking",
        "Firewalls",
    ),
    (
        "microsoft.network/virtualnetworkgateways",
        "networking",
        "Virtual_Network_Gateways",
    ),
    (
        "microsoft.network/expressroutecircuits",
        "networking",
        "ExpressRoute_Circuits",
    ),
    (
        "microsoft.network/privateendpoints",
        "networking",
        "Private_Link",
    ),
    (
        "microsoft.network/privatednszones",
        "networking",
        "DNS_Zones",
    ),
    ("microsoft.network/dnszones", "networking", "DNS_Zones"),
    (
        "microsoft.network/routetables",
        "networking",
        "Route_Tables",
    ),
    ("microsoft.network/natgateways", "networking", "NAT"),
    ("microsoft.network/bastionhosts", "networking", "Bastions"),
    ("microsoft.network/frontdoors", "networking", "Front_Doors"),
    ("microsoft.cdn/profiles", "app_services", "CDN_Profiles"),
    (
        "microsoft.storage/storageaccounts",
        "storage",
        "Storage_Accounts",
    ),
    (
        "microsoft.recoveryservices/vaults",
        "storage",
        "Recovery_Services_Vaults",
    ),
    ("microsoft.sql/servers", "databases", "SQL_Server"),
    (
        "microsoft.sql/servers/databases",
        "databases",
        "SQL_Database",
    ),
    (
        "microsoft.sql/managedinstances",
        "databases",
        "SQL_Managed_Instance",
    ),
    (
        "microsoft.documentdb/databaseaccounts",
        "databases",
        "Azure_Cosmos_DB",
    ),
    (
        "microsoft.dbforpostgresql/flexibleservers",
        "databases",
        "Azure_Database_PostgreSQL_Server",
    ),
    (
        "microsoft.dbformysql/flexibleservers",
        "databases",
        "Azure_Database_MySQL_Server",
    ),
    ("microsoft.cache/redis", "databases", "Cache_Redis"),
    ("microsoft.keyvault/vaults", "security", "Key_Vaults"),
    (
        "microsoft.managedidentity/userassignedidentities",
        "identity",
        "Managed_Identities",
    ),
    (
        "microsoft.insights/components",
        "devops",
        "Application_Insights",
    ),
    (
        "microsoft.operationalinsights/workspaces",
        "analytics",
        "Log_Analytics_Workspaces",
    ),
    ("microsoft.eventhub/namespaces", "analytics", "Event_Hubs"),
    (
        "microsoft.servicebus/namespaces",
        "integration",
        "Service_Bus",
    ),
    (
        "microsoft.eventgrid/topics",
        "integration",
        "Event_Grid_Topics",
    ),
    ("microsoft.logic/workflows", "integration", "Logic_Apps"),
    (
        "microsoft.apimanagement/service",
        "app_services",
        "API_Management_Services",
    ),
    (
        "microsoft.cognitiveservices/accounts",
        "ai_machine_learning",
        "Cognitive_Services",
    ),
    (
        "microsoft.machinelearningservices/workspaces",
        "ai_machine_learning",
        "Machine_Learning",
    ),
    (
        "microsoft.databricks/workspaces",
        "analytics",
        "Azure_Databricks",
    ),
    (
        "microsoft.datafactory/factories",
        "databases",
        "Data_Factory",
    ),
    (
        "microsoft.synapse/workspaces",
        "analytics",
        "Azure_Synapse_Analytics",
    ),
];

const GENERIC: (&str, &str) = ("general", "All_Resources");

/// Full draw.io style string for a resource icon node.
pub fn style_for(azure_type: &str) -> String {
    let (category, name) = ICONS
        .iter()
        .find(|(key, _, _)| *key == azure_type)
        .map(|(_, category, name)| (*category, *name))
        .unwrap_or(GENERIC);
    format!(
        "image;aspect=fixed;html=1;points=[];align=center;fontSize=10;\
         labelPosition=center;verticalLabelPosition=bottom;verticalAlign=top;\
         whiteSpace=wrap;image=img/lib/azure2/{category}/{name}.svg;"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_for_maps_known_types() {
        assert!(
            style_for("microsoft.network/virtualnetworks")
                .contains("img/lib/azure2/networking/Virtual_Networks.svg")
        );
    }

    #[test]
    fn style_for_falls_back_to_generic_icon() {
        assert!(style_for("microsoft.custom/widgets").contains("general/All_Resources.svg"));
    }
}
