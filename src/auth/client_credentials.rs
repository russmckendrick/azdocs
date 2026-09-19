use std::time::Duration;

use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::time::Instant;

use super::TokenProvider;
use crate::config::Credentials;
use crate::error::AuthError;
use crate::net::RetryPolicy;

/// Refresh when a cached token has less than this long to live.
const REFRESH_MARGIN: Duration = Duration::from_secs(300);

/// OAuth2 client-credentials flow against Microsoft Entra ID, with a cached
/// token refreshed shortly before expiry. The mutex makes concurrent callers
/// single-flight: only one refresh happens at a time.
pub struct ClientCredentialsProvider {
    http: reqwest::Client,
    token_url: String,
    scope: &'static str,
    credentials: Credentials,
    retry: RetryPolicy,
    cached: Mutex<Option<CachedToken>>,
}

struct CachedToken {
    access_token: String,
    expires_at: Instant,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

impl ClientCredentialsProvider {
    /// Authority and scope come from the credentials' cloud.
    pub fn new(http: reqwest::Client, credentials: Credentials) -> Self {
        let endpoints = credentials.cloud.endpoints();
        Self::with_authority(http, credentials, endpoints.authority)
    }

    /// `authority` override exists for tests (wiremock); the scope still
    /// follows the credentials' cloud.
    pub fn with_authority(
        http: reqwest::Client,
        credentials: Credentials,
        authority: &str,
    ) -> Self {
        let token_url = format!(
            "{authority}/{tenant}/oauth2/v2.0/token",
            tenant = credentials.tenant_id
        );
        Self {
            http,
            token_url,
            scope: credentials.cloud.endpoints().scope,
            credentials,
            retry: RetryPolicy::default(),
            cached: Mutex::new(None),
        }
    }

    pub fn with_retry_policy(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Bounded retry on transport failures and 5xx. A 4xx is a real answer
    /// (bad secret, unknown tenant) and is returned at once.
    async fn fetch_token(&self) -> Result<CachedToken, AuthError> {
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", self.credentials.client_id.as_str()),
            ("client_secret", self.credentials.client_secret.as_str()),
            ("scope", self.scope),
        ];
        let mut last: Option<AuthError> = None;
        for attempt in 0..self.retry.max_attempts.max(1) {
            let response = match self.http.post(&self.token_url).form(&params).send().await {
                Ok(response) => response,
                Err(err) => {
                    let wait = self.retry.backoff_delay(attempt);
                    tracing::warn!(%err, ?wait, attempt, "token request failed; retrying");
                    last = Some(AuthError::Http(err));
                    tokio::time::sleep(wait).await;
                    continue;
                }
            };
            let status = response.status();
            if status.is_success() {
                let token: TokenResponse = response
                    .json()
                    .await
                    .map_err(|err| AuthError::InvalidResponse(err.to_string()))?;
                return Ok(CachedToken {
                    access_token: token.access_token,
                    expires_at: Instant::now() + Duration::from_secs(token.expires_in),
                });
            }
            let detail = summarize_token_error(&response.text().await.unwrap_or_default())
                .replace(&self.credentials.client_secret, "[redacted]");
            let rejected = AuthError::Rejected {
                status: status.as_u16(),
                detail,
            };
            if !(status.is_server_error() || status.as_u16() == 429) {
                return Err(rejected);
            }
            let wait = self.retry.backoff_delay(attempt);
            tracing::warn!(%status, ?wait, attempt, "token endpoint unavailable; retrying");
            last = Some(rejected);
            tokio::time::sleep(wait).await;
        }
        // Statically infallible: the loop runs at least once and every
        // continue sets `last`.
        Err(last.expect("at least one attempt was made"))
    }
}

impl TokenProvider for ClientCredentialsProvider {
    async fn token(&self) -> Result<String, AuthError> {
        let mut cached = self.cached.lock().await;
        if let Some(token) = cached.as_ref()
            && token.expires_at.saturating_duration_since(Instant::now()) > REFRESH_MARGIN
        {
            return Ok(token.access_token.clone());
        }
        let fresh = self.fetch_token().await?;
        let access_token = fresh.access_token.clone();
        *cached = Some(fresh);
        Ok(access_token)
    }
}

/// Entra error bodies are JSON with a long `error_description`; surface the
/// error code and first line rather than the whole blob.
fn summarize_token_error(body: &str) -> String {
    #[derive(Deserialize)]
    struct EntraError {
        error: String,
        error_description: String,
    }
    match serde_json::from_str::<EntraError>(body) {
        Ok(parsed) => {
            let first_line = parsed.error_description.lines().next().unwrap_or_default();
            format!("{}: {first_line}", parsed.error)
        }
        Err(_) => body.chars().take(200).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_token_error_extracts_code_and_first_line() {
        let body = r#"{"error":"invalid_client","error_description":"AADSTS7000215: Invalid client secret provided.\nTrace ID: xyz"}"#;

        let summary = summarize_token_error(body);

        assert_eq!(
            summary,
            "invalid_client: AADSTS7000215: Invalid client secret provided."
        );
    }

    #[test]
    fn summarize_token_error_truncates_non_json_bodies() {
        let body = "x".repeat(500);

        let summary = summarize_token_error(&body);

        assert_eq!(summary.len(), 200);
    }
}
