use std::time::Duration;

use serde::Deserialize;
use tokio::sync::Mutex;
use tokio::time::Instant;

use super::TokenProvider;
use crate::config::Credentials;
use crate::error::AuthError;

pub const DEFAULT_AUTHORITY: &str = "https://login.microsoftonline.com";
const MANAGEMENT_SCOPE: &str = "https://management.azure.com/.default";
/// Refresh when a cached token has less than this long to live.
const REFRESH_MARGIN: Duration = Duration::from_secs(300);

/// OAuth2 client-credentials flow against Microsoft Entra ID, with a cached
/// token refreshed shortly before expiry. The mutex makes concurrent callers
/// single-flight: only one refresh happens at a time.
pub struct ClientCredentialsProvider {
    http: reqwest::Client,
    token_url: String,
    credentials: Credentials,
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
    pub fn new(http: reqwest::Client, credentials: Credentials) -> Self {
        Self::with_authority(http, credentials, DEFAULT_AUTHORITY)
    }

    /// `authority` override exists for tests (wiremock).
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
            credentials,
            cached: Mutex::new(None),
        }
    }

    async fn fetch_token(&self) -> Result<CachedToken, AuthError> {
        let params = [
            ("grant_type", "client_credentials"),
            ("client_id", self.credentials.client_id.as_str()),
            ("client_secret", self.credentials.client_secret.as_str()),
            ("scope", MANAGEMENT_SCOPE),
        ];
        let response = self.http.post(&self.token_url).form(&params).send().await?;
        let status = response.status();
        if !status.is_success() {
            let detail = summarize_token_error(&response.text().await.unwrap_or_default())
                .replace(&self.credentials.client_secret, "[redacted]");
            return Err(AuthError::Rejected {
                status: status.as_u16(),
                detail,
            });
        }
        let token: TokenResponse = response
            .json()
            .await
            .map_err(|err| AuthError::InvalidResponse(err.to_string()))?;
        Ok(CachedToken {
            access_token: token.access_token,
            expires_at: Instant::now() + Duration::from_secs(token.expires_in),
        })
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
