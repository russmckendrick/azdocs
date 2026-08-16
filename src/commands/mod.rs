pub mod check;
pub mod init;
pub mod query;

use crate::auth::ClientCredentialsProvider;
use crate::config::Config;

/// Shared HTTP client with sane timeouts for the management API.
pub fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("azdocs/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(120))
        .connect_timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("static client configuration is valid")
}

pub fn token_provider(config: &Config) -> anyhow::Result<ClientCredentialsProvider> {
    let credentials = config.credentials()?;
    Ok(ClientCredentialsProvider::new(http_client(), credentials))
}
