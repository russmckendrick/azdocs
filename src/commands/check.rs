use crate::auth::diagnostics::{self, ConnectionCheck};
use crate::config::Config;
use crate::labels::{Labels, fill};

pub async fn run(config: &Config, labels: &Labels) -> anyhow::Result<()> {
    let words = &labels.cli.check;
    let credentials = config.credentials()?;
    println!(
        "{}",
        fill(&words.config_ok, &[("tenant", &credentials.tenant_id)])
    );
    let provider = crate::auth::ClientCredentialsProvider::new(super::http_client(), credentials);
    let result = diagnostics::inspect(
        super::http_client(),
        &provider,
        &config.collect.subscriptions,
    )
    .await?;
    println!("{}", words.token_ok);
    println!(
        "{}",
        fill(
            &words.visible_subscriptions,
            &[("count", &result.subscriptions.len())]
        )
    );
    for subscription in &result.subscriptions {
        println!("  {}  {}", subscription.id, subscription.name);
    }
    print_permissions(&result, labels);
    Ok(())
}

pub fn print_permissions(check: &ConnectionCheck, labels: &Labels) {
    let words = &labels.common.access;
    if let Some(verdict) = words.verdicts.get(check.verdict.as_str()) {
        eprintln!("{verdict}");
    }
    eprintln!("{}", words.detail);
    for grant in &check.grants {
        if grant.verdict != diagnostics::PermissionVerdict::ReadOnly {
            eprintln!(
                "  {}  {}  {}",
                grant.scope,
                grant.role,
                grant.actions.join(", ")
            );
        }
    }
    for issue in &check.issues {
        if let Some(reason) = words.reasons.get(issue.kind.as_str()) {
            eprintln!("  {}  {reason}", issue.scope);
        }
    }
}
