use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::ConfigError;

pub const ENV_TENANT_ID: &str = "AZDOCS_TENANT_ID";
pub const ENV_CLIENT_ID: &str = "AZDOCS_CLIENT_ID";
pub const ENV_CLIENT_SECRET: &str = "AZDOCS_CLIENT_SECRET";

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub auth: AuthConfig,
    pub collect: CollectConfig,
    pub audit: AuditConfig,
    pub storage: StorageConfig,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuthConfig {
    pub tenant_id: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CollectConfig {
    /// Subscription ids to collect; empty means all visible to the credential.
    pub subscriptions: Vec<String>,
    pub concurrency: usize,
}

impl Default for CollectConfig {
    fn default() -> Self {
        Self {
            subscriptions: Vec::new(),
            concurrency: 4,
        }
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AuditConfig {
    pub required_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone)]
pub struct Credentials {
    pub tenant_id: String,
    pub client_id: String,
    pub client_secret: String,
}

impl Config {
    /// Load config from an explicit path, or search `./azdocs.toml` then the
    /// platform config dir. Environment variables override file values.
    pub fn load(explicit: Option<&Path>) -> Result<Self, ConfigError> {
        let mut config = match explicit {
            Some(path) => Self::from_file(path)?,
            None => {
                let candidates = Self::default_paths();
                match candidates.iter().find(|p| p.exists()) {
                    Some(path) => Self::from_file(path)?,
                    None => Self::default(),
                }
            }
        };
        config.apply_env_overrides();
        Ok(config)
    }

    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let raw = std::fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        toml::from_str(&raw).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source: Box::new(source),
        })
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
            .ok_or(ConfigError::MissingValue("auth.tenant_id", ENV_TENANT_ID))?;
        let client_id = self
            .auth
            .client_id
            .clone()
            .ok_or(ConfigError::MissingValue("auth.client_id", ENV_CLIENT_ID))?;
        let client_secret = self
            .auth
            .client_secret
            .clone()
            .ok_or(ConfigError::MissingValue(
                "auth.client_secret",
                ENV_CLIENT_SECRET,
            ))?;
        Ok(Credentials {
            tenant_id,
            client_id,
            client_secret,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn credentials_reports_missing_tenant() {
        let config = Config::default();

        let err = config.credentials().unwrap_err();

        assert!(err.to_string().contains("auth.tenant_id"));
    }
}
