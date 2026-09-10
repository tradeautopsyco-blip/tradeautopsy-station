//! Options chain / OI extracts. Wire `status` is unavailable until a named book
//! can Success. Do not default `s1_desk_symbol` (spot) as an NFO underlying.

use crate::api::AppState;
use crate::data::{
    apply_history_series, apply_quote, chain_input_honesty, chain_rows_for_contract,
    depth_snapshot_from_eapi_json, expiration_from_dated_contract, extract_chain_from,
    extract_greeks_from_mark, extract_open_interest, extract_open_interest_for_book,
    extract_open_interest_from, is_dated_option_contract, mark_row_for_symbol,
    normalize_options_instrument, oi_rows_from_json, option_symbols_from_exchange_info_json,
    options_depth_query, options_klines_query, options_ticker_query, parse_nfo_instrument_id,
    quote_tick_from_options_ticker_json_for_symbol, series_from_eapi_klines_json,
    underlying_asset_from_dated_contract, validate_options_kline_request, CachedMark, ChainRow,
    GlanceEnvelope, GreeksEnvelope, InputHonesty, OptionsOiRow, Transport, BINANCE_COM_ADAPTER_ID,
    BINANCE_COM_OPTIONS_BOOK_ID, DEFAULT_OPTIONS_HISTORY_INTERVAL, KOTAK_NSE_BSE_CASH_BOOK_ID,
    KOTAK_NSE_NFO_BOOK_ID, OPTIONS_DEPTH_HOST, OPTIONS_DEPTH_PATH, OPTIONS_EAPI_HOST,
    OPTIONS_KLINES_HOST, OPTIONS_KLINES_PATH, OPTIONS_KLINE_LIMIT_DEFAULT, OPTIONS_MARK_PATH,
    OPTIONS_TICKER_PATH,
};
use axum::extract::{Query, State};
use axum::Json;
use serde::Deserialize;
use std::time::Duration;

use crate::egress::{EgressCall, Lane};

/// Options contract lists change on expiry boundaries, not per request.
const EXCHANGE_INFO_MAX_AGE_MS: i64 = 300_000;
/// Open interest is a slow series; one call serves every panel asking at once.
const OPEN_INTEREST_MAX_AGE_MS: i64 = 5_000;
/// Same window as OI: one mark call serves every panel asking for this contract.
const OPTIONS_MARK_MAX_AGE_MS: i64 = 5_000;
/// A book moves fast, so the coalesce window is short — it exists so two obtain
/// calls in the same tick are one GET, not so a stale ladder is served.
const OPTIONS_DEPTH_MAX_AGE_MS: i64 = 1_000;
/// Same short window as depth / spot `ticker/price`: two quote GETs in one tick
/// share one last. REST.md publishes no ticker weight, so this is coalesce only,
/// not a budget. Never the unfiltered list.
const OPTIONS_TICKER_MAX_AGE_MS: i64 = 1_000;
/// Same coalesce window as COM klines. Weight 1; not a budget.
const OPTIONS_KLINES_MAX_AGE_MS: i64 = 2_000;

#[derive(Debug, Deserialize)]
pub struct GlanceQuery {
    pub book: Option<String>,
    pub instrument: Option<String>,
}

fn query_book(query: &GlanceQuery) -> Option<String> {
    query
        .book
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn query_instrument(query: &GlanceQuery) -> String {
    query
        .instrument
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("")
        .to_string()
}

fn nfo_chain_rows(state: &AppState, instrument: &str) -> Vec<ChainRow> {
    let master = state
        .kotak_nfo_scrip_master
        .lock()
        .expect("kotak nfo scrip master mutex poisoned");
    master
        .option_rows_for_underlying(instrument)
        .into_iter()
        .map(|row| row.to_chain_row())
        .collect()
}

fn kick_nfo_visible_quotes(state: &AppState, rows: &[ChainRow]) {
    for row in rows {
        if parse_nfo_instrument_id(&row.instrument_id).is_none() {
            continue;
        }
        state.kick_kotak_rest_quote(&row.instrument_id);
    }
}

fn options_chain_rows(state: &AppState, instrument: &str) -> Vec<ChainRow> {
    let master = state
        .options_option_symbols
        .lock()
        .expect("options option symbols mutex poisoned");
    chain_rows_for_contract(instrument, &master)
}

pub(crate) async fn ensure_options_master(state: &AppState) {
    {
        let master = state
            .options_option_symbols
            .lock()
            .expect("options option symbols mutex poisoned");
        if !master.is_empty() || !state.eapi_public_fetch {
            return;
        }
    }
    // The engine runs the same `authorize_book_call` fence before it charges
    // anything, so the check is not repeated here.
    let call = EgressCall::get(
        BINANCE_COM_OPTIONS_BOOK_ID,
        "eapi.binance.com",
        "/eapi/v1/exchangeInfo",
        Lane::MarketData,
    )
    .with_max_age_ms(EXCHANGE_INFO_MAX_AGE_MS)
    .with_timeout(Duration::from_secs(15));
    let Ok(resp) = crate::egress::shared().send(&call).await else {
        return;
    };
    if !resp.is_success() {
        return;
    }
    let rows = option_symbols_from_exchange_info_json(&resp.body);
    if rows.is_empty() {
        return;
    }
    *state
        .options_option_symbols
        .lock()
        .expect("options option symbols mutex poisoned") = rows;
}

pub(crate) async fn fetch_options_oi(state: &AppState, instrument: &str) -> Vec<OptionsOiRow> {
    {
        let planted = state
            .options_oi_rows
            .lock()
            .expect("options oi mutex poisoned");
        if !planted.is_empty() {
            return planted.clone();
        }
        if !state.eapi_public_fetch {
            return Vec::new();
        }
    }
    let Some(underlying) = underlying_asset_from_dated_contract(instrument) else {
        return Vec::new();
    };
    let Some(expiration) = expiration_from_dated_contract(instrument) else {
        return Vec::new();
    };
    let call = EgressCall::get(
        BINANCE_COM_OPTIONS_BOOK_ID,
        "eapi.binance.com",
        "/eapi/v1/openInterest",
        Lane::MarketData,
    )
    .with_query(format!(
        "underlyingAsset={underlying}&expiration={expiration}"
    ))
    .with_max_age_ms(OPEN_INTEREST_MAX_AGE_MS)
    .with_timeout(Duration::from_secs(15));
    let Ok(resp) = crate::egress::shared().send(&call).await else {
        return Vec::new();
    };
    if !resp.is_success() {
        return Vec::new();
    }
    oi_rows_from_json(&resp.body)
}

/// The stored mark, but only for the contract actually asked for.
///
/// Mark is per contract. A stored symbol that differs from `instrument` is a MISS:
/// returning it would repaint the previous contract's delta onto this one, which is
/// exactly the mislabelled number the greeks envelope exists to prevent. Match is
/// exact and case-sensitive — Binance option symbols are mixed-case dated contracts.
fn cached_mark_hit(cached: &Option<CachedMark>, instrument: &str) -> Option<CachedMark> {
    let instrument = instrument.trim();
    cached
        .as_ref()
        .filter(|hit| hit.symbol == instrument)
        .cloned()
}

/// The symbol to dial `/eapi/v1/mark` with, or `None` for "do not dial".
///
/// Two refusals, both silent by design: tests stay fixture-only
/// (`eapi_public_fetch` false), and a leftover `BTC` / `BTCUSDT` is not a dated
/// contract, so it must never reach the venue.
///
/// The shape test is [`is_dated_option_contract`], not
/// [`underlying_asset_from_dated_contract`]: the latter splits on the first `-`
/// and answers `Some("BTC")` for a bare `BTC`, which would have dialled mark with
/// a spot id. Only `UNDERLYING-YYMMDD-STRIKE-{C,P}` reaches the venue, and never
/// lowercased — Binance option symbols are mixed-case.
fn mark_dial_symbol(eapi_public_fetch: bool, instrument: &str) -> Option<String> {
    if !eapi_public_fetch {
        return None;
    }
    let instrument = instrument.trim();
    if !is_dated_option_contract(instrument) {
        return None;
    }
    underlying_asset_from_dated_contract(instrument)?;
    Some(instrument.to_string())
}

/// One `/eapi/v1/mark` row for exactly this contract, or `None`.
///
/// Never fabricates: a non-success HTTP, a body without this symbol, or a partial
/// greek row all return `None` rather than a half-lit delta.
pub(crate) async fn fetch_options_mark(state: &AppState, instrument: &str) -> Option<CachedMark> {
    let dial = {
        let cached = state
            .options_mark
            .lock()
            .expect("options mark mutex poisoned");
        if let Some(hit) = cached_mark_hit(&cached, instrument) {
            return Some(hit);
        }
        mark_dial_symbol(state.eapi_public_fetch, instrument)?
    };
    let call = EgressCall::get(
        BINANCE_COM_OPTIONS_BOOK_ID,
        "eapi.binance.com",
        OPTIONS_MARK_PATH,
        Lane::MarketData,
    )
    .with_query(format!("symbol={dial}"))
    .with_max_age_ms(OPTIONS_MARK_MAX_AGE_MS)
    .with_timeout(Duration::from_secs(15));
    let Ok(resp) = crate::egress::shared().send(&call).await else {
        return None;
    };
    if !resp.is_success() {
        return None;
    }
    let row = mark_row_for_symbol(&resp.body, &dial)?;
    Some(CachedMark {
        symbol: row.symbol.clone(),
        row,
        // Station's own fetch stamp. The official mark table has no timestamp
        // field, so there is no venue time to copy and none may be invented.
        as_of: chrono::Utc::now().to_rfc3339(),
    })
}

/// Obtain enriches synchronously, but the mark snapshot is an async fetch. Land
/// the row in the shared store so the enricher only ever reads state.
pub(crate) async fn ensure_options_mark(state: &AppState, instrument: &str) {
    {
        let cached = state
            .options_mark
            .lock()
            .expect("options mark mutex poisoned");
        if cached_mark_hit(&cached, instrument).is_some() {
            return;
        }
    }
    let Some(fetched) = fetch_options_mark(state, instrument).await else {
        return;
    };
    *state
        .options_mark
        .lock()
        .expect("options mark mutex poisoned") = Some(fetched);
}

/// The symbol to dial `/eapi/v1/depth` with, or `None` for "do not dial".
///
/// Same two refusals as mark, for the same reasons: tests stay fixture-only
/// (`eapi_public_fetch` false), and a leftover `BTC` / `BTCUSDT` is not a dated
/// contract. `symbol` is **mandatory** on this endpoint, so "no contract" means
/// "no call" — never an unfiltered book. Never lowercased.
fn depth_dial_symbol(eapi_public_fetch: bool, instrument: &str) -> Option<String> {
    if !eapi_public_fetch {
        return None;
    }
    let instrument = normalize_options_instrument(instrument);
    if !is_dated_option_contract(&instrument) {
        return None;
    }
    Some(instrument)
}

/// The symbol to dial `/eapi/v1/ticker` with, or `None` for "do not dial".
///
/// Same two refusals as mark/depth: tests stay fixture-only
/// (`eapi_public_fetch` false), and a leftover `BTC` / `BTCUSDT` is not a dated
/// contract. `?symbol=` is mandatory on this call — never the unfiltered 24hr
/// dump, whose first row is not this book's selected last. Never lowercased.
/// WS last is NOT SPECIFIED: this is REST `lastPrice` only.
fn ticker_dial_symbol(eapi_public_fetch: bool, instrument: &str) -> Option<String> {
    if !eapi_public_fetch {
        return None;
    }
    let instrument = normalize_options_instrument(instrument);
    if !is_dated_option_contract(&instrument) {
        return None;
    }
    Some(instrument)
}

/// One `GET /eapi/v1/ticker?symbol=` last into TickBook's **options** slot.
///
/// Never fabricates: a non-success HTTP or a body whose `symbol` is not this
/// contract leaves the book untouched, so quote/obtain stay Unavailable rather
/// than serving `last=0`. No HMAC. Does not subscribe TickBook (REST stays
/// open — this book has no specified WS last). Does not open
/// `nbstream…/eoptions`.
pub(crate) async fn ensure_options_ticker(state: &AppState, instrument: &str) {
    let Some(dial) = ticker_dial_symbol(state.eapi_public_fetch, instrument) else {
        return;
    };
    // The engine runs `authorize_book_call` itself; no HMAC is attached here —
    // a private credential on this public path is a refusal, not an upgrade.
    let call = EgressCall::get(
        BINANCE_COM_OPTIONS_BOOK_ID,
        OPTIONS_EAPI_HOST,
        OPTIONS_TICKER_PATH,
        Lane::MarketData,
    )
    .with_query(options_ticker_query(&dial))
    .with_max_age_ms(OPTIONS_TICKER_MAX_AGE_MS)
    .with_timeout(Duration::from_secs(15));
    let Ok(resp) = crate::egress::shared().send(&call).await else {
        return;
    };
    if !resp.is_success() {
        return;
    }
    let Some(tick) =
        quote_tick_from_options_ticker_json_for_symbol(&resp.body, &dial, chrono::Utc::now())
    else {
        return;
    };
    let mut book = state.tickbook.lock().expect("tickbook mutex poisoned");
    if let Err(err) = apply_quote(state.quote_registry.as_ref(), &mut book, tick) {
        tracing::warn!(error = %err, "options ticker: apply refused");
    }
}

/// One `GET /eapi/v1/depth` bounded snapshot into the DepthBook's **options** slot.
///
/// Never fabricates: a non-success HTTP or a body the parser refuses leaves the
/// book untouched, so obtain stays Unavailable rather than serving an empty
/// ladder as a Success. There is no gap machine on this book — no `stamp_gap_unusable`,
/// no `invalidate` — because a REST snapshot has nothing to fall out of sync with.
pub(crate) async fn ensure_options_depth(state: &AppState, instrument: &str) {
    let Some(dial) = depth_dial_symbol(state.eapi_public_fetch, instrument) else {
        return;
    };
    {
        let book = state.depthbook.lock().expect("depthbook mutex poisoned");
        if let Some(row) = book.get(BINANCE_COM_OPTIONS_BOOK_ID, &dial) {
            if row.completeness {
                return;
            }
        }
    }
    // The engine runs `authorize_book_call` itself; no HMAC is attached here —
    // a private credential on this public path is a refusal, not an upgrade.
    let call = EgressCall::get(
        BINANCE_COM_OPTIONS_BOOK_ID,
        OPTIONS_DEPTH_HOST,
        OPTIONS_DEPTH_PATH,
        Lane::MarketData,
    )
    .with_query(options_depth_query(&dial))
    .with_max_age_ms(OPTIONS_DEPTH_MAX_AGE_MS)
    .with_timeout(Duration::from_secs(15));
    let Ok(resp) = crate::egress::shared().send(&call).await else {
        return;
    };
    if !resp.is_success() {
        return;
    }
    let Some(snapshot) = depth_snapshot_from_eapi_json(&resp.body, &dial, chrono::Utc::now())
    else {
        return;
    };
    state
        .depthbook
        .lock()
        .expect("depthbook mutex poisoned")
        .upsert(snapshot);
}

/// One `GET /eapi/v1/klines?symbol=&interval=` series into HistoryBook.
///
/// Never fabricates: empty / unparseable body leaves the book untouched so
/// obtain stays Unavailable rather than a zero candle. Never `/api/v3/klines`.
/// Mixed-case `?symbol=`. No HMAC.
pub(crate) async fn ensure_options_klines(state: &AppState, instrument: &str) {
    if !state.eapi_public_fetch {
        return;
    }
    let dial = normalize_options_instrument(instrument);
    if !is_dated_option_contract(&dial) {
        return;
    }
    let interval = DEFAULT_OPTIONS_HISTORY_INTERVAL;
    {
        let book = state
            .historybook
            .lock()
            .expect("historybook mutex poisoned");
        if book.get(BINANCE_COM_ADAPTER_ID, &dial, interval).is_some() {
            return;
        }
    }
    let inflight_key = format!("{dial}\0{interval}");
    {
        let mut guard = state
            .klines_inflight
            .lock()
            .expect("klines inflight poisoned");
        if !guard.insert(inflight_key) {
            return;
        }
    }
    let Ok(limit) = validate_options_kline_request(interval, Some(OPTIONS_KLINE_LIMIT_DEFAULT))
    else {
        return;
    };
    let call = EgressCall::get(
        BINANCE_COM_OPTIONS_BOOK_ID,
        OPTIONS_KLINES_HOST,
        OPTIONS_KLINES_PATH,
        Lane::MarketData,
    )
    .with_query(options_klines_query(&dial, interval, limit))
    .with_max_age_ms(OPTIONS_KLINES_MAX_AGE_MS)
    .with_timeout(Duration::from_secs(15));
    let Ok(resp) = crate::egress::shared().send(&call).await else {
        return;
    };
    if !resp.is_success() {
        return;
    }
    let Some(series) = series_from_eapi_klines_json(&resp.body, &dial, interval, Transport::Rest)
    else {
        return;
    };
    apply_history_series(
        &mut state
            .historybook
            .lock()
            .expect("historybook mutex poisoned"),
        series,
    );
}

/// Obtain enriches synchronously, but the OI snapshot is an async fetch. Land
/// the rows in the shared store so the enricher only ever reads state.
pub(crate) async fn ensure_options_oi(state: &AppState, instrument: &str) {
    {
        let planted = state
            .options_oi_rows
            .lock()
            .expect("options oi mutex poisoned");
        if !planted.is_empty() {
            return;
        }
    }
    let rows = fetch_options_oi(state, instrument).await;
    if rows.is_empty() {
        return;
    }
    *state
        .options_oi_rows
        .lock()
        .expect("options oi mutex poisoned") = rows;
}

pub async fn chain_handler(
    State(state): State<AppState>,
    Query(query): Query<GlanceQuery>,
) -> Json<GlanceEnvelope> {
    let book = query_book(&query);
    let instrument = query_instrument(&query);
    match book.as_deref() {
        Some(id) if id == KOTAK_NSE_NFO_BOOK_ID => {
            let rows = nfo_chain_rows(&state, &instrument);
            kick_nfo_visible_quotes(&state, &rows);
            let tickbook = state.tickbook.lock().expect("tickbook mutex poisoned");
            Json(extract_chain_from(
                Some(KOTAK_NSE_NFO_BOOK_ID),
                &instrument,
                Some(rows.as_slice()),
                Some(&tickbook),
            ))
        }
        Some(id) if id == BINANCE_COM_OPTIONS_BOOK_ID => {
            ensure_options_master(&state).await;
            let rows = options_chain_rows(&state, &instrument);
            let tickbook = state.tickbook.lock().expect("tickbook mutex poisoned");
            Json(extract_chain_from(
                Some(BINANCE_COM_OPTIONS_BOOK_ID),
                &instrument,
                if rows.is_empty() {
                    None
                } else {
                    Some(rows.as_slice())
                },
                Some(&tickbook),
            ))
        }
        Some(id) if id == KOTAK_NSE_BSE_CASH_BOOK_ID => Json(extract_chain_from(
            Some(KOTAK_NSE_BSE_CASH_BOOK_ID),
            &instrument,
            None,
            None,
        )),
        other => Json(extract_chain_from(other, &instrument, None, None)),
    }
}

pub async fn oi_handler(
    State(state): State<AppState>,
    Query(query): Query<GlanceQuery>,
) -> Json<GlanceEnvelope> {
    let book = query_book(&query);
    let instrument = query_instrument(&query);
    match book.as_deref() {
        Some(id) if id == BINANCE_COM_OPTIONS_BOOK_ID => {
            let rows = fetch_options_oi(&state, &instrument).await;
            Json(extract_open_interest_from(
                Some(BINANCE_COM_OPTIONS_BOOK_ID),
                &instrument,
                if rows.is_empty() {
                    None
                } else {
                    Some(rows.as_slice())
                },
            ))
        }
        Some(id) if id == KOTAK_NSE_NFO_BOOK_ID => {
            // Reads the slot the `quote_type=all` quote fetch already filled for
            // `open_int`; session band fields come from `nfo_oi_session` when present.
            let reading = state
                .nfo_open_interest
                .lock()
                .expect("nfo open interest mutex poisoned")
                .get(instrument.trim().to_ascii_lowercase().as_str())
                .cloned();
            let session = state
                .nfo_oi_session
                .lock()
                .expect("nfo oi session mutex poisoned")
                .get(instrument.trim().to_ascii_lowercase().as_str())
                .cloned();
            Json(extract_open_interest_for_book(
                Some(KOTAK_NSE_NFO_BOOK_ID),
                &instrument,
                None,
                reading.as_ref(),
                session.as_ref(),
            ))
        }
        other => Json(extract_open_interest(other, &instrument)),
    }
}

/// Venue-published option greeks. `book` is required — this route never falls back
/// to `s1_desk_symbol`, because a spot id is not a contract and spot is not greeks.
pub async fn greeks_handler(
    State(state): State<AppState>,
    Query(query): Query<GlanceQuery>,
) -> Json<GreeksEnvelope> {
    let book = query_book(&query);
    let instrument = query_instrument(&query);
    match book.as_deref() {
        Some(id) if id == BINANCE_COM_OPTIONS_BOOK_ID => {
            let cached = fetch_options_mark(&state, &instrument).await;
            Json(extract_greeks_from_mark(
                Some(BINANCE_COM_OPTIONS_BOOK_ID),
                cached.as_ref().map(|hit| &hit.row),
                cached.as_ref().map(|hit| hit.as_of.as_str()),
                InputHonesty::Lit,
            ))
        }
        Some(id) if id == KOTAK_NSE_NFO_BOOK_ID => {
            // NFO stays on its own dark path and never reads a mark row. The chain
            // input is the real one, so the darkness is inherited honestly rather
            // than asserted.
            let rows = nfo_chain_rows(&state, &instrument);
            let chain = {
                let tickbook = state.tickbook.lock().expect("tickbook mutex poisoned");
                extract_chain_from(
                    Some(KOTAK_NSE_NFO_BOOK_ID),
                    &instrument,
                    Some(rows.as_slice()),
                    Some(&tickbook),
                )
            };
            Json(extract_greeks_from_mark(
                Some(KOTAK_NSE_NFO_BOOK_ID),
                None,
                None,
                chain_input_honesty(&chain),
            ))
        }
        other => Json(extract_greeks_from_mark(
            other,
            None,
            None,
            InputHonesty::Dark(crate::data::HonestyStatus::Unavailable),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{mark_row_for_symbol, CachedMark};

    const OFFICIAL_EXAMPLE: &str = r#"[ { "symbol": "BTC-200730-9000-C", "markPrice": "1343.2883", "bidIV": "1.40000077", "askIV": "1.50000153", "markIV": "1.45000000", "delta": "0.55937056", "theta": "3739.82509871", "gamma": "0.00010969", "vega": "978.58874732", "highPriceLimit": "1618.241", "lowPriceLimit": "1068.3356", "riskFreeInterest": "0.1" } ]"#;

    fn cached(symbol: &str) -> Option<CachedMark> {
        let row = mark_row_for_symbol(OFFICIAL_EXAMPLE, "BTC-200730-9000-C").expect("row");
        Some(CachedMark {
            symbol: symbol.to_string(),
            row,
            as_of: "2026-08-31T09:00:00Z".to_string(),
        })
    }

    #[test]
    fn a_stored_mark_serves_only_its_own_contract() {
        let store = cached("BTC-200730-9000-C");
        let hit = cached_mark_hit(&store, "BTC-200730-9000-C").expect("exact match is a hit");
        assert_eq!(hit.row.delta, "0.55937056");
        // A different contract is a miss, never the previous contract's delta.
        assert!(cached_mark_hit(&store, "BTC-200730-9500-C").is_none());
        assert!(cached_mark_hit(&store, "btc-200730-9000-c").is_none());
        assert!(cached_mark_hit(&store, "BTCUSDT").is_none());
        assert!(cached_mark_hit(&None, "BTC-200730-9000-C").is_none());
    }

    /// Same gate as mark/depth: no fixture-time dial, no spot id, no lowercase.
    #[test]
    fn ticker_is_not_dialled_without_public_fetch_or_a_dated_contract() {
        assert!(ticker_dial_symbol(false, "BTC-200730-9000-C").is_none());
        assert!(ticker_dial_symbol(true, "BTC").is_none());
        assert!(ticker_dial_symbol(true, "BTCUSDT").is_none());
        assert!(ticker_dial_symbol(true, "").is_none());
        assert!(ticker_dial_symbol(true, "nse_cm|2885").is_none());
        assert_eq!(
            ticker_dial_symbol(true, "  BTC-200730-9000-C  ").as_deref(),
            Some("BTC-200730-9000-C")
        );
        assert_ne!(
            ticker_dial_symbol(true, "BTC-200730-9000-C").as_deref(),
            Some("btc-200730-9000-c")
        );
        assert_eq!(OPTIONS_TICKER_PATH, "/eapi/v1/ticker");
        assert_eq!(OPTIONS_EAPI_HOST, "eapi.binance.com");
        assert_ne!(OPTIONS_TICKER_PATH, OPTIONS_MARK_PATH);
        assert_ne!(OPTIONS_TICKER_PATH, OPTIONS_DEPTH_PATH);
        assert_eq!(
            options_ticker_query("BTC-200730-9000-C"),
            "symbol=BTC-200730-9000-C"
        );
    }

    /// Same gate as mark: no fixture-time dial, no spot id, no lowercase.
    #[test]
    fn depth_is_not_dialled_without_public_fetch_or_a_dated_contract() {
        assert!(depth_dial_symbol(false, "BTC-200730-9000-C").is_none());
        assert!(depth_dial_symbol(true, "BTC").is_none());
        assert!(depth_dial_symbol(true, "BTCUSDT").is_none());
        assert!(depth_dial_symbol(true, "").is_none());
        assert!(depth_dial_symbol(true, "nse_cm|2885").is_none());
        assert_eq!(
            depth_dial_symbol(true, "  BTC-200730-9000-C  ").as_deref(),
            Some("BTC-200730-9000-C")
        );
    }

    #[test]
    fn mark_is_not_dialled_without_public_fetch_or_a_dated_contract() {
        // Tests stay fixture-only.
        assert!(mark_dial_symbol(false, "BTC-200730-9000-C").is_none());
        // A leftover spot id is not a contract and must never dial mark.
        assert!(mark_dial_symbol(true, "BTC").is_none());
        assert!(mark_dial_symbol(true, "BTCUSDT").is_none());
        assert!(mark_dial_symbol(true, "").is_none());
        // Only a dated contract dials, and never lowercased.
        assert_eq!(
            mark_dial_symbol(true, "BTC-200730-9000-C").as_deref(),
            Some("BTC-200730-9000-C")
        );
    }
}
