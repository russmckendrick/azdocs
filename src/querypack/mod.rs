mod loader;

pub use loader::{QueryPack, user_queries_dir};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::error::QueryPackError;
use crate::model::{AuthorizationScope, EvidenceFreshness, QueryProvenance, QuerySource, Severity};

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
    #[serde(default)]
    pub authorization_scope: Option<AuthorizationScope>,
    #[serde(default)]
    pub source: Option<QuerySource>,
    #[serde(default)]
    pub freshness: Option<EvidenceFreshness>,
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
    /// Affected-resource column for findings; `id` remains the unique evidence
    /// row used for pagination. Missing values do not fall back to evidence IDs.
    #[serde(default)]
    pub resource_id_field: Option<String>,
}

impl QueryDef {
    /// Store the exact executed text, including user overrides and KQL comments.
    /// The digest identifies the query; scope and thresholds remain explicit fields.
    pub fn provenance(&self, subscriptions: &[String]) -> QueryProvenance {
        let mut subscriptions: Vec<_> = subscriptions.iter().map(|s| s.to_lowercase()).collect();
        subscriptions.sort();
        subscriptions.dedup();
        QueryProvenance {
            kql: self.kql.clone(),
            kql_sha256: format!("{:x}", Sha256::digest(self.kql.as_bytes())),
            category: self.category.clone(),
            description: self.description.clone(),
            kind: match self.kind {
                QueryKind::Inventory => "inventory",
                QueryKind::Finding => "finding",
            }
            .to_owned(),
            severity: self.severity,
            title_field: self.title_field.clone(),
            resource_id_field: self.resource_id_field.clone(),
            source: self.source.clone(),
            freshness: self.freshness.clone(),
            authorization_scope: self.authorization_scope.unwrap_or_default(),
            subscriptions,
        }
    }

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
        if self
            .freshness
            .as_ref()
            .is_some_and(|f| f.timestamp_field.trim().is_empty() || f.max_age_hours == 0)
        {
            return Err(invalid(
                "freshness requires a timestamp_field and positive max_age_hours".to_owned(),
            ));
        }
        if self.source.as_ref().is_some_and(|s| {
            s.urls.is_empty()
                || s.urls.iter().any(|u| !u.starts_with("https://"))
                || chrono::NaiveDate::parse_from_str(&s.reviewed_on, "%Y-%m-%d").is_err()
        }) {
            return Err(invalid(
                "source requires HTTPS urls and a YYYY-MM-DD reviewed_on date".to_owned(),
            ));
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
