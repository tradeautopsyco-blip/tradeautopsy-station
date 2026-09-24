//! `zerodha_kite` NFO book component contract (P6 · `zerodha-nse-nfo`).

mod ubi_support;

use std::collections::HashMap;
use tradeautopsy_agent::{
    fill_event_to_broker_fill, run_fetch_fills, NfoRealizedPnlEngine,
    split_fills_by_book, AssetClass, BrokerHttpFixture, FillCursor, InstrumentClass,
    UbiHostConfig, UbiHostState, WitAssetClass, WitInstrumentClass, ZERODHA_NSE_NFO_BOOK_ID,
};
use ubi_support::{assert_component_never_saw_secrets, component_wasm, sentinel_hmac};

const TRADES_PATH: &str = "/trades";
/// 2026-07-25 11:30:00 IST (aligned with kotak 11:00 + 30m anchor).
const NFO_BUY_MS: i64 = 1_784_959_200_000;
/// 2026-07-25 12:00:00 IST.
const NFO_SELL_MS: i64 = 1_784_961_000_000;
const NFO_LOT: i64 = 50;

fn read_kite_fixture(name: &str) -> String {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/zerodha_kite")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn nfo_config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-zerodha-nfo-001".into(),
        broker_slug: "zerodha_kite".into(),
        book_id: ZERODHA_NSE_NFO_BOOK_ID.into(),
        asset_class: AssetClass::Equity,
        instrument_class: InstrumentClass::Option,
        is_inverse: false,
        credentials: sentinel_hmac(),
    }
}

fn fixture_state(status: u16, body: String) -> UbiHostState {
    let mut fixtures = HashMap::new();
    fixtures.insert(
        TRADES_PATH.to_string(),
        BrokerHttpFixture {
            status,
            body,
            headers: vec![],
            error_class: None,
        },
    );
    UbiHostState::new(nfo_config(), fixtures)
}

fn empty_cursor() -> FillCursor {
    FillCursor {
        since_unix_ms: None,
        from_id: None,
        symbol: None,
    }
}

#[test]
fn nfo_nrml_trades_map_to_nse_fo_fill_events() {
    let wasm = component_wasm("zerodha_kite");
    let state = fixture_state(200, read_kite_fixture("trades_nfo_nrml_qty2.json"));

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    let nfo: Vec<_> = fills
        .iter()
        .filter(|f| f.exchange_segment.as_deref() == Some("nse_fo"))
        .collect();
    assert_eq!(nfo.len(), 2, "BUY+SELL NRML on NFO; cash CNC row separate");
    assert_eq!(nfo[0].symbol, "NIFTY2625024000CE");
    assert_eq!(nfo[0].side, "BUY");
    assert!((nfo[0].qty - 2.0).abs() < 1e-9);
    assert!((nfo[0].price - 100.0).abs() < 1e-9);
    assert_eq!(nfo[0].currency, "INR");
    assert_eq!(nfo[0].product.as_deref(), Some("NRML"));
    assert_eq!(nfo[0].filled_at_unix_ms, NFO_BUY_MS);
    assert_eq!(nfo[0].broker_slug, "zerodha_kite");
    assert_eq!(nfo[0].connection_id, "conn-zerodha-nfo-001");
    assert_eq!(nfo[0].asset_class, WitAssetClass::Equity);
    assert_eq!(nfo[0].instrument_class, WitInstrumentClass::Option);

    assert_eq!(nfo[1].side, "SELL");
    assert!((nfo[1].price - 110.0).abs() < 1e-9);
    assert_eq!(nfo[1].filled_at_unix_ms, NFO_SELL_MS);

    assert!(
        fills
            .iter()
            .any(|f| f.exchange_segment.as_deref() == Some("NSE") && f.symbol == "RELIANCE"),
        "cash row still present for host split"
    );
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn split_puts_nfo_on_zerodha_nse_nfo_book_only() {
    let wasm = component_wasm("zerodha_kite");
    let (fills, _) = run_fetch_fills(
        &wasm,
        fixture_state(200, read_kite_fixture("trades_nfo_nrml_qty2.json")),
        empty_cursor(),
    )
    .expect("fetch_fills");

    let broker_fills: Vec<_> = fills.iter().map(fill_event_to_broker_fill).collect();
    let split = split_fills_by_book("zerodha_kite", broker_fills, None);
    let nfo = split
        .get(ZERODHA_NSE_NFO_BOOK_ID)
        .expect("nfo book bucket");
    let cash = split
        .get("zerodha-nse-bse-cash")
        .expect("cash book bucket");
    assert_eq!(nfo.len(), 2);
    assert_eq!(cash.len(), 1);
    assert!(cash[0].symbol == "RELIANCE");
    assert!(!cash.iter().any(|f| f.symbol.contains("NIFTY")));
}

#[test]
fn qty2_lot50_round_trip_uses_nfo_realized_owner() {
    let wasm = component_wasm("zerodha_kite");
    let (fills, _) = run_fetch_fills(
        &wasm,
        fixture_state(200, read_kite_fixture("trades_nfo_nrml_qty2.json")),
        empty_cursor(),
    )
    .expect("fetch_fills");

    let nfo_events: Vec<_> = fills
        .into_iter()
        .filter(|f| f.exchange_segment.as_deref() == Some("nse_fo"))
        .collect();
    let mut broker_fills: Vec<_> = nfo_events.iter().map(fill_event_to_broker_fill).collect();
    for fill in &mut broker_fills {
        fill.lot = Some(NFO_LOT);
    }

    let result = NfoRealizedPnlEngine::new().reconstruct(broker_fills);
    assert!(result.refused.is_empty());
    assert_eq!(result.round_trips.len(), 1);
    let rt = &result.round_trips[0];
    assert!((rt.qty - 2.0).abs() < 1e-9);
    assert_eq!(rt.lot, NFO_LOT);
    let pnl = rt.realized_pnl_inr.expect("golden pnl");
    assert!((pnl - 1000.0).abs() < 1e-6, "expected (110-100)*2*50");
}
