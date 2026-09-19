use std::time::Duration;

use reqwest::header::HeaderMap;

pub use crate::net::RetryPolicy;

/// Proactive pacing from ARG quota headers: when the remaining-quota header
/// hits zero, wait out the advertised reset window instead of provoking a 429.
pub fn quota_pause(headers: &HeaderMap) -> Option<Duration> {
    let remaining: u32 = headers
        .get("x-ms-user-quota-remaining")?
        .to_str()
        .ok()?
        .trim()
        .parse()
        .ok()?;
    if remaining > 0 {
        return None;
    }
    let resets_after = headers.get("x-ms-user-quota-resets-after")?.to_str().ok()?;
    parse_hms(resets_after)
}

/// Parses the `hh:mm:ss` (fractional seconds allowed) format used by
/// `x-ms-user-quota-resets-after`.
fn parse_hms(value: &str) -> Option<Duration> {
    let mut parts = value.trim().splitn(3, ':');
    let hours: u64 = parts.next()?.parse().ok()?;
    let minutes: u64 = parts.next()?.parse().ok()?;
    let seconds: f64 = parts.next()?.parse().ok()?;
    Some(Duration::from_secs_f64(
        (hours * 3600 + minutes * 60) as f64 + seconds,
    ))
}

#[cfg(test)]
mod tests {
    use reqwest::header::HeaderValue;

    use super::*;

    fn headers(pairs: &[(&str, &str)]) -> HeaderMap {
        let mut map = HeaderMap::new();
        for (name, value) in pairs {
            map.insert(
                reqwest::header::HeaderName::from_bytes(name.as_bytes()).unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }
        map
    }

    #[test]
    fn quota_pause_returns_none_while_quota_remains() {
        let headers = headers(&[("x-ms-user-quota-remaining", "3")]);

        assert_eq!(quota_pause(&headers), None);
    }

    #[test]
    fn quota_pause_waits_for_reset_when_quota_exhausted() {
        let headers = headers(&[
            ("x-ms-user-quota-remaining", "0"),
            ("x-ms-user-quota-resets-after", "00:00:05"),
        ]);

        assert_eq!(quota_pause(&headers), Some(Duration::from_secs(5)));
    }

    #[test]
    fn parse_hms_supports_fractional_seconds() {
        assert_eq!(
            parse_hms("00:00:03.1200000"),
            Some(Duration::from_secs_f64(3.12))
        );
    }

    #[test]
    fn retry_after_prefers_header_over_default() {
        let policy = RetryPolicy {
            jitter: 0.0,
            ..RetryPolicy::default()
        };
        let headers = headers(&[("retry-after", "12")]);

        assert_eq!(policy.retry_after(&headers), Duration::from_secs(12));
    }
}
