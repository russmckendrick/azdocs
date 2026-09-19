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

    /// The pack's own invariants, checked over every embedded query so a new
    /// TOML file cannot ship a shape the ARG client or ingest cannot handle.
    mod pack_lint {
        use crate::querypack::{QueryKind, QueryPack};

        fn last_stage(kql: &str) -> String {
            kql.trim()
                .rsplit('|')
                .next()
                .unwrap_or_default()
                .trim()
                .to_owned()
        }

        /// The last projection stage of a query, if any.
        fn last_projection(kql: &str) -> Option<String> {
            kql.split('|')
                .map(str::trim)
                .rfind(|stage| stage.starts_with("project") || stage.starts_with("summarize"))
                .map(str::to_owned)
        }

        /// Queries that project `id` page on it; the few that do not (an
        /// aggregate, or the subscription list keyed by subscriptionId)
        /// still need a deterministic order for `$skipToken`.
        #[test]
        fn unit_builtin_queries_end_with_deterministic_ordering() {
            let pack = QueryPack::builtin().unwrap();
            for def in pack.all() {
                let stage = last_stage(&def.kql);
                let projects_id = last_projection(&def.kql)
                    .is_none_or(|projection| !whole_words(&projection, "id").is_empty());
                if !projects_id {
                    assert!(
                        stage.starts_with("order by "),
                        "{} aggregates but ends with `{stage}`",
                        def.name
                    );
                } else {
                    assert!(
                        stage.starts_with("order by id asc"),
                        "{} ends with `{stage}` instead of `order by id asc`",
                        def.name
                    );
                }
            }
        }

        fn is_ident(c: char) -> bool {
            c.is_ascii_alphanumeric() || c == '_'
        }

        /// Every occurrence of `word` that is a whole identifier, with the
        /// characters just before and after it.
        fn whole_words(text: &str, word: &str) -> Vec<(Option<char>, Option<char>)> {
            let mut found = Vec::new();
            for (index, _) in text.match_indices(word) {
                let before = text[..index].chars().next_back();
                let after = text[index + word.len()..].chars().next();
                if before.is_none_or(|c| !is_ident(c)) && after.is_none_or(|c| !is_ident(c)) {
                    found.push((before, after));
                }
            }
            found
        }

        /// A projected column called `count`: `count =` or a bare `count` in a
        /// projection list. `count()` and `["count"]` are fine.
        fn projects_count_column(kql: &str) -> bool {
            for (index, _) in kql.match_indices("count") {
                let before = kql[..index].trim_end().chars().next_back();
                let raw_after = kql[index + 5..].chars().next();
                if kql[..index].chars().next_back().is_some_and(is_ident)
                    || raw_after.is_some_and(|c| is_ident(c) || c == '(' || c == '"')
                {
                    continue;
                }
                let rest = kql[index + 5..].trim_start_matches([' ', '\t']);
                let after = rest.chars().next();
                let assigned = after == Some('=') && rest.chars().nth(1) != Some('=');
                let listed = before == Some(',')
                    && matches!(after, Some(',') | Some('\n') | Some('\r') | None);
                if (assigned || listed) && before != Some('"') {
                    return true;
                }
            }
            false
        }

        #[test]
        fn unit_builtin_queries_never_project_a_count_column() {
            let pack = QueryPack::builtin().unwrap();
            for def in pack.all() {
                assert!(
                    !projects_count_column(&def.kql),
                    "{} projects a column named `count` (ARG rejects it)",
                    def.name
                );
            }
            assert!(projects_count_column(
                "resources | summarize count = count() by type"
            ));
            assert!(projects_count_column(
                "resources | project id, count\n| order by id asc"
            ));
            assert!(!projects_count_column(
                "resources | summarize total = count() by type"
            ));
        }

        /// `title_field` must be projected. `resource_id_field` may be named
        /// without being projected: that is how a scope-level finding (policy
        /// exemptions) declares "no resource association" on purpose.
        #[test]
        fn unit_builtin_findings_declare_severity_and_projected_fields() {
            let pack = QueryPack::builtin().unwrap();
            for def in pack
                .all()
                .into_iter()
                .filter(|d| d.kind == QueryKind::Finding)
            {
                assert!(def.severity.is_some(), "{} has no severity", def.name);
                if let Some(field) = &def.title_field {
                    assert!(
                        !whole_words(&def.kql, field).is_empty(),
                        "{} names `{field}` but never projects it",
                        def.name
                    );
                }
                assert!(
                    def.kql.contains("project id") || def.kql.contains("summarize by id"),
                    "{} must keep `id` for paging",
                    def.name
                );
            }
        }
    }
}
