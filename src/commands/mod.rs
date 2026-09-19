pub mod check;
pub mod collect;
pub mod config;
pub mod diagram;
pub mod init;
pub mod query;
pub mod report;
pub mod snapshots;

use crate::auth::ClientCredentialsProvider;
use crate::config::Config;

/// Shared HTTP client with the configured timeouts for the management and
/// identity endpoints.
pub fn http_client(config: &Config) -> reqwest::Client {
    crate::net::build_client(&config.collect.retry.http_settings())
}

pub fn token_provider(config: &Config) -> anyhow::Result<ClientCredentialsProvider> {
    let credentials = config.credentials()?;
    let retry = config.collect.retry.http_settings().retry;
    Ok(ClientCredentialsProvider::new(http_client(config), credentials).with_retry_policy(retry))
}

/// Resource Graph client for the configured cloud, retry policy and timeouts.
pub fn arg_client(
    config: &Config,
    provider: ClientCredentialsProvider,
) -> crate::arg::ArgClient<ClientCredentialsProvider> {
    let retry = config.collect.retry.http_settings().retry;
    crate::arg::ArgClient::for_cloud(http_client(config), provider, config.cloud)
        .with_retry_policy(retry)
}
