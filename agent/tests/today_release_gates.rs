//! Today v1 release gates.

use chrono::{TimeZone, Utc};
use tradeautopsy_agent::{aggregate_known_pnl, is_aggregate_eligible, BrokerFill, RoundTripEngine};

fn fill_at(
    id: &str,
    side: &str,
    qty: f64,
    price: f64,
    filled_at: chrono::DateTime<Utc>,
) -> BrokerFill {
    BrokerFill {
        fill_id: id.to_string(),
        trade_id: format!("t-{id}"),
        symbol: "BTCUSDT".to_string(),
        side: side.to_string(),
        qty,
        price,
        filled_at,
        broker: "binance_us".to_string(),
        fee_amount: Some(0.5),
        fee_asset: Some("USDT".to_string()),
        ..Default::default()
    }
}

fn fill(id: &str, side: &str, qty: f64, price: f64) -> BrokerFill {
    fill_at(
        id,
        side,
        qty,
        price,
        Utc.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap(),
    )
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
            ..Default::default()
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
fn today_trades_table_most_recent_first() {
    use chrono::{Local, NaiveDate, TimeZone, Utc};
    use tradeautopsy_agent::{BrokerFill, RoundTripEngine};

    fn local_noon(date: NaiveDate) -> chrono::DateTime<Utc> {
        let local_dt = date.and_hms_opt(12, 0, 0).unwrap();
        Local
            .from_local_datetime(&local_dt)
            .single()
            .expect("local noon")
            .with_timezone(&Utc)
    }

    fn fill(id: &str, side: &str, at: chrono::DateTime<Utc>) -> BrokerFill {
        BrokerFill {
            fill_id: id.to_string(),
            trade_id: format!("t-{id}"),
            symbol: "BTCUSDT".to_string(),
            side: side.to_string(),
            qty: 0.01,
            price: 60_000.0,
            filled_at: at,
            broker: "binance_us".to_string(),
            fee_amount: Some(0.5),
            fee_asset: Some("USDT".to_string()),
            ..Default::default()
        }
    }

    let day = Local::now().date_naive();
    let t_morning = local_noon(day);
    let t_mid = t_morning + chrono::Duration::hours(2);
    let t_late = t_morning + chrono::Duration::hours(4);

    let engine = RoundTripEngine::new();
    let result = engine.reconstruct(vec![
        fill("b1", "BUY", t_morning),
        fill("s1", "SELL", t_mid),
        fill("b2", "BUY", t_mid + chrono::Duration::minutes(5)),
        fill("s2", "SELL", t_late),
        fill("b3", "BUY", t_late + chrono::Duration::minutes(5)),
        fill("s3", "SELL", t_late + chrono::Duration::hours(1)),
    ]);
    assert_eq!(result.round_trips.len(), 3);
    let mut sorted = result.round_trips.clone();
    sorted.sort_by(|a, b| b.closed_at.cmp(&a.closed_at));
    assert!(sorted[0].closed_at > sorted[1].closed_at);
    assert!(sorted[1].closed_at > sorted[2].closed_at);
}

#[test]
fn today_degraded_reason_serializes_snake_case() {
    let reason = tradeautopsy_agent::TodayDegradedReason::SyncUnavailable;
    let j = serde_json::to_string(&reason).unwrap();
    assert_eq!(j, "\"sync_unavailable\"");
}

#[test]
fn today_stub_adapter_serializes_snake_case() {
    let reason = tradeautopsy_agent::TodayDegradedReason::StubAdapter;
    let j = serde_json::to_string(&reason).unwrap();
    assert_eq!(j, "\"stub_adapter\"");
}

#[test]
fn today_exchange_filters_not_ready_serializes_snake_case() {
    // I-S4
    let reason = tradeautopsy_agent::TodayDegradedReason::ExchangeFiltersNotReady;
    let j = serde_json::to_string(&reason).unwrap();
    assert_eq!(j, "\"exchange_filters_not_ready\"");
}

#[test]
fn open_inventory_keeps_leftover_buy_qty() {
    let open_at = Utc.with_ymd_and_hms(2026, 7, 4, 12, 0, 0).unwrap();
    let sell_at = Utc.with_ymd_and_hms(2026, 7, 4, 13, 0, 0).unwrap();
    let leftover = tradeautopsy_agent::open_inventory_from_fills(&[
        fill_at("b1", "BUY", 0.02, 60_000.0, open_at),
        fill_at("s1", "SELL", 0.01, 61_000.0, sell_at),
    ]);
    assert_eq!(leftover.len(), 1);
    assert_eq!(leftover[0].symbol, "BTCUSDT");
    assert!((leftover[0].qty - 0.01).abs() < 1e-12);
    assert_eq!(leftover[0].side, "LONG");
    assert_eq!(leftover[0].first_filled_at, Some(open_at));
}

#[test]
fn open_inventory_omits_fully_closed_symbol() {
    let closed = tradeautopsy_agent::open_inventory_from_fills(&[
        fill("b1", "BUY", 0.01, 60_000.0),
        fill("s1", "SELL", 0.01, 61_000.0),
    ]);
    assert!(closed.is_empty());
}

#[test]
fn open_inventory_reopen_uses_reopen_filled_at() {
    let first_open = Utc.with_ymd_and_hms(2026, 7, 4, 10, 0, 0).unwrap();
    let close = Utc.with_ymd_and_hms(2026, 7, 4, 11, 0, 0).unwrap();
    let reopen = Utc.with_ymd_and_hms(2026, 7, 4, 14, 0, 0).unwrap();
    // Input is reverse chronological so a missing sort would keep the closed lot's time.
    let rows = tradeautopsy_agent::open_inventory_from_fills(&[
        fill_at("b2", "BUY", 0.01, 62_000.0, reopen),
        fill_at("s1", "SELL", 0.01, 61_000.0, close),
        fill_at("b1", "BUY", 0.01, 60_000.0, first_open),
    ]);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].symbol, "BTCUSDT");
    assert!((rows[0].qty - 0.01).abs() < 1e-12);
    assert_eq!(rows[0].side, "LONG");
    assert_eq!(rows[0].first_filled_at, Some(reopen));
}
