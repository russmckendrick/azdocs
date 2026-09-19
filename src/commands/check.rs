use anyhow::bail;

use crate::auth::diagnostics::{self, ConnectionCheck};
use crate::cli::OutputFormat;
use crate::config::Config;
use crate::labels::{Labels, fill};

pub async fn run(config: &Config, format: OutputFormat, labels: &Labels) -> anyhow::Result<()> {
    let words = &labels.cli.check;
    let tenant = config.credentials()?.tenant_id;
    if format == OutputFormat::Table {
        println!("{}", fill(&words.config_ok, &[("tenant", &tenant)]));
    }
    let provider = super::token_provider(config)?;
    let result = diagnostics::inspect(
        super::http_client(config),
        &provider,
        &config.collect.subscriptions,
        config.cloud,
    )
    .await?;
    match format {
        OutputFormat::Json => println!("{}", render_json(&tenant, &result)?),
        OutputFormat::Table => {
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
        }
    }
    enforce_blocking(&result)
}

/// A principal that can see no subscriptions, or that Azure will not
/// describe, has not passed a connectivity check whatever the token said.
pub fn enforce_blocking(check: &ConnectionCheck) -> anyhow::Result<()> {
    let blocking: Vec<_> = check
        .blocking_issues()
        .map(|issue| format!("{} ({})", issue.kind.as_str(), issue.scope))
        .collect();
    if blocking.is_empty() {
        Ok(())
    } else {
        bail!("connection check failed: {}", blocking.join(", "))
    }
}

pub fn render_json(tenant: &str, check: &ConnectionCheck) -> anyhow::Result<String> {
    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "tenant": tenant,
        "ok": check.blocking_issues().next().is_none(),
        "check": check,
    }))?)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::diagnostics::{AccessIssue, AccessIssueKind, PermissionVerdict};

    fn check_with(issues: Vec<AccessIssueKind>) -> ConnectionCheck {
        ConnectionCheck {
            checked_at: "2026-01-01T00:00:00Z".into(),
            subscriptions: Vec::new(),
            inaccessible_subscriptions: Vec::new(),
            verdict: PermissionVerdict::UnableToVerify,
            grants: Vec::new(),
            issues: issues
                .into_iter()
                .map(|kind| AccessIssue {
                    kind,
                    scope: "/".into(),
                })
                .collect(),
        }
    }

    #[test]
    fn unit_check_blocks_when_no_subscriptions_visible() {
        let check = check_with(vec![AccessIssueKind::NoSubscriptions]);

        assert!(enforce_blocking(&check).is_err());
        assert!(render_json("t", &check).unwrap().contains("\"ok\": false"));
    }

    #[test]
    fn unit_check_passes_with_advisory_issues_only() {
        let check = check_with(vec![
            AccessIssueKind::ConditionalGrant,
            AccessIssueKind::AssignmentReadFailed,
        ]);

        assert!(enforce_blocking(&check).is_ok());
        assert!(render_json("t", &check).unwrap().contains("\"ok\": true"));
    }
}
