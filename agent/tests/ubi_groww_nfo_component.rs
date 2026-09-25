//! `groww` NFO book component contract (P6-W2 · `groww-nse-nfo`).

mod ubi_support;

use std::collections::HashMap;
use std::path::PathBuf;
use tradeautopsy_agent::{
    fill_event_to_broker_fill, run_fetch_fills, split_fills_by_book, NfoRealizedPnlEngine,
    AssetClass, BrokerHttpFixture, FillCursor, HostCredentialBlob, InstrumentClass,
    UbiHostConfig, UbiHostState, WitAssetClass, WitInstrumentClass,
};
use ubi_support::{assert_component_never_saw_secrets, component_wasm_from_crate};

const ORDER_LIST_PATH: &str = "/v1/order/list";
const GROWW_NSE_NFO_BOOK_ID: &str = "groww-nse-nfo";
const GROWW_NSE_BSE_CASH_BOOK_ID: &str = "groww-nse-bse-cash";
const NFO_BUY_MS: i64 = 1_784_959_200_000;
const NFO_SELL_MS: i64 = 1_784_961_000_000;
const NFO_LOT: i64 = 65;

fn groww_component_wasm() -> PathBuf {
    component_wasm_from_crate("ubi-groww-adapter", "ubi_groww_adapter.wasm")
}

fn read_groww_fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/groww")
        .join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

fn nfo_config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-groww-nfo-001".into(),
        broker_slug: "groww".into(),
        book_id: GROWW_NSE_NFO_BOOK_ID.into(),
        asset_class: AssetClass::Equity,
        instrument_class: InstrumentClass::Option,
        is_inverse: false,
        credentials: HostCredentialBlob::GrowwSession {
            token: "TEST_GROWW_TOKEN_NEVER_IN_COMPONENT".into(),
        },
    }
}

fn nfo_fixture_state() -> UbiHostState {
    let mut fixtures = HashMap::new();
    fixtures.insert(
        format!("{ORDER_LIST_PATH}?segment=CASH"),
        BrokerHttpFixture {
            status: 200,
            body: read_groww_fixture("order_list_cash_empty.json"),
            headers: vec![],
            error_class: None,
        },
    );
    fixtures.insert(
        format!("{ORDER_LIST_PATH}?segment=FNO"),
        BrokerHttpFixture {
            status: 200,
            body: read_groww_fixture("order_list_fno_page.json"),
            headers: vec![],
            error_class: None,
        },
    );
    fixtures.insert(
        "/v1/order/trades/GWKNFO01?segment=FNO".to_string(),
        BrokerHttpFixture {
            status: 200,
            body: read_groww_fixture("trades_fno_buy.json"),
            headers: vec![],
            error_class: None,
        },
    );
    fixtures.insert(
        "/v1/order/trades/GWKNFO02?segment=FNO".to_string(),
        BrokerHttpFixture {
            status: 200,
            body: read_groww_fixture("trades_fno_sell.json"),
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
fn fno_trades_map_to_nse_fo_fill_events() {
    let wasm = groww_component_wasm();
    let state = nfo_fixture_state();

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");
    let nfo: Vec<_> = fills
        .iter()
        .filter(|f| f.exchange_segment.as_deref() == Some("nse_fo"))
        .collect();
    assert_eq!(nfo.len(), 2);
    assert_eq!(nfo[0].side, "BUY");
    assert!((nfo[0].qty - 2.0).abs() < 1e-9);
    assert!((nfo[0].price - 100.0).abs() < 1e-9);
    assert_eq!(nfo[0].product.as_deref(), Some("NRML"));
    assert_eq!(nfo[0].filled_at_unix_ms, NFO_BUY_MS);
    assert_eq!(nfo[0].asset_class, WitAssetClass::Equity);
    assert_eq!(nfo[0].instrument_class, WitInstrumentClass::Option);

    assert_eq!(nfo[1].side, "SELL");
    assert!((nfo[1].price - 110.0).abs() < 1e-9);
    assert_eq!(nfo[1].filled_at_unix_ms, NFO_SELL_MS);
    assert_component_never_saw_secrets(&state, &wasm);
}

#[test]
fn split_puts_nfo_on_groww_nse_nfo_book_only() {
    let wasm = groww_component_wasm();
    let (fills, _) = run_fetch_fills(&wasm, nfo_fixture_state(), empty_cursor()).expect("fetch");
    let broker_fills: Vec<_> = fills.iter().map(fill_event_to_broker_fill).collect();
    let split = split_fills_by_book("groww", broker_fills, None);
    assert_eq!(
        split.get(GROWW_NSE_NFO_BOOK_ID).map(|v| v.len()),
        Some(2)
    );
    assert_eq!(
        split.get(GROWW_NSE_BSE_CASH_BOOK_ID).map(|v| v.len()),
        Some(0)
    );
}

#[test]
fn qty2_lot65_round_trip_uses_nfo_realized_owner() {
    let wasm = groww_component_wasm();
    let (fills, _) = run_fetch_fills(&wasm, nfo_fixture_state(), empty_cursor()).expect("fetch");
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
    let pnl = result.round_trips[0]
        .realized_pnl_inr
        .expect("pnl");
    assert!((pnl - 1300.0).abs() < 1e-6);
}
