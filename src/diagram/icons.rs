//! Azure resource type → draw.io azure2 icon SVG path (category, file name),
//! plus self-contained SVG icons for the SVG/PNG emitters.
//!
//! Icons render in draw.io via
//! `image;...;image=img/lib/azure2/<category>/<Name>.svg`. The SVG emitter
//! cannot reference draw.io's bundled art, so `svg_data_uri` serves icons
//! from the Microsoft Azure icon pack vendored under `data/icons/`,
//! resolved through `data/icon_mapping.toml` (exact type, then parent
//! type, then the mapping's fallback icon). A generated monogram tile
//! remains the last resort so output stays deterministic even if a mapping
//! entry points at a file the pack no longer ships.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use base64::Engine as _;
use include_dir::{Dir, include_dir};

use crate::diagram::graph::NodeKind;
use crate::diagram::page::Rung;
use crate::model::azure_types;

static ICON_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/data/icons");
const ICON_MAPPING: &str = include_str!("../../data/icon_mapping.toml");

struct IconMapping {
    fallback: String,
    types: BTreeMap<String, String>,
}

fn icon_mapping() -> &'static IconMapping {
    static MAPPING: OnceLock<IconMapping> = OnceLock::new();
    MAPPING.get_or_init(|| {
        #[derive(serde::Deserialize)]
        struct Raw {
            fallback: String,
            types: BTreeMap<String, String>,
        }
        let raw: Raw = toml::from_str(ICON_MAPPING)
            .expect("embedded icon_mapping.toml is valid; guaranteed by unit test");
        IconMapping {
            fallback: raw.fallback,
            types: raw
                .types
                .into_iter()
                .map(|(key, value)| (key.to_ascii_lowercase(), value))
                .collect(),
        }
    })
}

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
    // Static Web Apps ships under `preview`, not `app_services`; pointed at
    // the latter it rendered as draw.io's broken-image placeholder.
    ("microsoft.web/staticsites", "preview", "Static_Apps"),
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
pub fn style_for(azure_type: &str, rung: &Rung) -> String {
    let (category, name) = ICONS
        .iter()
        .find(|(key, _, _)| *key == azure_type)
        .map(|(_, category, name)| (*category, *name))
        .unwrap_or(GENERIC);
    // `labelWidth` is the slot, not the glyph. The label hangs below the cell,
    // so without it draw.io wraps a resource name to the icon's own width and
    // the result is a one-word-per-line column taller than the row it sits in.
    let font = rung.label_px;
    let label_width = rung.leaf_width;
    format!(
        "image;aspect=fixed;html=1;points=[];align=center;fontSize={font};\
         labelPosition=center;verticalLabelPosition=bottom;verticalAlign=top;\
         whiteSpace=wrap;labelWidth={label_width};\
         image=img/lib/azure2/{category}/{name}.svg;"
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

/// Embeddable `data:image/svg+xml;base64,...` icon for a resource type,
/// resolved per the module docs: exact type → parent type → fallback icon →
/// generated monogram tile.
pub fn svg_data_uri(azure_type: &str) -> String {
    format!(
        "data:image/svg+xml;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(svg_bytes(azure_type))
    )
}

/// Raw icon SVG for a resource type, resolved the same way as
/// [`svg_data_uri`]. Emitters that embed a file rather than a URI (the PDF
/// registers icons as virtual files; the DOCX rasterises them) use this.
pub fn svg_bytes(azure_type: &str) -> Vec<u8> {
    pack_icon(azure_type).unwrap_or_else(|| monogram_svg(azure_type).into_bytes())
}

/// Icon bytes from the embedded pack, or None when neither the type, its
/// parent, nor the fallback resolve to a shipped file.
fn pack_icon(azure_type: &str) -> Option<Vec<u8>> {
    let mapping = icon_mapping();
    let path = mapping
        .types
        .get(azure_type)
        .or_else(|| {
            azure_type
                .rsplit_once('/')
                .and_then(|(parent, _)| mapping.types.get(parent))
        })
        .unwrap_or(&mapping.fallback);
    ICON_DIR.get_file(path).map(|file| file.contents().to_vec())
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

/// Stencil for a container's title band, so a resource group, VNet or subnet
/// is recognisable by shape before its name is read — the convention every
/// Azure reference architecture follows.
///
/// The paths are draw.io's own bundled `azure2` library, verified against it;
/// a name that is not in the library renders as a broken image, so these are
/// not guesses.
pub fn header_style(kind: &NodeKind) -> Option<(String, f64)> {
    // Width-over-height of each stencil as the library actually ships it, read
    // out of draw.io's own `azure2` folder. These are not square: a virtual
    // network is 18.0 x 10.8. Drawn in a square cell it came out visibly
    // squashed, on every container in the workbook.
    let (stencil, aspect) = match kind {
        NodeKind::Tenant => ("general/Management_Groups", 1.0),
        NodeKind::Subscription => ("general/Subscriptions", 1.0),
        NodeKind::ResourceGroup => ("general/Resource_Groups", 17.000013 / 16.044224),
        NodeKind::Vnet => ("networking/Virtual_Networks", 18.006025 / 10.784314),
        NodeKind::Subnet => ("networking/Subnet", 16.998 / 10.175),
        NodeKind::Unnetworked => ("general/All_Resources", 1.0),
        // A resource is drawn as its own glyph; it has no band to sit in.
        NodeKind::Resource { .. } => return None,
    };
    // No `imageAspect=0` here: that is the flag that says "ignore the image's
    // own proportions and fill the box". The cell is cut to the stencil's
    // aspect instead, so it fills without being stretched.
    Some((
        format!(
            "image;aspect=fixed;html=1;points=[];\
             movable=0;resizable=0;deletable=0;connectable=0;editable=0;\
             image=img/lib/azure2/{stencil}.svg;"
        ),
        aspect,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_for_maps_known_types() {
        assert!(
            style_for(
                "microsoft.network/virtualnetworks",
                &crate::diagram::page::COMFORTABLE
            )
            .contains("img/lib/azure2/networking/Virtual_Networks.svg")
        );
    }

    #[test]
    fn style_for_falls_back_to_generic_icon() {
        assert!(
            style_for(
                "microsoft.custom/widgets",
                &crate::diagram::page::COMFORTABLE
            )
            .contains("general/All_Resources.svg")
        );
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

    #[test]
    fn unit_resolves_every_mapping_entry_when_pack_is_embedded() {
        let mapping = icon_mapping();
        assert!(
            ICON_DIR.get_file(&mapping.fallback).is_some(),
            "fallback icon missing: {}",
            mapping.fallback
        );
        for (azure_type, path) in &mapping.types {
            assert!(
                ICON_DIR.get_file(path).is_some(),
                "icon for {azure_type} missing from pack: {path}"
            );
        }
    }

    #[test]
    fn unit_serves_pack_icon_when_type_is_mapped() {
        let bytes = pack_icon("microsoft.compute/virtualmachines").expect("vm icon in pack");
        assert!(bytes.starts_with(b"<") || bytes.starts_with(b"<?xml".as_ref()));
    }

    #[test]
    fn unit_serves_fallback_icon_when_type_is_unmapped() {
        let unknown = pack_icon("microsoft.custom/widgets").expect("fallback icon in pack");
        let fallback = ICON_DIR
            .get_file(&icon_mapping().fallback)
            .expect("fallback file")
            .contents();
        assert_eq!(unknown, fallback);
    }
}
