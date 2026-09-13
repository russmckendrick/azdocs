use azdocs::config::secrets::SecretStore;
use azdocs::config::{ConfigDocument, SettingsValues, TenantProfile};
use azdocs::error::ConfigError;
use std::cell::RefCell;
use std::collections::BTreeMap;

const TENANT: &str = "11111111-1111-4111-8111-111111111111";
const CLIENT: &str = "22222222-2222-4222-8222-222222222222";
#[derive(Default)]
struct MemorySecrets {
    values: RefCell<BTreeMap<String, String>>,
    fail: bool,
    fail_read: bool,
}
impl SecretStore for MemorySecrets {
    fn get(&self, key: &str) -> Result<String, ConfigError> {
        if self.fail_read {
            return Err(ConfigError::Secret("locked".into()));
        }
        self.values
            .borrow()
            .get(key)
            .cloned()
            .ok_or_else(|| ConfigError::Secret("missing".into()))
    }
    fn set(&self, key: &str, value: &str) -> Result<(), ConfigError> {
        if self.fail {
            return Err(ConfigError::Secret("locked".into()));
        }
        self.values.borrow_mut().insert(key.into(), value.into());
        Ok(())
    }
    fn delete(&self, key: &str) -> Result<(), ConfigError> {
        self.values.borrow_mut().remove(key);
        Ok(())
    }
}
fn values() -> SettingsValues {
    let mut values = SettingsValues {
        default_tenant: Some("acme".into()),
        ..Default::default()
    };
    values.tenants.insert(
        "acme".into(),
        TenantProfile {
            name: "Acme".into(),
            tenant_id: TENANT.into(),
            client_id: CLIENT.into(),
            ..TenantProfile::default()
        },
    );
    values
}

#[test]
fn unit_resolves_empty_overrides_without_merging_lists() {
    let mut values = values();
    values.audit.required_tags = vec!["owner".into()];
    values.collect.subscriptions = vec![TENANT.into()];
    values.tenants.get_mut("acme").unwrap().audit.required_tags = Some(vec![]);
    values
        .tenants
        .get_mut("acme")
        .unwrap()
        .collect
        .subscriptions = Some(vec![]);
    let config = values.resolved(None).unwrap();
    assert!(config.audit.required_tags.is_empty());
    assert!(config.collect.subscriptions.is_empty());
    assert_eq!(config.collect.concurrency, 4);
}
#[test]
fn unit_requires_selection_for_multiple_profiles_without_default() {
    let mut values = values();
    let mut second = values.tenants["acme"].clone();
    second.tenant_id = CLIENT.into();
    values.tenants.insert("second".into(), second);
    values.default_tenant = None;
    assert!(values.resolved(None).is_err());
    assert!(values.resolved(Some("second")).is_ok());
}
#[test]
fn unit_preserves_comments_and_detects_external_file_changes() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    std::fs::write(
        &path,
        "# Keep this description\n[collect]\nconcurrency = 4 # Keep this explanation\n",
    )
    .unwrap();
    let document = ConfigDocument::load(Some(&path)).unwrap();
    let mut values = values();
    values.collect.concurrency = 6;
    let saved = document
        .save(
            &path,
            &document.revision,
            values.clone(),
            &BTreeMap::new(),
            false,
            &MemorySecrets::default(),
        )
        .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("# Keep this description"));
    assert!(text.contains("# Keep this explanation"));
    assert_eq!(saved.resolve(None).unwrap().collect.concurrency, 6);
    std::fs::write(&path, "# External edit\n").unwrap();
    assert!(matches!(
        saved.save(
            &path,
            &saved.revision,
            values,
            &BTreeMap::new(),
            false,
            &MemorySecrets::default()
        ),
        Err(ConfigError::Conflict)
    ));
    assert_eq!(std::fs::read_to_string(path).unwrap(), "# External edit\n");
}
#[test]
fn unit_migrates_only_file_secrets_and_keeps_a_protected_backup() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let raw = format!(
        "[auth]\ntenant_id='{TENANT}'\nclient_id='{CLIENT}'\nclient_secret='private-test-secret'\n"
    );
    std::fs::write(&path, &raw).unwrap();
    let document = ConfigDocument::load(Some(&path)).unwrap();
    let (values, secrets) = document.migration("acme", "Acme").unwrap();
    let memory = MemorySecrets::default();
    let saved = document
        .save(&path, &document.revision, values, &secrets, true, &memory)
        .unwrap();
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(!text.contains("private-test-secret"));
    assert!(saved.values.tenants["acme"].secret_ref.is_some());
    assert_eq!(memory.values.borrow().len(), 1);
    let backup = std::fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().contains("backup-"))
        .unwrap();
    assert_eq!(std::fs::read_to_string(&backup).unwrap(), raw);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(backup).unwrap().permissions().mode() & 0o777,
            0o600
        );
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
#[test]
fn unit_keeps_the_file_when_secret_storage_fails() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let raw = "# Original\n";
    std::fs::write(&path, raw).unwrap();
    let doc = ConfigDocument::load(Some(&path)).unwrap();
    for store in [
        MemorySecrets {
            fail: true,
            ..Default::default()
        },
        MemorySecrets {
            fail_read: true,
            ..Default::default()
        },
    ] {
        assert!(
            doc.save(
                &path,
                &doc.revision,
                values(),
                &BTreeMap::from([("acme".into(), "secret".into())]),
                false,
                &store
            )
            .is_err()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
        assert!(store.values.borrow().is_empty());
    }
}
#[test]
fn unit_redacts_parse_debug_and_auth_debug() {
    let secret = "never-show-this-secret";
    let raw = format!("[auth]\nclient_secret='{secret}'\nunknown_field='{secret}'");
    let error = ConfigDocument::parse(&raw, None).err().unwrap();
    assert!(!format!("{error:?} {error}").contains(secret));
    let auth = azdocs::config::AuthConfig {
        client_secret: Some(secret.into()),
        ..Default::default()
    };
    assert!(!format!("{auth:?}").contains(secret));
    let credentials = azdocs::config::Credentials {
        tenant_id: TENANT.into(),
        client_id: CLIENT.into(),
        client_secret: secret.into(),
    };
    assert!(!format!("{credentials:?}").contains(secret));
}
#[test]
fn unit_resolves_relative_paths_from_the_config_directory() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    let doc = ConfigDocument::parse(
        "[storage]\ndb_path='estate.db'\n[branding]\nlogo='logo.png'\nfont_dir='fonts'",
        Some(path),
    )
    .unwrap();
    let config = doc.resolve(None).unwrap();
    assert_eq!(config.storage.db_path, dir.path().join("estate.db"));
    assert_eq!(config.branding.logo, Some(dir.path().join("logo.png")));
    assert_eq!(config.branding.font_dir, Some(dir.path().join("fonts")));
}
#[test]
fn unit_keeps_branding_tenant_specific_without_fetching_credentials() {
    let mut values = values();
    values.branding.company = "Shared".into();
    values.tenants.get_mut("acme").unwrap().branding.company = Some("Acme".into());
    values.tenants.get_mut("acme").unwrap().secret_ref = Some(uuid::Uuid::new_v4().to_string());
    let document = ConfigDocument::parse(&toml::to_string(&values).unwrap(), None).unwrap();
    assert_eq!(
        document.for_snapshot(TENANT).unwrap().branding.company,
        "Acme"
    );
    assert_eq!(
        document.for_snapshot("unknown").unwrap().branding.company,
        "Shared"
    );
}
#[test]
fn unit_rejects_invalid_settings_and_unknown_fields() {
    for raw in [
        "schema_version=99",
        "[collect]\nconcurrency=0",
        "[branding]\nprimary_color='wrong'",
        "[tenants.acme]\nname='Acme'\ntenant_id='wrong'\nclient_id='wrong'",
        "[tenants.acme]\nunknown='typo'",
    ] {
        assert!(ConfigDocument::parse(raw, None).is_err(), "{raw}");
    }
}

#[test]
fn unit_named_profiles_ignore_global_identity_overrides_and_exports_redact_secrets() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    std::fs::write(&path, toml::to_string(&values()).unwrap()).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_azdocs"))
        .args(["--config", path.to_str().unwrap(), "config", "show"])
        .env("AZDOCS_TENANT_ID", CLIENT)
        .env("AZDOCS_CLIENT_ID", TENANT)
        .env("AZDOCS_CLIENT_SECRET", "never-export-environment-secret")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(!text.contains("never-export-environment-secret"));
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["resolved"]["auth"]["tenant_id"], TENANT);
    assert_eq!(json["resolved"]["auth"]["client_id"], CLIENT);
}
#[test]
fn unit_legacy_runtime_environment_overrides_do_not_change_the_editable_document() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    let raw = format!(
        "[auth]\ntenant_id='{TENANT}'\nclient_id='{CLIENT}'\nclient_secret='never-export-file-secret'\n"
    );
    std::fs::write(&path, &raw).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_azdocs"))
        .args(["--config", path.to_str().unwrap(), "config", "show"])
        .env("AZDOCS_TENANT_ID", CLIENT)
        .env("AZDOCS_CLIENT_SECRET", "never-export-environment-secret")
        .env_remove("AZDOCS_CLIENT_ID")
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(!text.contains("never-export"));
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["resolved"]["auth"]["tenant_id"], CLIENT);
    let document = ConfigDocument::load(Some(&path)).unwrap();
    let (migrated, _) = document.migration("acme", "Acme").unwrap();
    assert_eq!(migrated.tenants["acme"].tenant_id, TENANT);
    assert_eq!(std::fs::read_to_string(path).unwrap(), raw);
}
#[test]
fn unit_rolls_back_staged_secrets_when_the_file_changes_during_credential_storage() {
    struct ConflictingStore {
        inner: MemorySecrets,
        path: std::path::PathBuf,
    }
    impl SecretStore for ConflictingStore {
        fn get(&self, key: &str) -> Result<String, ConfigError> {
            self.inner.get(key)
        }
        fn delete(&self, key: &str) -> Result<(), ConfigError> {
            self.inner.delete(key)
        }
        fn set(&self, key: &str, secret: &str) -> Result<(), ConfigError> {
            self.inner.set(key, secret)?;
            std::fs::write(&self.path, "# external change\n").unwrap();
            Ok(())
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("config.toml");
    std::fs::write(&path, "# before\n").unwrap();
    let document = ConfigDocument::load(Some(&path)).unwrap();
    let store = ConflictingStore {
        inner: MemorySecrets::default(),
        path: path.clone(),
    };
    let result = document.save(
        &path,
        &document.revision,
        values(),
        &BTreeMap::from([("acme".into(), "secret".into())]),
        false,
        &store,
    );
    assert!(matches!(result, Err(ConfigError::Conflict)));
    assert!(store.inner.values.borrow().is_empty());
    assert_eq!(
        std::fs::read_to_string(path).unwrap(),
        "# external change\n"
    );
}
