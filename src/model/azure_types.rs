use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

/// Built-in display names live in a data file so they can be reviewed and
/// extended without touching Rust; user overrides merge on top at runtime.
const BUILTIN_DISPLAY_NAMES: &str = include_str!("../../assets/display_names.toml");

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
