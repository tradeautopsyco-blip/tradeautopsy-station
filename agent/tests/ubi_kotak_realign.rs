//! W0.6 `kotak_neo` realign guards (2026-09-23 IST, docs+tests only).
//!
//! Locks the Station-says side of the Wave 0 audit in
//! `issues/brokers/sheets/kotak_neo.md` against OpenAlgo `broker/kotak` leads
//! (pin `ad3cd54df476b330c4e4b01a31a3ad53deb9012b`, citation only).
//! No oracle code is copied, vendored, or depended upon; oracle mapping
//! (`NSE`/`NFO`, DB symbols) is explicitly never adopted (audit C8).

mod ubi_support;

use std::collections::HashMap;
use tradeautopsy_agent::{
    run_fetch_fills, AssetClass, BrokerHttpFixture, FillCursor, InstrumentClass, UbiFillEvent,
    UbiHostConfig, UbiHostState, WitAssetClass, WitInstrumentClass, descriptor_for_book_id,
    stamp_fill_identity, KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID,
};
use ubi_support::{assert_component_never_saw_secrets, component_wasm, sentinel_kotak_session};

const TRADES_PATH: &str = "/quick/user/trades";
/// 25-07-2026 14:05:00 IST → 08:35:00 UTC (reuses the component-suite constant).
const RELIANCE_FILLED_AT_MS: i64 = 1_784_968_500_000;

fn config() -> UbiHostConfig {
    UbiHostConfig {
        connection_id: "conn-kotak-realign".into(),
        broker_slug: "kotak_neo".into(),
        book_id: KOTAK_NSE_BSE_CASH_BOOK_ID.into(),
        asset_class: AssetClass::Equity,
        instrument_class: InstrumentClass::Spot,
        is_inverse: false,
        credentials: sentinel_kotak_session(),
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
    UbiHostState::new(config(), fixtures)
}

fn empty_cursor() -> FillCursor {
    FillCursor {
        since_unix_ms: None,
        from_id: None,
        symbol: None,
    }
}

/// Audit C5/C6: cash refuses NRML/BO even though the oracle knows NRML
/// generically, and only `nse_fo` passes among FO segments (`bse_fo`/`mcx_fo`/
/// `cde_fo`/`bcs_fo` dropped per sheet F1/F2).
#[test]
fn cash_refuses_nrml_bo_and_non_nfo_fo_segments() {
    let wasm = component_wasm("kotak_neo");
    let body = r#"{
        "stat": "Ok", "stCode": 200, "data": [
            {"trdSym": "INFY-EQ", "trnsTp": "B", "fldQty": "10", "avgPrc": "1500.00",
             "exSeg": "nse_cm", "prod": "CNC", "exOrdId": "NSE1",
             "exTm": "25-07-2026 10:15:30"},
            {"trdSym": "HDFC-EQ", "trnsTp": "B", "fldQty": "1", "avgPrc": "900.00",
             "exSeg": "nse_cm", "prod": "NRML", "exOrdId": "NSE2",
             "exTm": "25-07-2026 10:16:30"},
            {"trdSym": "SBIN-EQ", "trnsTp": "B", "fldQty": "1", "avgPrc": "600.00",
             "exSeg": "nse_cm", "prod": "BO", "exOrdId": "NSE3",
             "exTm": "25-07-2026 10:17:30"},
            {"trdSym": "SENSEX25JUL80000CE", "trnsTp": "B", "fldQty": "10",
             "avgPrc": "50.00", "exSeg": "bse_fo", "prod": "MIS",
             "exOrdId": "BFO1", "exTm": "25-07-2026 10:18:30"},
            {"trdSym": "GOLD25JUL7000CE", "trnsTp": "B", "fldQty": "1",
             "avgPrc": "100.00", "exSeg": "mcx_fo", "prod": "MIS",
             "exOrdId": "MCX1", "exTm": "25-07-2026 10:19:30"},
            {"trdSym": "USDINR25JUL84CE", "trnsTp": "B", "fldQty": "1",
             "avgPrc": "0.50", "exSeg": "cde_fo", "prod": "MIS",
             "exOrdId": "CDS1", "exTm": "25-07-2026 10:20:30"},
            {"trdSym": "BTC25JUL90000CE", "trnsTp": "B", "fldQty": "1",
             "avgPrc": "10.00", "exSeg": "bcs_fo", "prod": "MIS",
             "exOrdId": "BCD1", "exTm": "25-07-2026 10:21:30"},
            {"trdSym": "NIFTY25JUL24000CE", "trnsTp": "B", "fldQty": "50",
             "avgPrc": "112.35", "exSeg": "nse_fo", "prod": "NRML",
             "exOrdId": "FO1", "exTm": "25-07-2026 11:00:00"}
        ]
    }"#;
    let state = fixture_state(200, body.to_string());

    let (fills, _state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(fills.len(), 2, "cash CNC control + nse_fo NRML only");
    assert!(fills.iter().any(|f| f.symbol == "INFY"));
    assert!(fills.iter().any(|f| f.symbol == "NIFTY25JUL24000CE"));
    assert!(!fills.iter().any(|f| f.symbol == "HDFC"));
    assert!(!fills.iter().any(|f| f.symbol == "SBIN"));
    for segment in ["bse_fo", "mcx_fo", "cde_fo", "bcs_fo"] {
        assert!(
            !fills
                .iter()
                .any(|f| f.exchange_segment.as_deref() == Some(segment)),
            "segment {segment} must not ingress"
        );
    }
}

/// Audit C7/C8: raw `exchange_segment`/`product` survive verbatim (never the
/// oracle's `NSE`/`NFO` rewrite), INR is stamped at insert on both books.
#[test]
fn preserves_raw_segment_product_and_inr() {
    let wasm = component_wasm("kotak_neo");
    let body = r#"{
        "stat": "Ok", "stCode": 200, "data": [
            {"trdSym": "RELIANCE-EQ", "trnsTp": "S", "fldQty": "5",
             "avgPrc": "1450.75", "exSeg": "bse_cm", "prod": "MIS",
             "exOrdId": "BSE1", "exTm": "25-07-2026 14:05:00"},
            {"trdSym": "NIFTY25JUL24000CE", "trnsTp": "B", "fldQty": "50",
             "avgPrc": "112.35", "exSeg": "nse_fo", "prod": "NRML",
             "exOrdId": "FO1", "exTm": "25-07-2026 11:00:00"}
        ]
    }"#;
    let state = fixture_state(200, body.to_string());

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(fills.len(), 2);
    let cash = fills
        .iter()
        .find(|f| f.exchange_segment.as_deref() == Some("bse_cm"))
        .expect("bse_cm row");
    assert_eq!(cash.product.as_deref(), Some("MIS"));
    assert_eq!(cash.currency, "INR");
    assert_eq!(cash.symbol, "RELIANCE");
    let nfo = fills
        .iter()
        .find(|f| f.exchange_segment.as_deref() == Some("nse_fo"))
        .expect("nse_fo row");
    assert_eq!(nfo.product.as_deref(), Some("NRML"));
    assert_eq!(nfo.currency, "INR");
    assert_eq!(nfo.symbol, "NIFTY25JUL24000CE");
    // Oracle-mapped forms must never appear on Station fills.
    for fill in &fills {
        assert_ne!(fill.exchange_segment.as_deref(), Some("NSE"));
        assert_ne!(fill.exchange_segment.as_deref(), Some("BSE"));
        assert_ne!(fill.exchange_segment.as_deref(), Some("NFO"));
    }
    assert_component_never_saw_secrets(&state, &wasm);
}

/// Audit C12 (F3), FLIPPED by P1 (ADR 0004 §5): the host stamps every fill from
/// the connection's BOOK row, never from the slug's first pair.
///
/// OLD (pre-P1, `host_stamps_equities_on_nfo_row_f3`): this was a component
/// round-trip on a *cash-book* connection asserting the NFO row inherited the
/// slug's cash pair — asset `equities`, no instrument axis. That inheritance was
/// the F3 second-book STOP: an NFO fill wore cash clothes.
///
/// NEW: an NFO-book connection stamps NFO axes — `(equity, option, false)`.
/// Wasm-free by design (the stamp is a pure host function): it resolves the axes
/// from the `kotak-nse-nfo` catalog row and proves adapter-supplied identity is
/// overwritten. The component round-trip variant stays covered by the cash-config
/// tests above once the 0.3.0 adapters land (PENDING-ADAPTERS).
#[test]
fn host_stamps_nfo_axes_on_nfo_book_row_f3_flipped() {
    let descriptor =
        descriptor_for_book_id(KOTAK_NSE_NFO_BOOK_ID).expect("kotak-nse-nfo catalog row");
    assert_eq!(descriptor.slug, "kotak_neo");
    assert_eq!(descriptor.asset_class, AssetClass::Equity);
    assert_eq!(descriptor.instrument_class, InstrumentClass::Option);
    assert!(!descriptor.is_inverse);

    let config = UbiHostConfig {
        connection_id: "conn-kotak-nfo-f3".into(),
        broker_slug: "kotak_neo".into(),
        book_id: KOTAK_NSE_NFO_BOOK_ID.into(),
        asset_class: descriptor.asset_class,
        instrument_class: descriptor.instrument_class,
        is_inverse: descriptor.is_inverse,
        credentials: sentinel_kotak_session(),
    };
    // Adapter-side spoof: wrong identity on every field the host owns.
    let spoofed = UbiFillEvent {
        fill_id: "FO1".into(),
        broker_slug: "spoof".into(),
        connection_id: "spoof".into(),
        asset_class: WitAssetClass::Fx,
        instrument_class: WitInstrumentClass::Swap,
        is_inverse: true,
        symbol: "NIFTY25JUL24000CE".into(),
        side: "BUY".into(),
        qty: 50.0,
        price: 112.35,
        currency: "INR".into(),
        filled_at_unix_ms: 1_784_957_400_000,
        fee_amount: None,
        fee_currency: None,
        exchange_segment: Some("nse_fo".into()),
        product: Some("NRML".into()),
        trade_id: Some("FO1".into()),
    };

    let stamped = stamp_fill_identity(&config, spoofed);

    assert_eq!(stamped.asset_class, WitAssetClass::Equity);
    assert_eq!(stamped.instrument_class, WitInstrumentClass::Option);
    assert!(
        !stamped.is_inverse,
        "NFO is linearly margined; Coin-M alone sets is_inverse"
    );
    assert_eq!(stamped.connection_id, "conn-kotak-nfo-f3");
    assert_eq!(stamped.broker_slug, "kotak_neo");
    // Venue truth survives the stamp untouched.
    assert_eq!(stamped.exchange_segment.as_deref(), Some("nse_fo"));
    assert_eq!(stamped.symbol, "NIFTY25JUL24000CE");
}

/// Audit C3: the day book is one `GET /quick/user/trades` with no date-range
/// params — the cursor filters in memory and never widens the window.
#[test]
fn trade_book_sends_no_date_range_query() {
    let wasm = component_wasm("kotak_neo");
    let state = fixture_state(200, r#"{"stat":"Ok","stCode":200,"data":[]}"#.to_string());

    let (fills, state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert!(fills.is_empty());
    assert_eq!(state.calls.len(), 1);
    let call = &state.calls[0];
    assert_eq!(call.method, "GET");
    assert_eq!(call.path, TRADES_PATH);
    assert!(
        call.query.is_empty(),
        "trade book takes no range params: {:?}",
        call.query
            .iter()
            .map(|q| (&q.name, &q.value))
            .collect::<Vec<_>>()
    );
}

/// Audit C4: Station `qty`/`trdPrc`/`sym` aliases + `flDt`/`flTm` time +
/// case-insensitive segment/product match; values preserved verbatim.
#[test]
fn accepts_qty_trdprc_aliases_and_fldate_time() {
    let wasm = component_wasm("kotak_neo");
    let body = r#"{
        "stat": "Ok", "stCode": 200, "data": [
            {"sym": "TCS-EQ", "trnsTp": "s", "qty": "2", "trdPrc": "3900.00",
             "exSeg": "NSE_CM", "prod": "cnc", "nOrdNo": "2401010009",
             "flDt": "25-07-2026", "flTm": "14:05:00"}
        ]
    }"#;
    let state = fixture_state(200, body.to_string());

    let (fills, _state) = run_fetch_fills(&wasm, state, empty_cursor()).expect("fetch_fills");

    assert_eq!(fills.len(), 1);
    let fill = &fills[0];
    assert_eq!(fill.symbol, "TCS");
    assert_eq!(fill.side, "SELL");
    assert!((fill.qty - 2.0).abs() < 1e-9);
    assert!((fill.price - 3900.00).abs() < 1e-9);
    assert_eq!(fill.exchange_segment.as_deref(), Some("NSE_CM"));
    assert_eq!(fill.product.as_deref(), Some("cnc"));
    assert_eq!(fill.currency, "INR");
    assert_eq!(fill.trade_id.as_deref(), Some("2401010009"));
    assert_eq!(fill.filled_at_unix_ms, RELIANCE_FILLED_AT_MS);
}
