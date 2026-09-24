//! Binance European Options realized-PnL — **explicit none owner** (`binance-com-options`).
//!
//! Lock: `issues/compliance/locks/binance-com-options.md`.
//! Premium / settlement identity is **NOT SPECIFIED** until the lock is amended.
//! Do **not** treat `GET /eapi/v1/userTrades` as realized PnL (fees ≠ round-trip premium).

pub const OWNER_PATH: &str = "agent/src/options_realized_pnl.rs";
pub const BOOK_ID: &str = "binance-com-options";

/// Documented deferral — amend lock before implementing a sum identity.
pub const PREMIUM_SETTLEMENT_IDENTITY: &str = "NOT SPECIFIED IN SOURCE (lock not amended)";

/// Venue fill stream for display / fees — **not** this book's realized-PnL method.
pub const REFUSED_USER_TRADES_AS_REALIZED: &str = "eapi/v1/userTrades";

/// No realized owner yet. Always `None` — honesty: do not claim 0.
pub fn realized_pnl_from_user_trades(_body: &str) -> Option<f64> {
    None
}

/// Options fills must not enter COM spot WAC.
pub fn refuses_round_trip_engine_ownership() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const USER_TRADES_FIXTURE: &str = r#"[
        {"id":1,"symbol":"BTC-200730-9000-C","price":"100","qty":"1","commission":"0.01","commissionAsset":"USDT"}
    ]"#;

    #[test]
    fn explicit_none_owner_not_round_trip_engine() {
        assert_eq!(OWNER_PATH, "agent/src/options_realized_pnl.rs");
        assert_ne!(OWNER_PATH, "agent/src/round_trip_engine.rs");
        assert!(refuses_round_trip_engine_ownership());
        assert!(realized_pnl_from_user_trades(USER_TRADES_FIXTURE).is_none());
        assert_ne!(realized_pnl_from_user_trades(USER_TRADES_FIXTURE), Some(0.0));
        assert_eq!(
            PREMIUM_SETTLEMENT_IDENTITY,
            "NOT SPECIFIED IN SOURCE (lock not amended)"
        );
        assert_eq!(REFUSED_USER_TRADES_AS_REALIZED, "eapi/v1/userTrades");
        let rte = include_str!("round_trip_engine.rs");
        assert!(!rte.contains("binance-com-options"));
        assert!(!rte.contains("options_realized"));
    }
}
