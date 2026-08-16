mod loader;

pub use loader::{QueryPack, user_queries_dir};

use serde::Deserialize;

use crate::error::QueryPackError;
use crate::model::Severity;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryKind {
    Inventory,
    Finding,
}

/// One query definition, parsed from a TOML file in `queries/` (built-in,
/// embedded) or the user `queries.d/` directory.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryDef {
    pub name: String,
    pub category: String,
    pub kind: QueryKind,
    pub description: String,
    pub kql: String,
    /// Finding queries only.
    #[serde(default)]
    pub severity: Option<Severity>,
    /// Column used to build finding titles; falls back to `name`, then `id`.
    #[serde(default)]
    pub title_field: Option<String>,
}

impl QueryDef {
    pub fn parse(source: &str, origin: &str) -> Result<Self, QueryPackError> {
        let def: Self = toml::from_str(source).map_err(|source| QueryPackError::Parse {
            path: origin.to_owned(),
            source: Box::new(source),
        })?;
        def.validate(origin)?;
        Ok(def)
    }

    fn validate(&self, origin: &str) -> Result<(), QueryPackError> {
        let invalid = |reason: String| QueryPackError::Invalid {
            path: origin.to_owned(),
            reason,
        };
        if self.name.is_empty()
            || !self
                .name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            return Err(invalid(format!(
                "name `{}` must be non-empty snake_case",
                self.name
            )));
        }
        if self.kql.trim().is_empty() {
            return Err(invalid("kql must not be empty".to_owned()));
        }
        match self.kind {
            QueryKind::Finding if self.severity.is_none() => {
                Err(invalid("finding queries require a severity".to_owned()))
            }
            QueryKind::Inventory if self.severity.is_some() => Err(invalid(
                "inventory queries must not set a severity".to_owned(),
            )),
            _ => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_accepts_valid_finding_query() {
        let def = QueryDef::parse(
            r#"
name = "nsg_open"
category = "security"
kind = "finding"
description = "test"
severity = "high"
kql = "resources | project id"
"#,
            "test.toml",
        )
        .unwrap();

        assert_eq!(def.severity, Some(Severity::High));
    }

    #[test]
    fn parse_rejects_finding_without_severity() {
        let result = QueryDef::parse(
            r#"
name = "nsg_open"
category = "security"
kind = "finding"
description = "test"
kql = "resources"
"#,
            "test.toml",
        );

        assert!(matches!(result, Err(QueryPackError::Invalid { .. })));
    }

    #[test]
    fn parse_rejects_non_snake_case_names() {
        let result = QueryDef::parse(
            r#"
name = "Bad-Name"
category = "security"
kind = "inventory"
description = "test"
kql = "resources"
"#,
            "test.toml",
        );

        assert!(matches!(result, Err(QueryPackError::Invalid { .. })));
    }
}
