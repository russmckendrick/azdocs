use anyhow::Context;

use crate::arg::ArgClient;
use crate::config::Config;
use crate::labels::{Labels, fill};

const PROBE_QUERY: &str = "resourcecontainers \
| where type == \"microsoft.resources/subscriptions\" \
| project subscriptionId, name \
| order by subscriptionId asc";

/// Validate config and credentials end-to-end: acquire a token, then list the
/// subscriptions visible to the credential.
pub async fn run(config: &Config, labels: &Labels) -> anyhow::Result<()> {
    let words = &labels.cli.check;
    let credentials = config.credentials()?;
    println!(
        "{}",
        fill(&words.config_ok, &[("tenant", &credentials.tenant_id)])
    );

    let provider = super::token_provider(config)?;
    let client = ArgClient::new(super::http_client(), provider);
    let outcome = client
        .query_all(PROBE_QUERY, &config.collect.subscriptions)
        .await
        .context("probe query against Azure Resource Graph failed")?;

    println!("{}", words.token_ok);
    println!(
        "{}",
        fill(
            &words.visible_subscriptions,
            &[("count", &outcome.rows.len())]
        )
    );
    for row in &outcome.rows {
        let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let id = row
            .get("subscriptionId")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        println!("  {id}  {name}");
    }
    Ok(())
}
