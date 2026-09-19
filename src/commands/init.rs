use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, bail};
use dialoguer::{Input, Password};

use crate::config::secrets::SecretStore;
use crate::config::{ENV_CLIENT_ID, ENV_CLIENT_SECRET, ENV_TENANT_ID, default_config_path};
use crate::labels::{Labels, fill};

const SP_GUIDANCE: &str = "\
To create a read-only service principal for azdocs:

  az ad sp create-for-rbac --name azdocs-reader --role Reader \\
      --scopes /subscriptions/<subscription-id>

Use the returned tenant/appId/password values here. Grant the Reader role on
every subscription (or a management group) you want azdocs to see.";

/// Where the client secret will live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecretChoice {
    /// The file names an environment variable; nothing is stored.
    Env(String),
    /// Stored in the OS credential store under the tenant reference.
    Native(String),
}

/// Everything `init` needs, however it was gathered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitInputs {
    pub tenant_id: String,
    pub client_id: String,
    pub secret: SecretChoice,
}

#[derive(Debug, Clone, Default)]
pub struct InitOptions {
    pub force: bool,
    pub non_interactive: bool,
    /// Reference the secret via this variable instead of storing it.
    pub secret_env: Option<String>,
}

pub fn run(
    config_path: Option<&Path>,
    options: &InitOptions,
    labels: &Labels,
) -> anyhow::Result<()> {
    run_with_store(
        config_path,
        options,
        labels,
        &crate::config::secrets::NativeSecretStore,
    )
}

pub fn run_with_store<S: SecretStore>(
    config_path: Option<&Path>,
    options: &InitOptions,
    labels: &Labels,
    secrets: &S,
) -> anyhow::Result<()> {
    let words = &labels.cli.init;
    let default_path = default_config_path();
    let path = config_path.unwrap_or(&default_path);
    if path.exists() && !options.force {
        bail!(
            "{} already exists; pass --force to overwrite",
            path.display()
        );
    }
    let inputs = if options.non_interactive {
        inputs_from_env(options.secret_env.clone())?
    } else {
        inputs_interactive(options.secret_env.clone(), labels)?
    };
    write_config(path, &inputs, labels, secrets)?;
    println!("{}", fill(&words.wrote, &[("path", &path.display())]));
    println!("{}", words.next_step);
    Ok(())
}

/// Headless: identity from the environment, secret always by reference
/// (`--secret-env`, else the default variable). A headless host has no
/// credential store to unlock, so nothing is ever written to one here.
pub fn inputs_from_env(secret_env: Option<String>) -> anyhow::Result<InitInputs> {
    let read = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
    let (Some(tenant_id), Some(client_id)) = (read(ENV_TENANT_ID), read(ENV_CLIENT_ID)) else {
        bail!(
            "--non-interactive requires {ENV_TENANT_ID} and {ENV_CLIENT_ID} to be set \
             (the secret is referenced through --secret-env or {ENV_CLIENT_SECRET})"
        );
    };
    Ok(InitInputs {
        tenant_id,
        client_id,
        secret: SecretChoice::Env(secret_env.unwrap_or_else(|| ENV_CLIENT_SECRET.into())),
    })
}

fn inputs_interactive(secret_env: Option<String>, labels: &Labels) -> anyhow::Result<InitInputs> {
    let words = &labels.cli.init;
    println!("{SP_GUIDANCE}\n");
    let tenant_id: String = Input::new()
        .with_prompt(words.tenant_prompt.as_str())
        .interact_text()
        .context("reading tenant id")?;
    let client_id: String = Input::new()
        .with_prompt(words.client_prompt.as_str())
        .interact_text()
        .context("reading client id")?;
    let secret = match secret_env {
        Some(variable) => SecretChoice::Env(variable),
        None => {
            let typed = Password::new()
                .with_prompt(words.secret_prompt.as_str())
                .allow_empty_password(true)
                .interact()
                .context("reading client secret")?;
            if typed.is_empty() {
                SecretChoice::Env(ENV_CLIENT_SECRET.into())
            } else {
                SecretChoice::Native(typed)
            }
        }
    };
    Ok(InitInputs {
        tenant_id: tenant_id.trim().to_owned(),
        client_id: client_id.trim().to_owned(),
        secret,
    })
}

/// The settings document for one tenant profile plus the secret to store,
/// if any. Pure, so the shape can be tested without a terminal or keyring.
pub fn build_settings(
    inputs: &InitInputs,
    default_name: &str,
) -> (crate::config::SettingsValues, BTreeMap<String, String>) {
    let reference = "default";
    let mut values = crate::config::SettingsValues {
        default_tenant: Some(reference.into()),
        ..Default::default()
    };
    let mut secrets = BTreeMap::new();
    let secret_env = match &inputs.secret {
        SecretChoice::Env(variable) => Some(variable.clone()),
        SecretChoice::Native(secret) => {
            secrets.insert(reference.to_owned(), secret.clone());
            None
        }
    };
    values.tenants.insert(
        reference.into(),
        crate::config::TenantProfile {
            name: default_name.to_owned(),
            tenant_id: inputs.tenant_id.clone(),
            client_id: inputs.client_id.clone(),
            secret_env,
            ..crate::config::TenantProfile::default()
        },
    );
    (values, secrets)
}

fn write_config<S: SecretStore>(
    path: &Path,
    inputs: &InitInputs,
    labels: &Labels,
    secrets: &S,
) -> anyhow::Result<()> {
    let (values, new_secrets) = build_settings(inputs, &labels.cli.init.default_name);
    let document = crate::config::ConfigDocument::parse("", Some(path.into()))?;
    let revision =
        crate::config::document::revision(&crate::config::document::read_optional(path)?);
    document.save(path, &revision, values, &new_secrets, true, secrets)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const TENANT: &str = "11111111-1111-4111-8111-111111111111";
    const CLIENT: &str = "22222222-2222-4222-8222-222222222222";

    #[test]
    fn unit_build_settings_writes_secret_env_when_referenced() {
        let inputs = InitInputs {
            tenant_id: TENANT.into(),
            client_id: CLIENT.into(),
            secret: SecretChoice::Env("ACME_SECRET".into()),
        };

        let (values, secrets) = build_settings(&inputs, "Default tenant");

        let profile = &values.tenants["default"];
        assert_eq!(profile.secret_env.as_deref(), Some("ACME_SECRET"));
        assert!(profile.secret_ref.is_none());
        assert!(secrets.is_empty(), "nothing goes to the credential store");
        assert!(values.validate().is_ok());
    }

    #[test]
    fn unit_build_settings_sends_typed_secret_to_store_not_file() {
        let inputs = InitInputs {
            tenant_id: TENANT.into(),
            client_id: CLIENT.into(),
            secret: SecretChoice::Native("hunter2".into()),
        };

        let (values, secrets) = build_settings(&inputs, "Default tenant");

        assert!(values.tenants["default"].secret_env.is_none());
        assert_eq!(secrets.get("default").map(String::as_str), Some("hunter2"));
    }
}
