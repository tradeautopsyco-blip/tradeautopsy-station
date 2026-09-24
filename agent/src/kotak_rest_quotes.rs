//! Host-mediated Kotak REST quotes (`quote_type=all`) into TickBook and
//! `quote_type=depth` into DepthBook (bounded snapshot, not HSM `isDepth`).
//!
//! Session attach via `prepare_kotak_catalog_get` (consumer-key Authorization,
//! Auth + Sid, no trade-book `sId`).
//! HSM websocket skipped (WEBSOCKET.md: `mlhsm` not HTTP-allowlisted; `hsServerId` unspecified).

use crate::data::{
    apply_quote, depth_snapshots_from_kotak_json, json_array_first_object_keys,
    json_field_object_keys, json_first_nested_object_keys, json_object_keys, kotak_quote_book_id,
    quote_ticks_from_kotak_json_for_book, quotes_neosymbol_path,
    tick_cash_builders_from_kotak_json, CandleBuilders, DepthBook, Registry, TickBook,
    KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID, QUOTE_TYPE_ALL, QUOTE_TYPE_DEPTH,
    QUOTE_TYPE_OI,
};
use crate::kotak_scrip_master::KOTAK_NEO;
use crate::ubi::{
    classify_response, kotak_base_host, prepare_kotak_catalog_get, BrokerCredentialVault,
    HostCredentialBlob,
};
use chrono::Utc;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type SessionLocator = Arc<Mutex<Option<(String, String)>>>;
pub type QuoteFetchErrorMap = Arc<Mutex<HashMap<String, String>>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteFetchErrorClass {
    Session,
    QuotesHttp,
    QuotesUnusable,
}

impl QuoteFetchErrorClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Session => "session",
            Self::QuotesHttp => "quotes_http",
            Self::QuotesUnusable => "quotes_unusable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuoteFetchError {
    pub class: QuoteFetchErrorClass,
}

impl std::fmt::Display for QuoteFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.class.as_str())
    }
}

impl std::error::Error for QuoteFetchError {}

fn persist_quote_fetch_class(
    map: &QuoteFetchErrorMap,
    instrument: &str,
    class: QuoteFetchErrorClass,
) {
    map.lock()
        .expect("quote fetch error poisoned")
        .insert(instrument.to_string(), class.as_str().to_string());
}

fn clear_quote_fetch_class(map: &QuoteFetchErrorMap, instrument: &str) {
    map.lock()
        .expect("quote fetch error poisoned")
        .remove(instrument);
}

/// Fetch once per instrument when TickBook has no Kotak last. No book.subscribe
/// (that would close REST). Rate window is unspecified — do not poll.
#[allow(clippy::too_many_arguments)]
pub fn ensure_kotak_rest_quote(
    registry: Arc<Registry>,
    book: Arc<Mutex<TickBook>>,
    vault: Arc<dyn BrokerCredentialVault>,
    locator: SessionLocator,
    inflight: &Arc<Mutex<HashSet<String>>>,
    quote_fetch_error: QuoteFetchErrorMap,
    // NFO open interest rides the same body. `None` = do not collect.
    nfo_oi: Option<NfoOpenInterestSlot>,
    builders: Option<Arc<Mutex<CandleBuilders>>>,
    instrument_id: &str,
) {
    let instrument = instrument_id.trim().to_ascii_lowercase();
    if instrument.is_empty() || !instrument.contains('|') {
        return;
    }
    let Some(book_id) = kotak_quote_book_id(&instrument) else {
        return;
    };
    {
        let guard = book.lock().expect("tickbook mutex poisoned");
        if guard.get(book_id, &instrument).is_some() {
            clear_quote_fetch_class(&quote_fetch_error, &instrument);
            return;
        }
    }
    let inflight = inflight.clone();
    tokio::spawn(async move {
        await_kotak_rest_quote(
            registry,
            book,
            vault,
            locator,
            inflight,
            quote_fetch_error,
            nfo_oi,
            builders,
            &instrument,
        )
        .await;
    });
}

/// Same REST GET as `ensure_kotak_rest_quote`, but the caller waits until TickBook
/// or `quote_fetch_error` is set. `GET /api/station/quote` uses this so Last is not
/// painted from an empty book while the fetch is still in flight. Does not poll Kotak.
#[allow(clippy::too_many_arguments)]
pub async fn await_kotak_rest_quote(
    registry: Arc<Registry>,
    book: Arc<Mutex<TickBook>>,
    vault: Arc<dyn BrokerCredentialVault>,
    locator: SessionLocator,
    inflight: Arc<Mutex<HashSet<String>>>,
    quote_fetch_error: QuoteFetchErrorMap,
    // NFO open interest rides the same body. `None` = do not collect.
    nfo_oi: Option<NfoOpenInterestSlot>,
    builders: Option<Arc<Mutex<CandleBuilders>>>,
    instrument_id: &str,
) {
    let instrument = instrument_id.trim().to_ascii_lowercase();
    if instrument.is_empty() || !instrument.contains('|') {
        return;
    }
    let Some(book_id) = kotak_quote_book_id(&instrument) else {
        return;
    };
    {
        let guard = book.lock().expect("tickbook mutex poisoned");
        if guard.get(book_id, &instrument).is_some() {
            clear_quote_fetch_class(&quote_fetch_error, &instrument);
            return;
        }
    }
    let we_own = {
        let mut guard = inflight.lock().expect("kotak quote inflight poisoned");
        guard.insert(instrument.clone())
    };
    if !we_own {
        wait_for_inflight_quote(&book, &inflight, &quote_fetch_error, &instrument, book_id).await;
        return;
    }
    let loc = locator
        .lock()
        .expect("kotak session locator poisoned")
        .clone();
    let Some((environment, connection_id)) = loc else {
        persist_quote_fetch_class(
            &quote_fetch_error,
            &instrument,
            QuoteFetchErrorClass::Session,
        );
        inflight
            .lock()
            .expect("kotak quote inflight poisoned")
            .remove(&instrument);
        return;
    };
    let result = fetch_and_apply(
        registry.as_ref(),
        &book,
        vault.as_ref(),
        &environment,
        &connection_id,
        nfo_oi.as_ref(),
        builders.as_ref(),
        &instrument,
    )
    .await;
    inflight
        .lock()
        .expect("kotak quote inflight poisoned")
        .remove(&instrument);
    match result {
        Ok((n, _)) if n > 0 => {
            clear_quote_fetch_class(&quote_fetch_error, &instrument);
            tracing::info!(
                instrument = %instrument,
                ticks = n,
                "s1 desk: kotak REST quote applied"
            );
        }
        Ok((_, body)) => {
            persist_unusable_quotes(&quote_fetch_error, &instrument, &body);
        }
        Err(err) => {
            persist_quote_fetch_class(&quote_fetch_error, &instrument, err.class);
            tracing::warn!(
                instrument = %instrument,
                error = %err,
                "s1 desk: kotak REST quote failed"
            );
        }
    }
}

async fn wait_for_inflight_quote(
    book: &Arc<Mutex<TickBook>>,
    inflight: &Arc<Mutex<HashSet<String>>>,
    quote_fetch_error: &QuoteFetchErrorMap,
    instrument: &str,
    book_id: &str,
) {
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        {
            let guard = book.lock().expect("tickbook mutex poisoned");
            if guard.get(book_id, instrument).is_some() {
                return;
            }
        }
        if quote_fetch_error
            .lock()
            .expect("quote fetch error poisoned")
            .contains_key(instrument)
        {
            return;
        }
        if !inflight
            .lock()
            .expect("kotak quote inflight poisoned")
            .contains(instrument)
        {
            return;
        }
    }
}

/// Per-instrument NFO open interest, read off the `quote_type=all` body that
/// already feeds TickBook last. A slot beside the tick — OI is not a price and
/// never enters `QuoteTick::last`.
pub type NfoOpenInterestSlot = Arc<Mutex<HashMap<String, crate::data::NfoOpenInterest>>>;

/// Land every `open_int` in this body into the slot. Same body, no extra GET.
pub fn apply_nfo_open_interest_body(slot: &NfoOpenInterestSlot, body: &str) -> usize {
    let rows = crate::data::nfo_open_interest_from_kotak_json(body);
    if rows.is_empty() {
        return 0;
    }
    let mut guard = slot.lock().expect("nfo open interest mutex poisoned");
    let mut applied = 0;
    for row in rows {
        guard.insert(row.instrument_id.clone(), row);
        applied += 1;
    }
    applied
}

pub fn apply_kotak_quote_body_for_book(
    registry: &Registry,
    book: &mut TickBook,
    body: &str,
    received_at: chrono::DateTime<Utc>,
    book_id: &str,
) -> usize {
    let mut applied = 0;
    for tick in quote_ticks_from_kotak_json_for_book(body, received_at, book_id) {
        if apply_quote(registry, book, tick).is_ok() {
            applied += 1;
        }
    }
    applied
}

pub fn apply_kotak_depth_body(
    book: &mut DepthBook,
    body: &str,
    received_at: chrono::DateTime<Utc>,
) -> usize {
    let mut applied = 0;
    for snapshot in depth_snapshots_from_kotak_json(body, received_at) {
        book.upsert(snapshot);
        applied += 1;
    }
    applied
}

/// Fetch once per instrument when DepthBook has no Kotak snapshot. REST only —
/// do not open HSM `isDepth`. Rate window unspecified — do not poll.
pub fn ensure_kotak_rest_depth(
    book: Arc<Mutex<DepthBook>>,
    vault: Arc<dyn BrokerCredentialVault>,
    locator: SessionLocator,
    inflight: &Arc<Mutex<HashSet<String>>>,
    instrument_id: &str,
) {
    let instrument = instrument_id.trim().to_ascii_lowercase();
    if instrument.is_empty() || !instrument.contains('|') {
        return;
    }
    let Some(book_id) = kotak_quote_book_id(&instrument) else {
        return;
    };
    if book_id != KOTAK_NSE_BSE_CASH_BOOK_ID && book_id != KOTAK_NSE_NFO_BOOK_ID {
        return;
    }
    {
        let guard = book.lock().expect("depthbook mutex poisoned");
        if guard
            .get(book_id, &instrument)
            .is_some_and(|row| row.completeness)
        {
            return;
        }
    }
    {
        let mut guard = inflight.lock().expect("kotak depth inflight poisoned");
        if !guard.insert(instrument.clone()) {
            return;
        }
    }
    let loc = locator
        .lock()
        .expect("kotak session locator poisoned")
        .clone();
    let Some((environment, connection_id)) = loc else {
        inflight
            .lock()
            .expect("kotak depth inflight poisoned")
            .remove(&instrument);
        return;
    };
    let inflight = inflight.clone();
    tokio::spawn(async move {
        let result = fetch_and_apply_depth(
            &book,
            vault.as_ref(),
            &environment,
            &connection_id,
            &instrument,
        )
        .await;
        inflight
            .lock()
            .expect("kotak depth inflight poisoned")
            .remove(&instrument);
        match result {
            Ok((n, _)) if n > 0 => tracing::info!(
                instrument = %instrument,
                snapshots = n,
                "s1 desk: kotak REST depth snapshot applied"
            ),
            Ok((_, body)) => log_unusable_quotes_keys(&instrument, &body, "depth"),
            Err(err) => tracing::warn!(
                instrument = %instrument,
                error = %err,
                "s1 desk: kotak REST depth failed"
            ),
        }
    });
}

/// Same REST GET as `ensure_kotak_rest_depth`, but the caller waits for DepthBook
/// or a fetch failure. Obtain uses this so NFO depth is not left unavailable
/// while the spawn from ensure is still in flight.
pub async fn await_kotak_rest_depth(
    book: Arc<Mutex<DepthBook>>,
    vault: Arc<dyn BrokerCredentialVault>,
    locator: SessionLocator,
    instrument_id: &str,
) {
    let instrument = instrument_id.trim().to_ascii_lowercase();
    if instrument.is_empty() || !instrument.contains('|') {
        return;
    }
    let Some(book_id) = kotak_quote_book_id(&instrument) else {
        return;
    };
    if book_id != KOTAK_NSE_BSE_CASH_BOOK_ID && book_id != KOTAK_NSE_NFO_BOOK_ID {
        return;
    }
    {
        let guard = book.lock().expect("depthbook mutex poisoned");
        if guard
            .get(book_id, &instrument)
            .is_some_and(|row| row.completeness)
        {
            return;
        }
    }
    let loc = locator
        .lock()
        .expect("kotak session locator poisoned")
        .clone();
    let Some((environment, connection_id)) = loc else {
        return;
    };
    let _ = fetch_and_apply_depth(
        &book,
        vault.as_ref(),
        &environment,
        &connection_id,
        &instrument,
    )
    .await;
}

fn class_for_unusable_quotes_body(body: &str) -> QuoteFetchErrorClass {
    match classify_response(200, body).as_deref() {
        Some("session_expired") => QuoteFetchErrorClass::Session,
        _ => QuoteFetchErrorClass::QuotesUnusable,
    }
}

fn log_unusable_quotes_keys(instrument: &str, body: &str, kind: &str) {
    tracing::warn!(
        instrument = %instrument,
        kind = kind,
        root_keys = ?json_object_keys(body),
        data_keys = ?json_field_object_keys(body, "data"),
        data_row_keys = ?json_first_nested_object_keys(body, "data"),
        message_keys = ?json_field_object_keys(body, "message"),
        message_row_keys = ?json_first_nested_object_keys(body, "message"),
        array_row_keys = ?json_array_first_object_keys(body),
        "s1 desk: kotak REST 2xx unusable"
    );
}

fn persist_unusable_quotes(map: &QuoteFetchErrorMap, instrument: &str, body: &str) {
    let class = class_for_unusable_quotes_body(body);
    log_unusable_quotes_keys(instrument, body, "quote");
    persist_quote_fetch_class(map, instrument, class);
}

#[allow(clippy::too_many_arguments)]
async fn fetch_and_apply(
    registry: &Registry,
    book: &Arc<Mutex<TickBook>>,
    vault: &dyn BrokerCredentialVault,
    environment: &str,
    connection_id: &str,
    // NFO open interest off the same body. `None` = do not collect.
    nfo_oi: Option<&NfoOpenInterestSlot>,
    builders: Option<&Arc<Mutex<CandleBuilders>>>,
    instrument_id: &str,
) -> Result<(usize, String), QuoteFetchError> {
    let body = fetch_quotes_json(
        vault,
        environment,
        connection_id,
        instrument_id,
        QUOTE_TYPE_ALL,
    )
    .await?;
    let received_at = Utc::now();
    let book_id = kotak_quote_book_id(instrument_id).ok_or(QuoteFetchError {
        class: QuoteFetchErrorClass::QuotesHttp,
    })?;
    let applied = {
        let mut guard = book.lock().expect("tickbook mutex poisoned");
        apply_kotak_quote_body_for_book(registry, &mut guard, &body, received_at, book_id)
    };
    if let Some(builders) = builders {
        if book_id == KOTAK_NSE_BSE_CASH_BOOK_ID {
            let mut builders = builders.lock().expect("candle builders mutex poisoned");
            tick_cash_builders_from_kotak_json(&mut builders, &body, received_at);
        }
    }
    // Open interest off the SAME body — one GET serves last and OI. Only the NFO
    // book names `open_int`; cash rows simply produce no reading.
    if let Some(slot) = nfo_oi {
        if book_id == crate::data::KOTAK_NSE_NFO_BOOK_ID {
            apply_nfo_open_interest_body(slot, &body);
        }
    }
    Ok((applied, body))
}

async fn fetch_and_apply_depth(
    book: &Arc<Mutex<DepthBook>>,
    vault: &dyn BrokerCredentialVault,
    environment: &str,
    connection_id: &str,
    instrument_id: &str,
) -> anyhow::Result<(usize, String)> {
    let body = fetch_quotes_json(
        vault,
        environment,
        connection_id,
        instrument_id,
        QUOTE_TYPE_DEPTH,
    )
    .await?;
    let received_at = Utc::now();
    let mut guard = book.lock().expect("depthbook mutex poisoned");
    let applied = apply_kotak_depth_body(&mut guard, &body, received_at);
    Ok((applied, body))
}

/// Per-instrument NFO session OI band from `quote_type=oi`. A slot beside
/// [`NfoOpenInterestSlot`] — supplementary fields, never overwriting `open_int`.
pub type NfoOiSessionSlot = Arc<Mutex<HashMap<String, crate::data::NfoOiSessionSlice>>>;

/// Land every `oi_las`/`oi_high`/`oi_low` row in this body into the slot.
pub fn apply_nfo_oi_session_body(slot: &NfoOiSessionSlot, body: &str) -> usize {
    let rows = crate::data::nfo_oi_session_from_kotak_json(body);
    if rows.is_empty() {
        return 0;
    }
    let mut guard = slot.lock().expect("nfo oi session mutex poisoned");
    let mut applied = 0;
    for row in rows {
        guard.insert(row.instrument_id.clone(), row);
        applied += 1;
    }
    applied
}

/// Obtain waits for the `quote_type=oi` REST GET so session fields are not left
/// unavailable while a fetch is still in flight.
pub async fn await_kotak_rest_oi_session(
    slot: NfoOiSessionSlot,
    vault: Arc<dyn BrokerCredentialVault>,
    locator: SessionLocator,
    instrument_id: &str,
) {
    let instrument = instrument_id.trim().to_ascii_lowercase();
    if instrument.is_empty() || !instrument.contains('|') {
        return;
    }
    let Some(book_id) = kotak_quote_book_id(&instrument) else {
        return;
    };
    if book_id != KOTAK_NSE_NFO_BOOK_ID {
        return;
    }
    {
        let guard = slot.lock().expect("nfo oi session mutex poisoned");
        if guard.contains_key(&instrument) {
            return;
        }
    }
    let loc = locator
        .lock()
        .expect("kotak session locator poisoned")
        .clone();
    let Some((environment, connection_id)) = loc else {
        return;
    };
    let _ = fetch_and_apply_oi_session(
        &slot,
        vault.as_ref(),
        &environment,
        &connection_id,
        &instrument,
    )
    .await;
}

async fn fetch_and_apply_oi_session(
    slot: &NfoOiSessionSlot,
    vault: &dyn BrokerCredentialVault,
    environment: &str,
    connection_id: &str,
    instrument_id: &str,
) -> Result<(usize, String), QuoteFetchError> {
    let body = fetch_quotes_json(
        vault,
        environment,
        connection_id,
        instrument_id,
        QUOTE_TYPE_OI,
    )
    .await?;
    let applied = apply_nfo_oi_session_body(slot, &body);
    Ok((applied, body))
}

pub async fn fetch_quotes_json(
    vault: &dyn BrokerCredentialVault,
    environment: &str,
    connection_id: &str,
    instrument_id: &str,
    quote_type: &str,
) -> Result<String, QuoteFetchError> {
    let blob = vault
        .load(environment, KOTAK_NEO, connection_id)
        .map_err(|_| QuoteFetchError {
            class: QuoteFetchErrorClass::Session,
        })?
        .ok_or(QuoteFetchError {
            class: QuoteFetchErrorClass::Session,
        })?;
    let creds = HostCredentialBlob::from(&blob);
    let base_url = match &creds {
        HostCredentialBlob::KotakSession { base_url, .. } => base_url.clone(),
        HostCredentialBlob::Hmac { .. }
        | HostCredentialBlob::KiteSession { .. }
        | HostCredentialBlob::UpstoxSession { .. }
        | HostCredentialBlob::FyersSession { .. }
        | HostCredentialBlob::GrowwSession { .. }
        | HostCredentialBlob::DhanSession { .. } => {
            return Err(QuoteFetchError {
                class: QuoteFetchErrorClass::Session,
            });
        }
    };
    kotak_base_host(&base_url).ok_or(QuoteFetchError {
        class: QuoteFetchErrorClass::QuotesHttp,
    })?;
    let path = quotes_neosymbol_path(instrument_id, quote_type);
    let book_id = kotak_quote_book_id(instrument_id).ok_or(QuoteFetchError {
        class: QuoteFetchErrorClass::QuotesHttp,
    })?;
    let prepared = prepare_kotak_catalog_get(&path, &creds).map_err(|_| QuoteFetchError {
        class: QuoteFetchErrorClass::QuotesHttp,
    })?;
    // The engine re-runs `authorize_book_call` for this book and host before it
    // admits, so the fence is not duplicated here. Quotes are polled, which is
    // exactly the traffic Kotak's undocumented 429 budget is waiting for.
    let resp = crate::egress::shared()
        .send_prepared(
            book_id,
            crate::egress::Lane::PrivateRead,
            &prepared,
            Duration::from_secs(15),
        )
        .await
        .map_err(|_| QuoteFetchError {
            class: QuoteFetchErrorClass::QuotesHttp,
        })?;
    if matches!(resp.status, 401 | 403) {
        return Err(QuoteFetchError {
            class: QuoteFetchErrorClass::Session,
        });
    }
    if !resp.is_success() {
        return Err(QuoteFetchError {
            class: QuoteFetchErrorClass::QuotesHttp,
        });
    }
    Ok(resp.body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{
        authorize_book_call, depth_snapshot_from_kotak_json, kotak_neo_quote_descriptor,
        quote_tick_from_kotak_json, DepthBook, Registry, TickBook,
    };
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};

    fn apply_kotak_quote_body(
        registry: &Registry,
        book: &mut TickBook,
        body: &str,
        received_at: chrono::DateTime<Utc>,
    ) -> usize {
        apply_kotak_quote_body_for_book(
            registry,
            book,
            body,
            received_at,
            KOTAK_NSE_BSE_CASH_BOOK_ID,
        )
    }

    #[test]
    fn fixture_body_applies_without_network() {
        let registry = Registry::load(&[kotak_neo_quote_descriptor()]).unwrap();
        let mut book = TickBook::new();
        let json = include_str!("../fixtures/kotak/quotes_neosymbol.json");
        let n = apply_kotak_quote_body(&registry, &mut book, json, Utc::now());
        assert_eq!(n, 1);
        let row = book.get(KOTAK_NSE_BSE_CASH_BOOK_ID, "nse_cm|2885").unwrap();
        assert!(row.last.parse::<f64>().unwrap() > 0.0);
        assert_eq!(row.adapter_id, KOTAK_NEO);
        let tick = quote_tick_from_kotak_json(json, Utc::now()).unwrap();
        assert_eq!(tick.instrument_id, "nse_cm|2885");
    }

    #[test]
    fn fixture_depth_body_fills_depthbook_not_tickbook() {
        let registry = Registry::load(&[kotak_neo_quote_descriptor()]).unwrap();
        let mut ticks = TickBook::new();
        let mut depth = DepthBook::new();
        let json = include_str!("../fixtures/kotak/quotes_neosymbol_depth.json");
        assert_eq!(
            apply_kotak_quote_body(&registry, &mut ticks, json, Utc::now()),
            0
        );
        assert!(ticks
            .get(KOTAK_NSE_BSE_CASH_BOOK_ID, "nse_cm|2885")
            .is_none());
        assert_eq!(apply_kotak_depth_body(&mut depth, json, Utc::now()), 1);
        let row = depth
            .get(KOTAK_NSE_BSE_CASH_BOOK_ID, "nse_cm|2885")
            .unwrap();
        assert!(row.completeness);
        assert!(!row.bids.is_empty());
        let snap = depth_snapshot_from_kotak_json(json, Utc::now()).unwrap();
        assert_eq!(snap.instrument_id, "nse_cm|2885");
    }

    #[test]
    fn quote_fetch_error_class_tokens_are_stable() {
        assert_eq!(QuoteFetchErrorClass::Session.as_str(), "session");
        assert_eq!(QuoteFetchErrorClass::QuotesHttp.as_str(), "quotes_http");
        assert_eq!(
            QuoteFetchErrorClass::QuotesUnusable.as_str(),
            "quotes_unusable"
        );
    }

    #[test]
    fn http_200_stcode_1003_is_session_tickbook_empty() {
        let registry = Registry::load(&[kotak_neo_quote_descriptor()]).unwrap();
        let mut book = TickBook::new();
        let json = r#"{"stat":"Not_Ok","stCode":1003,"errMsg":"Invalid Session"}"#;
        assert_eq!(
            apply_kotak_quote_body(&registry, &mut book, json, Utc::now()),
            0
        );
        assert!(book
            .get(KOTAK_NSE_BSE_CASH_BOOK_ID, "nse_cm|2885")
            .is_none());
        assert_eq!(
            class_for_unusable_quotes_body(json),
            QuoteFetchErrorClass::Session
        );
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(HashMap::new()));
        persist_unusable_quotes(&errors, "nse_cm|3721", json);
        assert_eq!(
            errors
                .lock()
                .expect("errors")
                .get("nse_cm|3721")
                .map(String::as_str),
            Some("session")
        );
    }

    #[test]
    fn http_200_without_ltp_is_quotes_unusable_not_empty_success() {
        let json = r#"{"data":{"nse_cm|3721":{"foo":1}}}"#;
        let registry = Registry::load(&[kotak_neo_quote_descriptor()]).unwrap();
        let mut book = TickBook::new();
        assert_eq!(
            apply_kotak_quote_body(&registry, &mut book, json, Utc::now()),
            0
        );
        assert!(book
            .get(KOTAK_NSE_BSE_CASH_BOOK_ID, "nse_cm|3721")
            .is_none());
        assert_eq!(
            class_for_unusable_quotes_body(json),
            QuoteFetchErrorClass::QuotesUnusable
        );
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(HashMap::new()));
        persist_unusable_quotes(&errors, "nse_cm|3721", json);
        assert_eq!(
            errors
                .lock()
                .expect("errors")
                .get("nse_cm|3721")
                .map(String::as_str),
            Some("quotes_unusable")
        );
    }

    #[tokio::test]
    async fn await_skips_http_when_tickbook_already_has_kotak_last() {
        let registry = Arc::new(Registry::load(&[kotak_neo_quote_descriptor()]).unwrap());
        let mut book = TickBook::new();
        let json = include_str!("../fixtures/kotak/quotes_neosymbol.json");
        apply_kotak_quote_body(registry.as_ref(), &mut book, json, Utc::now());
        let book = Arc::new(Mutex::new(book));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(HashMap::new()));
        await_kotak_rest_quote(
            registry,
            book.clone(),
            Arc::new(crate::ubi::MemoryBrokerCredentialVault::new()),
            Arc::new(Mutex::new(None)),
            Arc::new(Mutex::new(HashSet::new())),
            errors.clone(),
            None,
            None,
            "nse_cm|2885",
        )
        .await;
        assert!(errors.lock().expect("errors").is_empty());
        assert!(book
            .lock()
            .expect("book")
            .get(KOTAK_NSE_BSE_CASH_BOOK_ID, "nse_cm|2885")
            .is_some());
    }

    #[tokio::test]
    async fn await_records_session_when_book_empty_and_locator_missing() {
        let registry = Arc::new(Registry::load(&[kotak_neo_quote_descriptor()]).unwrap());
        let book = Arc::new(Mutex::new(TickBook::new()));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(HashMap::new()));
        await_kotak_rest_quote(
            registry,
            book,
            Arc::new(crate::ubi::MemoryBrokerCredentialVault::new()),
            Arc::new(Mutex::new(None)),
            Arc::new(Mutex::new(HashSet::new())),
            errors.clone(),
            None,
            None,
            "nse_cm|2885",
        )
        .await;
        assert_eq!(
            errors
                .lock()
                .expect("errors")
                .get("nse_cm|2885")
                .map(String::as_str),
            Some("session")
        );
    }

    #[test]
    fn fo_body_applies_to_nfo_book_not_cash() {
        let registry = Registry::load(&[kotak_neo_quote_descriptor()]).unwrap();
        let mut book = TickBook::new();
        // Observed FO shape (probe 2026-08-31): `ltp` + `exchange`/`exchange_token`.
        let json =
            r#"[{"exchange":"nse_fo","exchange_token":"12345","ltp":"10.00","open_int":"480750"}]"#;
        assert_eq!(
            apply_kotak_quote_body(&registry, &mut book, json, Utc::now()),
            0
        );
        assert!(book
            .get(KOTAK_NSE_BSE_CASH_BOOK_ID, "nse_fo|12345")
            .is_none());
        assert_eq!(
            apply_kotak_quote_body_for_book(
                &registry,
                &mut book,
                json,
                Utc::now(),
                crate::data::KOTAK_NSE_NFO_BOOK_ID
            ),
            1
        );
        let row = book
            .get(crate::data::KOTAK_NSE_NFO_BOOK_ID, "nse_fo|12345")
            .unwrap();
        assert!(row.last.parse::<f64>().unwrap() > 0.0);
        assert_eq!(row.adapter_id, KOTAK_NEO);
        assert!(book
            .get(crate::data::KOTAK_NSE_NFO_BOOK_ID, "nse_cm|2885")
            .is_none());
    }

    #[test]
    fn quote_path_is_fenced_by_book() {
        use crate::data::KOTAK_NSE_NFO_BOOK_ID;
        let nfo = quotes_neosymbol_path("nse_fo|12345", QUOTE_TYPE_ALL);
        authorize_book_call(
            KOTAK_NSE_NFO_BOOK_ID,
            "gw-napi.kotaksecurities.com",
            "GET",
            &nfo,
            true,
        )
        .expect("nfo book may GET nse_fo quotes");
        assert!(authorize_book_call(
            KOTAK_NSE_BSE_CASH_BOOK_ID,
            "gw-napi.kotaksecurities.com",
            "GET",
            &nfo,
            true,
        )
        .is_err());
        let cash = quotes_neosymbol_path("nse_cm|2885", QUOTE_TYPE_ALL);
        authorize_book_call(
            KOTAK_NSE_BSE_CASH_BOOK_ID,
            "gw-napi.kotaksecurities.com",
            "GET",
            &cash,
            true,
        )
        .expect("cash book may GET nse_cm quotes");
        assert!(authorize_book_call(
            KOTAK_NSE_NFO_BOOK_ID,
            "gw-napi.kotaksecurities.com",
            "GET",
            &cash,
            true,
        )
        .is_err());
        assert_eq!(
            kotak_quote_book_id("nse_fo|12345"),
            Some(KOTAK_NSE_NFO_BOOK_ID)
        );
        assert_eq!(
            kotak_quote_book_id("nse_cm|2885"),
            Some(KOTAK_NSE_BSE_CASH_BOOK_ID)
        );
    }

    #[tokio::test]
    async fn await_nfo_skips_http_when_nfo_slot_has_last() {
        use crate::data::KOTAK_NSE_NFO_BOOK_ID;
        let registry = Arc::new(Registry::load(&[kotak_neo_quote_descriptor()]).unwrap());
        let mut book = TickBook::new();
        // Observed FO shape (probe 2026-08-31): `ltp` + `exchange`/`exchange_token`.
        let json =
            r#"[{"exchange":"nse_fo","exchange_token":"12345","ltp":"10.00","open_int":"480750"}]"#;
        apply_kotak_quote_body_for_book(
            registry.as_ref(),
            &mut book,
            json,
            Utc::now(),
            KOTAK_NSE_NFO_BOOK_ID,
        );
        let book = Arc::new(Mutex::new(book));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(HashMap::new()));
        await_kotak_rest_quote(
            registry,
            book.clone(),
            Arc::new(crate::ubi::MemoryBrokerCredentialVault::new()),
            Arc::new(Mutex::new(None)),
            Arc::new(Mutex::new(HashSet::new())),
            errors.clone(),
            None,
            None,
            "nse_fo|12345",
        )
        .await;
        assert!(errors.lock().expect("errors").is_empty());
        assert!(book
            .lock()
            .expect("book")
            .get(KOTAK_NSE_NFO_BOOK_ID, "nse_fo|12345")
            .is_some());
    }

    #[tokio::test]
    async fn await_nfo_records_session_without_cash_master() {
        let registry = Arc::new(Registry::load(&[kotak_neo_quote_descriptor()]).unwrap());
        let book = Arc::new(Mutex::new(TickBook::new()));
        let errors: QuoteFetchErrorMap = Arc::new(Mutex::new(HashMap::new()));
        await_kotak_rest_quote(
            registry,
            book,
            Arc::new(crate::ubi::MemoryBrokerCredentialVault::new()),
            Arc::new(Mutex::new(None)),
            Arc::new(Mutex::new(HashSet::new())),
            errors.clone(),
            None,
            None,
            "nse_fo|12345",
        )
        .await;
        assert_eq!(
            errors
                .lock()
                .expect("errors")
                .get("nse_fo|12345")
                .map(String::as_str),
            Some("session")
        );
    }
}

/// Bucket 2 research probe — **not** a CI test. Answers exactly two questions
/// against a live logged-in session: what JSON key is *last* on `nse_fo|{token}`,
/// and what key (if any) is *open interest* on the same path.
///
/// `#[ignore]` plus a required env var: this makes a real authenticated
/// `PrivateRead` GET to Kotak with the founder's session, so it must never run
/// from a bare `cargo test`. Run it deliberately:
///
/// ```text
/// KOTAK_FO_PROBE=1 KOTAK_FO_PROBE_TOKEN=56526 \
///   cargo test --lib fo_quotes_field_probe -- --ignored --nocapture
/// ```
///
/// It prints **key names and JSON types only** — never a value, never a header
/// value, never the body. The redacted example object is `{"key": "<type>"}`, so
/// the output is safe to paste into the lock.
#[cfg(test)]
mod fo_quotes_field_probe {
    use super::*;
    use crate::ubi::BrokerCredentialVault as _;
    use crate::ubi::{prepare_kotak_catalog_get, HostCredentialBlob, KeyringBrokerCredentialVault};
    use serde_json::Value;

    /// Type name only. A value never reaches the transcript.
    fn type_of(value: &Value) -> &'static str {
        match value {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    /// `root_keys=["<array>"]` / `["message"]` / sorted object keys — the same
    /// shape line the cash row records in REST.md.
    fn root_shape(value: &Value) -> String {
        match value {
            Value::Array(rows) => format!("root_keys=[\"<array>\"] len={}", rows.len()),
            Value::Object(map) => {
                let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
                keys.sort_unstable();
                format!("root_keys={keys:?}")
            }
            other => format!("root is {}", type_of(other)),
        }
    }

    /// First row of whatever envelope came back: bare array, `{message:[…]}`, or
    /// the object itself.
    fn first_row(value: &Value) -> Option<&Value> {
        match value {
            Value::Array(rows) => rows.first(),
            Value::Object(map) => map
                .get("message")
                .or_else(|| map.get("data"))
                .and_then(|inner| match inner {
                    Value::Array(rows) => rows.first(),
                    object @ Value::Object(_) => Some(object),
                    _ => None,
                })
                .or(Some(value)),
            _ => None,
        }
    }

    /// Every key whose *name* could plausibly carry a last price or an OI, so the
    /// report names what is actually present instead of asserting a guess. This
    /// list decides nothing — the lock cites whatever the body turns out to name.
    fn candidates(row: &Value, needles: &[&str]) -> Vec<String> {
        let Value::Object(map) = row else {
            return Vec::new();
        };
        map.iter()
            .filter(|(key, _)| {
                let lower = key.to_ascii_lowercase();
                needles.iter().any(|needle| lower.contains(needle))
            })
            .map(|(key, value)| format!("{key} ({})", type_of(value)))
            .collect()
    }

    async fn probe(quote_type: &str, instrument_id: &str) {
        let vault = KeyringBrokerCredentialVault::new();
        let connection_id = std::env::var("KOTAK_FO_PROBE_CONNECTION")
            .unwrap_or_else(|_| "00000000-0000-4000-8000-000000000003".to_string());
        let blob = match vault.load("prod", KOTAK_NEO, &connection_id) {
            Ok(Some(blob)) => blob,
            other => {
                println!("[{quote_type}] SESSION: no stored Kotak session ({other:?}) — this is a session problem, not a schema answer. Lock stays NOT SPECIFIED.");
                return;
            }
        };
        let creds = HostCredentialBlob::from(&blob);
        let HostCredentialBlob::KotakSession { ref base_url, .. } = creds else {
            println!("[{quote_type}] SESSION: stored blob is not a Kotak session.");
            return;
        };
        let host = match kotak_base_host(base_url) {
            Some(host) => host,
            None => {
                println!("[{quote_type}] SESSION: baseUrl has no host.");
                return;
            }
        };
        let path = quotes_neosymbol_path(instrument_id, quote_type);
        let book_id = match kotak_quote_book_id(instrument_id) {
            Some(id) => id,
            None => {
                println!("[{quote_type}] {instrument_id} is not a Kotak quote id.");
                return;
            }
        };
        let prepared = match prepare_kotak_catalog_get(&path, &creds) {
            Ok(prepared) => prepared,
            Err(err) => {
                println!("[{quote_type}] PREPARE failed: {err}");
                return;
            }
        };
        // Header NAMES only — values are session secrets.
        let header_names: Vec<&str> = prepared
            .headers
            .iter()
            .map(|(name, _)| name.as_str())
            .collect();
        println!("\n───── quote_type={quote_type} · instrument={instrument_id} · book={book_id}");
        println!("host: {host}");
        println!("path: {path}");
        println!("session headers attached (names only): {header_names:?}");

        let resp = match crate::egress::shared()
            .send_prepared(
                book_id,
                crate::egress::Lane::PrivateRead,
                &prepared,
                Duration::from_secs(20),
            )
            .await
        {
            Ok(resp) => resp,
            Err(err) => {
                println!("EGRESS refused/failed: {err:?} — session or fence, not a schema answer.");
                return;
            }
        };
        println!("HTTP status: {}", resp.status);
        if resp.status == 401 || resp.status == 403 {
            println!("=> Session, not schema. Do not parse. Lock stays NOT SPECIFIED.");
            return;
        }
        if !resp.is_success() {
            println!("=> Non-2xx. Lock stays NOT SPECIFIED.");
            return;
        }
        let Ok(value) = serde_json::from_str::<Value>(&resp.body) else {
            println!("body did not parse as JSON (len={})", resp.body.len());
            return;
        };
        println!("{}", root_shape(&value));
        let Some(row) = first_row(&value) else {
            println!("no first row to read keys from");
            return;
        };
        if let Value::Object(map) = row {
            let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
            keys.sort_unstable();
            println!("first-row keys (sorted): {keys:?}");
            // Redacted example object: names + types, no values.
            let redacted: Vec<String> = keys
                .iter()
                .map(|key| format!("\"{key}\": \"<{}>\"", type_of(&map[*key])))
                .collect();
            println!("redacted example row: {{{}}}", redacted.join(", "));
        } else {
            println!("first row is {} not an object", type_of(row));
        }
        println!(
            "LAST candidates: {:?}",
            candidates(row, &["ltp", "last", "close", "price"])
        );
        println!(
            "OI candidates: {:?}",
            candidates(row, &["oi", "openinterest", "open_interest", "interest"])
        );
        println!(
            "IDENTITY candidates: {:?}",
            candidates(row, &["exchange", "token", "segment", "symbol"])
        );
        // Instrument identity values only — public market metadata, never a secret.
        // Needed because TickBook builds `{exchange}|{exchange_token}`: the lock has
        // to say whether an FO row reports `nse_fo` there, or something else.
        if let Value::Object(map) = row {
            for key in ["exchange", "exchange_token", "display_symbol"] {
                if let Some(Value::String(value)) = map.get(key) {
                    println!("  identity {key} = {value:?}");
                }
            }
            // Do the named last / OI strings parse, and are they positive? A
            // quoted price is public market data, not a secret, and `"0"` vs a
            // real number is the difference between "unusable" and "a reading".
            for key in ["ltp", "open_int", "oi_las", "oi_high", "oi_low"] {
                if let Some(Value::String(value)) = map.get(key) {
                    // The numeric value is public market data, not a secret, and
                    // whether it can be `0` decides whether last is usable.
                    println!(
                        "  {key}: string {value:?} parses_as_f64={} positive={}",
                        value.trim().parse::<f64>().is_ok(),
                        value
                            .trim()
                            .parse::<f64>()
                            .map(|n| n > 0.0)
                            .unwrap_or(false)
                    );
                }
            }
        }
    }

    /// Live capture for PR A1 — `quote_type=depth` on `nse_fo|56526`.
    /// Writes the raw body to `fixtures/kotak/quotes_neosymbol_nfo_depth.json`.
    ///
    /// ```text
    /// KOTAK_NFO_DEPTH_CAPTURE=1 cargo test --lib nfo_depth_live_capture -- --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore = "live authenticated Kotak GET; run deliberately with KOTAK_NFO_DEPTH_CAPTURE=1"]
    async fn nfo_depth_live_capture() {
        if std::env::var("KOTAK_NFO_DEPTH_CAPTURE").is_err() {
            println!("set KOTAK_NFO_DEPTH_CAPTURE=1 to run the live depth capture");
            return;
        }
        let vault = KeyringBrokerCredentialVault::new();
        let connection_id = std::env::var("KOTAK_FO_PROBE_CONNECTION")
            .unwrap_or_else(|_| "00000000-0000-4000-8000-000000000003".to_string());
        let instrument_id = std::env::var("KOTAK_NFO_DEPTH_INSTRUMENT")
            .unwrap_or_else(|_| "nse_fo|56526".to_string());
        let body = match fetch_quotes_json(
            &vault,
            "prod",
            &connection_id,
            &instrument_id,
            QUOTE_TYPE_DEPTH,
        )
        .await
        {
            Ok(body) => body,
            Err(err) => {
                panic!(
                    "live depth capture failed ({err}) — session expired or HTTP refused; do not hand-author fixture"
                );
            }
        };
        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/kotak/quotes_neosymbol_nfo_depth.json");
        std::fs::write(&out, format!("{body}\n")).expect("write fixture");
        println!("wrote {} bytes to {}", body.len(), out.display());
        let Ok(value) = serde_json::from_str::<Value>(&body) else {
            panic!("body is not valid JSON");
        };
        println!("{}", root_shape(&value));
        if let Some(row) = first_row(&value) {
            if let Value::Object(map) = row {
                let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
                keys.sort_unstable();
                println!("first-row keys (sorted): {keys:?}");
                if let Some(depth) = map.get("depth") {
                    println!("depth field type: {}", type_of(depth));
                    if let Value::Object(depth_map) = depth {
                        let mut depth_keys: Vec<&str> =
                            depth_map.keys().map(String::as_str).collect();
                        depth_keys.sort_unstable();
                        println!("depth object keys: {depth_keys:?}");
                        for side in ["buy", "sell", "bids", "asks", "bid", "ask"] {
                            if let Some(levels) = depth_map.get(side).and_then(Value::as_array) {
                                if let Some(first) = levels.first() {
                                    if let Value::Object(level) = first {
                                        let mut lk: Vec<&str> =
                                            level.keys().map(String::as_str).collect();
                                        lk.sort_unstable();
                                        println!("  {side}[0] level keys: {lk:?}");
                                    }
                                }
                            }
                        }
                    }
                } else {
                    println!("no top-level depth key on first row");
                }
            }
        }
    }

    #[tokio::test]
    #[ignore = "live authenticated Kotak GET; run deliberately with KOTAK_FO_PROBE=1"]
    async fn fo_quotes_field_probe() {
        if std::env::var("KOTAK_FO_PROBE").is_err() {
            println!("set KOTAK_FO_PROBE=1 to run the live probe");
            return;
        }
        let token = std::env::var("KOTAK_FO_PROBE_TOKEN").unwrap_or_else(|_| "56526".to_string());
        let instrument_id = format!("nse_fo|{token}");
        println!(
            "Bucket 2 probe · {} · instrument={instrument_id}",
            chrono::Local::now().to_rfc3339()
        );
        // Order matters: `all` is what Station already dials, `ltp` confirms the
        // same key on a thinner slice, `oi` is the only thing that answers O1.
        for quote_type in ["all", "ltp", "oi"] {
            probe(quote_type, &instrument_id).await;
        }
    }
}
