//! `dhan` NFO book component contract (P6-W2 · `dhan-nse-nfo`).

mod ubi_support;

use std::collections::HashMap;
use std::path::PathBuf;
use tradeautopsy_agent::{
    fill_event_to_broker_fill, run_fetch_fills, split_fills_by_book, NfoRealizedPnlEngine,
    AssetClass, BrokerHttpFixture, FillCursor, InstrumentClass, UbiHostConfig, UbiHostState,
    WitAssetClass, WitInstrumentClass,
};
use ubi_support::{
    assert_component_never_saw_secrets, component_wasm_from_crate, sentinel_hmac,
};

const TRADES_PATH: &str = "/v2/trades";
const DHAN_NSE_NFO_BOOK_ID: &str = "dhan-nse-nfo";
const DHAN_NSE_BSE_CASH_BOOK_ID: &str = "dhan-nse-bse-cash";
const NFO_BUY_MS: i64 = 1_784_959_200_000;
const NFO_SELL_MS: i64 = 1_784_961_000_000;
const NFO_LOT: i64 = 65;

fn dhan_component_wasm() -> PathBuf {
    component_wasm_from_crate("ubi-dhan-adapter", "ubi_dhan_adapter.wasm")
}

fn read_dhan_fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/dhan")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn nfo_config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-dhan-nfo-001".into(),
        broker_slug: "dhan".into(),
        book_id: DHAN_NSE_NFO_BOOK_ID.into(),
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
fn nfo_margin_trades_map_to_nse_fo_fill_events() {
    let wasm = dhan_component_wasm();
    let state = fixture_state(200, read_dhan_fixture("trades_nfo_qty2.json"));

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    let nfo: Vec<_> = fills
        .iter()
        .filter(|f| f.exchange_segment.as_deref() == Some("nse_fo"))
        .collect();
    assert_eq!(nfo.len(), 2, "BUY+SELL MARGIN on NSE_FNO; cash row separate");
    assert_eq!(nfo[0].symbol, "NIFTY26JUL24000CE");
    assert_eq!(nfo[0].side, "BUY");
    assert!((nfo[0].qty - 2.0).abs() < 1e-9);
    assert!((nfo[0].price - 100.0).abs() < 1e-9);
    assert_eq!(nfo[0].currency, "INR");
    assert_eq!(nfo[0].product.as_deref(), Some("NRML"));
    assert_eq!(nfo[0].filled_at_unix_ms, NFO_BUY_MS);
    assert_eq!(nfo[0].broker_slug, "dhan");
    assert_eq!(nfo[0].connection_id, "conn-dhan-nfo-001");
    assert_eq!(nfo[0].asset_class, WitAssetClass::Equity);
    assert_eq!(nfo[0].instrument_class, WitInstrumentClass::Option);

    assert_eq!(nfo[1].side, "SELL");
    assert!((nfo[1].price - 110.0).abs() < 1e-9);
    assert_eq!(nfo[1].filled_at_unix_ms, NFO_SELL_MS);

    assert!(
        fills.iter().any(|f| f.exchange_segment.as_deref() == Some("NSE_EQ")),
        "cash row still present for host split"
    );
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn split_puts_nfo_on_dhan_nse_nfo_book_only() {
    let wasm = dhan_component_wasm();
    let (fills, _) = run_fetch_fills(
        &wasm,
        fixture_state(200, read_dhan_fixture("trades_nfo_qty2.json")),
        empty_cursor(),
    )
    .expect("fetch_fills");

    let broker_fills: Vec<_> = fills.iter().map(fill_event_to_broker_fill).collect();
    let split = split_fills_by_book("dhan", broker_fills.clone(), None);
    assert_eq!(
        split.get(DHAN_NSE_NFO_BOOK_ID).map(|v| v.len()),
        Some(2)
    );
    assert_eq!(
        split.get(DHAN_NSE_BSE_CASH_BOOK_ID).map(|v| v.len()),
        Some(1)
    );
}

#[test]
fn qty2_lot65_round_trip_uses_nfo_realized_owner() {
    let wasm = dhan_component_wasm();
    let (fills, _) = run_fetch_fills(
        &wasm,
        fixture_state(200, read_dhan_fixture("trades_nfo_qty2.json")),
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
    assert!(
        (pnl - 1300.0).abs() < 1e-6,
        "expected (110-100)*2*65 = 1300"
    );
}
