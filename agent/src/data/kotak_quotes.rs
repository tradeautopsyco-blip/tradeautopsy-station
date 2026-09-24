//! Slice C: Kotak Neo REST quotes → TickBook (`adapter_id: kotak_neo`).
//!
//! Path: `GET …/script-details/1.0/quotes/neosymbol/{neo_symbols}/{quote_type}`
//! (`PROD_URL["quotes_neo_symbol"]` / `quotes_neo_symbol_api.py`). PrivateRead session
//! attach — not `AuthMode::Public`, not HMAC. Do not call `quotes_neo_symbol_napi`.
//!
//! HSM `wss://mlhsm.kotaksecurities.com` is skipped: WEBSOCKET.md — `mlhsm` is not on
//! the HTTP allowlist; `hsServerId` attach on that socket is unspecified. REST
//! poll/on-select is enough. Do not subscribe `isDepth=true` until an ordered-state
//! replica exists. Slice D stores REST `quote_type=depth` as a bounded snapshot.

use super::descriptor::{KOTAK_NEO_ADAPTER_ID, KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID};
use super::source_manifest::shipping_book_id_for_slug;
use super::tick::{QuoteTick, SessionOhlc, Transport};
use chrono::{DateTime, Utc};
use serde_json::Value;

pub const QUOTES_NEOSYMBOL_MARK: &str = "/script-details/1.0/quotes/neosymbol/";
pub const QUOTE_TYPE_ALL: &str = "all";
#[cfg(test)]
pub const QUOTE_TYPE_LTP: &str = "ltp";
#[cfg(test)]
pub const QUOTE_TYPE_OHLC: &str = "ohlc";
pub const QUOTE_TYPE_DEPTH: &str = "depth";
pub const QUOTE_TYPE_OI: &str = "oi";

pub const DOCUMENTED_QUOTE_TYPES: &[&str] = &[
    "all",
    "depth",
    "ohlc",
    "ltp",
    "oi",
    "52w",
    "circuit_limits",
    "scrip_details",
];

/// `{segment}|{token}` e.g. `nse_cm|2885` (REST.md / Quotes.md `neo_symbols`).
pub fn kotak_instrument_id(segment: &str, token: &str) -> Option<String> {
    let segment = segment.trim().to_ascii_lowercase();
    let token = token.trim();
    if token.is_empty() || !is_cash_segment(&segment) {
        return None;
    }
    Some(format!("{segment}|{token}"))
}

pub fn is_cash_segment(segment: &str) -> bool {
    matches!(segment, "nse_cm" | "bse_cm")
}

/// Named NFO book only. Not `bse_fo` / `cde_fo` / `mcx_fo`.
pub fn is_nfo_segment(segment: &str) -> bool {
    segment == "nse_fo"
}

/// Named CDS book (`kotak-nse-cds`) only.
pub fn is_cds_segment(segment: &str) -> bool {
    segment.trim().eq_ignore_ascii_case("cde_fo")
}

pub fn is_mcx_segment(segment: &str) -> bool {
    segment.trim().eq_ignore_ascii_case("mcx_fo")
}

/// `{segment}|{token}` for `nse_fo` only.
pub fn kotak_nfo_instrument_id(segment: &str, token: &str) -> Option<String> {
    let segment = segment.trim().to_ascii_lowercase();
    let token = token.trim();
    if token.is_empty() || !is_nfo_segment(&segment) {
        return None;
    }
    Some(format!("{segment}|{token}"))
}

/// TickBook slot for a Kotak `neo_symbols` id. Unknown / other FO → `None`.
pub fn kotak_quote_book_id(instrument_id: &str) -> Option<&'static str> {
    let segment = instrument_id.split('|').next()?.trim().to_ascii_lowercase();
    match segment.as_str() {
        "nse_cm" | "bse_cm" => Some(KOTAK_NSE_BSE_CASH_BOOK_ID),
        "nse_fo" => Some(KOTAK_NSE_NFO_BOOK_ID),
        "cde_fo" => Some(crate::data::KOTAK_NSE_CDS_BOOK_ID),
        "mcx_fo" => Some(crate::data::KOTAK_MCX_FUTURE_BOOK_ID),
        _ => None,
    }
}

/// Well-formed `nse_fo|<token>` (numeric token). Cash and other FO segments → `None`.
pub fn parse_nfo_instrument_id(raw: &str) -> Option<String> {
    let decoded = raw.trim().replace("%7C", "|").replace("%7c", "|");
    let (segment, token) = decoded.split_once('|')?;
    let segment = segment.trim().to_ascii_lowercase();
    if !is_nfo_segment(&segment) {
        return None;
    }
    let token = token.trim();
    if token.is_empty() || token.parse::<i64>().is_err() {
        return None;
    }
    Some(format!("{segment}|{token}"))
}

/// SDK `urllib.parse.quote` of `{exchange_segment}|{instrument_token}`.
pub fn encode_neo_symbol(neo_symbol: &str) -> String {
    let mut out = String::with_capacity(neo_symbol.len());
    for byte in neo_symbol.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

pub fn quotes_neosymbol_path(neo_symbol: &str, quote_type: &str) -> String {
    format!(
        "{QUOTES_NEOSYMBOL_MARK}{}/{quote_type}",
        encode_neo_symbol(neo_symbol)
    )
}

/// Documented GET suffix + `quote_type` token. Does not match napi `apim/quotes/1.0/…`.
pub fn is_kotak_quotes_neosymbol_path(path: &str) -> bool {
    kotak_quotes_neosymbol_quote_type(path).is_some()
}

pub fn kotak_quotes_neosymbol_quote_type(path: &str) -> Option<&str> {
    let path = path.trim();
    let idx = path.find(QUOTES_NEOSYMBOL_MARK)?;
    let rest = &path[idx + QUOTES_NEOSYMBOL_MARK.len()..];
    let (symbols, quote_type) = rest.rsplit_once('/')?;
    if symbols.is_empty() {
        return None;
    }
    DOCUMENTED_QUOTE_TYPES
        .iter()
        .copied()
        .find(|token| quote_type.eq_ignore_ascii_case(token))
}

pub fn is_kotak_depth_path(path: &str) -> bool {
    is_kotak_quotes_neosymbol_path(path)
        && kotak_quotes_neosymbol_quote_type(path) == Some(QUOTE_TYPE_DEPTH)
}

pub fn is_kotak_latest_quote_path(path: &str) -> bool {
    is_kotak_quotes_neosymbol_path(path)
        && kotak_quotes_neosymbol_quote_type(path).is_some_and(|token| token != QUOTE_TYPE_DEPTH)
}

/// Map a Kotak REST quotes JSON body to the first cash LTP tick.
///
/// Envelope: v1 Quotes.md sample is `{ "message": [ { last_traded_price, instrument_token,
/// exchange_segment, ohlc } ] }`. Live GET (founder 2026-08-27, `nse_cm|11536`) is a
/// **root JSON array**. OpenAlgo `broker/kotak/api/data.py` on the same path reads
/// `response[0].ltp`, `.exchange`, `.exchange_token`, `.ohlc`, `.depth` (REST.md).
/// WEBSOCKET.md `ltp` / `op`/`h`/`lo`/`c` remain valid on an object. `quote_type=depth`
/// / `oi` without LTP or session close → `None` (depth lives in [`super::kotak_depth`]).
pub fn quote_tick_from_kotak_json(raw: &str, received_at: DateTime<Utc>) -> Option<QuoteTick> {
    quote_ticks_from_kotak_json(raw, received_at)
        .into_iter()
        .next()
}

pub fn quote_ticks_from_kotak_json(raw: &str, received_at: DateTime<Utc>) -> Vec<QuoteTick> {
    quote_ticks_from_kotak_json_for_book(raw, received_at, KOTAK_NSE_BSE_CASH_BOOK_ID)
}

/// Map Kotak REST quotes JSON to LTP ticks for a named book.
///
/// `kotak-nse-bse-cash`: `nse_cm` / `bse_cm` only. `kotak-nse-nfo`: `nse_fo` only
/// (`adapter_id` stays `kotak_neo`). Unknown book → empty.
pub fn quote_tick_from_kotak_json_for_book(
    raw: &str,
    received_at: DateTime<Utc>,
    book_id: &str,
) -> Option<QuoteTick> {
    quote_ticks_from_kotak_json_for_book(raw, received_at, book_id)
        .into_iter()
        .next()
}

pub fn quote_ticks_from_kotak_json_for_book(
    raw: &str,
    received_at: DateTime<Utc>,
    book_id: &str,
) -> Vec<QuoteTick> {
    let slot = book_id.trim();
    if slot != KOTAK_NSE_BSE_CASH_BOOK_ID && slot != KOTAK_NSE_NFO_BOOK_ID {
        return Vec::new();
    }
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    quote_objects(&value)
        .into_iter()
        .filter_map(|obj| tick_from_object_for_book(obj, received_at, slot))
        .collect()
}

/// Fold cash LTP + `last_traded_quantity` into the forming 15min bar.
/// NFO is not ticked. A quote without ltq updates OHLC only.
pub fn tick_cash_builders_from_kotak_json(
    builders: &mut super::candle_builder::CandleBuilders,
    raw: &str,
    received_at: DateTime<Utc>,
) {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return;
    };
    for obj in quote_objects(&value) {
        let Some(tick) = tick_from_object_for_book(obj, received_at, KOTAK_NSE_BSE_CASH_BOOK_ID)
        else {
            continue;
        };
        let qty = obj.get("last_traded_quantity").and_then(json_string);
        builders.tick(
            KOTAK_NEO_ADAPTER_ID,
            &tick.instrument_id,
            "15min",
            tick.as_of.timestamp_millis(),
            &tick.last,
            qty.as_deref(),
        );
    }
}

pub(crate) fn quote_objects(value: &Value) -> Vec<&Value> {
    if let Some(arr) = value.as_array() {
        return arr.iter().collect();
    }
    if let Some(arr) = value.get("message").and_then(Value::as_array) {
        return arr.iter().collect();
    }
    if let Some(data) = value.get("data") {
        return quote_objects(data);
    }
    if value.is_object() {
        return vec![value];
    }
    Vec::new()
}

fn quote_segment_and_token(value: &Value) -> Option<(String, String)> {
    let segment = json_string(
        value
            .get("exchange_segment")
            .or_else(|| value.get("pExchSeg"))
            .or_else(|| value.get("exchSeg"))
            .or_else(|| value.get("exchange"))?,
    )?;
    let token = json_string(
        value
            .get("instrument_token")
            .or_else(|| value.get("pToken"))
            .or_else(|| value.get("token"))
            .or_else(|| value.get("exchange_token"))?,
    )?;
    Some((segment, token))
}

/// Depth row identity + DepthBook slot. Cash → shipping cash book; `nse_fo` →
/// named NFO book (`kotak-nse-nfo`).
pub(crate) fn depth_book_and_instrument_from_quote_object(
    value: &Value,
) -> Option<(String, String)> {
    let (segment, token) = quote_segment_and_token(value)?;
    let segment = segment.trim().to_ascii_lowercase();
    if is_nfo_segment(&segment) {
        let instrument_id = kotak_nfo_instrument_id(&segment, &token)?;
        return Some((instrument_id, KOTAK_NSE_NFO_BOOK_ID.to_string()));
    }
    let instrument_id = kotak_instrument_id(&segment, &token)?;
    let book_id = shipping_book_id_for_slug(KOTAK_NEO_ADAPTER_ID)
        .unwrap_or_else(|| KOTAK_NSE_BSE_CASH_BOOK_ID.to_string());
    Some((instrument_id, book_id))
}

fn instrument_id_from_quote_object_for_book(value: &Value, book_id: &str) -> Option<String> {
    let (segment, token) = quote_segment_and_token(value)?;
    match book_id {
        KOTAK_NSE_BSE_CASH_BOOK_ID => kotak_instrument_id(&segment, &token),
        KOTAK_NSE_NFO_BOOK_ID => kotak_nfo_instrument_id(&segment, &token),
        _ => None,
    }
}

/// NFO last is **`ltp` and nothing else**.
///
/// Founder probe 2026-08-31 IST, `GET …/quotes/neosymbol/nse_fo%7C56526/{all,ltp}`
/// HTTP 200: the live FO row names `ltp` (JSON string) and does **not** carry
/// `last_traded_price`, `last`, `lastPrice`, or `LTP`. Those four were a guess,
/// and this is the fetch that retired them — see `locks/kotak-nse-nfo.md` LTP
/// field table and REST.md's FO observation.
///
/// **No `ohlc.close` fallback on this book**, deliberately:
/// - The lock names last as `ltp`, not session close.
/// - `quote_type=ltp` returns four keys and no `ohlc` at all, so a close
///   fallback could only ever invent a price on that slice.
/// - `quote_type=all` carries `ltp` *and* `ohlc` together, so a row missing
///   `ltp` is unusable — it is not a licence to publish the close.
/// - Close is a different quantity. Painting it as last is the same class of
///   lie as the or-chain this function replaced.
///
/// `last_traded_quantity` / `last_volume` are also present on the live body and
/// are **quantity and volume, not price** — they are not consulted here.
fn nfo_last_from_object(value: &Value) -> Option<String> {
    json_string(value.get("ltp")?)
}

/// Cash keeps the or-chain and the session-close fallback: a different book, a
/// different observation (`nse_cm|11536`, 2026-08-27). Nothing in the FO probe
/// licenses narrowing this one.
fn cash_last_from_object(value: &Value, session_ohlc: Option<&SessionOhlc>) -> Option<String> {
    json_string(
        value
            .get("last_traded_price")
            .or_else(|| value.get("ltp"))
            .or_else(|| value.get("last"))
            .or_else(|| value.get("lastPrice"))
            .or_else(|| value.get("LTP"))?,
    )
    .or_else(|| session_ohlc.map(|bar| bar.close.clone()))
}

/// One NFO open-interest reading, off the **same `quote_type=all` body** that
/// feeds TickBook last. Kept beside the tick, never inside it: OI is not a price
/// and must never reach `QuoteTick::last`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NfoOpenInterest {
    /// `nse_fo|{token}` — the same identity as last.
    pub instrument_id: String,
    /// The venue's own `open_int` string. Station copies; it does not rescale.
    pub open_interest: String,
}

/// NFO open interest is **`open_int` on `quote_type=all`**, and nothing else.
///
/// Founder probe 2026-08-31 IST: `…/nse_fo%7C56526/all` HTTP 200 names
/// `open_int` (JSON string, parses as f64). The separate `quote_type=oi` slice
/// names `oi_las` / `oi_high` / `oi_low` and carries **no** `open_int` — a second
/// slice with a different spelling is not a second observation of the same
/// number until a page says so, so those three stay recorded-but-unbound
/// (`oi_las` is verbatim and truncated; it is **not** `oi_last`).
///
/// This reads the body Station already fetches for last. It does **not** dial a
/// second GET, and it never reads FO master `dOpenInterest ` (a daily CSV cell,
/// not a quotes snapshot).
pub fn nfo_open_interest_from_kotak_json(raw: &str) -> Vec<NfoOpenInterest> {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    quote_objects(&value)
        .into_iter()
        .filter_map(nfo_open_interest_from_object)
        .collect()
}

fn nfo_open_interest_from_object(value: &Value) -> Option<NfoOpenInterest> {
    let (segment, token) = quote_segment_and_token(value)?;
    let instrument_id = kotak_nfo_instrument_id(&segment, &token)?;
    // `open_int` only. Not `oi_las`, not `oi_high`, not `oi_low`, not `openInterest`.
    let open_interest = json_string(value.get("open_int")?)?;
    // A non-numeric or non-positive cell is unusable, not a zero to publish.
    let parsed: f64 = open_interest.parse().ok()?;
    if !parsed.is_finite() || parsed < 0.0 {
        return None;
    }
    Some(NfoOpenInterest {
        instrument_id,
        open_interest,
    })
}

/// Session OI band from the separate `quote_type=oi` slice. Kept beside
/// [`NfoOpenInterest`], never merged into `open_interest`: the venue names
/// different keys on `all` vs `oi`, and live compare shows they can disagree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NfoOiSessionSlice {
    /// `nse_fo|{token}` — the same identity as last and `open_int`.
    pub instrument_id: String,
    /// Verbatim `oi_las` from the `oi` slice — truncated spelling, not `oi_last`.
    pub oi_session_las: String,
    /// Verbatim `oi_high` from the `oi` slice.
    pub oi_session_high: String,
    /// Verbatim `oi_low` from the `oi` slice.
    pub oi_session_low: String,
}

/// Parse every `quote_type=oi` row. Reads `oi_las` / `oi_high` / `oi_low`
/// verbatim; does not consult `open_int` (that lives on `quote_type=all` only).
pub fn nfo_oi_session_from_kotak_json(raw: &str) -> Vec<NfoOiSessionSlice> {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    quote_objects(&value)
        .into_iter()
        .filter_map(nfo_oi_session_from_object)
        .collect()
}

fn nfo_oi_session_from_object(value: &Value) -> Option<NfoOiSessionSlice> {
    let (segment, token) = quote_segment_and_token(value)?;
    let instrument_id = kotak_nfo_instrument_id(&segment, &token)?;
    let oi_session_las = json_string(value.get("oi_las")?)?;
    let oi_session_high = json_string(value.get("oi_high")?)?;
    let oi_session_low = json_string(value.get("oi_low")?)?;
    Some(NfoOiSessionSlice {
        instrument_id,
        oi_session_las,
        oi_session_high,
        oi_session_low,
    })
}

fn tick_from_object_for_book(
    value: &Value,
    received_at: DateTime<Utc>,
    book_id: &str,
) -> Option<QuoteTick> {
    let instrument_id = instrument_id_from_quote_object_for_book(value, book_id)?;
    // The session bar still rides the envelope on both books. On NFO it may not
    // become `last` — see below.
    let session_ohlc = session_ohlc_from_object(value);
    let last = if book_id == KOTAK_NSE_NFO_BOOK_ID {
        nfo_last_from_object(value)?
    } else {
        cash_last_from_object(value, session_ohlc.as_ref())?
    };
    let last_n: f64 = last.parse().ok()?;
    if last_n <= 0.0 {
        return None;
    }
    let book_id = if book_id == KOTAK_NSE_NFO_BOOK_ID {
        KOTAK_NSE_NFO_BOOK_ID.to_string()
    } else {
        shipping_book_id_for_slug(KOTAK_NEO_ADAPTER_ID)
            .unwrap_or_else(|| KOTAK_NEO_ADAPTER_ID.to_string())
    };
    Some(QuoteTick {
        instrument_id,
        last,
        as_of: received_at,
        received_at,
        age_unknown: true,
        transport: Transport::Rest,
        adapter_id: KOTAK_NEO_ADAPTER_ID.to_string(),
        book_id,
        session_ohlc,
    })
}

fn session_ohlc_from_object(value: &Value) -> Option<SessionOhlc> {
    if let Some(ohlc) = value.get("ohlc") {
        return Some(SessionOhlc {
            open: json_string(ohlc.get("open")?)?,
            high: json_string(ohlc.get("high")?)?,
            low: json_string(ohlc.get("low")?)?,
            close: json_string(ohlc.get("close")?)?,
        });
    }
    // WEBSOCKET.md live stock fields — session bar, not historical_series.
    let open = json_string(value.get("op")?)?;
    let high = json_string(value.get("h")?)?;
    let low = json_string(value.get("lo")?)?;
    let close = json_string(value.get("c")?)?;
    Some(SessionOhlc {
        open,
        high,
        low,
        close,
    })
}

pub(crate) fn json_string(value: &Value) -> Option<String> {
    match value {
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::apply::apply_quote;
    use crate::data::descriptor::kotak_neo_quote_descriptor;
    use crate::data::registry::Registry;
    use crate::data::tickbook::TickBook;
    use chrono::TimeZone;

    fn received() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 26, 12, 0, 0).unwrap()
    }

    fn desk_registry() -> Registry {
        Registry::load(&[kotak_neo_quote_descriptor()]).expect("kotak_neo quote loads")
    }

    const FIXTURE: &str = r#"{
        "message": [{
            "last_traded_price": "1400.50",
            "instrument_token": "2885",
            "exchange_segment": "nse_cm",
            "trading_symbol": "RELIANCE-EQ",
            "ohlc": {"open": "1390.00", "high": "1410.25", "low": "1385.10", "close": "1400.50"}
        }]
    }"#;

    #[test]
    fn fixture_quote_applies_to_tickbook_as_kotak_neo() {
        let tick = quote_tick_from_kotak_json(FIXTURE, received()).expect("fixture tick");
        assert_eq!(tick.instrument_id, "nse_cm|2885");
        assert_eq!(tick.adapter_id, KOTAK_NEO_ADAPTER_ID);
        assert_eq!(tick.book_id, "kotak-nse-bse-cash");
        assert_eq!(tick.transport, Transport::Rest);
        let last: f64 = tick.last.parse().unwrap();
        assert!(last > 0.0);
        assert_eq!(tick.last, "1400.50");
        let ohlc = tick
            .session_ohlc
            .as_ref()
            .expect("session ohlc on envelope");
        assert_eq!(ohlc.open, "1390.00");
        assert_eq!(ohlc.high, "1410.25");
        assert_eq!(ohlc.low, "1385.10");
        assert_eq!(ohlc.close, "1400.50");

        let registry = desk_registry();
        let mut book = TickBook::new();
        apply_quote(&registry, &mut book, tick).unwrap();
        let row = book
            .get("kotak-nse-bse-cash", "nse_cm|2885")
            .expect("stored");
        let stored_last: f64 = row.last.parse().unwrap();
        assert!(stored_last > 0.0);
        assert_eq!(row.adapter_id, KOTAK_NEO_ADAPTER_ID);
        assert_ne!(row.adapter_id, "binance_com");
        assert_ne!(row.adapter_id, "kotak_public");
    }

    #[test]
    fn fo_and_depth_without_ltp_are_ignored() {
        let fo_v1 = r#"{"instrument_token":"12345","exchange_segment":"nse_fo","last_traded_price":"10.00"}"#;
        assert!(quote_tick_from_kotak_json(fo_v1, received()).is_none());
        let fo_v2 = r#"{"exchange_token":"12345","exchange":"nse_fo","ltp":"10.00"}"#;
        assert!(quote_tick_from_kotak_json(fo_v2, received()).is_none());
        let depth = r#"{"instrument_token":"2885","exchange_segment":"nse_cm","depth":{"buy":[],"sell":[]}}"#;
        assert!(quote_tick_from_kotak_json(depth, received()).is_none());
    }

    #[test]
    fn ltp_only_and_ws_field_names_parse() {
        let ltp = r#"{"instrument_token":2885,"exchange_segment":"nse_cm","ltp":"99.50"}"#;
        let tick = quote_tick_from_kotak_json(ltp, received()).unwrap();
        assert_eq!(tick.last, "99.50");
        assert!(tick.session_ohlc.is_none());
        let ws = r#"{"instrument_token":"2885","exchange_segment":"nse_cm","ltp":"100.00","op":"98","h":"101","lo":"97","c":"99"}"#;
        let bar = quote_tick_from_kotak_json(ws, received()).unwrap();
        assert_eq!(bar.session_ohlc.as_ref().unwrap().high, "101");
    }

    #[test]
    fn neo_symbol_path_encodes_pipe_and_keeps_quote_type() {
        assert_eq!(
            quotes_neosymbol_path("nse_cm|2885", QUOTE_TYPE_LTP),
            "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/ltp"
        );
        assert!(is_kotak_quotes_neosymbol_path(
            "/trading/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/all"
        ));
        assert!(!is_kotak_quotes_neosymbol_path(
            "apim/quotes/1.0/quotes/neosymbol/nse_cm%7C2885/ltp"
        ));
        assert!(!is_kotak_quotes_neosymbol_path("/quick/quotes"));
        assert!(is_kotak_depth_path(
            "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/depth"
        ));
        assert!(is_kotak_latest_quote_path(
            "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/ltp"
        ));
        assert!(!is_kotak_depth_path(
            "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/ltp"
        ));
        assert!(!is_kotak_quotes_neosymbol_path(
            "/script-details/1.0/quotes/neosymbol/nse_cm%7C2885/market_depth"
        ));
    }

    #[test]
    fn live_root_array_uses_exchange_and_exchange_token() {
        let json = include_str!("../../fixtures/kotak/quotes_neosymbol_live_array.json");
        let tick = quote_tick_from_kotak_json(json, received()).expect("live array tick");
        assert_eq!(tick.instrument_id, "nse_cm|11536");
        assert_eq!(tick.last, "3224.50");
        assert_eq!(tick.session_ohlc.as_ref().unwrap().close, "3210.00");
        assert_eq!(tick.adapter_id, KOTAK_NEO_ADAPTER_ID);
        assert_eq!(tick.book_id, "kotak-nse-bse-cash");
    }

    /// The C11 regression. Founder probe 2026-08-31 named `ltp` on the live FO
    /// body and proved the other four names absent; each of these must go red if
    /// anyone restores the or-chain or the close fallback on this book.
    /// O1: OI is `open_int` on `quote_type=all`, and only that. The `oi` slice's
    /// `oi_las` / `oi_high` / `oi_low` stay recorded-but-unbound — a second slice
    /// with a different spelling is not a second observation of the same number.
    #[test]
    fn nfo_open_interest_is_open_int_from_all_only() {
        let all =
            r#"[{"exchange":"nse_fo","exchange_token":"56526","ltp":"10.00","open_int":"480750"}]"#;
        let rows = nfo_open_interest_from_kotak_json(all);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].instrument_id, "nse_fo|56526");
        // The venue's own string, copied — not reparsed and reformatted.
        assert_eq!(rows[0].open_interest, "480750");

        // The `oi` slice, verbatim as observed: no `open_int`, so no reading.
        let oi_slice = r#"[{"exchange":"nse_fo","exchange_token":"56526","oi_las":"480750","oi_high":"500000","oi_low":"400000"}]"#;
        assert!(
            nfo_open_interest_from_kotak_json(oi_slice).is_empty(),
            "oi_las/oi_high/oi_low are unbound — only open_int on `all` may light OI"
        );
        // Not even under the tidied-up spelling nobody observed.
        for unobserved in [
            "oi_last",
            "openInterest",
            "open_interest",
            "oi",
            "dOpenInterest",
        ] {
            let body = format!(
                r#"[{{"exchange":"nse_fo","exchange_token":"56526","{unobserved}":"480750"}}]"#
            );
            assert!(
                nfo_open_interest_from_kotak_json(&body).is_empty(),
                "{unobserved} is not the observed field name"
            );
        }
    }

    /// OI is not last, and last is not OI. The two never cross.
    #[test]
    fn open_interest_never_becomes_last_and_last_never_becomes_oi() {
        // `open_int` alone, no `ltp`: an OI reading, but no tick.
        let oi_only = r#"[{"exchange":"nse_fo","exchange_token":"56526","open_int":"480750"}]"#;
        assert_eq!(nfo_open_interest_from_kotak_json(oi_only).len(), 1);
        assert!(
            quote_tick_from_kotak_json_for_book(oi_only, received(), KOTAK_NSE_NFO_BOOK_ID)
                .is_none(),
            "open interest must never be published as last"
        );

        // `ltp` alone, no `open_int`: a tick, but no OI reading.
        let last_only = r#"[{"exchange":"nse_fo","exchange_token":"56526","ltp":"10.00"}]"#;
        let tick =
            quote_tick_from_kotak_json_for_book(last_only, received(), KOTAK_NSE_NFO_BOOK_ID)
                .expect("nfo tick");
        assert_eq!(tick.last, "10.00");
        assert!(nfo_open_interest_from_kotak_json(last_only).is_empty());
    }

    /// Cash rows never produce an NFO OI reading, even carrying `open_int`.
    #[test]
    fn cash_rows_never_produce_an_nfo_open_interest_reading() {
        let cash =
            r#"[{"exchange":"nse_cm","exchange_token":"11536","ltp":"3224.00","open_int":"1"}]"#;
        assert!(nfo_open_interest_from_kotak_json(cash).is_empty());
        for other in ["bse_fo", "cde_fo", "mcx_fo", "bse_cm"] {
            let body = format!(r#"[{{"exchange":"{other}","exchange_token":"1","open_int":"1"}}]"#);
            assert!(
                nfo_open_interest_from_kotak_json(&body).is_empty(),
                "{other}"
            );
        }
    }

    /// An unusable cell is no reading — never a zero to publish.
    #[test]
    fn unusable_open_int_cells_yield_no_reading() {
        for bad in ["", "   ", "abc", "-1", "null"] {
            let body =
                format!(r#"[{{"exchange":"nse_fo","exchange_token":"56526","open_int":"{bad}"}}]"#);
            assert!(
                nfo_open_interest_from_kotak_json(&body).is_empty(),
                "open_int {bad:?} must not light OI"
            );
        }
        // Zero is a real, publishable reading — an illiquid strike genuinely has none.
        let zero = r#"[{"exchange":"nse_fo","exchange_token":"56526","open_int":"0"}]"#;
        assert_eq!(nfo_open_interest_from_kotak_json(zero).len(), 1);
    }

    #[test]
    fn nfo_last_is_ltp_only_and_the_dead_aliases_stay_dark() {
        // Observed live shape: root array, `ltp`, `exchange` + `exchange_token`.
        let observed = r#"[{"display_symbol":"NIFTY2692221000PE","exchange":"nse_fo","exchange_token":"56526","ltp":"10.00","open_int":"480750"}]"#;
        let tick = quote_tick_from_kotak_json_for_book(observed, received(), KOTAK_NSE_NFO_BOOK_ID)
            .expect("ltp lights NFO last");
        assert_eq!(tick.last, "10.00");
        assert_eq!(tick.book_id, KOTAK_NSE_NFO_BOOK_ID);
        // Key is `nse_fo|{token}` — `display_symbol` never enters the identity.
        assert_eq!(tick.instrument_id, "nse_fo|56526");
        assert!(!tick.instrument_id.contains("NIFTY"));

        // Each dead alias, alone, on the NFO book: no tick.
        for dead in [
            "last_traded_price",
            "last",
            "lastPrice",
            "LTP",
            // Present on the live body, but quantity/volume — never a price.
            "last_traded_quantity",
            "last_volume",
        ] {
            let body =
                format!(r#"[{{"exchange":"nse_fo","exchange_token":"56526","{dead}":"10.00"}}]"#);
            assert!(
                quote_tick_from_kotak_json_for_book(&body, received(), KOTAK_NSE_NFO_BOOK_ID)
                    .is_none(),
                "{dead} must not light NFO last — it is absent from the live FO body"
            );
        }
    }

    /// `quote_type=all` carries `ltp` *and* `ohlc`. A row missing `ltp` is
    /// unusable; it is not a licence to publish the session close as last. And
    /// `quote_type=ltp` has no `ohlc` at all, so a fallback could only invent one.
    #[test]
    fn nfo_never_falls_back_to_session_close_as_last() {
        let no_ltp = r#"[{"exchange":"nse_fo","exchange_token":"56526","ohlc":{"open":"9.55","high":"11.20","low":"9.10","close":"9.80"}}]"#;
        assert!(
            quote_tick_from_kotak_json_for_book(no_ltp, received(), KOTAK_NSE_NFO_BOOK_ID)
                .is_none(),
            "close is a different quantity — it must never be painted as NFO last"
        );
        // Cash is a different book and a different observation, and its behaviour
        // is unchanged by this narrow. Note what the cash fallback actually is:
        // the `?` sits inside the or-chain, so a row with *no* last key at all is
        // None on cash too — the close only covers a last key that is present but
        // not stringable (here, `null`).
        let cash_no_key = r#"{"instrument_token":"2885","exchange_segment":"nse_cm","ohlc":{"open":"1","high":"2","low":"0.5","close":"1.5"}}"#;
        assert!(quote_tick_from_kotak_json_for_book(
            cash_no_key,
            received(),
            KOTAK_NSE_BSE_CASH_BOOK_ID
        )
        .is_none());
        let cash_null_last = r#"{"instrument_token":"2885","exchange_segment":"nse_cm","last_traded_price":null,"ohlc":{"open":"1","high":"2","low":"0.5","close":"1.5"}}"#;
        let cash = quote_tick_from_kotak_json_for_book(
            cash_null_last,
            received(),
            KOTAK_NSE_BSE_CASH_BOOK_ID,
        )
        .expect("cash keeps its session-close fallback");
        assert_eq!(cash.last, "1.5");

        // The same null-last row on NFO stays dark: no fallback on this book.
        let nfo_null_last = r#"[{"exchange":"nse_fo","exchange_token":"56526","ltp":null,"ohlc":{"open":"1","high":"2","low":"0.5","close":"1.5"}}]"#;
        assert!(quote_tick_from_kotak_json_for_book(
            nfo_null_last,
            received(),
            KOTAK_NSE_NFO_BOOK_ID
        )
        .is_none());

        // A lit NFO row still carries the session bar — it just is not last.
        let both = r#"[{"exchange":"nse_fo","exchange_token":"56526","ltp":"10.00","ohlc":{"open":"9.55","high":"11.20","low":"9.10","close":"9.80"}}]"#;
        let tick = quote_tick_from_kotak_json_for_book(both, received(), KOTAK_NSE_NFO_BOOK_ID)
            .expect("nfo tick");
        assert_eq!(tick.last, "10.00");
        assert_ne!(tick.last, "9.80");
        assert_eq!(
            tick.session_ohlc.as_ref().map(|bar| bar.close.as_str()),
            Some("9.80")
        );
    }

    /// Cash was not narrowed: both the v1 name and `ltp` still light it.
    #[test]
    fn cash_last_still_accepts_v1_and_ltp() {
        let v1 = r#"{"instrument_token":"2885","exchange_segment":"nse_cm","last_traded_price":"1400.50"}"#;
        assert_eq!(
            quote_tick_from_kotak_json(v1, received())
                .expect("cash v1")
                .last,
            "1400.50"
        );
        let live = r#"[{"exchange":"nse_cm","exchange_token":"11536","ltp":"3224.00"}]"#;
        let tick = quote_tick_from_kotak_json(live, received()).expect("cash ltp");
        assert_eq!(tick.last, "3224.00");
        assert_eq!(tick.instrument_id, "nse_cm|11536");
    }

    #[test]
    fn fo_json_on_nfo_book_stores_nfo_last_not_cash_reliance() {
        let fo_v1 = r#"{"instrument_token":"12345","exchange_segment":"nse_fo","last_traded_price":"10.00"}"#;
        let fo_v2 = r#"{"exchange_token":"12345","exchange":"nse_fo","ltp":"10.00"}"#;
        assert!(quote_tick_from_kotak_json(fo_v1, received()).is_none());
        assert!(quote_tick_from_kotak_json(fo_v2, received()).is_none());
        assert!(quote_ticks_from_kotak_json_for_book(
            fo_v1,
            received(),
            KOTAK_NSE_BSE_CASH_BOOK_ID
        )
        .is_empty());
        assert!(quote_ticks_from_kotak_json_for_book(fo_v1, received(), "unknown-book").is_empty());

        // Narrowed 2026-08-31: the v1 `last_traded_price` shape no longer lights
        // NFO last. The observed `ltp` shape does.
        assert!(
            quote_tick_from_kotak_json_for_book(fo_v1, received(), KOTAK_NSE_NFO_BOOK_ID).is_none(),
            "last_traded_price is absent from the live FO body — it must not light NFO last"
        );
        let tick = quote_tick_from_kotak_json_for_book(fo_v2, received(), KOTAK_NSE_NFO_BOOK_ID)
            .expect("nfo tick");
        assert_eq!(tick.instrument_id, "nse_fo|12345");
        assert_eq!(tick.book_id, KOTAK_NSE_NFO_BOOK_ID);
        assert_eq!(tick.adapter_id, KOTAK_NEO_ADAPTER_ID);
        let last: f64 = tick.last.parse().unwrap();
        assert!(last > 0.0);

        let registry = desk_registry();
        let mut book = TickBook::new();
        apply_quote(&registry, &mut book, tick).unwrap();
        let row = book
            .get(KOTAK_NSE_NFO_BOOK_ID, "nse_fo|12345")
            .expect("nfo stored");
        let stored_last: f64 = row.last.parse().unwrap();
        assert!(stored_last > 0.0);
        assert_eq!(row.adapter_id, KOTAK_NEO_ADAPTER_ID);

        let cash = quote_tick_from_kotak_json(FIXTURE, received()).expect("cash reliance");
        apply_quote(&registry, &mut book, cash).unwrap();
        assert!(book.get(KOTAK_NSE_NFO_BOOK_ID, "nse_cm|2885").is_none());
        assert!(book
            .get(KOTAK_NSE_BSE_CASH_BOOK_ID, "nse_cm|2885")
            .is_some());
        assert!(book.get(KOTAK_NSE_NFO_BOOK_ID, "nse_fo|12345").is_some());

        let bse_fo = r#"{"exchange_token":"1","exchange":"bse_fo","ltp":"10.00"}"#;
        assert!(
            quote_tick_from_kotak_json_for_book(bse_fo, received(), KOTAK_NSE_NFO_BOOK_ID)
                .is_none()
        );
        let cde_fo = r#"{"exchange_token":"1","exchange":"cde_fo","ltp":"10.00"}"#;
        assert!(
            quote_tick_from_kotak_json_for_book(cde_fo, received(), KOTAK_NSE_NFO_BOOK_ID)
                .is_none()
        );
    }

    #[test]
    fn parse_nfo_instrument_id_is_nse_fo_numeric_only() {
        assert_eq!(
            parse_nfo_instrument_id("nse_fo|12345").as_deref(),
            Some("nse_fo|12345")
        );
        assert_eq!(
            parse_nfo_instrument_id("NSE_FO%7C99").as_deref(),
            Some("nse_fo|99")
        );
        assert!(parse_nfo_instrument_id("nse_cm|2885").is_none());
        assert!(parse_nfo_instrument_id("bse_fo|1").is_none());
        assert!(parse_nfo_instrument_id("nse_fo|").is_none());
        assert!(parse_nfo_instrument_id("nse_fo|abc").is_none());
        assert_eq!(
            kotak_quote_book_id("nse_fo|12345"),
            Some(KOTAK_NSE_NFO_BOOK_ID)
        );
        assert_eq!(
            kotak_quote_book_id("nse_cm|2885"),
            Some(KOTAK_NSE_BSE_CASH_BOOK_ID)
        );
        assert!(kotak_quote_book_id("bse_fo|1").is_none());
    }

    #[test]
    fn nfo_oi_session_reads_oi_slice_verbatim() {
        let json = include_str!("../../fixtures/kotak/quotes_neosymbol_nfo_oi.json");
        let rows = nfo_oi_session_from_kotak_json(json);
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].instrument_id, "nse_fo|12345");
        assert_eq!(rows[0].oi_session_las, "475000");
        assert_eq!(rows[0].oi_session_high, "500000");
        assert_eq!(rows[0].oi_session_low, "450000");

        // `open_int` on `all` is a different slice — never parsed here.
        let all =
            r#"[{"exchange":"nse_fo","exchange_token":"12345","open_int":"480750","oi_las":"1"}]"#;
        assert!(
            nfo_oi_session_from_kotak_json(all).is_empty(),
            "oi_las without oi_high/oi_low is not a session slice row"
        );
    }

    #[test]
    fn cash_ltq_sums_on_forming_bar_and_skips_nfo() {
        use crate::data::binance_klines::{HistoryCandle, HistorySeries};
        use crate::data::candle_builder::CandleBuilders;
        use crate::data::kotak_historical::DEFAULT_INTERVAL;
        let t = received().timestamp_millis();
        let mut builders = CandleBuilders::new();
        builders.seed_from_series(&HistorySeries {
            instrument_id: "nse_cm|2885".into(),
            adapter_id: KOTAK_NEO_ADAPTER_ID.into(),
            interval: DEFAULT_INTERVAL.into(),
            candles: vec![HistoryCandle {
                open_time_ms: t,
                open: "1400".into(),
                high: "1400".into(),
                low: "1400".into(),
                close: "1400".into(),
                volume: "10".into(),
                close_time_ms: t + 899_999,
            }],
            transport: Transport::Rest,
        });
        let cash = r#"[{"exchange":"nse_cm","exchange_token":"2885","ltp":"1402","last_traded_quantity":"3"}]"#;
        tick_cash_builders_from_kotak_json(&mut builders, cash, received());
        let forming = builders
            .forming(KOTAK_NEO_ADAPTER_ID, "nse_cm|2885", DEFAULT_INTERVAL)
            .expect("cash forming");
        assert_eq!(forming.open, "1400");
        assert_eq!(forming.close, "1402");
        assert_eq!(forming.volume, "13");
        let nfo = r#"[{"exchange":"nse_fo","exchange_token":"56526","ltp":"10.00","last_traded_quantity":"99"}]"#;
        tick_cash_builders_from_kotak_json(&mut builders, nfo, received());
        assert!(builders
            .forming(KOTAK_NEO_ADAPTER_ID, "nse_fo|56526", DEFAULT_INTERVAL)
            .is_none());
    }
}
