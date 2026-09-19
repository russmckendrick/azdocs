use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub mod document;
pub mod secrets;
pub use document::{ConfigDocument, SettingsValues, TenantProfile};

use crate::error::ConfigError;

pub const ENV_TENANT_ID: &str = "AZDOCS_TENANT_ID";
pub const ENV_CLIENT_ID: &str = "AZDOCS_CLIENT_ID";
pub const ENV_CLIENT_SECRET: &str = "AZDOCS_CLIENT_SECRET";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    /// Which Azure cloud the tenant lives in. Shared default; a tenant
    /// profile may override it.
    pub cloud: crate::cloud::Cloud,
    pub auth: AuthConfig,
    pub collect: CollectConfig,
    pub audit: AuditConfig,
    pub storage: StorageConfig,
    pub branding: BrandingConfig,
    pub report: ReportConfig,
}

#[derive(Default, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthConfig {
    pub tenant_id: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub secret_ref: Option<String>,
    pub secret_env: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(default, deny_unknown_fields)]
pub struct CollectConfig {
    /// Subscription ids to collect; empty means all visible to the credential.
    pub subscriptions: Vec<String>,
    pub concurrency: usize,
    pub retry: RetryConfig,
}

impl Default for CollectConfig {
    fn default() -> Self {
        Self {
            subscriptions: Vec::new(),
            concurrency: 4,
            retry: RetryConfig::default(),
        }
    }
}

/// `[collect.retry]`: how hard to try before a query is recorded as failed.
/// The defaults reproduce the previous fixed behaviour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ts_rs::TS)]
#[serde(default, deny_unknown_fields)]
pub struct RetryConfig {
    /// Attempts per request, 1–10.
    pub max_attempts: u32,
    /// First backoff wait in milliseconds; doubles per attempt.
    pub base_delay_ms: u64,
    /// Longest single wait in seconds, including a server's Retry-After.
    pub max_delay_secs: u64,
    /// Whole-request timeout in seconds.
    pub timeout_secs: u64,
    /// TCP/TLS connect timeout in seconds.
    pub connect_timeout_secs: u64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        let http = crate::net::HttpSettings::default();
        Self {
            max_attempts: http.retry.max_attempts,
            base_delay_ms: http.retry.base_delay.as_millis() as u64,
            max_delay_secs: http.retry.max_delay.as_secs(),
            timeout_secs: http.timeout.as_secs(),
            connect_timeout_secs: http.connect_timeout.as_secs(),
        }
    }
}

impl RetryConfig {
    pub fn http_settings(&self) -> crate::net::HttpSettings {
        let defaults = crate::net::RetryPolicy::default();
        crate::net::HttpSettings {
            timeout: std::time::Duration::from_secs(self.timeout_secs),
            connect_timeout: std::time::Duration::from_secs(self.connect_timeout_secs),
            retry: crate::net::RetryPolicy {
                max_attempts: self.max_attempts,
                base_delay: std::time::Duration::from_millis(self.base_delay_ms),
                max_delay: std::time::Duration::from_secs(self.max_delay_secs),
                ..defaults
            },
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if !(1..=10).contains(&self.max_attempts) {
            return Err(ConfigError::Invalid(
                "collect.retry.max_attempts must be between 1 and 10".into(),
            ));
        }
        if self.max_delay_secs == 0 || self.timeout_secs == 0 || self.connect_timeout_secs == 0 {
            return Err(ConfigError::Invalid(
                "collect.retry delays and timeouts must be at least 1 second".into(),
            ));
        }
        if self.base_delay_ms > self.max_delay_secs * 1000 {
            return Err(ConfigError::Invalid(
                "collect.retry.base_delay_ms must not exceed max_delay_secs".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(default, deny_unknown_fields)]
pub struct AuditConfig {
    pub required_tags: Vec<String>,
    /// Also require the tags on resource groups (tag-at-group is a common
    /// governance rule). On by default.
    pub tag_resource_groups: bool,
    /// Also require the tags on subscriptions. Off by default: few estates
    /// tag subscriptions, and every miss would be an estate-level finding.
    pub tag_subscriptions: bool,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            required_tags: Vec::new(),
            tag_resource_groups: true,
            tag_subscriptions: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(default, deny_unknown_fields)]
pub struct StorageConfig {
    pub db_path: PathBuf,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            db_path: default_db_path(),
        }
    }
}

/// How much of the estate a report prints before it stops summarising. Every
/// cap is honest: the emitter says how many rows or figures it left out, and
/// the snapshot database and data exports keep the rest.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(default, deny_unknown_fields)]
pub struct ReportConfig {
    /// Resource groups drawn individually in the technical reference; the
    /// remainder are described without a figure.
    pub max_group_diagrams: usize,
    /// Boxes per assessment figure before a relationship family is split
    /// across pages. The shared A4 layout keeps labels readable up to here.
    pub max_figure_nodes: usize,
    /// Rows a printed evidence, change or website table shows before
    /// pointing at the data exports.
    pub max_evidence_rows: usize,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            max_group_diagrams: crate::diagram::assets::MAX_GROUP_DIAGRAMS,
            max_figure_nodes: 6,
            max_evidence_rows: 20,
        }
    }
}

impl ReportConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        for (name, value) in [
            ("max_group_diagrams", self.max_group_diagrams),
            ("max_figure_nodes", self.max_figure_nodes),
            ("max_evidence_rows", self.max_evidence_rows),
        ] {
            if value == 0 {
                return Err(ConfigError::Invalid(format!(
                    "report.{name} must be at least 1"
                )));
            }
        }
        Ok(())
    }
}

/// Look and feel of the exported reports (HTML, PDF, DOCX). The defaults
/// reproduce the unbranded output exactly; every field can be overridden in
/// `[branding]`.
#[derive(Debug, Clone, Serialize, Deserialize, ts_rs::TS)]
#[serde(default, deny_unknown_fields)]
pub struct BrandingConfig {
    /// Organisation name shown on covers and footers; empty hides it.
    pub company: String,
    /// Report title (cover page, HTML `<h1>`).
    pub title: String,
    /// Optional subtitle under the title; empty hides it.
    pub subtitle: String,
    /// Main brand color as `#rrggbb` (headings, links, badges).
    pub primary_color: String,
    /// Secondary color as `#rrggbb` (dark-mode links, rules, highlights).
    pub accent_color: String,
    /// Path to a logo image (.png/.jpg/.jpeg/.gif/.svg), relative paths are
    /// resolved against the config file's directory.
    pub logo: Option<PathBuf>,
    /// PDF paper size (a Typst paper name, e.g. "a4" or "us-letter").
    pub page_size: String,
    /// PDF page margin (a Typst length, e.g. "2cm" or "1in").
    pub margin: String,
    /// Footer text on every report page.
    pub footer: String,
    /// Document theme: a file stem from `data/themes/` or from
    /// `<config dir>/azdocs/themes/`. An unknown name lists the valid ones.
    pub theme: String,
    /// Wording set: a file stem from `data/labels/` or from
    /// `<config dir>/azdocs/labels/`. An unknown name lists the valid ones.
    pub labels: String,
    /// Overrides the theme's `typography.sans` for the PDF and HTML; the
    /// family must be vendored or supplied via `font_dir`. Empty keeps it.
    pub font_family: String,
    /// Overrides the theme's `typography.mono`; empty keeps it.
    pub mono_family: String,
    /// Directory of extra `.ttf`/`.otf` faces loaded into the PDF font book,
    /// so a corporate typeface can be used without vendoring it. Relative
    /// paths resolve against this config file's directory.
    pub font_dir: Option<PathBuf>,
}

impl Default for BrandingConfig {
    fn default() -> Self {
        Self {
            company: String::new(),
            title: "Azure Estate Report".to_owned(),
            subtitle: String::new(),
            // The pre-branding hard-coded palette: Azure blue with the
            // lighter dark-mode link blue from the docs site.
            primary_color: "#0078d4".to_owned(),
            accent_color: "#4da3e8".to_owned(),
            logo: None,
            page_size: "a4".to_owned(),
            margin: "2cm".to_owned(),
            footer: "Generated by azdocs.".to_owned(),
            theme: crate::report::theme::DEFAULT_THEME.to_owned(),
            labels: crate::labels::DEFAULT_LABELS.to_owned(),
            font_family: String::new(),
            mono_family: String::new(),
            font_dir: None,
        }
    }
}

/// Platform data dir (e.g. `~/Library/Application Support/azdocs` on macOS,
/// `~/.local/share/azdocs` on Linux), falling back to the cwd.
pub fn default_db_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "azdocs")
        .map(|dirs| dirs.data_dir().join("azdocs.db"))
        .unwrap_or_else(|| PathBuf::from("azdocs.db"))
}

/// Platform config file location (e.g. `~/.config/azdocs/azdocs.toml` on
/// Linux); `azdocs init` writes here unless `--config` overrides it.
pub fn default_config_path() -> PathBuf {
    directories::ProjectDirs::from("", "", "azdocs")
        .map(|dirs| dirs.config_dir().join("azdocs.toml"))
        .unwrap_or_else(|| PathBuf::from("azdocs.toml"))
}

/// Credentials with all required values present, ready for token acquisition.
#[derive(Clone)]
pub struct Credentials {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
    pub cloud: crate::cloud::Cloud,
}

impl Config {
    /// Load config from an explicit path, or search `./azdocs.toml` then the
    /// platform config dir. Environment variables override file values.
    pub fn load(explicit: Option<&Path>) -> Result<Self, ConfigError> {
        Ok(Self::load_with_source(explicit)?.0)
    }

    /// Like [`Config::load`], but also reports which file (if any) the config
    /// came from, so relative paths inside it (e.g. `branding.logo`) can be
    /// resolved against its directory.
    pub fn load_with_source(
        explicit: Option<&Path>,
    ) -> Result<(Self, Option<PathBuf>), ConfigError> {
        let document = ConfigDocument::load(explicit)?;
        Ok((document.resolve(None)?, document.source.clone()))
    }

    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        ConfigDocument::load(Some(path))?.resolve(None)
    }

    pub fn default_paths() -> Vec<PathBuf> {
        let mut paths = vec![PathBuf::from("azdocs.toml")];
        if let Some(dirs) = directories::ProjectDirs::from("", "", "azdocs") {
            paths.push(dirs.config_dir().join("azdocs.toml"));
        }
        paths
    }

    fn apply_env_overrides(&mut self) {
        for (var, field) in [
            (ENV_TENANT_ID, &mut self.auth.tenant_id),
            (ENV_CLIENT_ID, &mut self.auth.client_id),
            (ENV_CLIENT_SECRET, &mut self.auth.client_secret),
        ] {
            if let Ok(value) = std::env::var(var)
                && !value.is_empty()
            {
                *field = Some(value);
            }
        }
    }

    /// Resolve complete credentials or say exactly what is missing and how to set it.
    pub fn credentials(&self) -> Result<Credentials, ConfigError> {
        let tenant_id = self
            .auth
            .tenant_id
            .clone()
            .filter(|s| !s.trim().is_empty())
            .ok_or(ConfigError::MissingValue("auth.tenant_id", ENV_TENANT_ID))?;
        let client_id = self
            .auth
            .client_id
            .clone()
            .filter(|s| !s.trim().is_empty())
            .ok_or(ConfigError::MissingValue("auth.client_id", ENV_CLIENT_ID))?;
        let client_secret = match (&self.auth.secret_ref, &self.auth.secret_env) {
            (Some(reference), _) => {
                secrets::SecretStore::get(&secrets::NativeSecretStore, reference)?
            }
            (_, Some(variable)) => std::env::var(variable)
                .ok()
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    ConfigError::Invalid(format!(
                        "secret environment variable `{variable}` is not set"
                    ))
                })?,
            _ => self
                .auth
                .client_secret
                .clone()
                .filter(|s| !s.is_empty())
                .ok_or(ConfigError::MissingValue(
                    "auth.client_secret",
                    ENV_CLIENT_SECRET,
                ))?,
        };
        Ok(Credentials {
            tenant_id,
            client_id,
            client_secret,
            cloud: self.cloud,
        })
    }
}

impl std::fmt::Debug for AuthConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthConfig")
            .field("tenant_id", &self.tenant_id)
            .field("client_id", &self.client_id)
            .field("client_secret", &"[redacted]")
            .field("secret_ref", &self.secret_ref)
            .field("secret_env", &self.secret_env)
            .finish()
    }
}
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("tenant_id", &self.tenant_id)
            .field("client_id", &self.client_id)
            .field("client_secret", &"[redacted]")
            .field("cloud", &self.cloud)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_report_limits_reject_zero() {
        let mut report = ReportConfig::default();
        assert!(report.validate().is_ok());
        report.max_evidence_rows = 0;
        assert!(report.validate().is_err());
    }

    #[test]
    fn from_file_parses_full_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("azdocs.toml");
        std::fs::write(
            &path,
            r#"
[auth]
tenant_id = "t"
client_id = "c"
client_secret = "s"

[collect]
subscriptions = ["sub1"]
concurrency = 8

[audit]
required_tags = ["owner"]

[storage]
db_path = "estate.db"
"#,
        )
        .unwrap();

        let config = Config::from_file(&path).unwrap();

        assert_eq!(config.collect.concurrency, 8);
    }

    #[test]
    fn from_file_rejects_unknown_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("azdocs.toml");
        std::fs::write(&path, "[auth]\ntenant = \"typo\"\n").unwrap();

        let result = Config::from_file(&path);

        assert!(matches!(result, Err(ConfigError::Parse { .. })));
    }

    #[test]
    fn unit_branding_defaults_reproduce_current_look() {
        let branding = BrandingConfig::default();

        assert_eq!(branding.title, "Azure Estate Report");
        assert_eq!(branding.primary_color, "#0078d4");
        assert_eq!(branding.accent_color, "#4da3e8");
        assert_eq!(branding.footer, "Generated by azdocs.");
        assert_eq!(branding.theme, "field-report");
        assert_eq!(branding.labels, "en");
        assert!(branding.logo.is_none());
        assert_eq!(branding.page_size, "a4");
        assert_eq!(branding.margin, "2cm");
    }

    #[test]
    fn unit_branding_rejects_unknown_field_when_typoed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("azdocs.toml");
        std::fs::write(&path, "[branding]\ncolour = \"#123456\"\n").unwrap();

        let result = Config::from_file(&path);

        assert!(matches!(result, Err(ConfigError::Parse { .. })));
    }

    #[test]
    fn unit_branding_parses_overrides_when_present() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("azdocs.toml");
        std::fs::write(
            &path,
            "[branding]\ncompany = \"Contoso\"\nprimary_color = \"#112233\"\n",
        )
        .unwrap();

        let config = Config::from_file(&path).unwrap();

        assert_eq!(config.branding.company, "Contoso");
        assert_eq!(config.branding.primary_color, "#112233");
        // Untouched fields keep their defaults.
        assert_eq!(config.branding.title, "Azure Estate Report");
    }

    #[test]
    fn from_file_parses_retry_limits_and_cloud() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("azdocs.toml");
        std::fs::write(
            &path,
            "cloud = \"usgov\"\n[collect.retry]\nmax_attempts = 3\ntimeout_secs = 30\n",
        )
        .unwrap();

        let config = Config::from_file(&path).unwrap();

        assert_eq!(config.cloud, crate::cloud::Cloud::UsGov);
        assert_eq!(config.collect.retry.max_attempts, 3);
        assert_eq!(config.collect.retry.http_settings().timeout.as_secs(), 30);
        assert_eq!(config.collect.retry.base_delay_ms, 500, "untouched default");
    }

    #[test]
    fn unit_config_rejects_retry_attempts_above_ten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("azdocs.toml");
        std::fs::write(&path, "[collect.retry]\nmax_attempts = 11\n").unwrap();

        assert!(matches!(
            Config::from_file(&path),
            Err(ConfigError::Invalid(_))
        ));
    }

    #[test]
    fn unit_config_rejects_unknown_cloud() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("azdocs.toml");
        std::fs::write(&path, "cloud = \"mars\"\n").unwrap();

        assert!(matches!(
            Config::from_file(&path),
            Err(ConfigError::Parse { .. })
        ));
    }

    #[test]
    fn credentials_reports_missing_tenant() {
        let config = Config::default();

        let err = config.credentials().unwrap_err();

        assert!(err.to_string().contains("auth.tenant_id"));
    }
}
