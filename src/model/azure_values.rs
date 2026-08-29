use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const BUILTIN_LOCATIONS: &str = include_str!("../../data/azure_locations.toml");
const BUILTIN_KINDS: &str = include_str!("../../data/azure_kinds.toml");

fn user_data_path(file_name: &str) -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "azdocs").map(|dirs| dirs.config_dir().join(file_name))
}

fn parse(content: &str) -> Result<BTreeMap<String, String>, toml::de::Error> {
    let map: BTreeMap<String, String> = toml::from_str(content)?;
    Ok(map
        .into_iter()
        .map(|(key, value)| (key.to_ascii_lowercase(), value))
        .collect())
}

fn build_map(builtins: &str, user_path: Option<&Path>, label: &str) -> BTreeMap<String, String> {
    let mut map = parse(builtins).unwrap_or_else(|err| {
        panic!("embedded {label} metadata is valid; guaranteed by unit test: {err}")
    });
    let Some(path) = user_path else {
        return map;
    };
    let Ok(content) = std::fs::read_to_string(path) else {
        return map;
    };
    match parse(&content) {
        Ok(overrides) => map.extend(overrides),
        Err(err) => tracing::warn!("ignoring invalid {} override: {err}", path.display()),
    }
    map
}

/// Effective programmatic-location to display-name mappings, including the
/// optional `<platform config dir>/azdocs/azure_locations.toml` overrides.
pub fn location_display_names() -> &'static BTreeMap<String, String> {
    static NAMES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let user_path = user_data_path("azure_locations.toml");
        build_map(BUILTIN_LOCATIONS, user_path.as_deref(), "Azure location")
    })
}

/// Effective resource-kind display names, keyed by
/// `<lowercase ARM type>:<lowercase kind>`.
pub fn kind_display_names() -> &'static BTreeMap<String, String> {
    static NAMES: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    NAMES.get_or_init(|| {
        let user_path = user_data_path("azure_kinds.toml");
        build_map(BUILTIN_KINDS, user_path.as_deref(), "Azure kind")
    })
}

/// Friendly name for an Azure programmatic location. Unknown locations remain
/// unchanged because concatenated region codes cannot be split reliably.
pub fn display_location(location: &str) -> Cow<'_, str> {
    location_display_names()
        .get(&location.to_ascii_lowercase())
        .map_or_else(|| Cow::Borrowed(location), |name| Cow::Owned(name.clone()))
}

/// Friendly name for a resource-specific Azure `kind` value. Exact and
/// wildcard mappings win; otherwise CamelCase and separators are humanised.
pub fn display_kind<'a>(azure_type: &str, kind: &'a str) -> Cow<'a, str> {
    let kind_lower = kind.to_ascii_lowercase();
    let exact = format!("{}:{kind_lower}", azure_type.to_ascii_lowercase());
    let wildcard = format!("*:{kind_lower}");
    if let Some(name) = kind_display_names()
        .get(&exact)
        .or_else(|| kind_display_names().get(&wildcard))
    {
        return Cow::Owned(name.clone());
    }

    let humanised = humanize_identifier(kind);
    if humanised == kind {
        Cow::Borrowed(kind)
    } else {
        Cow::Owned(humanised)
    }
}

fn humanize_identifier(value: &str) -> String {
    let characters: Vec<char> = value.chars().collect();
    let mut output = String::with_capacity(value.len() + 8);
    for (index, current) in characters.iter().copied().enumerate() {
        let previous = index
            .checked_sub(1)
            .and_then(|i| characters.get(i))
            .copied();
        let next = characters.get(index + 1).copied();

        if matches!(current, '_' | '-') {
            if !output.ends_with(' ') && !output.is_empty() {
                output.push(' ');
            }
            continue;
        }
        if current == ',' {
            while output.ends_with(' ') {
                output.pop();
            }
            output.push_str(", ");
            continue;
        }

        let starts_word = current.is_uppercase()
            && previous.is_some_and(|char| char.is_lowercase() || char.is_ascii_digit());
        let ends_acronym = current.is_uppercase()
            && previous.is_some_and(char::is_uppercase)
            && next.is_some_and(char::is_lowercase);
        if (starts_word || ends_acronym) && !output.ends_with(' ') && !output.is_empty() {
            output.push(' ');
        }
        output.push(current);
    }
    output.trim().to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_parses_embedded_location_metadata_when_loading_builtins() {
        let map = parse(BUILTIN_LOCATIONS).expect("embedded locations must parse");
        assert_eq!(map.get("uksouth").map(String::as_str), Some("UK South"));
    }

    #[test]
    fn unit_maps_known_location_when_displaying_programmatic_name() {
        assert_eq!(display_location("UKSOUTH"), "UK South");
    }

    #[test]
    fn unit_preserves_unknown_location_when_displaying_programmatic_name() {
        assert_eq!(display_location("customedgezone"), "customedgezone");
    }

    #[test]
    fn unit_maps_scoped_kind_when_displaying_known_value() {
        assert_eq!(
            display_kind("microsoft.documentdb/databaseaccounts", "GlobalDocumentDB"),
            "Global Document DB"
        );
    }

    #[test]
    fn unit_splits_camel_case_when_displaying_unknown_kind() {
        assert_eq!(
            display_kind("microsoft.custom/widgets", "SQLDatabaseV2"),
            "SQL Database V2"
        );
    }
}
