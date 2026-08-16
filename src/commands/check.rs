use anyhow::Context;

use crate::arg::ArgClient;
use crate::config::Config;

const PROBE_QUERY: &str = "resourcecontainers \
| where type == \"microsoft.resources/subscriptions\" \
| project subscriptionId, name \
| order by subscriptionId asc";

/// Validate config and credentials end-to-end: acquire a token, then list the
/// subscriptions visible to the credential.
pub async fn run(config: &Config) -> anyhow::Result<()> {
    let credentials = config.credentials()?;
    println!("Config OK (tenant {})", credentials.tenant_id);

    let provider = super::token_provider(config)?;
    let client = ArgClient::new(super::http_client(), provider);
    let outcome = client
        .query_all(PROBE_QUERY, &config.collect.subscriptions)
        .await
        .context("probe query against Azure Resource Graph failed")?;

    println!("Token OK, Resource Graph reachable.");
    println!("Visible subscriptions: {}", outcome.rows.len());
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
