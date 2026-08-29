//! S1 desk helpers: resolve/subscribe against the connected broker, not env-only.

use super::AppState;
use crate::data::{
    apply_history_series, authorize_inferred_call, binance_exchange_info_cache_path,
    ensure_binance_com_depth_stream, ensure_binance_com_trade_stream, extract_quote_for_book,
    normalize_quote_instrument, parse_nfo_instrument_id, resolve_among, series_from_klines_json,
    validate_kline_request, write_raw_cache, HistoryBook, InstrumentMasterErrorClass,
    InstrumentMasterFetchError, InstrumentMasterStatus, QuoteStatus, Transport,
    DEFAULT_HISTORY_INTERVAL, KLINE_LIMIT_DEFAULT, KOTAK_NSE_NFO_BOOK_ID,
};
use crate::exchange_info::ExchangeInfoSymbolCache;
use crate::kotak_scrip_master::{self, KotakScripMaster, KOTAK_NEO};
use crate::ubi::BrokerCredentialVault;
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

const BINANCE_COM: &str = "binance_com";
const REFRESH_BACKOFFS: [Duration; 3] = [
    Duration::from_secs(5),
    Duration::from_secs(30),
    Duration::from_secs(120),
];

pub(crate) fn instrument_master_target(slug: &str) -> Option<&'static str> {
    match slug {
        BINANCE_COM => Some(BINANCE_COM),
        KOTAK_NEO => Some(KOTAK_NEO),
        _ => None,
    }
}

pub fn spawn_instrument_master_refresh(
    state: &AppState,
    slug: &str,
    environment: &str,
    connection_id: &str,
) {
    let slug = slug.trim().to_ascii_lowercase();
    if !state.source_manifests.iter().any(|manifest| {
        manifest.adapter_id == slug && manifest.implemented.iter().any(|op| op == "instruments")
    }) {
        return;
    }
    match instrument_master_target(&slug) {
        Some(BINANCE_COM) => {
            try_load_binance_cache(
                &state.instrument_master_cache_dir,
                &state.instrument_master,
                &state.instrument_master_status,
            );
            spawn_exchange_info_refresh(
                state.instrument_master.clone(),
                state.instrument_master_status.clone(),
                state.instrument_master_cancel.clone(),
                state.instrument_master_cache_dir.clone(),
                state.broker_connections.clone(),
                true,
            );
        }
        Some(KOTAK_NEO) => {
            {
                let mut loc = state
                    .kotak_session_locator
                    .lock()
                    .expect("kotak session locator poisoned");
                *loc = Some((environment.to_string(), connection_id.to_string()));
            }
            kotak_scrip_master::try_load_kotak_cache(
                &state.instrument_master_cache_dir,
                &state.kotak_scrip_master,
                &state.instrument_master_status,
            );
            spawn_kotak_scrip_master_refresh(
                state.kotak_scrip_master.clone(),
                state.broker_sync_control.credential_vault(),
                environment.to_string(),
                connection_id.to_string(),
                state.instrument_master_status.clone(),
                state.instrument_master_cancel.clone(),
                state.instrument_master_cache_dir.clone(),
                state.broker_connections.clone(),
                state.kotak_session_locator.clone(),
            );
        }
        _ => {}
    }
}

/// Named NFO book catalog. Cash `spawn_instrument_master_refresh` must not call this.
pub fn spawn_nfo_master_refresh(state: &AppState, environment: &str, connection_id: &str) {
    if !state.source_manifests.iter().any(|manifest| {
        manifest.book_id == crate::data::KOTAK_NSE_NFO_BOOK_ID
            && manifest.implemented.iter().any(|op| op == "instruments")
    }) {
        return;
    }
    crate::kotak_nfo_scrip::try_load_nfo_cache(
        &state.instrument_master_cache_dir,
        &state.kotak_nfo_scrip_master,
    );
    crate::kotak_nfo_scrip::spawn_refresh(
        state.kotak_nfo_scrip_master.clone(),
        state.broker_sync_control.credential_vault(),
        environment.to_string(),
        connection_id.to_string(),
        state.instrument_master_cancel.clone(),
        state.instrument_master_cache_dir.clone(),
        state.broker_connections.clone(),
        state.kotak_session_locator.clone(),
    );
}

impl AppState {
    pub fn active_adapter_id(&self) -> Option<String> {
        {
            let st = self
                .broker_status
                .lock()
                .expect("broker_status mutex poisoned");
            if let Some(slug) = st.active_broker_slug.as_ref() {
                let trimmed = slug.trim();
                if !trimmed.is_empty() {
                    return Some(trimmed.to_ascii_lowercase());
                }
            }
        }
        let map = self
            .broker_connections
            .lock()
            .expect("broker connections mutex poisoned");
        map.values()
            .next()
            .map(|runtime| runtime.adapter_id.clone())
    }

    pub fn is_binance_com_desk(&self) -> bool {
        self.active_adapter_id()
            .is_some_and(|slug| slug == BINANCE_COM)
    }

    pub fn is_kotak_neo_desk(&self) -> bool {
        self.active_adapter_id()
            .is_some_and(|slug| slug == KOTAK_NEO)
    }

    fn runtime_subscriptions(&self) -> Vec<String> {
        let slug = self.active_adapter_id();
        let map = self
            .broker_connections
            .lock()
            .expect("broker connections mutex poisoned");
        slug.as_deref()
            .and_then(|id| map.get(id))
            .map(|runtime| runtime.subscriptions.clone())
            .unwrap_or_default()
    }

    pub fn resolve_candidates(&self) -> Vec<String> {
        let mut candidates = self.runtime_subscriptions();
        if self.is_kotak_neo_desk() {
            let master = self
                .kotak_scrip_master
                .lock()
                .expect("kotak scrip master mutex poisoned");
            for id in master.iter_instrument_ids() {
                if !candidates.iter().any(|s| s == &id) {
                    candidates.push(id);
                }
            }
            return candidates;
        }
        if let Some(env) = &self.s1_desk_symbol {
            if !candidates.iter().any(|s| s == env) {
                candidates.push(env.clone());
            }
        }
        let master = self
            .instrument_master
            .lock()
            .expect("instrument master mutex poisoned");
        for symbol in master.iter_symbols() {
            let id = normalize_quote_instrument(symbol);
            if !candidates.iter().any(|s| s == &id) {
                candidates.push(id);
            }
        }
        candidates
    }

    pub fn resolve_instrument(&self, raw: &str) -> String {
        if self.is_kotak_neo_desk() {
            let master = self
                .kotak_scrip_master
                .lock()
                .expect("kotak scrip master mutex poisoned");
            if let Some(id) = master.resolve_id(raw) {
                return id;
            }
            if let Some((segment, token)) = kotak_scrip_master::parse_instrument_id(raw) {
                return kotak_scrip_master::instrument_id(&segment, token);
            }
        }
        resolve_among(raw, self.resolve_candidates().iter().map(String::as_str))
            .unwrap_or_else(|| normalize_quote_instrument(raw))
    }

    pub fn subscribe_instrument(&self, instrument: &str) {
        let id = normalize_quote_instrument(instrument);
        if id.is_empty() {
            return;
        }
        let slug = self.active_adapter_id();
        {
            let mut map = self
                .broker_connections
                .lock()
                .expect("broker connections mutex poisoned");
            if let Some(key) = slug.as_deref() {
                if let Some(runtime) = map.get_mut(key) {
                    runtime.subscribe(&id);
                }
            }
        }
        if self.is_binance_path(&id) {
            ensure_binance_com_trade_stream(
                self.quote_registry.clone(),
                self.tickbook.clone(),
                &self.quote_streams,
                &id,
            );
            ensure_binance_com_depth_stream(self.depthbook.clone(), &self.depth_streams, &id);
            // REST.md: unsigned GET /api/v3/klines on api.binance.com (NONE). Not uiKlines.
            ensure_binance_klines(self.historybook.clone(), &self.klines_inflight, &id);
        }
        if self.is_kotak_neo_desk() {
            self.kick_kotak_rest_quote(&id);
        }
    }

    /// Fire-and-forget Kotak REST quote + depth. Prefer `prime_kotak_quote` on the
    /// quote extract so Last waits for the first GET.
    pub fn kick_kotak_rest_quote(&self, instrument: &str) {
        let id = instrument.trim().to_ascii_lowercase();
        if !id.contains('|') {
            return;
        }
        let in_master = self
            .kotak_scrip_master
            .lock()
            .expect("kotak scrip master mutex poisoned")
            .contains_id(&id);
        if !in_master && parse_nfo_instrument_id(&id).is_none() {
            return;
        }
        crate::kotak_rest_quotes::ensure_kotak_rest_quote(
            self.quote_registry.clone(),
            self.tickbook.clone(),
            self.broker_sync_control.credential_vault(),
            self.kotak_session_locator.clone(),
            &self.kotak_quote_inflight,
            self.quote_fetch_error.clone(),
            &id,
        );
        crate::kotak_rest_quotes::ensure_kotak_rest_depth(
            self.depthbook.clone(),
            self.broker_sync_control.credential_vault(),
            self.kotak_session_locator.clone(),
            &self.kotak_depth_inflight,
            &id,
        );
    }

    /// Wait for the first Kotak REST quote (or a session/HTTP class) before the
    /// extract returns. Depth stays fire-and-forget.
    pub async fn prime_kotak_quote(&self, instrument: &str) {
        let id = instrument.trim().to_ascii_lowercase();
        if !id.contains('|') {
            return;
        }
        let in_master = self
            .kotak_scrip_master
            .lock()
            .expect("kotak scrip master mutex poisoned")
            .contains_id(&id);
        if !in_master && parse_nfo_instrument_id(&id).is_none() {
            return;
        }
        crate::kotak_rest_quotes::await_kotak_rest_quote(
            self.quote_registry.clone(),
            self.tickbook.clone(),
            self.broker_sync_control.credential_vault(),
            self.kotak_session_locator.clone(),
            self.kotak_quote_inflight.clone(),
            self.quote_fetch_error.clone(),
            &id,
        )
        .await;
        crate::kotak_rest_quotes::ensure_kotak_rest_depth(
            self.depthbook.clone(),
            self.broker_sync_control.credential_vault(),
            self.kotak_session_locator.clone(),
            &self.kotak_depth_inflight,
            &id,
        );
    }

    pub fn validated_quote_id(&self, raw: &str) -> Option<String> {
        let raw = raw.trim();
        if raw.is_empty() {
            return None;
        }
        if self.is_kotak_neo_desk() {
            if let Some(id) = parse_nfo_instrument_id(raw) {
                return Some(id);
            }
            let (segment, token) = kotak_scrip_master::parse_instrument_id(raw)?;
            let id = kotak_scrip_master::instrument_id(&segment, token);
            let master = self
                .kotak_scrip_master
                .lock()
                .expect("kotak scrip master mutex poisoned");
            if master.contains_id(&id) {
                return Some(id);
            }
            return None;
        }
        let id = self.resolve_instrument(raw);
        if self.is_binance_com_desk() {
            if self.is_binance_path(&id) {
                return Some(id);
            }
            return None;
        }
        if id.is_empty() {
            None
        } else {
            Some(id)
        }
    }

    pub fn should_subscribe_quote(&self, id: &str) -> bool {
        if id.is_empty() {
            return false;
        }
        if self.is_kotak_neo_desk() {
            return id.contains('|')
                && self
                    .kotak_scrip_master
                    .lock()
                    .expect("kotak scrip master mutex poisoned")
                    .contains_id(id);
        }
        if self.is_binance_com_desk() {
            return self.is_binance_path(id);
        }
        false
    }

    pub fn is_binance_path(&self, instrument: &str) -> bool {
        if self.is_kotak_neo_desk() {
            return false;
        }
        let id = normalize_quote_instrument(instrument);
        if self
            .active_adapter_id()
            .is_some_and(|slug| slug == BINANCE_COM)
        {
            return true;
        }
        if self
            .instrument_master
            .lock()
            .expect("instrument master mutex poisoned")
            .contains_symbol(&id)
        {
            return true;
        }
        self.runtime_subscriptions().iter().any(|s| s == &id)
            || self
                .s1_desk_symbol
                .as_deref()
                .is_some_and(|env| normalize_quote_instrument(env) == id)
    }

    pub fn quote_capability_status(&self) -> &'static str {
        let selected = self
            .selected_quote_instrument
            .lock()
            .expect("selected quote instrument poisoned")
            .clone();
        let Some(instrument) = selected.filter(|s| !s.is_empty()) else {
            return "unavailable";
        };
        let adapter = self.active_adapter_id();
        let named_book = parse_nfo_instrument_id(&instrument).map(|_| KOTAK_NSE_NFO_BOOK_ID);
        let book = self.tickbook.lock().expect("tickbook mutex poisoned");
        let env = extract_quote_for_book(
            self.quote_registry.as_ref(),
            &book,
            &instrument,
            Utc::now(),
            self.quote_freshness,
            adapter.as_deref(),
            named_book,
        );
        if env.status != QuoteStatus::Unavailable {
            return quote_status_wire(env.status);
        }
        drop(book);
        let class = self
            .quote_fetch_error
            .lock()
            .expect("quote fetch error poisoned")
            .get(&instrument)
            .cloned();
        match class.as_deref() {
            Some("quotes_http") => "quotes_http",
            Some("session") => "session",
            Some("quotes_unusable") => "quotes_unusable",
            _ => "unavailable",
        }
    }

    pub fn instrument_master_wire(&self) -> &'static str {
        self.instrument_master_status
            .lock()
            .expect("instrument master status poisoned")
            .capability_wire()
    }

    pub fn maybe_kick_instrument_master_refresh(&self) {
        if !(self.is_binance_com_desk() || self.is_kotak_neo_desk()) {
            return;
        }
        let Some(slug) = self.active_adapter_id() else {
            return;
        };
        {
            let mut st = self
                .instrument_master_status
                .lock()
                .expect("instrument master status poisoned");
            if !st.try_begin_loading(&slug) {
                return;
            }
        }
        let (env, conn) = if slug == KOTAK_NEO {
            match self
                .kotak_session_locator
                .lock()
                .expect("kotak session locator poisoned")
                .clone()
            {
                Some(pair) => pair,
                None => {
                    self.instrument_master_status
                        .lock()
                        .expect("instrument master status poisoned")
                        .mark_idle();
                    return;
                }
            }
        } else {
            (String::new(), String::new())
        };
        spawn_instrument_master_refresh(self, &slug, &env, &conn);
    }
}

pub fn quote_status_wire(status: QuoteStatus) -> &'static str {
    match status {
        QuoteStatus::Fresh => "fresh",
        QuoteStatus::Stale => "stale",
        QuoteStatus::Unknown => "unknown",
        QuoteStatus::Unavailable => "unavailable",
    }
}

pub fn class_freshness_wire(current: bool, last_success_at_ms: Option<i64>) -> &'static str {
    if current {
        "fresh"
    } else if last_success_at_ms.is_some() {
        "stale"
    } else {
        "unavailable"
    }
}

pub fn try_load_binance_cache(
    cache_dir: &Path,
    master: &Mutex<ExchangeInfoSymbolCache>,
    status: &Mutex<InstrumentMasterStatus>,
) {
    let path = binance_exchange_info_cache_path(cache_dir);
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return;
    };
    let Ok(cache) = ExchangeInfoSymbolCache::from_exchange_info_json(text) else {
        return;
    };
    if cache.is_empty() {
        return;
    }
    let n = cache.len();
    *master.lock().expect("instrument master mutex poisoned") = cache;
    status
        .lock()
        .expect("instrument master status poisoned")
        .record_cache_rows(BINANCE_COM, n);
    tracing::info!(
        symbols = n,
        "s1 desk: exchangeInfo master loaded from disk cache"
    );
}

pub fn spawn_exchange_info_refresh(
    master: Arc<Mutex<ExchangeInfoSymbolCache>>,
    status: Arc<Mutex<InstrumentMasterStatus>>,
    cancel: Arc<AtomicBool>,
    cache_dir: PathBuf,
    connections: Arc<Mutex<HashMap<String, crate::data::BrokerConnectionRuntime>>>,
    require_started: bool,
) {
    {
        let mut st = status.lock().expect("instrument master status poisoned");
        st.mark_loading(BINANCE_COM);
    }
    tokio::spawn(async move {
        let mut attempt = 0u8;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            if require_started
                && !connections
                    .lock()
                    .ok()
                    .is_some_and(|m| m.contains_key(BINANCE_COM))
            {
                return;
            }
            match fetch_com_exchange_info(&cache_dir).await {
                Ok(cache) => {
                    let n = cache.len();
                    if n == 0 {
                        let kept = master
                            .lock()
                            .expect("instrument master mutex poisoned")
                            .len();
                        status
                            .lock()
                            .expect("instrument master status poisoned")
                            .mark_error(BINANCE_COM, InstrumentMasterErrorClass::Empty, None, kept);
                    } else {
                        tracing::info!(symbols = n, "s1 desk: unsigned exchangeInfo master loaded");
                        *master.lock().expect("instrument master mutex poisoned") = cache;
                        status
                            .lock()
                            .expect("instrument master status poisoned")
                            .mark_loaded(BINANCE_COM, n);
                        return;
                    }
                }
                Err(err) => {
                    let kept = master
                        .lock()
                        .expect("instrument master mutex poisoned")
                        .len();
                    tracing::warn!(error = %err, "s1 desk: exchangeInfo fetch failed");
                    status
                        .lock()
                        .expect("instrument master status poisoned")
                        .mark_error(BINANCE_COM, err.class, err.http_status, kept);
                }
            }
            if attempt >= 3 {
                return;
            }
            let delay = REFRESH_BACKOFFS[attempt.min(2) as usize];
            attempt += 1;
            tokio::time::sleep(delay).await;
        }
    });
}

/// Public unsigned klines → HistoryBook once per instrument+interval.
/// REST.md: `GET /api/v3/klines` on `api.binance.com`, Security NONE. Never HMAC.
/// Never TickBook. Never Yahoo. Kotak desk never reaches this (not `is_binance_path`).
pub fn ensure_binance_klines(
    book: Arc<Mutex<HistoryBook>>,
    inflight: &Arc<Mutex<HashSet<String>>>,
    symbol: &str,
) {
    let instrument = normalize_quote_instrument(symbol);
    if instrument.is_empty() {
        return;
    }
    let interval = DEFAULT_HISTORY_INTERVAL;
    let key = format!("{instrument}\0{interval}");
    {
        let mut guard = inflight.lock().expect("klines inflight poisoned");
        if !guard.insert(key) {
            return;
        }
    }
    spawn_binance_klines_refresh(book, instrument);
}

/// Public unsigned klines → HistoryBook. Never HMAC. CI tests leave `s1_desk_symbol` unset.
pub fn spawn_binance_klines_refresh(book: Arc<Mutex<HistoryBook>>, symbol: String) {
    tokio::spawn(async move {
        match fetch_com_klines(&symbol, DEFAULT_HISTORY_INTERVAL, KLINE_LIMIT_DEFAULT).await {
            Ok(series) => {
                tracing::info!(
                    instrument = %series.instrument_id,
                    candles = series.candles.len(),
                    "s2 desk: unsigned klines series stored"
                );
                let mut guard = book.lock().expect("historybook mutex poisoned");
                apply_history_series(&mut guard, series);
            }
            Err(err) => {
                tracing::warn!(error = %err, "s2 desk: klines fetch failed");
            }
        }
    });
}

pub fn spawn_kotak_scrip_master_refresh(
    master: Arc<Mutex<KotakScripMaster>>,
    vault: Arc<dyn BrokerCredentialVault>,
    environment: String,
    connection_id: String,
    status: Arc<Mutex<InstrumentMasterStatus>>,
    cancel: Arc<AtomicBool>,
    cache_dir: PathBuf,
    connections: Arc<Mutex<HashMap<String, crate::data::BrokerConnectionRuntime>>>,
    locator: crate::kotak_rest_quotes::SessionLocator,
) {
    kotak_scrip_master::spawn_refresh(
        master,
        vault,
        environment,
        connection_id,
        status,
        cancel,
        cache_dir,
        connections,
        locator,
    );
}

async fn fetch_com_exchange_info(
    cache_dir: &Path,
) -> Result<ExchangeInfoSymbolCache, InstrumentMasterFetchError> {
    crate::data::authorize_inferred_call("api.binance.com", "GET", "/api/v3/exchangeInfo", false)
        .map_err(|_| {
        InstrumentMasterFetchError::new(InstrumentMasterErrorClass::ExchangeInfoHttp, None)
    })?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| {
            InstrumentMasterFetchError::new(InstrumentMasterErrorClass::ExchangeInfoHttp, None)
        })?;
    let resp = client
        .get("https://api.binance.com/api/v3/exchangeInfo")
        .send()
        .await
        .map_err(|_| {
            InstrumentMasterFetchError::new(InstrumentMasterErrorClass::ExchangeInfoHttp, None)
        })?;
    let status = resp.status();
    if !status.is_success() {
        return Err(InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::ExchangeInfoHttp,
            Some(status.as_u16()),
        ));
    }
    let body = resp.text().await.map_err(|_| {
        InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::ExchangeInfoHttp,
            Some(status.as_u16()),
        )
    })?;
    let cache = ExchangeInfoSymbolCache::from_exchange_info_json(&body).map_err(|_| {
        InstrumentMasterFetchError::new(InstrumentMasterErrorClass::Empty, Some(status.as_u16()))
    })?;
    if cache.is_empty() {
        return Err(InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::Empty,
            Some(status.as_u16()),
        ));
    }
    write_raw_cache(
        &binance_exchange_info_cache_path(cache_dir),
        body.as_bytes(),
    );
    Ok(cache)
}

async fn fetch_com_klines(
    symbol: &str,
    interval: &str,
    limit: u32,
) -> anyhow::Result<crate::data::HistorySeries> {
    authorize_inferred_call("api.binance.com", "GET", "/api/v3/klines", false)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    validate_kline_request(interval, Some(limit))
        .map_err(|e| anyhow::anyhow!("{}", e.as_ineligible()))?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;
    let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval={}&limit={}",
        symbol.trim().to_ascii_uppercase(),
        interval,
        limit
    );
    let body = client
        .get(&url)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    series_from_klines_json(&body, symbol, interval, Transport::Rest)
        .ok_or_else(|| anyhow::anyhow!("klines body unusable"))
}

pub fn account_identity(capability_id: &str) -> Value {
    json!({
        "family": "account",
        "capability_id": capability_id,
        "physics": "bounded_snapshot",
    })
}

pub fn reference_identity() -> Value {
    json!({
        "family": "reference",
        "capability_id": "instrument_master",
        "physics": "versioned_snapshot",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::QuoteStatus;

    #[test]
    fn quote_and_account_identities_stay_separate() {
        assert_eq!(quote_status_wire(QuoteStatus::Unavailable), "unavailable");
        let funds = account_identity("funds");
        assert_eq!(funds["family"], "account");
        assert_eq!(funds["capability_id"], "funds");
        assert_ne!(funds["capability_id"], "quote");
        let master = reference_identity();
        assert_eq!(master["family"], "reference");
    }

    #[test]
    fn dispatcher_binance_does_not_select_kotak_catalog() {
        assert_eq!(instrument_master_target("binance_com"), Some("binance_com"));
        assert_eq!(instrument_master_target("kotak_neo"), Some("kotak_neo"));
        assert_ne!(instrument_master_target("binance_com"), Some("kotak_neo"));
        assert_eq!(instrument_master_target("fixture"), None);
        assert_eq!(instrument_master_target("zerodha"), None);
    }
}
