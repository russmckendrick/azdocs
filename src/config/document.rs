//! File editing and resolution are independent of credential acquisition.
use super::secrets::SecretStore;
use super::{
    AuditConfig, AuthConfig, BrandingConfig, CollectConfig, Config, ReportConfig, StorageConfig,
};
use crate::error::ConfigError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use ts_rs::TS;

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default, deny_unknown_fields)]
#[ts(optional_fields = nullable)]
pub struct BrandingOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mono_family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub font_dir: Option<PathBuf>,
}

impl BrandingOverrides {
    fn apply(&self, target: &mut BrandingConfig) {
        if let Some(value) = &self.company {
            target.company = value.clone();
        }
        if let Some(value) = &self.title {
            target.title = value.clone();
        }
        if let Some(value) = &self.subtitle {
            target.subtitle = value.clone();
        }
        if let Some(value) = &self.primary_color {
            target.primary_color = value.clone();
        }
        if let Some(value) = &self.accent_color {
            target.accent_color = value.clone();
        }
        if let Some(value) = &self.logo {
            target.logo = Some(value.clone()).filter(|p| !p.as_os_str().is_empty());
        }
        if let Some(value) = &self.page_size {
            target.page_size = value.clone();
        }
        if let Some(value) = &self.margin {
            target.margin = value.clone();
        }
        if let Some(value) = &self.footer {
            target.footer = value.clone();
        }
        if let Some(value) = &self.theme {
            target.theme = value.clone();
        }
        if let Some(value) = &self.labels {
            target.labels = value.clone();
        }
        if let Some(value) = &self.font_family {
            target.font_family = value.clone();
        }
        if let Some(value) = &self.mono_family {
            target.mono_family = value.clone();
        }
        if let Some(value) = &self.font_dir {
            target.font_dir = Some(value.clone()).filter(|p| !p.as_os_str().is_empty());
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default, deny_unknown_fields)]
#[ts(optional_fields = nullable)]
pub struct CollectOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscriptions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<super::RetryConfig>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default, deny_unknown_fields)]
#[ts(optional_fields = nullable)]
pub struct AuditOverrides {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_resource_groups: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tag_subscriptions: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, TS)]
#[serde(default, deny_unknown_fields)]
#[ts(optional_fields = nullable)]
pub struct TenantProfile {
    pub name: String,
    pub tenant_id: String,
    pub client_id: String,
    /// Overrides the shared `cloud` for this tenant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloud: Option<crate::cloud::Cloud>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret_env: Option<String>,
    pub collect: CollectOverrides,
    pub audit: AuditOverrides,
    pub branding: BrandingOverrides,
}

/// This is the editable, secret-free document and the settings wire contract.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(default, deny_unknown_fields)]
#[ts(optional_fields = nullable)]
pub struct SettingsValues {
    pub schema_version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_tenant: Option<String>,
    /// Shared cloud default; `TenantProfile.cloud` overrides it.
    pub cloud: crate::cloud::Cloud,
    pub tenants: BTreeMap<String, TenantProfile>,
    pub collect: CollectConfig,
    pub audit: AuditConfig,
    pub storage: StorageConfig,
    pub branding: BrandingConfig,
    pub report: ReportConfig,
}

impl Default for SettingsValues {
    fn default() -> Self {
        let config = Config::default();
        Self {
            schema_version: 2,
            default_tenant: None,
            cloud: config.cloud,
            tenants: BTreeMap::new(),
            collect: config.collect,
            audit: config.audit,
            storage: config.storage,
            branding: config.branding,
            report: config.report,
        }
    }
}

impl SettingsValues {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if !matches!(self.schema_version, 1 | 2) {
            return invalid("unsupported config schema_version");
        }
        if self
            .default_tenant
            .as_ref()
            .is_some_and(|r| !self.tenants.contains_key(r))
        {
            return invalid("default_tenant must name a configured tenant");
        }
        let mut ids = std::collections::BTreeSet::new();
        for (reference, profile) in &self.tenants {
            if reference.is_empty()
                || !reference
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                return invalid(
                    "tenant references must contain only letters, numbers, hyphens or underscores",
                );
            }
            if profile.name.trim().is_empty() {
                return invalid(&format!("tenants.{reference}.name is required"));
            }
            for (field, id) in [
                ("tenant_id", &profile.tenant_id),
                ("client_id", &profile.client_id),
            ] {
                if uuid::Uuid::parse_str(id).is_err() {
                    return invalid(&format!("tenants.{reference}.{field} must be a UUID"));
                }
            }
            if !ids.insert(profile.tenant_id.to_ascii_lowercase()) {
                return invalid("configure each tenant ID once; edit its existing profile instead");
            }
            if profile.secret_ref.is_some() && profile.secret_env.is_some() {
                return invalid(&format!(
                    "tenants.{reference} must select one secret source"
                ));
            }
            if profile
                .secret_ref
                .as_ref()
                .is_some_and(|v| uuid::Uuid::parse_str(v).is_err())
            {
                return invalid(&format!(
                    "tenants.{reference}.secret_ref must be a credential reference UUID"
                ));
            }
            if let Some(variable) = &profile.secret_env
                && (variable.is_empty()
                    || variable.starts_with(|c: char| c.is_ascii_digit())
                    || !variable
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_'))
            {
                return invalid(&format!(
                    "tenants.{reference}.secret_env must name an environment variable"
                ));
            }
            let resolved = self.resolved(Some(reference))?;
            if resolved
                .collect
                .subscriptions
                .iter()
                .any(|id| parse_subscription_id(id).is_err())
            {
                return invalid(&format!(
                    "tenants.{reference}.collect.subscriptions must contain subscription UUIDs"
                ));
            }
            validate_runtime(&resolved)?;
        }
        validate_runtime(&self.resolved_defaults())
    }

    pub fn selected_reference(
        &self,
        explicit: Option<&str>,
    ) -> Result<Option<String>, ConfigError> {
        if self.tenants.is_empty() {
            return Ok(None);
        }
        if let Some(reference) = explicit {
            if self.tenants.contains_key(reference) {
                return Ok(Some(reference.to_owned()));
            }
            if let Some((key, _)) = self
                .tenants
                .iter()
                .find(|(_, p)| p.tenant_id.eq_ignore_ascii_case(reference))
            {
                return Ok(Some(key.clone()));
            }
            return invalid(&format!("unknown tenant `{reference}`"));
        }
        if let Some(reference) = &self.default_tenant {
            return Ok(Some(reference.clone()));
        }
        if self.tenants.len() == 1 {
            return Ok(self.tenants.keys().next().cloned());
        }
        invalid(&format!(
            "select a tenant with --tenant: {}",
            self.tenants.keys().cloned().collect::<Vec<_>>().join(", ")
        ))
    }

    pub fn resolved_defaults(&self) -> Config {
        Config {
            cloud: self.cloud,
            auth: AuthConfig::default(),
            collect: self.collect.clone(),
            audit: self.audit.clone(),
            storage: self.storage.clone(),
            branding: self.branding.clone(),
            report: self.report.clone(),
        }
    }

    pub fn resolved(&self, reference: Option<&str>) -> Result<Config, ConfigError> {
        let mut config = self.resolved_defaults();
        if let Some(reference) = self.selected_reference(reference)? {
            let profile = self
                .tenants
                .get(&reference)
                .ok_or_else(|| ConfigError::Invalid("default tenant no longer exists".into()))?;
            config.auth = AuthConfig {
                tenant_id: Some(profile.tenant_id.to_ascii_lowercase()),
                client_id: Some(profile.client_id.clone()),
                client_secret: None,
                secret_ref: profile.secret_ref.clone(),
                secret_env: profile.secret_env.clone(),
            };
            if let Some(subscriptions) = &profile.collect.subscriptions {
                config.collect.subscriptions = subscriptions.clone();
            }
            if let Some(concurrency) = profile.collect.concurrency {
                config.collect.concurrency = concurrency;
            }
            if let Some(retry) = &profile.collect.retry {
                config.collect.retry = retry.clone();
            }
            if let Some(cloud) = profile.cloud {
                config.cloud = cloud;
            }
            if let Some(tags) = &profile.audit.required_tags {
                config.audit.required_tags = tags.clone();
            }
            if let Some(groups) = profile.audit.tag_resource_groups {
                config.audit.tag_resource_groups = groups;
            }
            if let Some(subscriptions) = profile.audit.tag_subscriptions {
                config.audit.tag_subscriptions = subscriptions;
            }
            profile.branding.apply(&mut config.branding);
        }
        Ok(config)
    }
}

fn invalid<T>(message: &str) -> Result<T, ConfigError> {
    Err(ConfigError::Invalid(message.into()))
}

/// The one rule for a subscription id, shared by the config validator and
/// the command line so `--subscriptions` cannot smuggle in what the file
/// would reject.
pub fn parse_subscription_id(value: &str) -> Result<String, String> {
    if uuid::Uuid::parse_str(value).is_ok() {
        Ok(value.to_owned())
    } else {
        Err(format!("`{value}` is not a subscription UUID"))
    }
}

/// The one rule for query concurrency, shared with the command line.
pub fn parse_concurrency(value: &str) -> Result<usize, String> {
    let parsed: usize = value
        .parse()
        .map_err(|_| format!("`{value}` is not a number"))?;
    if (1..=64).contains(&parsed) {
        Ok(parsed)
    } else {
        Err("concurrency must be between 1 and 64".into())
    }
}

fn validate_runtime(config: &Config) -> Result<(), ConfigError> {
    if parse_concurrency(&config.collect.concurrency.to_string()).is_err() {
        return invalid("collect.concurrency must be between 1 and 64");
    }
    config.collect.retry.validate()?;
    config.report.validate()?;
    for color in [
        &config.branding.primary_color,
        &config.branding.accent_color,
    ] {
        if color.len() != 7
            || !color.starts_with('#')
            || !color[1..].chars().all(|c| c.is_ascii_hexdigit())
        {
            return invalid("branding colours must be six-digit #rrggbb values");
        }
    }
    if config.storage.db_path.as_os_str().is_empty() {
        return invalid("storage.db_path must not be empty");
    }
    for tag in &config.audit.required_tags {
        if tag.trim().is_empty() {
            return invalid("audit.required_tags must not contain empty tags");
        }
    }
    Ok(())
}

pub struct ConfigDocument {
    pub values: SettingsValues,
    pub source: Option<PathBuf>,
    pub revision: String,
    pub legacy_auth: AuthConfig,
    raw: String,
}

impl ConfigDocument {
    pub fn load(explicit: Option<&Path>) -> Result<Self, ConfigError> {
        let source = explicit
            .map(Path::to_path_buf)
            .or_else(|| Config::default_paths().into_iter().find(|p| p.exists()));
        let raw = match &source {
            Some(path) => std::fs::read_to_string(path).map_err(|source| {
                // An explicit `--config` that does not exist is a typo, not
                // an invitation to run with defaults; say where we looked.
                if explicit.is_some() && source.kind() == std::io::ErrorKind::NotFound {
                    ConfigError::NotFound {
                        paths_tried: vec![path.clone()],
                    }
                } else {
                    ConfigError::Read {
                        path: path.clone(),
                        source,
                    }
                }
            })?,
            None => String::new(),
        };
        Self::parse(&raw, source)
    }

    pub fn parse(raw: &str, source: Option<PathBuf>) -> Result<Self, ConfigError> {
        let source = source.map(|p| {
            if p.is_absolute() {
                p
            } else {
                std::env::current_dir().unwrap_or_default().join(p)
            }
        });
        let path = source.clone().unwrap_or_else(super::default_config_path);
        let parse_error = |source| ConfigError::Parse {
            path: path.clone(),
            source: Box::new(crate::error::ConfigParseError::new(&source, raw)),
        };
        let mut table: toml::Table = toml::from_str(raw).map_err(parse_error)?;
        let legacy_auth: AuthConfig = table
            .remove("auth")
            .map(|v| v.try_into())
            .transpose()
            .map_err(parse_error)?
            .unwrap_or_default();
        let mut values: SettingsValues =
            toml::Value::Table(table).try_into().map_err(parse_error)?;
        if legacy_auth.tenant_id.is_some()
            || legacy_auth.client_id.is_some()
            || legacy_auth.client_secret.is_some()
        {
            if !values.tenants.is_empty() {
                return invalid(
                    "legacy [auth] cannot be combined with named tenants; migrate the config first",
                );
            }
            values.schema_version = 1;
        }
        values.validate()?;
        Ok(Self {
            values,
            source,
            revision: revision(raw),
            legacy_auth,
            raw: raw.into(),
        })
    }

    pub fn resolve(&self, reference: Option<&str>) -> Result<Config, ConfigError> {
        let mut config = self.values.resolved(reference)?;
        if self.values.tenants.is_empty() {
            config.auth = self.legacy_auth.clone();
            config.apply_env_overrides();
            if let Some(reference) = reference
                && !config
                    .auth
                    .tenant_id
                    .as_ref()
                    .is_some_and(|id| id.eq_ignore_ascii_case(reference))
            {
                return invalid(&format!("unknown tenant `{reference}`"));
            }
        }
        if let Some(parent) = self.source.as_ref().and_then(|p| p.parent()) {
            if config.storage.db_path.is_relative() {
                config.storage.db_path = parent.join(&config.storage.db_path);
            }
            for path in [&mut config.branding.logo, &mut config.branding.font_dir]
                .into_iter()
                .flatten()
            {
                if path.is_relative() {
                    *path = parent.join(&*path);
                }
            }
        }
        Ok(config)
    }

    pub fn for_snapshot(&self, tenant_id: &str) -> Result<Config, ConfigError> {
        if self
            .values
            .tenants
            .values()
            .any(|p| p.tenant_id.eq_ignore_ascii_case(tenant_id))
        {
            return self.resolve(Some(tenant_id));
        }
        let mut config = self.values.resolved_defaults();
        if let Some(parent) = self.source.as_ref().and_then(|p| p.parent()) {
            for path in [&mut config.branding.logo, &mut config.branding.font_dir]
                .into_iter()
                .flatten()
            {
                if path.is_relative() {
                    *path = parent.join(&*path);
                }
            }
        }
        Ok(config)
    }

    pub fn has_legacy(&self) -> bool {
        self.values.schema_version == 1
    }

    pub fn save<S: SecretStore>(
        &self,
        path: &Path,
        expected_revision: &str,
        mut values: SettingsValues,
        new_secrets: &BTreeMap<String, String>,
        migrate: bool,
        secrets: &S,
    ) -> Result<Self, ConfigError> {
        let parent = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        std::fs::create_dir_all(parent).map_err(|source| ConfigError::Read {
            path: parent.into(),
            source,
        })?;
        let lock_path = path.with_extension("toml.lock");
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let lock = options
            .open(&lock_path)
            .map_err(|source| ConfigError::Read {
                path: lock_path,
                source,
            })?;
        lock.try_lock().map_err(|_| ConfigError::Conflict)?;
        let current = read_optional(path)?;
        if revision(&current) != expected_revision {
            return Err(ConfigError::Conflict);
        }
        if self.has_legacy() && !migrate {
            return invalid("migrate the legacy config before saving settings");
        }
        values.schema_version = 2;
        values.validate()?;
        let mut staged = Vec::new();
        let result = (|| {
            for (reference, secret) in new_secrets {
                let profile = values.tenants.get_mut(reference).ok_or_else(|| {
                    ConfigError::Invalid("secret update names an unknown tenant".into())
                })?;
                if secret.is_empty() {
                    return invalid("new secret must not be empty");
                }
                let id = uuid::Uuid::new_v4().to_string();
                secrets.set(&id, secret)?;
                staged.push(id.clone());
                if secrets.get(&id)? != *secret {
                    return invalid("credential store could not verify the saved secret");
                }
                profile.secret_ref = Some(id);
                profile.secret_env = None;
            }
            let mut document = self
                .raw
                .parse::<toml_edit::DocumentMut>()
                .map_err(|_| ConfigError::Invalid("cannot edit invalid TOML".into()))?;
            document.remove("auth");
            let rendered = toml::to_string(&values)
                .map_err(|_| ConfigError::Invalid("could not serialize configuration".into()))?;
            let replacement = rendered
                .parse::<toml_edit::DocumentMut>()
                .map_err(|_| ConfigError::Invalid("could not render configuration".into()))?;
            update_table(document.as_table_mut(), replacement.as_table());
            let rendered = document.to_string();
            if revision(&read_optional(path)?) != expected_revision {
                return Err(ConfigError::Conflict);
            }
            if !current.is_empty() {
                let backup = path.with_extension(format!("toml.backup-{}", uuid::Uuid::new_v4()));
                atomic_write(&backup, current.as_bytes())?;
            }
            let next = Self::parse(&rendered, Some(path.to_path_buf()))?;
            atomic_write(path, rendered.as_bytes())?;
            Ok(next)
        })();
        if result.is_err() {
            for id in staged {
                let _ = secrets.delete(&id);
            }
        }
        // Old references can be shared by a copied config or a protected backup.
        // Retain them rather than breaking rollback or another installation.
        result
    }

    pub fn migration(
        &self,
        reference: &str,
        name: &str,
    ) -> Result<(SettingsValues, BTreeMap<String, String>), ConfigError> {
        if !self.values.tenants.is_empty() {
            return invalid("config already uses named tenants");
        }
        let mut values = self.values.clone();
        values.schema_version = 2;
        values.default_tenant = Some(reference.into());
        values.tenants.insert(
            reference.into(),
            TenantProfile {
                name: name.into(),
                tenant_id: self
                    .legacy_auth
                    .tenant_id
                    .clone()
                    .ok_or_else(|| ConfigError::Invalid("tenant ID is missing".into()))?,
                client_id: self
                    .legacy_auth
                    .client_id
                    .clone()
                    .ok_or_else(|| ConfigError::Invalid("client ID is missing".into()))?,
                secret_env: Some(super::ENV_CLIENT_SECRET.into()),
                ..TenantProfile::default()
            },
        );
        let mut secrets = BTreeMap::new();
        // Only migrate file secrets. Environment values remain in the environment.
        if let Some(secret) = &self.legacy_auth.client_secret {
            secrets.insert(reference.into(), secret.clone());
        }
        values.validate()?;
        Ok((values, secrets))
    }
}

fn update_table(target: &mut toml_edit::Table, replacement: &toml_edit::Table) {
    target.retain(|key, _| replacement.contains_key(key));
    for (key, value) in replacement {
        if let (Some(left), Some(right)) = (
            target.get_mut(key).and_then(toml_edit::Item::as_table_mut),
            value.as_table(),
        ) {
            update_table(left, right);
        } else if target.get(key).map(ToString::to_string) != Some(value.to_string()) {
            let mut next = value.clone();
            if let (Some(old), Some(new)) = (
                target.get(key).and_then(toml_edit::Item::as_value),
                next.as_value_mut(),
            ) {
                *new.decor_mut() = old.decor().clone();
            }
            target.insert(key, next);
        }
    }
}

pub fn revision(raw: &str) -> String {
    format!("{:x}", Sha256::digest(raw.as_bytes()))
}

pub fn read_optional(path: &Path) -> Result<String, ConfigError> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(source) => Err(ConfigError::Read {
            path: path.into(),
            source,
        }),
    }
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ConfigError> {
    let io_error = |source| ConfigError::Read {
        path: path.into(),
        source,
    };
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent).map_err(io_error)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(io_error)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temporary
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(io_error)?;
    }
    #[cfg(windows)]
    protect_windows_file(temporary.path()).map_err(io_error)?;
    temporary.write_all(bytes).map_err(io_error)?;
    temporary.as_file().sync_all().map_err(io_error)?;
    temporary
        .persist(path)
        .map_err(|error| io_error(error.error))?;
    Ok(())
}

#[cfg(windows)]
fn protect_windows_file(path: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::{
        ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
    };
    use windows_sys::Win32::Security::{
        DACL_SECURITY_INFORMATION, PROTECTED_DACL_SECURITY_INFORMATION, SetFileSecurityW,
    };
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    // Protected DACL: only the file owner and LocalSystem can access backups,
    // even when the destination directory grants access to other users.
    let sddl: Vec<u16> = "D:P(A;;FA;;;OW)(A;;FA;;;SY)"
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut descriptor = std::ptr::null_mut();
    // SAFETY: both strings are terminated; Windows allocates the descriptor
    // and it is freed exactly once after SetFileSecurityW has consumed it.
    unsafe {
        if ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            SDDL_REVISION_1,
            &mut descriptor,
            std::ptr::null_mut(),
        ) == 0
        {
            return Err(std::io::Error::last_os_error());
        }
        let applied = SetFileSecurityW(
            path.as_ptr(),
            DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION,
            descriptor,
        );
        let result = if applied == 0 {
            Err(std::io::Error::last_os_error())
        } else {
            Ok(())
        };
        LocalFree(descriptor);
        result
    }
}
