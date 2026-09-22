use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use include_dir::{Dir, include_dir};

use super::QueryDef;
use crate::error::QueryPackError;

static BUILTIN: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/queries");

/// The merged set of built-in and user query definitions, keyed by name.
/// User definitions with the same name replace built-ins.
#[derive(Debug, Default)]
pub struct QueryPack {
    queries: BTreeMap<String, QueryDef>,
}

/// `<platform config dir>/azdocs/queries.d`, the drop-in directory for user
/// query definitions.
pub fn user_queries_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "", "azdocs").map(|dirs| dirs.config_dir().join("queries.d"))
}

impl QueryPack {
    /// Built-ins plus, when it exists, the user drop-in directory.
    pub fn load() -> Result<Self, QueryPackError> {
        let mut pack = Self::builtin()?;
        if let Some(dir) = user_queries_dir()
            && dir.is_dir()
        {
            pack.merge_dir(&dir)?;
        }
        Ok(pack)
    }

    pub fn builtin() -> Result<Self, QueryPackError> {
        let mut pack = Self::default();
        for file in files_recursive(&BUILTIN) {
            let Some(source) = file.contents_utf8() else {
                continue;
            };
            let origin = format!("builtin:{}", file.path().display());
            let def = QueryDef::parse(source, &origin)?;
            if let Some(existing) = pack.queries.get(&def.name) {
                return Err(QueryPackError::Duplicate {
                    name: def.name,
                    first: format!("builtin:{}", existing.category.clone()),
                    second: origin,
                });
            }
            pack.queries.insert(def.name.clone(), def);
        }
        Ok(pack)
    }

    /// Load every `*.toml` under `dir` (recursively), replacing same-named queries.
    pub fn merge_dir(&mut self, dir: &Path) -> Result<(), QueryPackError> {
        let mut paths = Vec::new();
        collect_toml_paths(dir, &mut paths).map_err(|source| QueryPackError::ReadDir {
            path: dir.display().to_string(),
            source,
        })?;
        paths.sort();
        for path in paths {
            let source =
                std::fs::read_to_string(&path).map_err(|source| QueryPackError::ReadDir {
                    path: path.display().to_string(),
                    source,
                })?;
            let def = QueryDef::parse(&source, &path.display().to_string())?;
            self.queries.insert(def.name.clone(), def);
        }
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<&QueryDef> {
        self.queries.get(name)
    }

    /// All queries, sorted by (category, name).
    pub fn all(&self) -> Vec<&QueryDef> {
        let mut queries: Vec<_> = self.queries.values().collect();
        queries.sort_by(|a, b| (&a.category, &a.name).cmp(&(&b.category, &b.name)));
        queries
    }

    /// Filtered selection for a collect run. Empty filters mean "all".
    pub fn select(
        &self,
        categories: &[String],
        names: &[String],
        skip: &[String],
    ) -> Vec<&QueryDef> {
        self.all()
            .into_iter()
            .filter(|def| categories.is_empty() || categories.contains(&def.category))
            .filter(|def| names.is_empty() || names.contains(&def.name))
            .filter(|def| !skip.contains(&def.name))
            .collect()
    }
}

fn files_recursive<'a>(dir: &'a Dir<'a>) -> Vec<&'a include_dir::File<'a>> {
    let mut files: Vec<_> = dir.files().collect();
    for sub in dir.dirs() {
        files.extend(files_recursive(sub));
    }
    files.retain(|f| f.path().extension().is_some_and(|ext| ext == "toml"));
    files
}

fn collect_toml_paths(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_toml_paths(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "toml") {
            out.push(path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_pack_loads_and_validates() {
        let pack = QueryPack::builtin().unwrap();

        assert!(
            pack.get("all_resources").is_some(),
            "expected all_resources in {:?}",
            pack.all().iter().map(|q| &q.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn unit_builtin_queries_use_only_arg_supported_join_flavours() {
        let pack = QueryPack::builtin().unwrap();
        for query in pack.all() {
            for clause in query.kql.split("join kind=").skip(1) {
                let flavour = clause.split_whitespace().next().unwrap();
                assert!(
                    matches!(flavour, "innerunique" | "inner" | "leftouter" | "fullouter"),
                    "{} uses unsupported ARG join {flavour}",
                    query.name
                );
            }
        }
    }

    #[test]
    fn merge_dir_overrides_builtin_by_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("all_resources.toml"),
            r#"
name = "all_resources"
category = "inventory"
kind = "inventory"
description = "user override"
kql = "resources | take 1"
"#,
        )
        .unwrap();
        let mut pack = QueryPack::builtin().unwrap();

        pack.merge_dir(dir.path()).unwrap();

        assert_eq!(
            pack.get("all_resources").unwrap().description,
            "user override"
        );
    }

    #[test]
    fn select_filters_by_category_and_skip() {
        let pack = QueryPack::builtin().unwrap();

        let selected = pack.select(
            &["inventory".to_owned()],
            &[],
            &["subscriptions".to_owned()],
        );

        assert!(
            selected
                .iter()
                .all(|q| q.category == "inventory" && q.name != "subscriptions")
        );
    }
}
