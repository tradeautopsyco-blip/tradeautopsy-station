//! Today v1 — round-trip + WAC engine unit tests (Slice 1).

use chrono::{DateTime, TimeZone, Utc};
use tradeautopsy_agent::{
    aggregate_known_pnl, is_aggregate_eligible, BrokerFill, ExchangeInfoSymbolCache,
    RoundTripEngine,
};

fn fill(
    id: &str,
    symbol: &str,
    side: &str,
    qty: f64,
    price: f64,
    at: DateTime<Utc>,
    fee_amount: Option<f64>,
    fee_asset: Option<&str>,
) -> BrokerFill {
    BrokerFill {
        fill_id: id.to_string(),
        trade_id: format!("trade-{id}"),
        symbol: symbol.to_string(),
        side: side.to_string(),
        qty,
        price,
        filled_at: at,
        broker: "binance_us".to_string(),
        fee_amount,
        fee_asset: fee_asset.map(str::to_string),
    }
}

fn t(h: u32, m: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 2, h, m, 0).unwrap()
}

/// 1. Basic round-trip: 2 buys + 1 sell fully closes BTC/USDT.
#[test]
fn basic_round_trip_three_fills_net_of_fees() {
    let engine = RoundTripEngine::new();
    let fills = vec![
        fill(
            "b1",
            "BTCUSDT",
            "BUY",
            0.5,
            60_000.0,
            t(9, 0),
            Some(30.0),
            Some("USDT"),
        ),
        fill(
            "b2",
            "BTCUSDT",
            "BUY",
            0.5,
            62_000.0,
            t(10, 0),
            Some(31.0),
            Some("USDT"),
        ),
        fill(
            "s1",
            "BTCUSDT",
            "SELL",
            1.0,
            65_000.0,
            t(14, 0),
            Some(32.5),
            Some("USDT"),
        ),
    ];

    let result = engine.reconstruct(fills);
    assert_eq!(result.unhandled_fees.len(), 0);
    assert_eq!(result.round_trips.len(), 1);

    let rt = &result.round_trips[0];
    // avg_entry = (0.5×60k + 0.5×62k) / 1 = 61_000
    assert!((rt.avg_entry_price - 61_000.0).abs() < 1e-6, "avg_entry");
    assert!((rt.avg_exit_price - 65_000.0).abs() < 1e-6, "avg_exit");
    assert!((rt.qty - 1.0).abs() < 1e-6);
    // gross = 1 × (65k − 61k) = 4_000; fees = 30 + 31 + 32.5 = 93.5
    assert!((rt.fees_usd.unwrap() - 93.5).abs() < 1e-6, "fees_usd");
    assert!((rt.realized_pnl_usd.unwrap() - 3_906.5).abs() < 1e-6, "net pnl");
    assert!(!rt.unknown_basis);
    assert!(!rt.fee_unhandled);
    assert!(!rt.quote_not_usd);
    assert_eq!(rt.opened_at, t(9, 0));
    assert_eq!(rt.closed_at, t(14, 0));
}

/// 2. WAC averaging across two buys, one full exit.
#[test]
fn wac_averaging_two_buys_one_sell() {
    let engine = RoundTripEngine::new();
    let fills = vec![
        fill("b1", "BTCUSDT", "BUY", 1.0, 50_000.0, t(8, 0), None, None),
        fill("b2", "BTCUSDT", "BUY", 1.0, 60_000.0, t(9, 0), None, None),
        fill("s1", "BTCUSDT", "SELL", 2.0, 70_000.0, t(12, 0), None, None),
    ];

    let rt = &engine.reconstruct(fills).round_trips[0];
    assert!((rt.avg_entry_price - 55_000.0).abs() < 1e-6);
    assert!((rt.avg_exit_price - 70_000.0).abs() < 1e-6);
    // 2 × (70k − 55k) = 30_000 before fees
    assert!((rt.realized_pnl_usd.unwrap() - 30_000.0).abs() < 1e-6);
}

/// 3. Partial close stays one round-trip; avg_entry unchanged across partial.
#[test]
fn partial_close_is_one_round_trip_not_two() {
    let engine = RoundTripEngine::new();
    let fills = vec![
        fill("b1", "BTCUSDT", "BUY", 2.0, 50_000.0, t(10, 0), None, None),
        fill("s1", "BTCUSDT", "SELL", 1.0, 70_000.0, t(11, 0), None, None),
        fill("s2", "BTCUSDT", "SELL", 1.0, 70_000.0, t(12, 0), None, None),
    ];

    let result = engine.reconstruct(fills);
    assert_eq!(result.round_trips.len(), 1, "partial close must not split round-trips");

    let rt = &result.round_trips[0];
    assert!((rt.avg_entry_price - 50_000.0).abs() < 1e-6);
    assert!((rt.avg_exit_price - 70_000.0).abs() < 1e-6);
    assert!((rt.qty - 2.0).abs() < 1e-6);
    // 2 × (70k − 50k) = 40_000
    assert!((rt.realized_pnl_usd.unwrap() - 40_000.0).abs() < 1e-6);
}

/// 4. Flat → new buy starts a fresh round-trip with independent WAC.
#[test]
fn reset_after_flat_produces_two_independent_round_trips() {
    let engine = RoundTripEngine::new();
    let fills = vec![
        fill("b1", "BTCUSDT", "BUY", 1.0, 50_000.0, t(8, 0), None, None),
        fill("s1", "BTCUSDT", "SELL", 1.0, 55_000.0, t(9, 0), None, None),
        fill("b2", "BTCUSDT", "BUY", 1.0, 60_000.0, t(10, 0), None, None),
        fill("s2", "BTCUSDT", "SELL", 1.0, 65_000.0, t(11, 0), None, None),
    ];

    let trips = engine.reconstruct(fills).round_trips;
    assert_eq!(trips.len(), 2);

    assert!((trips[0].avg_entry_price - 50_000.0).abs() < 1e-6);
    assert!((trips[0].realized_pnl_usd.unwrap() - 5_000.0).abs() < 1e-6);

    assert!((trips[1].avg_entry_price - 60_000.0).abs() < 1e-6);
    assert!((trips[1].realized_pnl_usd.unwrap() - 5_000.0).abs() < 1e-6);
    assert_ne!(trips[0].opened_at, trips[1].opened_at);
}

/// 5. Unknown-basis sell does not corrupt other round-trips or aggregates.
#[test]
fn unknown_basis_sell_excluded_from_aggregate() {
    let engine = RoundTripEngine::new();
    let fills = vec![
        fill("b1", "BTCUSDT", "BUY", 1.0, 50_000.0, t(8, 0), None, None),
        fill("s1", "BTCUSDT", "SELL", 1.0, 55_000.0, t(9, 0), None, None),
        fill("u1", "SOLUSDT", "SELL", 10.0, 150.0, t(10, 0), Some(1.5), Some("USDT")),
    ];

    let result = engine.reconstruct(fills);
    assert_eq!(result.round_trips.len(), 2);

    let known = result
        .round_trips
        .iter()
        .find(|rt| rt.symbol == "BTCUSDT")
        .expect("btc trip");
    assert!(!known.unknown_basis);
    assert!(!known.fee_unhandled);
    assert!((known.realized_pnl_usd.unwrap() - 5_000.0).abs() < 1e-6);

    let unknown = result
        .round_trips
        .iter()
        .find(|rt| rt.symbol == "SOLUSDT")
        .expect("sol trip");
    assert!(unknown.unknown_basis);
    assert!(!unknown.fee_unhandled);
    assert!(unknown.realized_pnl_usd.is_none());

    let aggregate = aggregate_known_pnl(&result.round_trips);
    assert!((aggregate - 5_000.0).abs() < 1e-6);
}

/// 6a. Base-asset fee converted at fill price via exchangeInfo cache (no heuristic).
#[test]
fn fee_normalization_base_asset_via_exchange_info() {
    let json = r#"{
        "symbols": [
            { "symbol": "BTCUSDT", "baseAsset": "BTC", "quoteAsset": "USDT", "status": "TRADING" }
        ]
    }"#;
    let cache = ExchangeInfoSymbolCache::from_exchange_info_json(json).expect("parse");
    let engine = RoundTripEngine::with_exchange_info(cache, false);

    let fills = vec![
        fill(
            "b1",
            "BTCUSDT",
            "BUY",
            0.01,
            60_000.0,
            t(10, 0),
            Some(0.0001),
            Some("BTC"),
        ),
        fill(
            "s1",
            "BTCUSDT",
            "SELL",
            0.01,
            61_000.0,
            t(11, 0),
            Some(0.0001),
            Some("BTC"),
        ),
    ];

    let rt = &engine.reconstruct(fills).round_trips[0];
    assert!(!rt.fee_unhandled);
    // gross = 0.01 × (61k − 60k) = 10; fees = 6 + 6.1 = 12.1
    assert!((rt.fees_usd.unwrap() - 12.1).abs() < 1e-4);
    assert!((rt.realized_pnl_usd.unwrap() - (-2.1)).abs() < 1e-4);
}

/// 6b. Non-USDT quote pair requires exchangeInfo — heuristic cannot resolve ETHBTC.
#[test]
fn fee_normalization_ethbtc_base_asset_requires_exchange_info() {
    let json = r#"{
        "symbols": [
            { "symbol": "ETHBTC", "baseAsset": "ETH", "quoteAsset": "BTC", "status": "TRADING" }
        ]
    }"#;
    let cache = ExchangeInfoSymbolCache::from_exchange_info_json(json).expect("parse");
    let engine = RoundTripEngine::with_exchange_info(cache, false);

    // ETH fee on ETHBTC — only works with authoritative baseAsset=ETH from exchangeInfo.
    let fills = vec![
        fill(
            "b1",
            "ETHBTC",
            "BUY",
            1.0,
            0.05,
            t(10, 0),
            Some(0.001),
            Some("ETH"),
        ),
        fill(
            "s1",
            "ETHBTC",
            "SELL",
            1.0,
            0.06,
            t(11, 0),
            Some(0.001),
            Some("ETH"),
        ),
    ];

    let rt = &engine.reconstruct(fills).round_trips[0];
    assert!(!rt.fee_unhandled);
    assert!(rt.quote_not_usd);
    assert!(rt.fees_usd.is_none());
    assert!(rt.realized_pnl_usd.is_none());
    assert!(!is_aggregate_eligible(rt));
}

/// 8. USD-quoted BTCUSDT round-trip unaffected by quote_not_usd gate.
#[test]
fn usd_quoted_pair_computes_normally() {
    let json = r#"{
        "symbols": [
            { "symbol": "BTCUSDT", "baseAsset": "BTC", "quoteAsset": "USDT", "status": "TRADING" }
        ]
    }"#;
    let cache = ExchangeInfoSymbolCache::from_exchange_info_json(json).expect("parse");
    let engine = RoundTripEngine::with_exchange_info(cache, false);

    let fills = vec![
        fill("b1", "BTCUSDT", "BUY", 1.0, 50_000.0, t(8, 0), None, None),
        fill("s1", "BTCUSDT", "SELL", 1.0, 55_000.0, t(9, 0), None, None),
    ];

    let rt = &engine.reconstruct(fills).round_trips[0];
    assert!(!rt.quote_not_usd);
    assert!(!rt.fee_unhandled);
    assert!(!rt.unknown_basis);
    assert!((rt.realized_pnl_usd.unwrap() - 5_000.0).abs() < 1e-6);
    assert!(is_aggregate_eligible(rt));
}

/// 9. Non-USD-quoted pair flagged; P&L not mislabeled as USD.
#[test]
fn non_usd_quoted_pair_flagged_not_mislabeled() {
    let json = r#"{
        "symbols": [
            { "symbol": "ETHBTC", "baseAsset": "ETH", "quoteAsset": "BTC", "status": "TRADING" },
            { "symbol": "BTCUSDT", "baseAsset": "BTC", "quoteAsset": "USDT", "status": "TRADING" }
        ]
    }"#;
    let cache = ExchangeInfoSymbolCache::from_exchange_info_json(json).expect("parse");
    let engine = RoundTripEngine::with_exchange_info(cache, false);

    let fills = vec![
        fill("b1", "ETHBTC", "BUY", 1.0, 0.05, t(8, 0), None, None),
        fill("s1", "ETHBTC", "SELL", 1.0, 0.06, t(9, 0), None, None),
        fill("b2", "BTCUSDT", "BUY", 1.0, 50_000.0, t(10, 0), None, None),
        fill("s2", "BTCUSDT", "SELL", 1.0, 55_000.0, t(11, 0), None, None),
    ];

    let result = engine.reconstruct(fills);
    assert_eq!(result.round_trips.len(), 2);

    let ethbtc = result
        .round_trips
        .iter()
        .find(|rt| rt.symbol == "ETHBTC")
        .expect("ethbtc trip");
    assert!(ethbtc.quote_not_usd);
    assert!(ethbtc.realized_pnl_usd.is_none());
    assert!(!is_aggregate_eligible(ethbtc));

    let btcusdt = result
        .round_trips
        .iter()
        .find(|rt| rt.symbol == "BTCUSDT")
        .expect("btcusdt trip");
    assert!(!btcusdt.quote_not_usd);
    assert!((btcusdt.realized_pnl_usd.unwrap() - 5_000.0).abs() < 1e-6);

    let aggregate = aggregate_known_pnl(&result.round_trips);
    assert!((aggregate - 5_000.0).abs() < 1e-6, "ETHBTC BTC-denominated P&L excluded");
}

/// 10. Honesty flags are independent; aggregate excludes when any is set.
#[test]
fn all_three_flags_independent() {
    let json = r#"{
        "symbols": [
            { "symbol": "ETHBTC", "baseAsset": "ETH", "quoteAsset": "BTC", "status": "TRADING" }
        ]
    }"#;
    let cache = ExchangeInfoSymbolCache::from_exchange_info_json(json).expect("parse");
    let engine = RoundTripEngine::with_exchange_info(cache, false);

    // Unknown-basis sell on non-USD-quoted pair with unhandled BNB fee.
    let fills = vec![fill(
        "u1",
        "ETHBTC",
        "SELL",
        1.0,
        0.06,
        t(10, 0),
        Some(0.001),
        Some("BNB"),
    )];

    let result = engine.reconstruct(fills);
    assert_eq!(result.round_trips.len(), 1);
    assert_eq!(result.unhandled_fees.len(), 1);

    let rt = &result.round_trips[0];
    assert!(rt.unknown_basis);
    assert!(rt.fee_unhandled);
    assert!(rt.quote_not_usd);
    assert!(rt.realized_pnl_usd.is_none());
    assert!(!is_aggregate_eligible(rt));
    assert_eq!(aggregate_known_pnl(&result.round_trips), 0.0);
}

/// 11. Non-USD-quoted trip must not expose mislabeled fees_usd.
#[test]
fn quote_not_usd_trip_has_no_fees_usd() {
    let json = r#"{
        "symbols": [
            { "symbol": "ETHBTC", "baseAsset": "ETH", "quoteAsset": "BTC", "status": "TRADING" }
        ]
    }"#;
    let cache = ExchangeInfoSymbolCache::from_exchange_info_json(json).expect("parse");
    let engine = RoundTripEngine::with_exchange_info(cache, false);

    let fills = vec![
        fill(
            "b1",
            "ETHBTC",
            "BUY",
            1.0,
            0.05,
            t(8, 0),
            Some(0.001),
            Some("ETH"),
        ),
        fill(
            "s1",
            "ETHBTC",
            "SELL",
            1.0,
            0.06,
            t(9, 0),
            Some(0.001),
            Some("ETH"),
        ),
    ];

    let rt = &engine.reconstruct(fills).round_trips[0];
    assert!(rt.quote_not_usd);
    assert!(rt.realized_pnl_usd.is_none());
    assert!(rt.fees_usd.is_none());
    assert!(!is_aggregate_eligible(rt));
}

/// 7. BNB fee → fee_unhandled, no P&L, excluded from aggregate (same as unknown_basis).
#[test]
fn fee_unhandled_excludes_from_aggregate_like_unknown_basis() {
    let engine = RoundTripEngine::new();
    let fills = vec![
        fill(
            "b1",
            "BTCUSDT",
            "BUY",
            0.01,
            60_000.0,
            t(10, 0),
            Some(0.001),
            Some("BNB"),
        ),
        fill(
            "s1",
            "BTCUSDT",
            "SELL",
            0.01,
            61_000.0,
            t(11, 0),
            None,
            None,
        ),
        // Known-basis control trip — must not be corrupted by the BNB trip above.
        fill("b2", "ETHUSDT", "BUY", 1.0, 3_000.0, t(12, 0), None, None),
        fill("s2", "ETHUSDT", "SELL", 1.0, 3_100.0, t(13, 0), None, None),
    ];

    let result = engine.reconstruct(fills);
    assert_eq!(result.unhandled_fees.len(), 1);
    assert_eq!(result.unhandled_fees[0].fee_asset, "BNB");

    let bnb_trip = result
        .round_trips
        .iter()
        .find(|rt| rt.symbol == "BTCUSDT")
        .expect("btc trip");
    assert!(bnb_trip.fee_unhandled);
    assert!(!bnb_trip.unknown_basis);
    assert!(bnb_trip.realized_pnl_usd.is_none());
    assert!(!is_aggregate_eligible(bnb_trip));

    let eth_trip = result
        .round_trips
        .iter()
        .find(|rt| rt.symbol == "ETHUSDT")
        .expect("eth trip");
    assert!(!eth_trip.fee_unhandled);
    assert!((eth_trip.realized_pnl_usd.unwrap() - 100.0).abs() < 1e-6);

    let aggregate = aggregate_known_pnl(&result.round_trips);
    assert!((aggregate - 100.0).abs() < 1e-6, "BNB trip must not inflate aggregate");
}
