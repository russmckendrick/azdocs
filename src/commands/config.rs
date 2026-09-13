use crate::cli::ConfigCommand;
use crate::config::secrets::NativeSecretStore;
use crate::config::{ConfigDocument, default_config_path};
use crate::labels::Labels;
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

pub fn run(
    command: &ConfigCommand,
    path: Option<&Path>,
    tenant: Option<&str>,
    labels: &Labels,
) -> anyhow::Result<()> {
    let document = ConfigDocument::load(path)?;
    match command {
        ConfigCommand::Show => show_document(document, tenant)?,
        ConfigCommand::Validate => {
            document.values.validate()?;
            let resolved = document.resolve(tenant)?;
            crate::report::branding::BrandingContext::resolve(
                &resolved.branding,
                document.source.as_deref().and_then(Path::parent),
            )?;
            println!(
                "{}",
                labels.cli.check.config_ok.replace(
                    "{tenant}",
                    resolved.auth.tenant_id.as_deref().unwrap_or_default()
                )
            );
        }
        ConfigCommand::Migrate { reference, name } => {
            let (values, secrets) = document.migration(reference, name)?;
            let destination = document.source.clone().unwrap_or_else(default_config_path);
            document.save(
                &destination,
                &document.revision,
                values,
                &secrets,
                true,
                &NativeSecretStore,
            )?;
            println!(
                "{}",
                crate::labels::fill(&labels.cli.init.wrote, &[("path", &destination.display())])
            );
        }
        ConfigCommand::SetSecret { stdin } => {
            let reference = document.values.selected_reference(tenant)?.ok_or_else(|| {
                anyhow::anyhow!("migrate the legacy config before setting a tenant secret")
            })?;
            let secret = if *stdin {
                let mut text = String::new();
                std::io::stdin().read_to_string(&mut text)?;
                text.trim_end_matches(['\r', '\n']).to_owned()
            } else {
                dialoguer::Password::new()
                    .with_prompt(&labels.cli.init.secret_prompt)
                    .interact()?
            };
            let destination = document.source.clone().unwrap_or_else(default_config_path);
            document.save(
                &destination,
                &document.revision,
                document.values.clone(),
                &BTreeMap::from([(reference, secret)]),
                false,
                &NativeSecretStore,
            )?;
            println!(
                "{}",
                crate::labels::fill(&labels.cli.init.wrote, &[("path", &destination.display())])
            );
        }
    }
    Ok(())
}

pub fn show(path: Option<&Path>, tenant: Option<&str>) -> anyhow::Result<()> {
    show_document(ConfigDocument::load(path)?, tenant)
}

fn show_document(document: ConfigDocument, tenant: Option<&str>) -> anyhow::Result<()> {
    // Runtime environment values are not part of the editable document.
    // The envelope keeps configured and resolved values distinct.
    let resolved = if tenant.is_some() || document.values.selected_reference(None).is_ok() {
        let mut config = document.resolve(tenant)?;
        config.auth.client_secret = None;
        Some(config)
    } else {
        None
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "configured": document.values,
            "resolved": resolved,
            "path": document.source,
        }))?
    );
    Ok(())
}
