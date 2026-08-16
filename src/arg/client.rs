use serde::Deserialize;
use serde_json::{Value, json};

use super::throttle::{RetryPolicy, quota_pause};
use crate::auth::TokenProvider;
use crate::error::ArgError;

pub const DEFAULT_ENDPOINT: &str = "https://management.azure.com";
const API_VERSION: &str = "2022-10-01";
/// ARG's maximum page size.
const PAGE_SIZE: u32 = 1000;

/// Azure Resource Graph client: runs a KQL query across subscriptions and
/// follows `$skipToken` pagination until all rows are returned.
pub struct ArgClient<P> {
    http: reqwest::Client,
    tokens: P,
    query_url: String,
    retry: RetryPolicy,
}

/// All rows of a fully-paginated query.
#[derive(Debug)]
pub struct QueryOutcome {
    pub rows: Vec<Value>,
    pub pages: u32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueryResponse {
    data: Vec<Value>,
    #[serde(rename = "$skipToken")]
    skip_token: Option<String>,
}

impl<P: TokenProvider> ArgClient<P> {
    pub fn new(http: reqwest::Client, tokens: P) -> Self {
        Self::with_endpoint(http, tokens, DEFAULT_ENDPOINT)
    }

    /// `endpoint` override exists for tests (wiremock).
    pub fn with_endpoint(http: reqwest::Client, tokens: P, endpoint: &str) -> Self {
        Self {
            http,
            tokens,
            query_url: format!(
                "{endpoint}/providers/Microsoft.ResourceGraph/resources?api-version={API_VERSION}"
            ),
            retry: RetryPolicy::default(),
        }
    }

    pub fn with_retry_policy(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Run `kql` scoped to `subscriptions` (empty = all visible) and collect
    /// every page. Queries that paginate past one page must have a
    /// deterministic sort (`| order by id asc`) to make `$skipToken` stable.
    pub async fn query_all(
        &self,
        kql: &str,
        subscriptions: &[String],
    ) -> Result<QueryOutcome, ArgError> {
        let mut rows = Vec::new();
        let mut pages = 0;
        let mut skip_token: Option<String> = None;

        loop {
            let response = self
                .query_page(kql, subscriptions, skip_token.as_deref())
                .await?;
            rows.extend(response.data);
            pages += 1;
            match response.skip_token {
                Some(token) => skip_token = Some(token),
                None => break,
            }
        }

        Ok(QueryOutcome { rows, pages })
    }

    async fn query_page(
        &self,
        kql: &str,
        subscriptions: &[String],
        skip_token: Option<&str>,
    ) -> Result<QueryResponse, ArgError> {
        let mut options = json!({
            "resultFormat": "objectArray",
            "$top": PAGE_SIZE,
        });
        if let Some(token) = skip_token {
            options["$skipToken"] = json!(token);
        }
        let mut body = json!({ "query": kql, "options": options });
        if !subscriptions.is_empty() {
            body["subscriptions"] = json!(subscriptions);
        }

        for attempt in 0..self.retry.max_attempts {
            let token = self.tokens.token().await?;
            let response = self
                .http
                .post(&self.query_url)
                .bearer_auth(token)
                .json(&body)
                .send()
                .await?;
            let status = response.status();

            if status.is_success() {
                if let Some(pause) = quota_pause(response.headers()) {
                    tracing::debug!(?pause, "ARG quota exhausted; pausing before next request");
                    tokio::time::sleep(pause).await;
                }
                return response
                    .json()
                    .await
                    .map_err(|err| ArgError::InvalidResponse(err.to_string()));
            }

            if status.as_u16() == 429 {
                let wait = self.retry.retry_after(response.headers());
                tracing::warn!(?wait, attempt, "ARG throttled (429); backing off");
                tokio::time::sleep(wait).await;
                continue;
            }

            if status.is_server_error() {
                let wait = self.retry.backoff_delay(attempt);
                tracing::warn!(%status, ?wait, attempt, "ARG server error; retrying");
                tokio::time::sleep(wait).await;
                continue;
            }

            let detail = summarize_api_error(&response.text().await.unwrap_or_default());
            return Err(ArgError::Api {
                status: status.as_u16(),
                detail,
            });
        }

        Err(ArgError::ThrottledOut {
            attempts: self.retry.max_attempts,
        })
    }
}

/// ARG error bodies nest as {"error": {"code", "message", "details": [...]}}.
fn summarize_api_error(body: &str) -> String {
    #[derive(Deserialize)]
    struct ErrorBody {
        error: ErrorDetail,
    }
    #[derive(Deserialize)]
    struct ErrorDetail {
        code: String,
        message: String,
    }
    match serde_json::from_str::<ErrorBody>(body) {
        Ok(parsed) => format!("{}: {}", parsed.error.code, parsed.error.message),
        Err(_) => body.chars().take(300).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summarize_api_error_extracts_code_and_message() {
        let body = r#"{"error":{"code":"BadRequest","message":"Query is invalid.","details":[]}}"#;

        assert_eq!(summarize_api_error(body), "BadRequest: Query is invalid.");
    }
}
