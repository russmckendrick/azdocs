//! Azure resource type → draw.io azure2 icon SVG path (category, file name),
//! plus self-contained SVG icons for the SVG/PNG emitters.
//!
//! Icons render in draw.io via
//! `image;...;image=img/lib/azure2/<category>/<Name>.svg`. The SVG emitter
//! cannot reference draw.io's bundled art, so `svg_data_uri` serves embedded
//! icons instead. No icon pack is vendored yet, so every type currently gets
//! a deterministic generated placeholder (category-coloured rounded rect with
//! a two-letter monogram); vendoring real SVGs later only changes what the
//! data URI carries, not the API.

use base64::Engine as _;

use crate::model::azure_types;

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

/// Icon category for a resource type, resolved in three tiers: exact ARM
/// type match, parent-type match (last segment stripped), then keyword
/// inference over the type string. Unknown types land in "general".
fn category_for(azure_type: &str) -> &'static str {
    let lookup = |wanted: &str| {
        ICONS
            .iter()
            .find(|(key, _, _)| *key == wanted)
            .map(|(_, category, _)| *category)
    };
    if let Some(category) = lookup(azure_type) {
        return category;
    }
    if let Some((parent, _)) = azure_type.rsplit_once('/')
        && let Some(category) = lookup(parent)
    {
        return category;
    }
    infer_category(azure_type)
}

/// Keyword inference for types outside the icon table, mirroring the legacy
/// mapping. Order matters: earlier arms win for ambiguous names.
fn infer_category(azure_type: &str) -> &'static str {
    const KEYWORDS: &[(&str, &[&str])] = &[
        (
            "networking",
            &["network", "dns", "frontdoor", "cdn", "bastion", "firewall"],
        ),
        ("compute", &["compute", "virtualmachine", "hybridcompute"]),
        ("storage", &["storage", "recoveryservices"]),
        (
            "databases",
            &["sql", "documentdb", "cosmos", "cache", "dbfor"],
        ),
        ("app_services", &["web", "sites"]),
        ("security", &["keyvault", "security"]),
        ("identity", &["managedidentity", "authorization"]),
        ("containers", &["container", "kubernetes", "microsoft.app/"]),
        (
            "devops",
            &["insights", "operationalinsights", "monitor", "alerts"],
        ),
        (
            "integration",
            &[
                "logic",
                "servicebus",
                "eventgrid",
                "eventhub",
                "apimanagement",
                "relay",
                "appconfiguration",
            ],
        ),
        (
            "ai_machine_learning",
            &["cognitive", "machinelearning", "search"],
        ),
        (
            "analytics",
            &["datafactory", "synapse", "databricks", "kusto", "purview"],
        ),
    ];
    for (category, needles) in KEYWORDS {
        if needles.iter().any(|needle| azure_type.contains(needle)) {
            return category;
        }
    }
    "general"
}

/// Fill colour for a resource type's icon category.
pub fn category_color(azure_type: &str) -> &'static str {
    match category_for(azure_type) {
        "compute" => "#0078D4",
        "networking" => "#107C10",
        "storage" => "#C19C00",
        "databases" => "#B146C2",
        "app_services" => "#D83B01",
        "containers" => "#0F6CBD",
        "security" => "#D13438",
        "identity" => "#8661C5",
        "devops" => "#038387",
        "analytics" => "#A4262C",
        "integration" => "#CA5010",
        "ai_machine_learning" => "#E3008C",
        _ => "#605E5C",
    }
}

/// Embeddable `data:image/svg+xml;base64,...` icon for a resource type.
/// Currently always the generated placeholder (see module docs).
pub fn svg_data_uri(azure_type: &str) -> String {
    let svg = monogram_svg(azure_type);
    format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(svg)
    )
}

/// Placeholder icon: rounded rect in the category colour with a two-letter
/// monogram from the type's display name.
fn monogram_svg(azure_type: &str) -> String {
    let color = category_color(azure_type);
    let monogram = monogram(azure_types::display_name(azure_type));
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 48 48"><rect x="1" y="1" width="46" height="46" rx="8" fill="{color}"/><text x="24" y="31" font-family="Arial, Helvetica, sans-serif" font-size="18" font-weight="bold" text-anchor="middle" fill="#FFFFFF">{monogram}</text></svg>"##
    )
}

/// First letters of the first two words, or the first two letters of a
/// single word, uppercased.
fn monogram(display_name: &str) -> String {
    let mut words = display_name.split_whitespace().filter(|w| !w.is_empty());
    let first = words.next().unwrap_or("?");
    match words.next() {
        Some(second) => first
            .chars()
            .take(1)
            .chain(second.chars().take(1))
            .collect::<String>()
            .to_uppercase(),
        None => first.chars().take(2).collect::<String>().to_uppercase(),
    }
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

    #[test]
    fn category_for_resolves_exact_type_first() {
        assert_eq!(category_for("microsoft.compute/virtualmachines"), "compute");
    }

    #[test]
    fn category_for_strips_child_segment_when_exact_type_unknown() {
        assert_eq!(
            category_for("microsoft.compute/virtualmachines/extensions"),
            "compute"
        );
    }

    #[test]
    fn category_for_infers_from_keywords_when_type_unknown() {
        assert_eq!(category_for("microsoft.network/somethingnew"), "networking");
        assert_eq!(category_for("microsoft.custom/widgets"), "general");
    }

    #[test]
    fn svg_data_uri_is_deterministic_base64_svg() {
        let uri = svg_data_uri("microsoft.compute/virtualmachines");
        assert!(uri.starts_with("data:image/svg+xml;base64,"), "uri: {uri}");
        assert_eq!(uri, svg_data_uri("microsoft.compute/virtualmachines"));
    }

    #[test]
    fn monogram_takes_word_initials_or_first_two_letters() {
        assert_eq!(monogram("Virtual Machine"), "VM");
        assert_eq!(monogram("widgets"), "WI");
    }
}
