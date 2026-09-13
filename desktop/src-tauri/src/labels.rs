//! The wording the desktop carries: resolved once at startup, handed to the
//! frontend on bootstrap and read by the topology builder and export phases.
//!
//! Only `common` and `desktop` cross the wire — the report, diagram, CLI and
//! TUI sections are the CLI's business. `desktop/src/generated-labels.json`
//! is written from [`AppLabels::builtin`] by the bindings test so the
//! frontend has a typed default before bootstrap and in the browser preview.

use std::sync::Arc;

use azdocs::config::Config;
use azdocs::labels::{CommonLabels, DesktopLabels, LabelPack, Labels};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AppLabels {
    /// Backend-only wording for shared report analysis; never expands bootstrap labels.
    #[serde(skip)]
    pub posture: azdocs::labels::PostureLabels,
    pub common: CommonLabels,
    pub desktop: DesktopLabels,
}

impl AppLabels {
    /// Built-ins only, ignoring any user directory — what the bindings test
    /// serialises, so CI output never depends on the host's config dir.
    pub fn builtin() -> Self {
        Self::from(&Labels::default())
    }

    /// `[branding] labels` resolved against the built-ins and the user
    /// directory. The desktop has no screen on which to explain a startup
    /// failure, so a broken override is logged and the built-ins are used;
    /// `azdocs check` reports the same error properly.
    pub fn load(config: Option<&Config>) -> Arc<Self> {
        let name = config
            .map(|config| config.branding.labels.as_str())
            .unwrap_or(azdocs::labels::DEFAULT_LABELS);
        let labels = match LabelPack::load().and_then(|pack| pack.get(name)) {
            Ok(labels) => labels,
            Err(error) => {
                eprintln!("azdocs labels could not be loaded, using the built-ins: {error}");
                Labels::default()
            }
        };
        Arc::new(Self::from(&labels))
    }
}

impl From<&Labels> for AppLabels {
    fn from(labels: &Labels) -> Self {
        Self {
            posture: labels.report.posture.clone(),
            common: labels.common.clone(),
            desktop: labels.desktop.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The frontend infers its type from this JSON, which only works when
    /// every leaf is a string, a plural table or a string map.
    #[test]
    fn unit_builtin_labels_serialise_without_nulls_or_arrays() {
        let json = serde_json::to_value(AppLabels::builtin()).unwrap();
        fn walk(value: &serde_json::Value, path: &str) {
            match value {
                serde_json::Value::Object(map) => {
                    for (key, inner) in map {
                        walk(inner, &format!("{path}.{key}"));
                    }
                }
                serde_json::Value::String(text) => assert!(!text.is_empty(), "{path} is empty"),
                other => panic!("{path}: unexpected {other}"),
            }
        }
        walk(&json, "labels");
    }

    #[test]
    fn unit_load_falls_back_to_builtins_for_an_unknown_name() {
        let mut config = Config::default();
        config.branding.labels = "no-such-labels".to_owned();

        let labels = AppLabels::load(Some(&config));

        assert_eq!(labels.common.subscription_scope, "Subscription scope");
    }
}
