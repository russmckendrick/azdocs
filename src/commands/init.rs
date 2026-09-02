use std::path::Path;

use anyhow::{Context, bail};
use dialoguer::{Input, Password};

use crate::config::{Config, ENV_CLIENT_ID, ENV_CLIENT_SECRET, ENV_TENANT_ID, default_config_path};
use crate::labels::{Labels, fill};

const SP_GUIDANCE: &str = "\
To create a read-only service principal for azdocs:

  az ad sp create-for-rbac --name azdocs-reader --role Reader \\
      --scopes /subscriptions/<subscription-id>

Use the returned tenant/appId/password values here. Grant the Reader role on
every subscription (or a management group) you want azdocs to see.";

pub fn run(
    config_path: Option<&Path>,
    force: bool,
    non_interactive: bool,
    labels: &Labels,
) -> anyhow::Result<()> {
    let words = &labels.cli.init;
    let default_path = default_config_path();
    let path = config_path.unwrap_or(&default_path);
    if path.exists() && !force {
        bail!(
            "{} already exists; pass --force to overwrite",
            path.display()
        );
    }

    let mut config = Config::default();
    if non_interactive {
        config.auth.tenant_id = std::env::var(ENV_TENANT_ID).ok().filter(|v| !v.is_empty());
        config.auth.client_id = std::env::var(ENV_CLIENT_ID).ok().filter(|v| !v.is_empty());
        config.auth.client_secret = std::env::var(ENV_CLIENT_SECRET)
            .ok()
            .filter(|v| !v.is_empty());
        if config.auth.tenant_id.is_none() || config.auth.client_id.is_none() {
            bail!(
                "--non-interactive requires {ENV_TENANT_ID} and {ENV_CLIENT_ID} to be set \
                 ({ENV_CLIENT_SECRET} may stay in the environment instead of the file)"
            );
        }
    } else {
        println!("{SP_GUIDANCE}\n");
        let tenant_id: String = Input::new()
            .with_prompt(words.tenant_prompt.as_str())
            .interact_text()
            .context("reading tenant id")?;
        let client_id: String = Input::new()
            .with_prompt(words.client_prompt.as_str())
            .interact_text()
            .context("reading client id")?;
        let client_secret = Password::new()
            .with_prompt(words.secret_prompt.as_str())
            .allow_empty_password(true)
            .interact()
            .context("reading client secret")?;
        config.auth.tenant_id = Some(tenant_id.trim().to_owned());
        config.auth.client_id = Some(client_id.trim().to_owned());
        config.auth.client_secret = Some(client_secret).filter(|s| !s.is_empty());
    }

    let rendered = toml::to_string_pretty(&config).context("serializing config")?;
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(path, rendered).with_context(|| format!("writing {}", path.display()))?;
    restrict_permissions(path)?;

    println!("{}", fill(&words.wrote, &[("path", &path.display())]));
    if config.auth.client_secret.is_some() {
        println!(
            "{}",
            fill(&words.plaintext_note, &[("env", &ENV_CLIENT_SECRET)])
        );
    }
    println!("{}", words.next_step);
    Ok(())
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("setting permissions on {}", path.display()))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}
