//! Host-mediated Kotak REST quotes (`quote_type=all`) into TickBook and
//! `quote_type=depth` into DepthBook (bounded snapshot, not HSM `isDepth`).
//!
//! Session attach via `prepare_kotak_catalog_get` (consumer-key Authorization,
//! Auth + Sid, no trade-book `sId`).
//! HSM websocket skipped (WEBSOCKET.md: `mlhsm` not HTTP-allowlisted; `hsServerId` unspecified).

use crate::data::{
    apply_quote, authorize_book_call, depth_snapshots_from_kotak_json, is_cash_segment,
    json_array_first_object_keys, json_field_object_keys, json_first_nested_object_keys,
    json_object_keys, kotak_quote_book_id, quote_ticks_from_kotak_json_for_book,
    quotes_neosymbol_path, DepthBook, Registry, TickBook, KOTAK_NSE_BSE_CASH_BOOK_ID,
    QUOTE_TYPE_ALL, QUOTE_TYPE_DEPTH,
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
pub fn ensure_kotak_rest_quote(
    registry: Arc<Registry>,
    book: Arc<Mutex<TickBook>>,
    vault: Arc<dyn BrokerCredentialVault>,
    locator: SessionLocator,
    inflight: &Arc<Mutex<HashSet<String>>>,
    quote_fetch_error: QuoteFetchErrorMap,
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
            &instrument,
        )
        .await;
    });
}

/// Same REST GET as `ensure_kotak_rest_quote`, but the caller waits until TickBook
/// or `quote_fetch_error` is set. `GET /api/station/quote` uses this so Last is not
/// painted from an empty book while the fetch is still in flight. Does not poll Kotak.
pub async fn await_kotak_rest_quote(
    registry: Arc<Registry>,
    book: Arc<Mutex<TickBook>>,
    vault: Arc<dyn BrokerCredentialVault>,
    locator: SessionLocator,
    inflight: Arc<Mutex<HashSet<String>>>,
    quote_fetch_error: QuoteFetchErrorMap,
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

pub fn apply_kotak_quote_body(
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
    let segment = instrument.split('|').next().unwrap_or_default();
    if !is_cash_segment(segment) {
        return;
    }
    {
        let guard = book.lock().expect("depthbook mutex poisoned");
        if guard
            .get(KOTAK_NSE_BSE_CASH_BOOK_ID, &instrument)
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

async fn fetch_and_apply(
    registry: &Registry,
    book: &Arc<Mutex<TickBook>>,
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
        QUOTE_TYPE_ALL,
    )
    .await?;
    let received_at = Utc::now();
    let book_id = kotak_quote_book_id(instrument_id).ok_or(QuoteFetchError {
        class: QuoteFetchErrorClass::QuotesHttp,
    })?;
    let mut guard = book.lock().expect("tickbook mutex poisoned");
    let applied =
        apply_kotak_quote_body_for_book(registry, &mut guard, &body, received_at, book_id);
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
        HostCredentialBlob::Hmac { .. } => {
            return Err(QuoteFetchError {
                class: QuoteFetchErrorClass::Session,
            });
        }
    };
    let host = kotak_base_host(&base_url).ok_or(QuoteFetchError {
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
        depth_snapshot_from_kotak_json, kotak_neo_quote_descriptor, quote_tick_from_kotak_json,
        DepthBook, Registry, TickBook,
    };
    use std::collections::{HashMap, HashSet};
    use std::sync::{Arc, Mutex};

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
        let json = r#"{"instrument_token":"12345","exchange_segment":"nse_fo","last_traded_price":"10.00"}"#;
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
        let json = r#"{"instrument_token":"12345","exchange_segment":"nse_fo","last_traded_price":"10.00"}"#;
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
