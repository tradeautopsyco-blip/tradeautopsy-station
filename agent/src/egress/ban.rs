//! Reading a venue's own "stop until" out of its response, so the local clock is
//! pinned to the venue's clock rather than to a guess.

/// `Retry-After` in delta-seconds. The HTTP-date form is not parsed: neither
/// Binance nor Kotak is documented to send it, and a misparsed date would set a
/// wrong expiry in the direction that hurts.
pub fn retry_after_ms(value: &str) -> Option<i64> {
    let secs: i64 = value.trim().parse().ok()?;
    if secs < 0 {
        return None;
    }
    Some(secs.saturating_mul(1000))
}

/// Binance -1003 ban bodies carry an absolute epoch-millisecond expiry:
/// `"...IP(1.2.3.4) banned until 1668134400000. Please use the websocket..."`.
/// Returns the absolute timestamp, not a duration.
pub fn banned_until_ms_from_body(body: &str) -> Option<i64> {
    let lower = body.to_ascii_lowercase();
    let idx = lower.find("banned until")?;
    let rest = &body[idx + "banned until".len()..];
    let digits: String = rest
        .chars()
        .skip_while(|c| c.is_whitespace())
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        return None;
    }
    let ts: i64 = digits.parse().ok()?;
    // Guard against a seconds-precision or otherwise nonsensical value being
    // read as milliseconds: anything before 2001 is not a real ban expiry.
    if ts < 1_000_000_000_000 {
        return None;
    }
    Some(ts)
}

/// Absolute expiry for a ban, in order of trust: the venue's own timestamp, then
/// `Retry-After`, then the caller's escalating default.
pub fn resolve_ban_until_ms(
    now_ms: i64,
    body: &str,
    retry_after: Option<&str>,
    fallback_ms: i64,
) -> i64 {
    if let Some(until) = banned_until_ms_from_body(body) {
        if until > now_ms {
            return until;
        }
    }
    if let Some(ms) = retry_after.and_then(retry_after_ms) {
        return now_ms.saturating_add(ms);
    }
    now_ms.saturating_add(fallback_ms)
}

/// Full jitter, so a fleet of stalled callers does not resume in the same
/// millisecond. Never shortens the wait below the venue's instruction.
pub fn jittered(base_ms: i64, jitter_ms: i64) -> i64 {
    if jitter_ms <= 0 {
        return base_ms;
    }
    use rand::Rng;
    base_ms.saturating_add(rand::thread_rng().gen_range(0..jitter_ms))
}

/// Capped exponential backoff for a reconnect loop. `attempt` is 0-based.
pub fn reconnect_backoff_ms(attempt: u32, base_ms: i64, cap_ms: i64) -> i64 {
    let shift = attempt.min(16);
    let raw = base_ms.saturating_mul(1i64 << shift);
    let capped = raw.min(cap_ms);
    jittered(capped, capped / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retry_after_seconds_become_millis() {
        assert_eq!(retry_after_ms("120"), Some(120_000));
        assert_eq!(retry_after_ms(" 5 "), Some(5_000));
        assert_eq!(retry_after_ms("0"), Some(0));
    }

    #[test]
    fn retry_after_rejects_dates_and_junk() {
        assert_eq!(retry_after_ms("Wed, 21 Oct 2015 07:28:00 GMT"), None);
        assert_eq!(retry_after_ms(""), None);
        assert_eq!(retry_after_ms("-5"), None);
    }

    #[test]
    fn binance_ban_body_yields_an_absolute_expiry() {
        let body = r#"{"code":-1003,"msg":"Way too many requests; IP(1.2.3.4) banned until 1668134400000. Please use the websocket for live updates to avoid bans."}"#;
        assert_eq!(banned_until_ms_from_body(body), Some(1_668_134_400_000));
    }

    #[test]
    fn a_body_without_a_ban_says_nothing() {
        assert_eq!(banned_until_ms_from_body("{\"code\":-1003}"), None);
        assert_eq!(banned_until_ms_from_body(""), None);
        // Seconds-precision value is not silently read as millis.
        assert_eq!(banned_until_ms_from_body("banned until 1668134400"), None);
    }

    #[test]
    fn venue_timestamp_outranks_retry_after_and_default() {
        let until = resolve_ban_until_ms(1_000, "banned until 1668134400000", Some("60"), 300_000);
        assert_eq!(until, 1_668_134_400_000);
    }

    #[test]
    fn retry_after_outranks_the_default() {
        assert_eq!(resolve_ban_until_ms(1_000, "", Some("60"), 300_000), 61_000);
    }

    #[test]
    fn a_stale_venue_timestamp_falls_through_to_the_default() {
        // Ban expiry already in the past: do not unfreeze on it.
        let until = resolve_ban_until_ms(
            1_700_000_000_000,
            "banned until 1668134400000",
            None,
            300_000,
        );
        assert_eq!(until, 1_700_000_300_000);
    }

    #[test]
    fn backoff_grows_and_then_stops_growing() {
        let cap = 60_000;
        let a0 = reconnect_backoff_ms(0, 1_000, cap);
        let a5 = reconnect_backoff_ms(5, 1_000, cap);
        let a30 = reconnect_backoff_ms(30, 1_000, cap);
        assert!((1_000..=1_500).contains(&a0), "got {a0}");
        assert!((32_000..=48_000).contains(&a5), "got {a5}");
        assert!(a30 <= cap + cap / 2);
        assert!(a30 >= cap);
    }

    #[test]
    fn jitter_never_shortens_the_wait() {
        for _ in 0..200 {
            let j = jittered(1_000, 500);
            assert!((1_000..1_500).contains(&j), "got {j}");
        }
    }
}
