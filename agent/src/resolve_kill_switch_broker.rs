//! Resolve broker slug for L3 DNS — body `broker` or `AGENT_PROTECTIVE_BROKER_SLUG` (#192).

/// Slug from request body / live-state `protective_broker_slug`, else agent env fallback.
pub fn resolve_kill_switch_broker(
    body_broker: Option<&str>,
    env_slug: Option<&str>,
) -> Result<String, String> {
    if let Some(raw) = body_broker {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_ascii_lowercase());
        }
    }
    if let Some(raw) = env_slug {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_ascii_lowercase());
        }
    }
    Err(
        "broker slug required — pass body.broker (from live-state protective_broker_slug) or set AGENT_PROTECTIVE_BROKER_SLUG"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_body_broker_over_env() {
        assert_eq!(
            resolve_kill_switch_broker(Some("zerodha"), Some("kotak_neo")).unwrap(),
            "zerodha"
        );
    }

    #[test]
    fn falls_back_to_env_when_body_missing() {
        assert_eq!(
            resolve_kill_switch_broker(None, Some("kotak_neo")).unwrap(),
            "kotak_neo"
        );
    }

    #[test]
    fn normalizes_case_and_whitespace() {
        assert_eq!(
            resolve_kill_switch_broker(Some("  Kite  "), None).unwrap(),
            "kite"
        );
    }

    #[test]
    fn rejects_empty_body_and_env() {
        assert!(resolve_kill_switch_broker(None, None).is_err());
        assert!(resolve_kill_switch_broker(Some("  "), Some("")).is_err());
    }
}
