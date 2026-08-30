use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Built-in display names live in a data file so they can be reviewed and
/// extended without touching Rust; user overrides merge on top at runtime.
const BUILTIN_DISPLAY_NAMES: &str = include_str!("../../data/display_names.toml");

/// `<platform config dir>/azdocs/display_names.toml`, the user override file
/// merged over the built-in display names by key (same pattern as
/// `querypack::loader::user_queries_dir`).
pub fn user_display_names_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "azdocs")
        .map(|dirs| dirs.config_dir().join("display_names.toml"))
}

fn parse(content: &str) -> Result<BTreeMap<String, String>, toml::de::Error> {
    let map: BTreeMap<String, String> = toml::from_str(content)?;
    // Lookups are by normalized (lowercase) ARM type; forgive casing in files.
    Ok(map
        .into_iter()
        .map(|(key, value)| (key.to_ascii_lowercase(), value))
        .collect())
}

fn build_map(user_content: Option<&str>) -> BTreeMap<String, String> {
    let mut map = parse(BUILTIN_DISPLAY_NAMES)
        .expect("embedded display_names.toml is valid; guaranteed by unit test");
    if let Some(content) = user_content {
        match parse(content) {
            Ok(overrides) => map.extend(overrides),
            Err(err) => tracing::warn!("ignoring invalid display_names.toml override: {err}"),
        }
    }
    map
}

fn names() -> &'static BTreeMap<String, String> {
    static NAMES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let user_content =
            user_display_names_path().and_then(|path| std::fs::read_to_string(path).ok());
        build_map(user_content.as_deref())
    })
}

/// Types drawn as part of their virtual machine rather than beside it.
///
/// A NIC and an OS disk are attachments: on a diagram they say nothing the VM
/// does not already say, and at one to three per VM they crowd out the
/// resources a reader is actually looking for. Both the print diagrams and the
/// desktop topology fold them; this is the one list, because the two disagreed
/// about disks for as long as they each had their own.
pub const FOLDS_INTO_VM: [&str; 2] = [
    "microsoft.network/networkinterfaces",
    "microsoft.compute/disks",
];

/// Does this type get folded into an attached virtual machine?
pub fn folds_into_vm(azure_type: &str) -> bool {
    FOLDS_INTO_VM.contains(&azure_type)
}

/// Is this a *child* resource type — `provider/parent/child`, like a VM
/// extension or a SQL database — rather than a top-level one?
///
/// ARM encodes the nesting in the type string: a top-level type has exactly one
/// slash (`microsoft.sql/servers`), a child has two or more
/// (`microsoft.sql/servers/databases`). Three call sites had written this test
/// three ways — `> 1`, `>= 2`, and `< 2` negated — which is the same number
/// said differently and one edit away from disagreeing.
///
/// Note this asks about the *type* only. Whether a particular child belongs to
/// a particular parent is an id-prefix question the caller answers itself.
pub fn is_child_type(azure_type: &str) -> bool {
    azure_type.matches('/').count() > 1
}

/// Display name for a (lowercased) Azure resource type. Types without a
/// mapping fall back to the last path segment of the type string.
pub fn display_name(azure_type: &str) -> &str {
    names()
        .get(azure_type)
        .map(String::as_str)
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
    fn unit_parses_embedded_display_names_when_loading_builtins() {
        let map = parse(BUILTIN_DISPLAY_NAMES).expect("embedded file must parse");
        assert!(map.len() > 60);
        assert_eq!(
            map.get("microsoft.desktopvirtualization/hostpools")
                .map(String::as_str),
            Some("AVD Host Pool")
        );
    }

    #[test]
    fn unit_maps_known_types_when_looking_up_display_name() {
        assert_eq!(
            display_name("microsoft.network/virtualnetworks"),
            "Virtual Network"
        );
        assert_eq!(
            display_name("microsoft.hybridcompute/machines"),
            "Arc-enabled Server"
        );
    }

    #[test]
    fn unit_falls_back_to_last_segment_when_type_is_unmapped() {
        assert_eq!(display_name("microsoft.custom/widgets"), "widgets");
    }

    #[test]
    fn unit_merges_user_overrides_when_building_map() {
        let map = build_map(Some(
            "\"microsoft.compute/virtualmachines\" = \"VM\"\n\"Custom.Thing/gadgets\" = \"Gadget\"\n",
        ));
        assert_eq!(
            map.get("microsoft.compute/virtualmachines")
                .map(String::as_str),
            Some("VM")
        );
        assert_eq!(
            map.get("custom.thing/gadgets").map(String::as_str),
            Some("Gadget")
        );
        // Built-ins not overridden survive the merge.
        assert_eq!(
            map.get("microsoft.keyvault/vaults").map(String::as_str),
            Some("Key Vault")
        );
    }

    #[test]
    fn unit_keeps_builtins_when_user_override_is_invalid() {
        let map = build_map(Some("not valid toml ==="));
        assert_eq!(
            map.get("microsoft.compute/virtualmachines")
                .map(String::as_str),
            Some("Virtual Machine")
        );
    }
}

#[cfg(test)]
mod child_type_tests {
    use super::is_child_type;

    #[test]
    fn unit_treats_two_segment_types_as_top_level_when_classifying() {
        assert!(!is_child_type("microsoft.compute/virtualmachines"));
        assert!(!is_child_type("microsoft.sql/servers"));
    }

    #[test]
    fn unit_treats_three_segment_types_as_children_when_classifying() {
        assert!(is_child_type("microsoft.sql/servers/databases"));
        assert!(is_child_type(
            "microsoft.compute/virtualmachines/extensions"
        ));
    }
}
