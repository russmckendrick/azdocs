//! HTTP settings shared by every Azure call: timeouts, and the one retry
//! policy that decides how transport failures, throttling and server errors
//! are retried. Auth, ARG and the ARM diagnostics all read it from here so a
//! `[collect.retry]` change reaches all of them.

use std::time::Duration;

use reqwest::header::HeaderMap;

/// Backoff behaviour for throttled (429), transient (5xx) and transport
/// failures.
#[derive(Debug, Clone, PartialEq)]
pub struct RetryPolicy {
    pub max_attempts: u32,
    /// Base delay for exponential backoff (doubles per attempt).
    pub base_delay: Duration,
    /// Ceiling for any single wait, including a server's `Retry-After`.
    pub max_delay: Duration,
    /// Fallback wait when a 429 carries no usable Retry-After header.
    pub default_retry_after: Duration,
    /// Fraction of the computed delay to randomise, so concurrent queries
    /// retrying after the same failure do not retry in lockstep. 0 disables.
    pub jitter: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(60),
            default_retry_after: Duration::from_secs(5),
            jitter: 0.2,
        }
    }
}

impl RetryPolicy {
    /// `base · 2^attempt`, capped at `max_delay`, then jittered.
    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        let raw = self
            .base_delay
            .saturating_mul(2u32.saturating_pow(attempt.min(16)));
        self.jittered(raw.min(self.max_delay))
    }

    /// The server's `Retry-After` when present and sane, else the fallback;
    /// never more than `max_delay`, so a misconfigured header cannot park a
    /// collect for an hour.
    pub fn retry_after(&self, headers: &HeaderMap) -> Duration {
        parse_retry_after(headers)
            .unwrap_or(self.default_retry_after)
            .min(self.max_delay)
    }

    fn jittered(&self, delay: Duration) -> Duration {
        if self.jitter <= 0.0 || delay.is_zero() {
            return delay;
        }
        // Enough entropy for spreading retries; no extra crate needed.
        let bytes = uuid::Uuid::new_v4().into_bytes();
        let unit = f64::from(u16::from_be_bytes([bytes[0], bytes[1]])) / f64::from(u16::MAX);
        let factor = 1.0 + self.jitter * (2.0 * unit - 1.0);
        delay.mul_f64(factor.max(0.0))
    }
}

fn parse_retry_after(headers: &HeaderMap) -> Option<Duration> {
    let value = headers.get(reqwest::header::RETRY_AFTER)?.to_str().ok()?;
    let seconds: u64 = value.trim().parse().ok()?;
    Some(Duration::from_secs(seconds))
}

/// Everything a `reqwest::Client` needs from configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct HttpSettings {
    pub timeout: Duration,
    pub connect_timeout: Duration,
    pub retry: RetryPolicy,
}

impl Default for HttpSettings {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(120),
            connect_timeout: Duration::from_secs(15),
            retry: RetryPolicy::default(),
        }
    }
}

/// Shared HTTP client for the management and identity endpoints.
pub fn build_client(settings: &HttpSettings) -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("azdocs/", env!("CARGO_PKG_VERSION")))
        .timeout(settings.timeout)
        .connect_timeout(settings.connect_timeout)
        .build()
        // Statically infallible: only timeouts and a user agent are set.
        .expect("static client configuration is valid")
}

#[cfg(test)]
mod tests {
    use reqwest::header::HeaderValue;

    use super::*;

    fn policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 5,
            base_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(2),
            default_retry_after: Duration::from_secs(5),
            jitter: 0.0,
        }
    }

    #[test]
    fn unit_backoff_never_exceeds_max_delay() {
        let policy = policy();
        assert_eq!(policy.backoff_delay(0), Duration::from_millis(100));
        assert_eq!(policy.backoff_delay(3), Duration::from_millis(800));
        assert_eq!(policy.backoff_delay(10), Duration::from_secs(2));
        assert_eq!(policy.backoff_delay(u32::MAX), Duration::from_secs(2));
    }

    #[test]
    fn unit_retry_after_is_capped_at_max_delay() {
        let policy = policy();
        let mut headers = HeaderMap::new();
        headers.insert(
            reqwest::header::RETRY_AFTER,
            HeaderValue::from_static("3600"),
        );

        assert_eq!(policy.retry_after(&headers), Duration::from_secs(2));
        assert_eq!(
            policy.retry_after(&HeaderMap::new()),
            Duration::from_secs(2)
        );
    }

    #[test]
    fn unit_jitter_stays_within_the_configured_band() {
        let policy = RetryPolicy {
            jitter: 0.5,
            ..policy()
        };
        for _ in 0..50 {
            let delay = policy.backoff_delay(0).as_millis();
            assert!((50..=150).contains(&delay), "{delay}ms");
        }
    }
}
