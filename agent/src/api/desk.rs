//! S1 desk helpers: resolve/subscribe against the connected broker, not env-only.

use super::quote_selection::{
    is_known_quote_book, QuoteBindError, QuoteSource, ValidatedQuoteBinding,
};
use super::AppState;
use crate::data::{
    apply_history_series, binance_exchange_info_cache_path, ensure_binance_com_depth_stream,
    ensure_binance_com_options_quote, ensure_binance_com_trade_stream, extract_quote_for_book,
    is_dated_option_contract, kotak_quote_book_id, normalize_options_instrument,
    normalize_quote_instrument, parse_nfo_instrument_id, resolve_among, series_from_klines_json,
    validate_kline_request, write_raw_cache, HistoryBook, InstrumentMasterErrorClass,
    InstrumentMasterFetchError, InstrumentMasterStatus, MarketBind, QuoteStatus, Transport,
    BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID, DEFAULT_HISTORY_INTERVAL,
    KLINE_LIMIT_DEFAULT, KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID,
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

use crate::egress::{EgressCall, Lane};

/// Listing changes are rare; the symbol set does not need refetching per call.
const EXCHANGE_INFO_MAX_AGE_MS: i64 = 300_000;
/// Two charts on the same series are one call.
const KLINES_MAX_AGE_MS: i64 = 2_000;

const BINANCE_COM: &str = "binance_com";
const REFRESH_BACKOFFS: [Duration; 3] = [
    Duration::from_secs(5),
    Duration::from_secs(30),
    Duration::from_secs(120),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BindSpotKind {
    Options,
    Kotak,
    Spot,
}

/// Shape dispatch for COM bind. Dated never lowercases into the trade/depth slot.
pub(crate) fn bind_spot_market_ids(
    id: &str,
    trade: &MarketBind,
    depth: &MarketBind,
    quote_streams: &Arc<Mutex<HashSet<String>>>,
) -> BindSpotKind {
    if is_dated_option_contract(id) {
        ensure_binance_com_options_quote(quote_streams, id);
        trade.bind(None);
        depth.bind(None);
        return BindSpotKind::Options;
    }
    if id.contains('|') {
        return BindSpotKind::Kotak;
    }
    ensure_binance_com_trade_stream(trade, id);
    ensure_binance_com_depth_stream(depth, id);
    BindSpotKind::Spot
}

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
        if is_dated_option_contract(instrument) {
            let id = normalize_options_instrument(instrument);
            if id.is_empty() {
                return;
            }
            self.bind_spot_market(&id);
            return;
        }
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
        self.bind_spot_market(&id);
    }

    /// Dispatch on instrument *shape*, not `is_binance_path` (that is "COM desk is on").
    /// Dated → options record + unbind spot. Kotak `|` → REST kick. Else spot binds.
    pub fn bind_spot_market(&self, id: &str) {
        match bind_spot_market_ids(id, &self.com_trade, &self.com_depth, &self.quote_streams) {
            BindSpotKind::Options => {
                self.replace_klines_bind(None);
                self.kick_options_rest_ticker(id);
                self.kick_options_rest_depth(id);
            }
            BindSpotKind::Kotak => self.kick_kotak_rest_quote(id),
            BindSpotKind::Spot => {
                let spot = normalize_quote_instrument(id);
                self.replace_klines_bind(Some(spot.as_str()).filter(|s| !s.is_empty()));
            }
        }
    }

    /// Warm options last at bind time. One GET, gated on `eapi_public_fetch`
    /// and on the id actually being a dated contract; quote bind awaits the
    /// same ensure so Last lands before extract, and obtain kicks again when
    /// TickBook is still empty.
    ///
    /// This does **not** open `nbstream…/eoptions` — WS last is NOT SPECIFIED.
    /// REST `lastPrice` only, mixed-case `?symbol=`, no HMAC, never the
    /// unfiltered ticker dump.
    fn kick_options_rest_ticker(&self, id: &str) {
        if !self.eapi_public_fetch || !is_dated_option_contract(id) {
            return;
        }
        let state = self.clone();
        let instrument = crate::data::normalize_options_instrument(id);
        tokio::spawn(async move {
            super::glance::ensure_options_ticker(&state, &instrument).await;
        });
    }

    /// Warm the options order book at bind time, the way a Kotak bind kicks its
    /// REST depth. One GET, gated on `eapi_public_fetch` and on the id actually
    /// being a dated contract; obtain kicks again if the slot is still empty, and
    /// `ensure_options_depth` is a no-op once a complete ladder is resident.
    ///
    /// This does **not** open a COM `@depth` stream — the WS depth slot was just
    /// unbound for this id, and it stays that way.
    fn kick_options_rest_depth(&self, id: &str) {
        if !self.eapi_public_fetch || !is_dated_option_contract(id) {
            return;
        }
        let state = self.clone();
        let instrument = crate::data::normalize_options_instrument(id);
        tokio::spawn(async move {
            super::glance::ensure_options_depth(&state, &instrument).await;
        });
    }

    fn replace_klines_bind(&self, next: Option<&str>) {
        let interval = DEFAULT_HISTORY_INTERVAL;
        let old = self.com_klines.current();
        if old.as_deref() == next {
            return;
        }
        if let Some(old_id) = old {
            let key = format!("{old_id}\0{interval}");
            self.klines_inflight
                .lock()
                .expect("klines inflight poisoned")
                .remove(&key);
        }
        match next {
            Some(spot) if !spot.is_empty() => {
                self.com_klines.bind(Some(spot.to_string()));
                ensure_binance_klines(self.historybook.clone(), &self.klines_inflight, spot);
            }
            _ => self.com_klines.bind(None),
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
            Some(self.nfo_open_interest.clone()),
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
            Some(self.nfo_open_interest.clone()),
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

    /// Wait for one unsigned `GET /eapi/v1/ticker?symbol=` before extract.
    /// Options last has no specified WS field, so REST is the only last —
    /// unlike spot, this must land *instead of* a trade-stream subscribe.
    ///
    /// Gated inside `ensure_options_ticker`: tests stay fixture-only, and a
    /// leftover `BTC` / `BTCUSDT` never reaches eapi.
    pub async fn prime_binance_options_ticker(&self, instrument: &str) {
        super::glance::ensure_options_ticker(self, instrument).await;
    }

    /// Wait for one unsigned COM `GET /api/v3/ticker/price` before the caller
    /// subscribes this id. Ordering is load-bearing: once `subscribe_instrument`
    /// binds the trade stream, `apply_quote` refuses the REST tick `RestClosed`
    /// and Last stays empty until the first `@trade` arrives.
    ///
    /// Skips when TickBook already has a last for the id, and when the id is
    /// already subscribed — boot's bound `btcusdt` therefore costs nothing.
    /// Depth and klines are untouched; they still ride `subscribe_instrument`.
    pub async fn prime_binance_spot_ticker(&self, instrument: &str) {
        crate::data::await_binance_spot_ticker_price(
            self.quote_registry.clone(),
            self.tickbook.clone(),
            self.com_ticker_inflight.clone(),
            self.quote_fetch_error.clone(),
            instrument,
        )
        .await;
    }

    pub fn validated_quote_id(&self, raw: &str) -> Option<String> {
        self.validate_quote_binding(raw, None)
            .ok()
            .map(|binding| binding.instrument_id)
    }

    pub fn validate_quote_binding(
        &self,
        raw: &str,
        requested_book: Option<&str>,
    ) -> Result<ValidatedQuoteBinding, QuoteBindError> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(QuoteBindError::InstrumentInvalid);
        }
        if let Some(book) = requested_book
            .map(str::trim)
            .filter(|book| !book.is_empty())
        {
            if !is_known_quote_book(book) {
                return Err(QuoteBindError::UnknownBook);
            }
        }

        if is_dated_option_contract(raw) {
            return self.validate_dated_options_binding(raw, requested_book);
        }

        if let Some(id) = parse_nfo_instrument_id(raw) {
            return self.validate_kotak_nfo_binding(&id, requested_book);
        }

        if let Some(id) = self.resolve_kotak_cash_instrument(raw) {
            return self.validate_kotak_cash_binding(&id, requested_book);
        }

        self.validate_binance_spot_binding(raw, requested_book)
    }

    fn validate_dated_options_binding(
        &self,
        raw: &str,
        requested_book: Option<&str>,
    ) -> Result<ValidatedQuoteBinding, QuoteBindError> {
        if let Some(book) = requested_book
            .map(str::trim)
            .filter(|book| !book.is_empty())
        {
            if book != BINANCE_COM_OPTIONS_BOOK_ID {
                return Err(QuoteBindError::InstrumentBookMismatch);
            }
        }
        let instrument_id = normalize_options_instrument(raw);
        if instrument_id.is_empty() {
            return Err(QuoteBindError::InstrumentInvalid);
        }
        Ok(ValidatedQuoteBinding {
            book_id: BINANCE_COM_OPTIONS_BOOK_ID,
            instrument_id,
            source: QuoteSource::BinanceOptionsPublic,
        })
    }

    fn validate_kotak_nfo_binding(
        &self,
        instrument_id: &str,
        requested_book: Option<&str>,
    ) -> Result<ValidatedQuoteBinding, QuoteBindError> {
        if let Some(book) = requested_book
            .map(str::trim)
            .filter(|book| !book.is_empty())
        {
            if book == BINANCE_COM_OPTIONS_BOOK_ID || book != KOTAK_NSE_NFO_BOOK_ID {
                return Err(QuoteBindError::InstrumentBookMismatch);
            }
        }
        self.ensure_kotak_private_ready()?;
        Ok(ValidatedQuoteBinding {
            book_id: KOTAK_NSE_NFO_BOOK_ID,
            instrument_id: instrument_id.to_string(),
            source: QuoteSource::KotakPrivate,
        })
    }

    fn resolve_kotak_cash_instrument(&self, raw: &str) -> Option<String> {
        if let Some((segment, token)) = kotak_scrip_master::parse_instrument_id(raw) {
            return Some(kotak_scrip_master::instrument_id(&segment, token));
        }
        let master = self
            .kotak_scrip_master
            .lock()
            .expect("kotak scrip master mutex poisoned");
        let id = master.resolve_id(raw)?;
        kotak_quote_book_id(&id)
            .filter(|book| *book == KOTAK_NSE_BSE_CASH_BOOK_ID)
            .map(|_| id)
    }

    fn validate_kotak_cash_binding(
        &self,
        instrument_id: &str,
        requested_book: Option<&str>,
    ) -> Result<ValidatedQuoteBinding, QuoteBindError> {
        if let Some(book) = requested_book
            .map(str::trim)
            .filter(|book| !book.is_empty())
        {
            if book != KOTAK_NSE_BSE_CASH_BOOK_ID {
                return Err(QuoteBindError::InstrumentBookMismatch);
            }
        }
        self.ensure_kotak_private_ready()?;
        let in_master = self
            .kotak_scrip_master
            .lock()
            .expect("kotak scrip master mutex poisoned")
            .contains_id(instrument_id);
        if !in_master {
            return Err(QuoteBindError::InstrumentNotInMaster);
        }
        Ok(ValidatedQuoteBinding {
            book_id: KOTAK_NSE_BSE_CASH_BOOK_ID,
            instrument_id: instrument_id.to_string(),
            source: QuoteSource::KotakPrivate,
        })
    }

    fn validate_binance_spot_binding(
        &self,
        raw: &str,
        requested_book: Option<&str>,
    ) -> Result<ValidatedQuoteBinding, QuoteBindError> {
        if let Some(book) = requested_book
            .map(str::trim)
            .filter(|book| !book.is_empty())
        {
            if book != BINANCE_COM_SPOT_BOOK_ID {
                return Err(QuoteBindError::InstrumentBookMismatch);
            }
        }
        let instrument_id = resolve_among(
            raw,
            self.spot_resolve_candidates().iter().map(String::as_str),
        )
        .unwrap_or_else(|| normalize_quote_instrument(raw));
        if instrument_id.is_empty() || !self.is_binance_spot_instrument(&instrument_id) {
            return Err(QuoteBindError::InstrumentInvalid);
        }
        Ok(ValidatedQuoteBinding {
            book_id: BINANCE_COM_SPOT_BOOK_ID,
            instrument_id,
            source: QuoteSource::BinanceSpotPublic,
        })
    }

    fn spot_resolve_candidates(&self) -> Vec<String> {
        let mut candidates = Vec::new();
        if let Some(env) = &self.s1_desk_symbol {
            candidates.push(normalize_quote_instrument(env));
        }
        {
            let master = self
                .instrument_master
                .lock()
                .expect("instrument master mutex poisoned");
            for symbol in master.iter_symbols() {
                let id = normalize_quote_instrument(symbol);
                if !candidates.iter().any(|existing| existing == &id) {
                    candidates.push(id);
                }
            }
        }
        {
            let map = self
                .broker_connections
                .lock()
                .expect("broker connections mutex poisoned");
            if let Some(runtime) = map.get(BINANCE_COM) {
                for id in &runtime.subscriptions {
                    if !candidates.iter().any(|existing| existing == id) {
                        candidates.push(id.clone());
                    }
                }
            }
        }
        candidates
    }

    fn is_binance_spot_instrument(&self, instrument: &str) -> bool {
        let id = normalize_quote_instrument(instrument);
        if id.is_empty() || id.contains('|') || is_dated_option_contract(instrument) {
            return false;
        }
        if self.is_binance_com_desk() {
            return self.is_binance_path(instrument);
        }
        if self
            .instrument_master
            .lock()
            .expect("instrument master mutex poisoned")
            .contains_symbol(&id)
        {
            return true;
        }
        if self
            .s1_desk_symbol
            .as_deref()
            .is_some_and(|env| normalize_quote_instrument(env) == id)
        {
            return true;
        }
        if self
            .broker_connections
            .lock()
            .expect("broker connections mutex poisoned")
            .get(BINANCE_COM)
            .is_some_and(|runtime| runtime.subscriptions.iter().any(|sub| sub == &id))
        {
            return true;
        }
        // Public spot pairs are valid without a loaded master or active COM desk.
        id.chars().all(|c| c.is_ascii_alphanumeric())
    }

    fn ensure_kotak_private_ready(&self) -> Result<(), QuoteBindError> {
        let connections = self
            .broker_connections
            .lock()
            .expect("broker connections mutex poisoned");
        if !connections.contains_key(KOTAK_NEO) {
            return Err(QuoteBindError::BookNotShipping);
        }
        drop(connections);
        let locator = self
            .kotak_session_locator
            .lock()
            .expect("kotak session locator poisoned");
        if locator.is_none() {
            return Err(QuoteBindError::PrivateSessionUnavailable);
        }
        Ok(())
    }

    pub async fn bind_quote_selection(&self, binding: &ValidatedQuoteBinding) {
        {
            let mut selections = self
                .quote_selections
                .lock()
                .expect("quote selections poisoned");
            selections.bind(binding.book_id, binding.instrument_id.clone());
        }
        match binding.source {
            QuoteSource::BinanceOptionsPublic => {
                self.prime_binance_options_ticker(&binding.instrument_id)
                    .await;
                self.bind_spot_market(&binding.instrument_id);
            }
            QuoteSource::BinanceSpotPublic => {
                if self.is_binance_com_desk()
                    || self
                        .broker_connections
                        .lock()
                        .expect("broker connections mutex poisoned")
                        .contains_key(BINANCE_COM)
                {
                    self.prime_binance_spot_ticker(&binding.instrument_id).await;
                    self.subscribe_instrument(&binding.instrument_id);
                }
                // Else: selection only until COM Start — no streams, no REST dial.
            }
            QuoteSource::KotakPrivate => {
                self.prime_kotak_quote(&binding.instrument_id).await;
            }
        }
    }

    pub fn selected_quote_for(&self, book_id: &str) -> Option<String> {
        self.quote_selections
            .lock()
            .expect("quote selections poisoned")
            .selected(book_id)
            .map(str::to_string)
    }

    fn capability_selection(&self) -> Option<(String, &'static str)> {
        if self.is_kotak_neo_desk() {
            if let Some(id) = self.selected_quote_for(KOTAK_NSE_NFO_BOOK_ID) {
                return Some((id, KOTAK_NSE_NFO_BOOK_ID));
            }
            if let Some(id) = self.selected_quote_for(KOTAK_NSE_BSE_CASH_BOOK_ID) {
                return Some((id, KOTAK_NSE_BSE_CASH_BOOK_ID));
            }
        }
        if self.is_binance_com_desk() {
            if let Some(id) = self.selected_quote_for(BINANCE_COM_SPOT_BOOK_ID) {
                return Some((id, BINANCE_COM_SPOT_BOOK_ID));
            }
            if let Some(id) = self.selected_quote_for(BINANCE_COM_OPTIONS_BOOK_ID) {
                return Some((id, BINANCE_COM_OPTIONS_BOOK_ID));
            }
        }
        for book in [
            BINANCE_COM_OPTIONS_BOOK_ID,
            BINANCE_COM_SPOT_BOOK_ID,
            KOTAK_NSE_NFO_BOOK_ID,
            KOTAK_NSE_BSE_CASH_BOOK_ID,
        ] {
            if let Some(id) = self.selected_quote_for(book) {
                return Some((id, book));
            }
        }
        None
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
            return is_dated_option_contract(id) || self.is_binance_path(id);
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
        let Some((instrument, book_id)) = self.capability_selection() else {
            return "unavailable";
        };
        if instrument.trim().is_empty() {
            return "unavailable";
        }
        let adapter = self.active_adapter_id();
        let book = self.tickbook.lock().expect("tickbook mutex poisoned");
        let env = extract_quote_for_book(
            self.quote_registry.as_ref(),
            &book,
            &instrument,
            Utc::now(),
            self.quote_freshness,
            adapter.as_deref(),
            Some(book_id),
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

#[allow(dead_code)] // poller completeness axis; account caps now read AccountBook
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
    // Unparameterised exchangeInfo is weight 20 (spot/REST.md:68) and the symbol
    // set changes on listing events, not per call.
    let call = EgressCall::get(
        crate::data::BINANCE_COM_SPOT_BOOK_ID,
        "api.binance.com",
        "/api/v3/exchangeInfo",
        Lane::MarketData,
    )
    .with_max_age_ms(EXCHANGE_INFO_MAX_AGE_MS)
    .with_timeout(Duration::from_secs(15));
    let resp = crate::egress::shared().send(&call).await.map_err(|_| {
        InstrumentMasterFetchError::new(InstrumentMasterErrorClass::ExchangeInfoHttp, None)
    })?;
    let status = resp.status;
    if !resp.is_success() {
        return Err(InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::ExchangeInfoHttp,
            Some(status),
        ));
    }
    let body = resp.body;
    let cache = ExchangeInfoSymbolCache::from_exchange_info_json(&body).map_err(|_| {
        InstrumentMasterFetchError::new(InstrumentMasterErrorClass::Empty, Some(status))
    })?;
    if cache.is_empty() {
        return Err(InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::Empty,
            Some(status),
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
    validate_kline_request(interval, Some(limit))
        .map_err(|e| anyhow::anyhow!("{}", e.as_ineligible()))?;
    let call = EgressCall::get(
        crate::data::BINANCE_COM_SPOT_BOOK_ID,
        "api.binance.com",
        "/api/v3/klines",
        Lane::MarketData,
    )
    .with_query(format!(
        "symbol={}&interval={}&limit={}",
        symbol.trim().to_ascii_uppercase(),
        interval,
        limit
    ))
    .with_max_age_ms(KLINES_MAX_AGE_MS)
    .with_timeout(Duration::from_secs(15));
    let resp = crate::egress::shared()
        .send(&call)
        .await
        .map_err(anyhow::Error::new)?;
    if !resp.is_success() {
        return Err(anyhow::anyhow!("klines http {}", resp.status));
    }
    series_from_klines_json(&resp.body, symbol, interval, Transport::Rest)
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

    /// Options depth is a REST bounded snapshot on `eapi.binance.com` (Slice 3).
    /// Lighting it must not open spot's managed COM `@depth` WS for a dated
    /// contract: that stream carries `u`/`pu` reconstruction this book has no
    /// parser for, and `stream.binance.com` does not list these contracts.
    #[test]
    fn subscribe_instrument_dated_does_not_insert_into_depth_bind() {
        let trade = MarketBind::new();
        let depth = MarketBind::new();
        let streams = Arc::new(Mutex::new(HashSet::new()));
        let kind = bind_spot_market_ids("BTC-260925-145000-C", &trade, &depth, &streams);
        assert_eq!(kind, BindSpotKind::Options);
        assert!(trade.current().is_none());
        assert!(depth.current().is_none());
        // Belt-and-braces: even called directly, the COM depth binder unbinds a
        // dated id rather than lowercasing it into the spot slot.
        crate::data::ensure_binance_com_depth_stream(&depth, "BTC-260925-145000-C");
        assert!(depth.current().is_none());
        assert_ne!(depth.current().as_deref(), Some("btc-260925-145000-c"));
        assert_ne!(depth.current().as_deref(), Some("BTC-260925-145000-C"));
        let set = streams.lock().expect("quote stream set poisoned");
        assert!(set.contains(&format!(
            "{}\0BTC-260925-145000-C",
            crate::data::BINANCE_COM_OPTIONS_BOOK_ID
        )));
        assert!(!set.contains("btc-260925-145000-c"));
    }

    #[test]
    fn subscribe_instrument_spot_replaces_trade_and_depth_bind() {
        let trade = MarketBind::new();
        let depth = MarketBind::new();
        let streams = Arc::new(Mutex::new(HashSet::new()));
        bind_spot_market_ids("ETHUSDT", &trade, &depth, &streams);
        bind_spot_market_ids("BTCUSDT", &trade, &depth, &streams);
        assert_eq!(trade.current().as_deref(), Some("btcusdt"));
        assert_eq!(depth.current().as_deref(), Some("btcusdt"));
        assert_ne!(trade.current().as_deref(), Some("ethusdt"));
        assert!(streams
            .lock()
            .expect("quote stream set poisoned")
            .is_empty());
    }
}
