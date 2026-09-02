//! Labels are data: every user-facing string lives in `data/labels/*.toml`,
//! embedded in the binary, the same shape as the query pack, themes and
//! `display_names.toml`. `[branding] labels` names the set; a partial file at
//! `<config dir>/azdocs/labels/<name>.toml` is deep-merged over the built-in
//! of that name (or over `en` when no built-in has it), so an override states
//! only what it changes and still fails on a typo.

mod schema;
pub mod template;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};

pub use schema::*;
pub use template::{counted, fill};

use crate::config::BrandingConfig;
use crate::error::LabelsError;

static BUILTIN_LABELS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/data/labels");

/// The set every install ships and every other set falls back to.
pub const DEFAULT_LABELS: &str = "en";

/// `<config dir>/azdocs/labels/` — the user drop-in directory.
pub fn user_labels_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "azdocs").map(|dirs| dirs.config_dir().join("labels"))
}

/// Built-in and user label files as raw tables, keyed by file stem. Files stay
/// unparsed until [`LabelPack::get`] merges the pair a name resolves to.
#[derive(Debug, Default)]
pub struct LabelPack {
    builtin: BTreeMap<String, toml::Table>,
    user: BTreeMap<String, toml::Table>,
}

impl LabelPack {
    /// Built-ins plus, when it exists, the user drop-in directory. A broken
    /// user file is an error rather than a silent fallback: output would
    /// otherwise carry the wrong words with no explanation.
    pub fn load() -> Result<Self, LabelsError> {
        let mut pack = Self::builtin()?;
        if let Some(dir) = user_labels_dir()
            && dir.is_dir()
        {
            pack.merge_dir(&dir)?;
        }
        Ok(pack)
    }

    pub fn builtin() -> Result<Self, LabelsError> {
        let mut pack = Self::default();
        for file in BUILTIN_LABELS.files() {
            let path = file.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let Some(source) = file.contents_utf8() else {
                continue;
            };
            let origin = format!("builtin:{}", path.display());
            pack.builtin
                .insert(stem(path), parse_table(source, &origin)?);
        }
        Ok(pack)
    }

    /// Load every `*.toml` in `dir` as a user file keyed by its stem.
    pub fn merge_dir(&mut self, dir: &Path) -> Result<(), LabelsError> {
        let read_err = |source: std::io::Error| LabelsError::ReadDir {
            path: dir.display().to_string(),
            source,
        };
        let mut paths: Vec<PathBuf> = Vec::new();
        for entry in std::fs::read_dir(dir).map_err(read_err)? {
            let path = entry.map_err(read_err)?.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("toml") {
                paths.push(path);
            }
        }
        // Deterministic order so two files claiming one name resolve the same
        // way on every platform.
        paths.sort();
        for path in paths {
            let origin = path.display().to_string();
            let source = std::fs::read_to_string(&path).map_err(|source| LabelsError::ReadDir {
                path: origin.clone(),
                source,
            })?;
            self.user
                .insert(stem(&path), parse_table(&source, &origin)?);
        }
        Ok(())
    }

    /// Every name a config may select: built-in and user, sorted.
    pub fn names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self
            .builtin
            .keys()
            .chain(self.user.keys())
            .map(String::as_str)
            .collect();
        names.sort_unstable();
        names.dedup();
        names
    }

    /// Resolve `name`: the built-in of that name (else `en`) with the user
    /// file of that name deep-merged over it. The built-in is complete, so
    /// any failure after merging is the user file's and is reported as such.
    pub fn get(&self, name: &str) -> Result<Labels, LabelsError> {
        let builtin = self.builtin.get(name);
        let user = self.user.get(name);
        if builtin.is_none() && user.is_none() {
            return Err(LabelsError::Unknown {
                name: name.to_owned(),
                available: self.names().join(", "),
            });
        }
        let mut merged = builtin
            .or_else(|| self.builtin.get(DEFAULT_LABELS))
            .cloned()
            .ok_or_else(|| LabelsError::Unknown {
                name: DEFAULT_LABELS.to_owned(),
                available: self.names().join(", "),
            })?;
        let origin = match user {
            Some(overlay) => {
                deep_merge(&mut merged, overlay.clone());
                user_labels_dir()
                    .map(|dir| dir.join(format!("{name}.toml")).display().to_string())
                    .unwrap_or_else(|| format!("{name}.toml"))
            }
            None => format!("builtin:{name}.toml"),
        };
        toml::Value::Table(merged)
            .try_into()
            .map_err(|source| LabelsError::Parse {
                path: origin,
                source: Box::new(source),
            })
    }
}

impl Labels {
    /// The shipped defaults, ignoring any user directory.
    pub fn builtin() -> Result<Self, LabelsError> {
        LabelPack::builtin()?.get(DEFAULT_LABELS)
    }
}

impl Default for Labels {
    fn default() -> Self {
        Self::builtin().expect("embedded en.toml parses; guaranteed by unit test")
    }
}

/// The labels a config selects: `[branding] labels` resolved against the
/// built-ins and the user directory. One call for every boundary.
pub fn resolve(branding: &BrandingConfig) -> Result<Labels, LabelsError> {
    LabelPack::load()?.get(&branding.labels)
}

/// Overlay `overlay` onto `base`: tables merge recursively, anything else
/// replaces. A partial user file therefore leaves every unmentioned key alone.
fn deep_merge(base: &mut toml::Table, overlay: toml::Table) {
    for (key, value) in overlay {
        match (base.get_mut(&key), value) {
            (Some(toml::Value::Table(existing)), toml::Value::Table(incoming)) => {
                deep_merge(existing, incoming);
            }
            (_, value) => {
                base.insert(key, value);
            }
        }
    }
}

fn parse_table(source: &str, origin: &str) -> Result<toml::Table, LabelsError> {
    toml::from_str(source).map_err(|source| LabelsError::Parse {
        path: origin.to_owned(),
        source: Box::new(source),
    })
}

fn stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every placeholder the code fills. A `{cnt}` typo in the built-in file
    /// fails here rather than printing itself.
    const KNOWN_PLACEHOLDERS: &[&str] = &[
        "a",
        "added",
        "audit",
        "b",
        "category",
        "action",
        "changed",
        "command",
        "connectors",
        "comparison",
        "date",
        "descriptions",
        "direction",
        "drawn",
        "edges",
        "concurrency",
        "count",
        "env",
        "error",
        "failed",
        "filter",
        "findings",
        "from",
        "format",
        "group",
        "groups",
        "high",
        "hops",
        "id",
        "index",
        "inventory",
        "key",
        "kind",
        "label",
        "medium",
        "ms",
        "name",
        "noun",
        "other",
        "pages",
        "path",
        "percent",
        "preposition",
        "preset",
        "queries",
        "regions",
        "relationships",
        "removed",
        "resource_groups",
        "resources",
        "rows",
        "scope",
        "search",
        "share",
        "status",
        "subscription",
        "subscriptions",
        "succeeded",
        "suffix",
        "tagged",
        "tags",
        "target",
        "tenant",
        "theme",
        "threshold",
        "to",
        "total",
        "type",
        "unit",
        "types",
        "untagged",
        "value",
        "visible",
    ];

    fn builtin_source() -> &'static str {
        BUILTIN_LABELS
            .get_file("en.toml")
            .and_then(|file| file.contents_utf8())
            .expect("en.toml is embedded")
    }

    fn walk(table: &toml::Table, path: &str, visit: &mut dyn FnMut(&str, &str)) {
        for (key, value) in table {
            let here = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            match value {
                toml::Value::Table(inner) => walk(inner, &here, visit),
                toml::Value::String(text) => visit(&here, text),
                other => panic!("{here}: labels hold strings and tables, found {other}"),
            }
        }
    }

    #[test]
    fn unit_builtin_labels_parse_and_default_name_ships() {
        let pack = LabelPack::builtin().unwrap();
        assert!(pack.names().contains(&DEFAULT_LABELS));
        let labels = pack.get(DEFAULT_LABELS).unwrap();
        assert_eq!(labels.report.summary.chapter, "Executive Summary");
    }

    #[test]
    fn unit_builtin_labels_have_no_empty_strings() {
        let table: toml::Table = toml::from_str(builtin_source()).unwrap();
        walk(&table, "", &mut |path, text| {
            assert!(!text.is_empty(), "{path} is empty");
        });
    }

    #[test]
    fn unit_builtin_placeholders_are_all_known() {
        let table: toml::Table = toml::from_str(builtin_source()).unwrap();
        walk(&table, "", &mut |path, text| {
            for segment in template::segments(text) {
                if let template::Segment::Placeholder(name) = segment {
                    assert!(
                        KNOWN_PLACEHOLDERS.contains(&name),
                        "{path} uses unknown placeholder {{{name}}}"
                    );
                }
            }
        });
    }

    #[test]
    fn unit_get_deep_merges_a_partial_user_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("en.toml"),
            "[report.summary]\nchapter = \"Management summary\"\n\
             [common.plurals.resource]\none = \"asset\"\n",
        )
        .unwrap();
        let mut pack = LabelPack::builtin().unwrap();
        pack.merge_dir(dir.path()).unwrap();

        let labels = pack.get("en").unwrap();

        assert_eq!(labels.report.summary.chapter, "Management summary");
        assert_eq!(labels.common.plurals.resource.one, "asset");
        assert_eq!(labels.common.plurals.resource.other, "resources");
        assert_eq!(labels.report.findings.chapter, "Findings");
    }

    #[test]
    fn unit_get_rejects_unknown_key_in_user_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("en.toml"),
            "[report.summary]\nchaptre = \"Typo\"\n",
        )
        .unwrap();
        let mut pack = LabelPack::builtin().unwrap();
        pack.merge_dir(dir.path()).unwrap();

        let err = pack.get("en").unwrap_err();

        assert!(
            matches!(&err, LabelsError::Parse { path, .. } if path.ends_with("en.toml") && !path.starts_with("builtin:")),
            "{err}"
        );
    }

    #[test]
    fn unit_get_uses_en_as_base_for_a_user_only_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("FR.toml"),
            "[report.findings]\nchapter = \"Constats\"\n",
        )
        .unwrap();
        let mut pack = LabelPack::builtin().unwrap();
        pack.merge_dir(dir.path()).unwrap();

        let labels = pack.get("fr").unwrap();

        assert_eq!(labels.report.findings.chapter, "Constats");
        assert_eq!(labels.report.summary.chapter, "Executive Summary");
        assert_eq!(pack.names(), vec!["en", "fr"]);
    }

    #[test]
    fn unit_get_reports_available_names_for_unknown_labels() {
        let err = LabelPack::builtin().unwrap().get("nope").unwrap_err();
        assert!(
            matches!(&err, LabelsError::Unknown { name, available } if name == "nope" && available == "en"),
            "{err}"
        );
    }

    #[test]
    fn unit_deep_merge_replaces_leaves_and_recurses_into_tables() {
        let mut base: toml::Table = toml::from_str("[a]\nx = \"1\"\ny = \"2\"\n").unwrap();
        let overlay: toml::Table = toml::from_str("[a]\ny = \"3\"\n").unwrap();
        deep_merge(&mut base, overlay);
        let a = base["a"].as_table().unwrap();
        assert_eq!(a["x"].as_str(), Some("1"));
        assert_eq!(a["y"].as_str(), Some("3"));
    }
}
