//! Desktop settings commands expose only secret-free configuration and diagnostics.
use crate::{AppState, error::AppError, labels::AppLabels};
use azdocs::auth::diagnostics::ConnectionCheck;
use azdocs::config::secrets::{NativeSecretStore, SecretStore};
use azdocs::config::{ConfigDocument, SettingsValues};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tauri::State;
use ts_rs::TS;

#[derive(Clone)]
pub(crate) struct SavedCheck {
    pub revision: String,
    pub check: ConnectionCheck,
}

#[derive(Clone)]
pub(crate) struct Session {
    pub database_path: PathBuf,
    pub config_path: Option<PathBuf>,
    pub tenant_id: Option<String>,
    pub labels: Arc<AppLabels>,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Preferences {
    config_path: Option<PathBuf>,
    tenant_id: Option<String>,
}
fn preferences_path() -> PathBuf {
    azdocs::config::default_config_path().with_file_name("desktop-preferences.json")
}

impl Session {
    pub fn initial(database_path: PathBuf, labels: Arc<AppLabels>) -> Self {
        let preferences: Preferences = std::fs::read(preferences_path())
            .ok()
            .and_then(|s| serde_json::from_slice(&s).ok())
            .unwrap_or_default();
        let mut session = Self {
            database_path,
            labels,
            config_path: preferences.config_path,
            tenant_id: preferences.tenant_id,
        };
        session.refresh();
        session
    }
    pub fn document(&self) -> Result<ConfigDocument, AppError> {
        ConfigDocument::load(self.config_path.as_deref()).map_err(config_error)
    }
    pub fn refresh(&mut self) {
        if let Ok(document) = self.document() {
            self.config_path = document.source.clone();
            let mut path = document.values.storage.db_path.clone();
            if path.is_relative()
                && let Some(parent) = document.source.as_deref().and_then(Path::parent)
            {
                path = parent.join(path);
            }
            self.database_path = path;
            let config = document
                .resolve(self.tenant_id.as_deref())
                .or_else(|_| document.resolve(None))
                .ok();
            if let Some(config) = config {
                self.tenant_id = config.auth.tenant_id.clone();
                self.labels = AppLabels::load(Some(&config));
            } else {
                self.labels = AppLabels::load(Some(&document.values.resolved_defaults()));
            }
        }
        if self.tenant_id.is_none()
            && let Ok(store) = azdocs::store::Store::open(&self.database_path)
            && let Ok(ids) = store.tenant_ids()
            && ids.len() == 1
        {
            self.tenant_id = ids.first().cloned();
        }
    }
    fn persist(&self) -> Result<(), AppError> {
        let bytes = serde_json::to_vec(&Preferences {
            config_path: self.config_path.clone(),
            tenant_id: self.tenant_id.clone(),
        })
        .map_err(|e| AppError::State(e.to_string()))?;
        azdocs::config::document::atomic_write(&preferences_path(), &bytes).map_err(config_error)
    }
}

pub(crate) fn session(state: &AppState) -> Result<Session, AppError> {
    state
        .session
        .read()
        .map(|s| s.clone())
        .map_err(|e| AppError::State(e.to_string()))
}
pub(crate) fn install(state: &AppState, next: Session) -> Result<(), AppError> {
    next.persist()?;
    *state
        .session
        .write()
        .map_err(|e| AppError::State(e.to_string()))? = next;
    state
        .pending_secrets
        .lock()
        .map_err(|e| AppError::State(e.to_string()))?
        .clear();
    state
        .checks
        .write()
        .map_err(|e| AppError::State(e.to_string()))?
        .clear();
    Ok(())
}
pub(crate) fn config_error(error: azdocs::error::ConfigError) -> AppError {
    AppError::Config(error.to_string())
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TenantSummary {
    pub reference: String,
    pub name: String,
    pub tenant_id: String,
    pub configured: bool,
}

#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields=nullable)]
pub struct SettingsDocumentDto {
    pub path: String,
    pub revision: String,
    pub values: Option<SettingsValues>,
    pub legacy: bool,
    pub legacy_tenant_id: Option<String>,
    pub legacy_client_id: Option<String>,
    pub error: Option<String>,
    pub check: Option<ConnectionCheck>,
}

#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsSaveRequest {
    pub revision: String,
    pub values: SettingsValues,
    pub new_secrets: BTreeMap<String, String>,
    pub secret_tokens: BTreeMap<String, String>,
}

#[derive(Deserialize, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SettingsTestRequest {
    pub reference: String,
    pub values: SettingsValues,
    pub new_secret: Option<String>,
    pub secret_token: Option<String>,
}

#[tauri::command]
pub fn settings_load(state: State<'_, AppState>) -> Result<SettingsDocumentDto, AppError> {
    let session = session(&state)?;
    let cached = session
        .tenant_id
        .as_ref()
        .and_then(|id| state.checks.read().ok()?.get(id).cloned());
    Ok(load_settings(&session, cached))
}

fn load_settings(session: &Session, cached: Option<SavedCheck>) -> SettingsDocumentDto {
    let path = session
        .config_path
        .clone()
        .unwrap_or_else(azdocs::config::default_config_path);
    match session.document() {
        Ok(document) => {
            let check = cached
                .filter(|c| c.revision == document.revision)
                .map(|c| c.check);
            SettingsDocumentDto {
                path: document
                    .source
                    .as_ref()
                    .unwrap_or(&path)
                    .display()
                    .to_string(),
                revision: document.revision.clone(),
                values: Some(document.values.clone()),
                legacy: document.has_legacy(),
                legacy_tenant_id: document.legacy_auth.tenant_id.clone(),
                legacy_client_id: document.legacy_auth.client_id.clone(),
                error: None,
                check,
            }
        }
        Err(error) => SettingsDocumentDto {
            path: path.display().to_string(),
            revision: String::new(),
            values: None,
            legacy: false,
            legacy_tenant_id: None,
            legacy_client_id: None,
            error: Some(error.to_string()),
            check: None,
        },
    }
}

#[tauri::command]
pub fn settings_load_config(
    path: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::dto::AppBootstrap, AppError> {
    let _lease = state.captures.begin()?;
    let mut next = session(&state)?;
    if let Some(path) = path {
        next.config_path = Some(PathBuf::from(path));
        next.tenant_id = None;
    }
    next.refresh();
    install(&state, next)?;
    crate::commands::bootstrap_for(&session(&state)?)
}

#[tauri::command]
pub fn select_tenant(
    tenant_id: String,
    state: State<'_, AppState>,
) -> Result<crate::dto::AppBootstrap, AppError> {
    let _lease = state.captures.begin()?;
    let mut next = session(&state)?;
    let document = next.document().ok();
    let stored = azdocs::store::Store::open(&next.database_path)?.tenant_ids()?;
    let configured = document.as_ref().is_some_and(|d| {
        d.values
            .tenants
            .values()
            .any(|p| p.tenant_id.eq_ignore_ascii_case(&tenant_id))
    }) || document
        .as_ref()
        .and_then(|d| d.resolve(None).ok())
        .and_then(|c| c.auth.tenant_id)
        .is_some_and(|id| id.eq_ignore_ascii_case(&tenant_id));
    if !configured && !stored.iter().any(|id| id.eq_ignore_ascii_case(&tenant_id)) {
        return Err(AppError::Config("unknown tenant".into()));
    }
    next.tenant_id = Some(tenant_id.to_ascii_lowercase());
    next.labels = document
        .as_ref()
        .and_then(|d| d.for_snapshot(&tenant_id).ok())
        .map(|c| AppLabels::load(Some(&c)))
        .unwrap_or_else(|| AppLabels::load(None));
    install(&state, next)?;
    crate::commands::bootstrap_for(&session(&state)?)
}

fn validate_assets(values: &SettingsValues, source: Option<&Path>) -> Result<(), AppError> {
    values.validate().map_err(config_error)?;
    for config in std::iter::once(Ok(values.resolved_defaults()))
        .chain(values.tenants.keys().map(|r| values.resolved(Some(r))))
    {
        let config = config.map_err(config_error)?;
        azdocs::report::branding::BrandingContext::resolve(
            &config.branding,
            source.and_then(Path::parent),
        )
        .map_err(config_error)?;
    }
    Ok(())
}

fn save_document(
    session: &Session,
    request: SettingsSaveRequest,
    secrets: &impl SecretStore,
) -> Result<PathBuf, AppError> {
    let document = session.document()?;
    let destination = document
        .source
        .clone()
        .unwrap_or_else(azdocs::config::default_config_path);
    validate_assets(&request.values, Some(&destination))?;
    let mut database_path = request.values.storage.db_path.clone();
    if database_path.is_relative()
        && let Some(parent) = destination.parent()
    {
        database_path = parent.join(database_path);
    }
    // Reject unusable databases before any secret or configuration is replaced.
    azdocs::store::Store::open(&database_path)?;
    document
        .save(
            &destination,
            &request.revision,
            request.values,
            &request.new_secrets,
            false,
            secrets,
        )
        .map_err(config_error)?;
    Ok(destination)
}

#[tauri::command]
pub async fn settings_save(
    mut request: SettingsSaveRequest,
    state: State<'_, AppState>,
) -> Result<crate::dto::AppBootstrap, AppError> {
    let _lease = state.captures.begin()?;
    let mut next = session(&state)?;
    let current = next.clone();
    for (reference, token) in &request.secret_tokens {
        if !request.new_secrets.contains_key(reference) {
            let profile = request
                .values
                .tenants
                .get(reference)
                .ok_or_else(|| AppError::Config("unknown tenant for pending secret".into()))?;
            request
                .new_secrets
                .insert(reference.clone(), pending_secret(&state, token, profile)?);
        }
    }
    let saved = tauri::async_runtime::spawn_blocking(move || {
        save_document(&current, request, &NativeSecretStore)
    })
    .await
    .map_err(|e| AppError::State(e.to_string()))??;
    next.config_path = Some(saved);
    next.refresh();
    install(&state, next)?;
    crate::commands::bootstrap_for(&session(&state)?)
}

#[tauri::command]
pub async fn settings_migrate(
    reference: String,
    name: String,
    state: State<'_, AppState>,
) -> Result<crate::dto::AppBootstrap, AppError> {
    let _lease = state.captures.begin()?;
    let mut next = session(&state)?;
    let current = next.clone();
    let destination = tauri::async_runtime::spawn_blocking(move || {
        let document = current.document()?;
        let (values, secrets) = document
            .migration(&reference, &name)
            .map_err(config_error)?;
        let destination = document
            .source
            .clone()
            .unwrap_or_else(azdocs::config::default_config_path);
        document
            .save(
                &destination,
                &document.revision,
                values,
                &secrets,
                true,
                &NativeSecretStore,
            )
            .map_err(config_error)?;
        Ok::<_, AppError>(destination)
    })
    .await
    .map_err(|e| AppError::State(e.to_string()))??;
    next.config_path = Some(destination);
    next.refresh();
    install(&state, next)?;
    crate::commands::bootstrap_for(&session(&state)?)
}

pub(crate) struct PendingSecret {
    secret: String,
    tenant_id: String,
    client_id: String,
    created: std::time::Instant,
}
fn pending_secret(
    state: &AppState,
    token: &str,
    profile: &azdocs::config::TenantProfile,
) -> Result<String, AppError> {
    let mut pending = state
        .pending_secrets
        .lock()
        .map_err(|e| AppError::State(e.to_string()))?;
    pending.retain(|_, p| p.created.elapsed() < std::time::Duration::from_secs(900));
    let secret = pending
        .get(token)
        .filter(|p| {
            p.tenant_id == profile.tenant_id
                && p.client_id == profile.client_id
                && profile.secret_env.is_none()
        })
        .ok_or_else(|| {
            AppError::Config(
                "the pending secret expired or its identity changed; enter it again".into(),
            )
        })?;
    Ok(secret.secret.clone())
}
#[derive(Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(optional_fields=nullable)]
pub struct SettingsTestResult {
    pub check: ConnectionCheck,
    pub secret_token: Option<String>,
}

#[tauri::command]
pub fn settings_discard_secrets(
    tokens: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let mut pending = state
        .pending_secrets
        .lock()
        .map_err(|e| AppError::State(e.to_string()))?;
    for token in tokens {
        pending.remove(&token);
    }
    Ok(())
}

#[tauri::command]
pub async fn settings_test(
    mut request: SettingsTestRequest,
    state: State<'_, AppState>,
) -> Result<SettingsTestResult, AppError> {
    let _lease = state.captures.begin()?;
    request.values.validate().map_err(config_error)?;
    let profile = request
        .values
        .tenants
        .get(&request.reference)
        .cloned()
        .ok_or_else(|| AppError::Config("unknown tenant".into()))?;
    if request.new_secret.as_ref().is_none_or(|s| s.is_empty())
        && let Some(token) = &request.secret_token
    {
        request.new_secret = Some(pending_secret(&state, token, &profile)?);
    }
    let new_secret = request.new_secret.clone().filter(|s| !s.is_empty());
    let checked = tauri::async_runtime::spawn_blocking(move || {
        let mut config = request
            .values
            .resolved(Some(&request.reference))
            .map_err(config_error)?;
        if let Some(secret) = request.new_secret.filter(|s| !s.is_empty()) {
            config.auth.client_secret = Some(secret);
            config.auth.secret_ref = None;
            config.auth.secret_env = None;
        }
        let provider = azdocs::commands::token_provider(&config)
            .map_err(|e| AppError::Config(e.to_string()))?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| AppError::State(e.to_string()))?;
        runtime
            .block_on(azdocs::auth::diagnostics::inspect(
                azdocs::commands::http_client(),
                &provider,
                &config.collect.subscriptions,
            ))
            .map_err(|e| AppError::Collection(e.to_string()))
    })
    .await
    .map_err(|e| AppError::State(e.to_string()))??;
    let secret_token = if let Some(secret) = new_secret {
        let mut pending = state
            .pending_secrets
            .lock()
            .map_err(|e| AppError::State(e.to_string()))?;
        pending.retain(|_, p| p.created.elapsed() < std::time::Duration::from_secs(900));
        let token = uuid::Uuid::new_v4().to_string();
        pending.insert(
            token.clone(),
            PendingSecret {
                secret,
                tenant_id: profile.tenant_id,
                client_id: profile.client_id,
                created: std::time::Instant::now(),
            },
        );
        Some(token)
    } else {
        None
    };
    Ok(SettingsTestResult {
        check: checked,
        secret_token,
    })
}

#[tauri::command]
pub async fn settings_export(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<bool, AppError> {
    use tauri_plugin_dialog::DialogExt;
    let session = session(&state)?;
    let cached = session
        .tenant_id
        .as_ref()
        .and_then(|id| state.checks.read().ok()?.get(id).cloned());
    let bytes = diagnostic_bytes(&session, cached)?;
    let (tx, rx) = tokio::sync::oneshot::channel();
    let title = session
        .labels
        .desktop
        .settings
        .editor
        .get("export")
        .cloned()
        .unwrap_or_default();
    app.dialog()
        .file()
        .set_title(title)
        .set_file_name("azdocs-settings-redacted.json")
        .save_file(move |path| {
            let _ = tx.send(path);
        });
    let Some(path) = rx.await.map_err(|e| AppError::State(e.to_string()))? else {
        return Ok(false);
    };
    let path = path
        .into_path()
        .map_err(|e| AppError::State(e.to_string()))?;
    azdocs::config::document::atomic_write(&path, &bytes).map_err(config_error)?;
    Ok(true)
}

fn diagnostic_bytes(session: &Session, cached: Option<SavedCheck>) -> Result<Vec<u8>, AppError> {
    let settings = load_settings(session, cached);
    serde_json::to_vec_pretty(&serde_json::json!({
        "databasePath": session.database_path,
        "activeTenantId": session.tenant_id,
        "settings": settings,
    }))
    .map_err(|e| AppError::Config(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use azdocs::auth::diagnostics::PermissionVerdict;
    use azdocs::error::ConfigError;
    use azdocs::store::Store;

    const TENANT: &str = "11111111-1111-1111-1111-111111111111";
    const CLIENT: &str = "22222222-2222-2222-2222-222222222222";
    const SECRET: &str = "test-only-secret-that-must-not-reach-the-webview";

    fn fixture(raw: &str) -> (tempfile::TempDir, Session) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("settings.toml");
        std::fs::write(&path, raw).unwrap();
        let session = Session {
            database_path: dir.path().join("history.db"),
            config_path: Some(path),
            tenant_id: Some(TENANT.into()),
            labels: Arc::new(AppLabels::builtin()),
        };
        (dir, session)
    }

    fn named_config() -> String {
        format!(
            "schema_version = 2\ndefault_tenant = 'acme'\n[tenants.acme]\nname = 'Acme'\ntenant_id = '{TENANT}'\nclient_id = '{CLIENT}'\nsecret_ref = '33333333-3333-3333-3333-333333333333'\n[storage]\ndb_path = 'history.db'\n"
        )
    }

    struct NoSecretAccess;
    impl SecretStore for NoSecretAccess {
        fn get(&self, _: &str) -> Result<String, ConfigError> {
            panic!("offline settings must not read credentials")
        }
        fn set(&self, _: &str, _: &str) -> Result<(), ConfigError> {
            panic!("this save must not write credentials")
        }
        fn delete(&self, _: &str) -> Result<(), ConfigError> {
            panic!("this save must not delete credentials")
        }
    }

    #[test]
    fn unit_redacts_legacy_secrets_from_every_desktop_response() {
        let (_dir, session) = fixture(&format!(
            "[auth]\ntenant_id = '{TENANT}'\nclient_id = '{CLIENT}'\nclient_secret = '{SECRET}'\n"
        ));
        let settings = load_settings(&session, None);
        assert!(settings.legacy);
        assert!(settings.values.is_some());
        assert_eq!(settings.legacy_client_id.as_deref(), Some(CLIENT));
        let bootstrap = crate::commands::bootstrap_for(&session).unwrap();
        for output in [
            serde_json::to_string(&settings).unwrap(),
            serde_json::to_string(&bootstrap).unwrap(),
            String::from_utf8(diagnostic_bytes(&session, None).unwrap()).unwrap(),
        ] {
            assert!(!output.contains(SECRET));
            assert!(!output.contains("client_secret"));
        }
    }

    #[test]
    fn unit_keeps_invalid_and_missing_configuration_errors_secret_free() {
        let (_dir, session) = fixture(&format!("[auth]\nclient_secret = \"{SECRET}\n"));
        for remove_file in [false, true] {
            if remove_file {
                std::fs::remove_file(session.config_path.as_ref().unwrap()).unwrap();
            }
            let settings = load_settings(&session, None);
            assert!(settings.values.is_none());
            assert!(settings.error.is_some());
            let bootstrap = crate::commands::bootstrap_for(&session).unwrap();
            assert!(bootstrap.config_error.is_some());
            assert!(!serde_json::to_string(&bootstrap).unwrap().contains(SECRET));
            let diagnostic = String::from_utf8(diagnostic_bytes(&session, None).unwrap()).unwrap();
            assert!(!diagnostic.contains(SECRET));
            assert!(!diagnostic.contains("client_secret"));
        }
    }

    #[test]
    fn unit_invalidates_saved_diagnostics_after_external_config_changes() {
        let (_dir, session) = fixture(&named_config());
        let cached = SavedCheck {
            revision: session.document().unwrap().revision,
            check: ConnectionCheck {
                checked_at: "2026-09-13T12:00:00Z".into(),
                subscriptions: Vec::new(),
                inaccessible_subscriptions: Vec::new(),
                verdict: PermissionVerdict::UnableToVerify,
                grants: Vec::new(),
                issues: Vec::new(),
            },
        };
        assert!(
            load_settings(&session, Some(cached.clone()))
                .check
                .is_some()
        );
        std::fs::write(
            session.config_path.as_ref().unwrap(),
            named_config().replace("name = 'Acme'", "name = 'Renamed'"),
        )
        .unwrap();
        assert!(load_settings(&session, Some(cached)).check.is_none());
    }

    #[test]
    fn unit_rejects_unusable_database_before_replacing_config_or_secrets() {
        let (dir, session) = fixture(&named_config());
        let blocked = dir.path().join("not-a-database");
        std::fs::write(&blocked, "not SQLite").unwrap();
        let document = session.document().unwrap();
        let mut values = document.values;
        values.storage.db_path = blocked;
        let request = SettingsSaveRequest {
            revision: document.revision,
            values,
            new_secrets: BTreeMap::from([("acme".into(), SECRET.into())]),
            secret_tokens: BTreeMap::new(),
        };
        assert!(save_document(&session, request, &NoSecretAccess).is_err());
        assert_eq!(
            std::fs::read_to_string(session.config_path.unwrap()).unwrap(),
            named_config()
        );
    }

    #[test]
    fn unit_opens_relative_database_destination_without_moving_history() {
        let (dir, session) = fixture(&named_config());
        let old = Store::open(&session.database_path).unwrap();
        let snapshot = old.create_snapshot(TENANT, None).unwrap();
        let document = session.document().unwrap();
        let mut values = document.values;
        values.storage.db_path = "another/estate.db".into();
        save_document(
            &session,
            SettingsSaveRequest {
                revision: document.revision,
                values,
                new_secrets: BTreeMap::new(),
                secret_tokens: BTreeMap::new(),
            },
            &NoSecretAccess,
        )
        .unwrap();
        let new = Store::open(&dir.path().join("another/estate.db")).unwrap();
        assert!(new.list_snapshots().unwrap().is_empty());
        assert_eq!(old.get_snapshot(&snapshot.id).unwrap().id, snapshot.id);
        assert_eq!(
            session.document().unwrap().values.storage.db_path,
            PathBuf::from("another/estate.db")
        );
    }
}
