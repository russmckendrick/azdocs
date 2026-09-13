mod client_credentials;
pub mod diagnostics;

pub use client_credentials::ClientCredentialsProvider;

use crate::error::AuthError;

/// Supplies bearer tokens for the Azure management API.
///
/// Implementations cache and refresh internally; callers just ask for a token
/// per request.
pub trait TokenProvider: Send + Sync {
    fn token(&self) -> impl Future<Output = Result<String, AuthError>> + Send;
}

/// Fixed token for tests.
#[derive(Debug, Clone)]
pub struct StaticTokenProvider(pub String);

impl TokenProvider for StaticTokenProvider {
    async fn token(&self) -> Result<String, AuthError> {
        Ok(self.0.clone())
    }
}

impl<P: TokenProvider> TokenProvider for &P {
    async fn token(&self) -> Result<String, AuthError> {
        (*self).token().await
    }
}
