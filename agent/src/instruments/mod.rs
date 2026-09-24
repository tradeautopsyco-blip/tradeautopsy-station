mod store;
mod ticker;

pub use store::{stamp_zerodha_kite_nfo_fills, InstrumentStore};
pub use ticker::normalize_broker_ticker;

fn env_flag_truthy(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes"
    )
}

/// Station default is off. Set `AGENT_ENABLE_ZERODHA_INSTRUMENTS` for old Bar-only tests.
pub fn zerodha_instruments_enabled() -> bool {
    std::env::var("AGENT_ENABLE_ZERODHA_INSTRUMENTS")
        .map(|v| env_flag_truthy(&v))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zerodha_instruments_env_flag_truthy() {
        assert!(env_flag_truthy("1"));
        assert!(env_flag_truthy("TRUE"));
        assert!(env_flag_truthy(" yes "));
        assert!(!env_flag_truthy("0"));
        assert!(!env_flag_truthy("false"));
        assert!(!env_flag_truthy(""));
    }
}
