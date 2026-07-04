//! Today v1 release gates.

use tradeautopsy_agent::{
    aggregate_known_pnl, is_aggregate_eligible, BrokerFill, RoundTripEngine,
};
use chrono::{DateTime, TimeZone, Utc};

fn fill(id: &str, side: &str, qty: f64, price: f64) -> BrokerFill {
    BrokerFill {
        fill_id: id.to_string(),
        trade_id: format!("t-{id}"),
        symbol: "BTCUSDT".to_string(),
        side: side.to_string(),
        qty,
        price,
        filled_at: Utc.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap(),
        broker: "binance_us".to_string(),
        fee_amount: Some(0.5),
        fee_asset: Some("USDT".to_string()),
    }
}

#[test]
fn hero_aggregate_excludes_unknown_basis() {
    let engine = RoundTripEngine::new();
    let result = engine.reconstruct(vec![
        fill("b1", "BUY", 0.01, 60_000.0),
        fill("s1", "SELL", 0.01, 61_000.0),
        BrokerFill {
            fill_id: "u1".into(),
            trade_id: "tu1".into(),
            symbol: "SOLUSDT".into(),
            side: "SELL".into(),
            qty: 1.0,
            price: 150.0,
            filled_at: Utc.with_ymd_and_hms(2026, 7, 4, 13, 0, 0).unwrap(),
            broker: "binance_us".into(),
            fee_amount: None,
            fee_asset: None,
        },
    ]);
    let known: Vec<_> = result
        .round_trips
        .iter()
        .filter(|rt| is_aggregate_eligible(rt))
        .collect();
    assert_eq!(known.len(), 1);
    assert!(aggregate_known_pnl(&result.round_trips) > 0.0);
}

#[test]
fn today_degraded_reason_serializes_snake_case() {
    let reason = tradeautopsy_agent::TodayDegradedReason::SyncUnavailable;
    let j = serde_json::to_string(&reason).unwrap();
    assert_eq!(j, "\"sync_unavailable\"");
}
