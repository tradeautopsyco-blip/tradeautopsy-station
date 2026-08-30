//! Host-owned describe / obtain. Loopback like quote. No Wasm `describe`/`obtain`.

use crate::api::desk::{account_identity, quote_status_wire, reference_identity};
use crate::api::AppState;
use crate::data::{
    chain_rows_for_contract, depth_obtain_data, describe, extract_chain_from, extract_depth,
    extract_licensed_history, extract_open_interest_from, extract_quote_for_book,
    history_obtain_data, obtain, parse_nfo_instrument_id, DepthStatus, GlanceStatus,
    ObtainEnvelope, ObtainStatus, QuoteStatus, Registry, SourceManifest, TickBook,
    BINANCE_COM_OPTIONS_BOOK_ID, DEFAULT_HISTORY_INTERVAL, KOTAK_NSE_BSE_CASH_BOOK_ID,
    KOTAK_NSE_NFO_BOOK_ID,
};
use axum::extract::{Query, State};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use serde::Serialize;
use serde_json::json;

#[derive(Debug, Deserialize)]
pub struct ManifestQuery {
    pub adapter: Option<String>,
    pub book: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ManifestList {
    pub manifests: Vec<SourceManifest>,
}

#[derive(Debug, Deserialize)]
pub struct ObtainQuery {
    pub adapter: Option<String>,
    pub book: Option<String>,
    pub operation: Option<String>,
}

fn query_id(raw: Option<&str>) -> Option<&str> {
    raw.map(str::trim).filter(|s| !s.is_empty())
}

/// Resolve `?adapter=` / `?book=` to one first-party manifest.
/// Both set and disagree (different manifests, or slug book_id ≠ book) → None.
pub(crate) fn resolve_manifest<'a>(
    manifests: &'a [SourceManifest],
    adapter: Option<&str>,
    book: Option<&str>,
) -> Option<&'a SourceManifest> {
    let adapter = query_id(adapter);
    let book = query_id(book);
    let by_adapter = adapter.and_then(|slug| {
        if let Some(book) = book {
            manifests.iter().find(|manifest| {
                manifest.adapter_id == slug
                    && manifest.book_id == book
                    && describe(manifest).is_ok()
            })
        } else {
            manifests
                .iter()
                .find(|manifest| manifest.adapter_id == slug && describe(manifest).is_ok())
        }
    });
    let by_book = book.and_then(|id| {
        manifests
            .iter()
            .find(|manifest| manifest.book_id == id && describe(manifest).is_ok())
    });
    match (adapter.is_some(), book.is_some(), by_adapter, by_book) {
        (true, true, Some(from_adapter), Some(from_book))
            if from_adapter.adapter_id == from_book.adapter_id
                && from_adapter.book_id == from_book.book_id
                && from_adapter.book_id == book.unwrap_or("") =>
        {
            Some(from_adapter)
        }
        (true, true, _, _) => None,
        (true, false, Some(from_adapter), _) => Some(from_adapter),
        (false, true, _, Some(from_book)) => Some(from_book),
        _ => None,
    }
}

fn unsupported_obtain(adapter_id: &str, book_id: &str, operation: &str) -> ObtainEnvelope {
    ObtainEnvelope {
        adapter_id: adapter_id.to_string(),
        book_id: book_id.to_string(),
        operation: operation.to_string(),
        status: ObtainStatus::Unsupported,
        data: None,
        provenance_adapter_id: None,
        provenance_path: None,
    }
}

pub async fn manifest_handler(
    State(state): State<AppState>,
    Query(query): Query<ManifestQuery>,
) -> Json<ManifestList> {
    let adapter = query_id(query.adapter.as_deref());
    let book = query_id(query.book.as_deref());
    let manifests = if adapter.is_none() && book.is_none() {
        state
            .source_manifests
            .iter()
            .filter(|manifest| describe(manifest).is_ok())
            .cloned()
            .collect()
    } else {
        resolve_manifest(state.source_manifests.as_ref(), adapter, book)
            .cloned()
            .into_iter()
            .collect()
    };
    Json(ManifestList { manifests })
}

pub async fn obtain_handler(
    State(state): State<AppState>,
    Query(query): Query<ObtainQuery>,
) -> Json<ObtainEnvelope> {
    let adapter = query_id(query.adapter.as_deref()).unwrap_or_default();
    let book = query_id(query.book.as_deref()).unwrap_or_default();
    let operation = query_id(query.operation.as_deref()).unwrap_or_default();
    if operation.is_empty() || (adapter.is_empty() && book.is_empty()) {
        return Json(unsupported_obtain(adapter, book, operation));
    }
    let Some(manifest) = resolve_manifest(
        state.source_manifests.as_ref(),
        query_id(query.adapter.as_deref()),
        query_id(query.book.as_deref()),
    ) else {
        return Json(unsupported_obtain(adapter, book, operation));
    };
    let mut envelope = obtain(manifest, operation);
    if envelope.status == ObtainStatus::Unavailable {
        // Enrichers are sync, but the options snapshots are async fetches that
        // only glance used to run. Kick them here, or obtain stays unavailable
        // forever unless a glance happened to land first.
        kick_options_snapshot(&state, &envelope).await;
        envelope = enrich_obtain(&state, envelope);
    }
    Json(envelope)
}

/// The dated contract obtain is scoped to. A leftover `BTC` / `BTCUSDT` is not
/// a contract: it matches no `optionSymbols` row, so obtain stays dark rather
/// than smashing a spot chain in.
fn selected_options_contract(state: &AppState) -> String {
    state
        .selected_quote_instrument
        .lock()
        .expect("selected quote instrument poisoned")
        .clone()
        .unwrap_or_default()
        .trim()
        .to_string()
}

/// Options are the only book whose obtain snapshot is fetched rather than
/// resident. Kept out of `enrich_obtain` so every other enricher stays sync.
async fn kick_options_snapshot(state: &AppState, envelope: &ObtainEnvelope) {
    if envelope.book_id != BINANCE_COM_OPTIONS_BOOK_ID {
        return;
    }
    match envelope.operation.as_str() {
        "optionchain" => super::glance::ensure_options_master(state).await,
        "open_interest" => {
            // OI is instrument-scoped — fetch only once the contract is known.
            let instrument = selected_options_contract(state);
            if instrument.is_empty() {
                return;
            }
            super::glance::ensure_options_oi(state, &instrument).await;
        }
        _ => {}
    }
}

fn enrich_obtain(state: &AppState, envelope: ObtainEnvelope) -> ObtainEnvelope {
    match enricher(&envelope.book_id, &envelope.operation) {
        Some(apply) => apply(state, envelope),
        None => envelope,
    }
}

fn enricher(
    book_id: &str,
    operation: &str,
) -> Option<fn(&AppState, ObtainEnvelope) -> ObtainEnvelope> {
    match (book_id, operation) {
        ("binance-com-spot", "quotes") => Some(enrich_tickbook_quotes),
        ("binance-com-spot", "depth") => Some(enrich_depth),
        ("binance-com-spot", "instruments") => Some(enrich_binance_instruments),
        ("binance-com-spot", "history") => Some(enrich_binance_history),
        ("binance-com-spot", "funds") => Some(enrich_binance_funds),
        ("binance-com-spot", "tradebook") => Some(enrich_tradebook),
        ("kotak-nse-bse-cash", "quotes") => Some(enrich_tickbook_quotes),
        ("kotak-nse-bse-cash", "depth") => Some(enrich_depth),
        ("kotak-nse-bse-cash", "instruments") => Some(enrich_kotak_instruments),
        ("kotak-nse-bse-cash", "tradebook") => Some(enrich_tradebook),
        ("kotak-nse-nfo", "quotes") => Some(enrich_tickbook_quotes),
        ("kotak-nse-nfo", "instruments") => Some(enrich_kotak_nfo_instruments),
        ("kotak-nse-nfo", "optionchain") => Some(enrich_optionchain),
        ("binance-com-options", "quotes") => Some(enrich_tickbook_quotes),
        ("binance-com-options", "optionchain") => Some(enrich_optionchain),
        ("binance-com-options", "open_interest") => Some(enrich_open_interest),
        _ => None,
    }
}

fn enrich_tickbook_quotes(state: &AppState, envelope: ObtainEnvelope) -> ObtainEnvelope {
    let selected = state
        .selected_quote_instrument
        .lock()
        .expect("selected quote instrument poisoned")
        .clone();
    let book = state.tickbook.lock().expect("tickbook mutex poisoned");
    enrich_tickbook_quotes_with(
        state.quote_registry.as_ref(),
        &book,
        selected.as_deref(),
        state.quote_freshness,
        envelope,
    )
}

/// One global `selected_quote_instrument`. NFO identity must not paint cash obtain.
fn quotes_instrument_for_envelope(
    selected: Option<&str>,
    book: &TickBook,
    adapter: &str,
    envelope_book_id: &str,
) -> String {
    let mut instrument = selected.unwrap_or("").trim().to_string();
    if envelope_book_id == KOTAK_NSE_BSE_CASH_BOOK_ID
        && parse_nfo_instrument_id(&instrument).is_some()
    {
        instrument.clear();
    }
    let selected_ok = !instrument.is_empty() && book.get(envelope_book_id, &instrument).is_some();
    if !selected_ok {
        if let Some((_, row)) = book.iter().find(|(_, row)| row.book_id == envelope_book_id) {
            instrument = row.instrument_id.clone();
        } else if let Some((_, row)) = book
            .iter()
            .find(|(_, row)| row.adapter_id == adapter && row.book_id == envelope_book_id)
        {
            instrument = row.instrument_id.clone();
        } else {
            instrument.clear();
        }
    }
    instrument
}

fn enrich_tickbook_quotes_with(
    registry: &Registry,
    book: &TickBook,
    selected: Option<&str>,
    freshness: std::time::Duration,
    mut envelope: ObtainEnvelope,
) -> ObtainEnvelope {
    let adapter = envelope.adapter_id.clone();
    let book_id = envelope.book_id.clone();
    let instrument = quotes_instrument_for_envelope(selected, book, &adapter, &book_id);
    if instrument.is_empty() {
        return envelope;
    }
    let quote = extract_quote_for_book(
        registry,
        book,
        &instrument,
        Utc::now(),
        freshness,
        Some(&adapter),
        Some(&book_id),
    );
    if let Some(data) = tickbook_quote_obtain_data(&quote) {
        envelope.status = ObtainStatus::Success;
        envelope.data = Some(data);
        envelope.provenance_adapter_id = Some(quote.provenance.adapter_id);
    }
    envelope
}

fn enrich_depth(state: &AppState, mut envelope: ObtainEnvelope) -> ObtainEnvelope {
    let adapter = envelope.adapter_id.clone();
    let book_id = envelope.book_id.clone();
    let book = state.depthbook.lock().expect("depthbook mutex poisoned");
    let mut instrument = state
        .resolve_candidates()
        .first()
        .cloned()
        .unwrap_or_else(|| state.s1_desk_symbol.clone().unwrap_or_default());
    if envelope.book_id == "binance-com-spot" {
        instrument = crate::data::normalize_quote_instrument(&instrument);
    }
    if instrument.is_empty() || book.get(&book_id, &instrument).is_none() {
        if let Some((_, row)) = book
            .iter()
            .find(|(_, row)| row.book_id == book_id && row.completeness)
        {
            instrument = row.instrument_id.clone();
        } else if let Some((_, row)) = book
            .iter()
            .find(|(_, row)| row.adapter_id == adapter && row.completeness)
        {
            instrument = row.instrument_id.clone();
        }
    }
    if instrument.is_empty() {
        return envelope;
    }
    let depth = extract_depth(&book, &instrument, Some(adapter.as_str()));
    if depth.status == DepthStatus::Success {
        if let Some(data) = depth_obtain_data(&depth) {
            envelope.status = ObtainStatus::Success;
            envelope.data = Some(data);
            envelope.provenance_adapter_id = Some(depth.provenance.adapter_id);
        }
    }
    envelope
}

fn enrich_binance_funds(state: &AppState, mut envelope: ObtainEnvelope) -> ObtainEnvelope {
    let snap = state
        .broker_status
        .lock()
        .expect("broker_status mutex poisoned");
    let Some(balances) = snap.last_balances.as_ref() else {
        return envelope;
    };
    envelope.status = ObtainStatus::Success;
    envelope.data = Some(json!({
        "identity": account_identity("funds"),
        "holdings": balances
            .holdings
            .iter()
            .map(|h| json!({
                "asset": h.asset,
                "free": h.free,
                "locked": h.locked,
            }))
            .collect::<Vec<_>>(),
        "unrealized_pnl": balances.unrealized_pnl,
        "as_of_ms": snap.data_classes.balances_holdings.last_success_at_ms,
    }));
    envelope
}

fn enrich_tradebook(state: &AppState, mut envelope: ObtainEnvelope) -> ObtainEnvelope {
    let snap = state
        .broker_status
        .lock()
        .expect("broker_status mutex poisoned");
    if snap.active_broker_slug.as_deref() != Some(envelope.adapter_id.as_str())
        && snap.last_fills_count.is_none()
    {
        return envelope;
    }
    let Some(count) = snap.last_fills_count else {
        return envelope;
    };
    envelope.status = ObtainStatus::Success;
    envelope.data = Some(json!({
        "identity": account_identity("fills"),
        "fill_count": count,
        "as_of_ms": snap.data_classes.fills_trade_history.last_success_at_ms,
    }));
    envelope
}

fn enrich_binance_instruments(state: &AppState, mut envelope: ObtainEnvelope) -> ObtainEnvelope {
    let master = state
        .instrument_master
        .lock()
        .expect("instrument master mutex poisoned");
    if master.is_empty() {
        return envelope;
    }
    envelope.status = ObtainStatus::Success;
    envelope.data = Some(json!({
        "identity": reference_identity(),
        "symbol_count": master.len(),
    }));
    envelope
}

fn enrich_kotak_instruments(state: &AppState, mut envelope: ObtainEnvelope) -> ObtainEnvelope {
    let master = state
        .kotak_scrip_master
        .lock()
        .expect("kotak scrip master mutex poisoned");
    if let Some(data) = kotak_instruments_data(&master) {
        envelope.status = ObtainStatus::Success;
        envelope.data = Some(data);
    }
    envelope
}

fn enrich_kotak_nfo_instruments(state: &AppState, mut envelope: ObtainEnvelope) -> ObtainEnvelope {
    let master = state
        .kotak_nfo_scrip_master
        .lock()
        .expect("kotak nfo scrip master mutex poisoned");
    if master.is_empty() {
        return envelope;
    }
    let rows: Vec<crate::data::ContractRow> =
        master.iter().map(|row| row.to_contract_row()).collect();
    drop(master);
    let extracted = crate::data::extract_contracts_from_rows(
        Some(crate::data::KOTAK_NSE_NFO_BOOK_ID),
        Some(&rows),
    );
    if let Some(data) = extracted.data {
        envelope.status = ObtainStatus::Success;
        envelope.data = Some(data);
    }
    envelope
}

fn enrich_optionchain(state: &AppState, mut envelope: ObtainEnvelope) -> ObtainEnvelope {
    let book_id = envelope.book_id.clone();
    let selected = state
        .selected_quote_instrument
        .lock()
        .expect("selected quote instrument poisoned")
        .clone()
        .unwrap_or_default();
    let chain = match book_id.as_str() {
        KOTAK_NSE_NFO_BOOK_ID => {
            let master = state
                .kotak_nfo_scrip_master
                .lock()
                .expect("kotak nfo scrip master mutex poisoned");
            if master.is_empty() {
                return envelope;
            }
            let instrument = {
                let trimmed = selected.trim();
                if !trimmed.is_empty() {
                    trimmed.to_string()
                } else {
                    master
                        .iter()
                        .next()
                        .map(|row| row.name.clone())
                        .unwrap_or_default()
                }
            };
            let rows: Vec<_> =
                if parse_nfo_instrument_id(&instrument).is_some() || !instrument.is_empty() {
                    master
                        .rows_for_underlying(&instrument)
                        .into_iter()
                        .map(|row| row.to_chain_row())
                        .collect()
                } else {
                    master.iter().map(|row| row.to_chain_row()).collect()
                };
            drop(master);
            let tickbook = state.tickbook.lock().expect("tickbook mutex poisoned");
            extract_chain_from(
                Some(KOTAK_NSE_NFO_BOOK_ID),
                &instrument,
                Some(rows.as_slice()),
                Some(&tickbook),
            )
        }
        BINANCE_COM_OPTIONS_BOOK_ID => {
            // Must be the dated contract. An empty selection, an empty master,
            // or no matching (underlying, expiry) all stay Unavailable — never
            // a Success with an empty rows array.
            let instrument = selected.trim().to_string();
            if instrument.is_empty() {
                return envelope;
            }
            let master = state
                .options_option_symbols
                .lock()
                .expect("options option symbols mutex poisoned");
            if master.is_empty() {
                return envelope;
            }
            let rows = chain_rows_for_contract(&instrument, &master);
            drop(master);
            if rows.is_empty() {
                return envelope;
            }
            let tickbook = state.tickbook.lock().expect("tickbook mutex poisoned");
            extract_chain_from(
                Some(BINANCE_COM_OPTIONS_BOOK_ID),
                &instrument,
                Some(rows.as_slice()),
                Some(&tickbook),
            )
        }
        _ => return envelope,
    };
    if chain.status == GlanceStatus::Success {
        envelope.status = ObtainStatus::Success;
        envelope.data = chain.data;
        if !chain.provenance.adapter_id.is_empty() {
            envelope.provenance_adapter_id = Some(chain.provenance.adapter_id);
        }
        if !chain.provenance.path.is_empty() {
            envelope.provenance_path = Some(chain.provenance.path);
        }
    }
    envelope
}

/// Open interest is its own arm — never folded into the chain enricher. No rows
/// for the selected contract means no `sumOpenInterest`, and that stays
/// Unavailable rather than reporting 0.
fn enrich_open_interest(state: &AppState, mut envelope: ObtainEnvelope) -> ObtainEnvelope {
    if envelope.book_id != BINANCE_COM_OPTIONS_BOOK_ID {
        return envelope;
    }
    let instrument = selected_options_contract(state);
    if instrument.is_empty() {
        return envelope;
    }
    let rows = state
        .options_oi_rows
        .lock()
        .expect("options oi mutex poisoned")
        .clone();
    if !rows.iter().any(|row| row.symbol == instrument) {
        return envelope;
    }
    let oi = extract_open_interest_from(
        Some(BINANCE_COM_OPTIONS_BOOK_ID),
        &instrument,
        Some(rows.as_slice()),
    );
    if oi.status == GlanceStatus::Success {
        envelope.status = ObtainStatus::Success;
        envelope.data = oi.data;
        if !oi.provenance.adapter_id.is_empty() {
            envelope.provenance_adapter_id = Some(oi.provenance.adapter_id);
        }
        if !oi.provenance.path.is_empty() {
            envelope.provenance_path = Some(oi.provenance.path);
        }
    }
    envelope
}

fn enrich_binance_history(state: &AppState, mut envelope: ObtainEnvelope) -> ObtainEnvelope {
    let instrument = {
        let selected = state
            .selected_quote_instrument
            .lock()
            .expect("selected quote instrument poisoned")
            .clone()
            .filter(|s| !s.is_empty());
        let desk = state
            .s1_desk_symbol
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        selected.or(desk).unwrap_or_default()
    };
    let book = state
        .historybook
        .lock()
        .expect("historybook mutex poisoned");
    let history =
        extract_licensed_history(&book, &instrument, Some(DEFAULT_HISTORY_INTERVAL), None);
    if let Some(data) = history_obtain_data(&history) {
        envelope.status = ObtainStatus::Success;
        envelope.data = Some(data);
        envelope.provenance_adapter_id = Some(history.provenance.adapter_id);
    }
    envelope
}

fn kotak_instruments_data(
    master: &crate::kotak_scrip_master::KotakScripMaster,
) -> Option<serde_json::Value> {
    if master.is_empty() {
        return None;
    }
    Some(json!({
        "identity": reference_identity(),
        "symbol_count": master.len(),
    }))
}

/// Obtain(quotes) succeeds only with a positive TickBook last for the requested adapter.
fn tickbook_quote_obtain_data(quote: &crate::data::QuoteEnvelope) -> Option<serde_json::Value> {
    if quote.status == QuoteStatus::Unavailable {
        return None;
    }
    let data = quote.data.as_ref()?;
    let last = data.last.parse::<f64>().ok()?;
    if last <= 0.0 {
        return None;
    }
    Some(json!({
        "identity": quote.identity,
        "instrument_id": quote.instrument_id,
        "last": &data.last,
        "as_of": &data.as_of,
        "session_ohlc": data.session_ohlc.as_ref(),
        "quote_status": quote_status_wire(quote.status),
        "source": "tickbook",
        "quote_kind": "last",
        "mark": serde_json::Value::Null,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::KOTAK_NEO_ADAPTER_ID;
    use crate::kotak_scrip_master::KotakScripMaster;

    #[test]
    fn kotak_obtain_quotes_succeeds_when_tickbook_has_kotak_tick() {
        use crate::data::{
            apply_quote, extract_quote_for, kotak_neo_quote_descriptor, quote_tick_from_kotak_json,
            TickBook,
        };
        let registry =
            crate::data::Registry::load(&[kotak_neo_quote_descriptor()]).expect("kotak quote");
        let mut book = TickBook::new();
        let json = include_str!("../../fixtures/kotak/quotes_neosymbol.json");
        let tick = quote_tick_from_kotak_json(json, chrono::Utc::now()).unwrap();
        apply_quote(&registry, &mut book, tick).unwrap();
        let quote = extract_quote_for(
            &registry,
            &book,
            "nse_cm|2885",
            chrono::Utc::now(),
            std::time::Duration::from_millis(1000),
            Some(KOTAK_NEO_ADAPTER_ID),
        );
        let data = tickbook_quote_obtain_data(&quote).expect("kotak tick is obtain success");
        assert_ne!(quote.status, QuoteStatus::Unavailable);
        let last: f64 = data["last"].as_str().unwrap().parse().unwrap();
        assert!(last > 0.0);
        assert_eq!(quote.provenance.adapter_id, "kotak_neo");
        assert!(data["session_ohlc"].is_object());
        assert_eq!(data["source"], "tickbook");
        assert_eq!(data["quote_kind"], "last");
        assert!(data["mark"].is_null());
        assert!(!book.has_positive_tick_for("binance_com"));
        assert!(book.has_positive_tick_for("kotak_neo"));
    }

    #[test]
    fn kotak_obtain_quotes_empty_or_binance_tick_is_unavailable() {
        use crate::data::{
            apply_quote, binance_com_quote_descriptor, extract_quote_for,
            kotak_neo_quote_descriptor, quote_tick_from_binance_json, TickBook,
        };
        let registry = crate::data::Registry::load(&[
            kotak_neo_quote_descriptor(),
            binance_com_quote_descriptor(),
        ])
        .expect("both quote bindings");
        let empty = TickBook::new();
        let missing = extract_quote_for(
            &registry,
            &empty,
            "nse_cm|2885",
            chrono::Utc::now(),
            std::time::Duration::from_millis(1000),
            Some(KOTAK_NEO_ADAPTER_ID),
        );
        assert_eq!(missing.status, QuoteStatus::Unavailable);
        assert!(tickbook_quote_obtain_data(&missing).is_none());

        let mut book = TickBook::new();
        let binance = quote_tick_from_binance_json(
            r#"{"e":"trade","E":1,"s":"BTCUSDT","p":"65000.00","T":1700000000000}"#,
            chrono::Utc::now(),
        )
        .expect("binance trade fixture");
        apply_quote(&registry, &mut book, binance).unwrap();
        let kotak_view = extract_quote_for(
            &registry,
            &book,
            "btcusdt",
            chrono::Utc::now(),
            std::time::Duration::from_millis(1000),
            Some(KOTAK_NEO_ADAPTER_ID),
        );
        assert_eq!(kotak_view.status, QuoteStatus::Unavailable);
        assert!(tickbook_quote_obtain_data(&kotak_view).is_none());
        assert!(!book.has_positive_tick_for("kotak_neo"));
    }

    #[test]
    fn kotak_obtain_instruments_succeeds_when_fixture_master_loaded() {
        let csv = include_str!("../../fixtures/kotak/nse_cm_cash.csv");
        let master = KotakScripMaster::from_csv_bytes(csv.as_bytes(), None).unwrap();
        let data = kotak_instruments_data(&master).expect("non-empty master is obtain success");
        assert_eq!(data["identity"]["family"], "reference");
        assert_eq!(data["identity"]["capability_id"], "instrument_master");
        assert!(data["symbol_count"].as_u64().unwrap() >= 1);
        assert!(kotak_instruments_data(&KotakScripMaster::empty()).is_none());
    }

    #[test]
    fn kotak_obtain_depth_succeeds_from_depthbook_not_tickbook() {
        use crate::data::{
            depth_obtain_data, depth_snapshot_from_kotak_json, extract_depth, DepthBook,
            DepthStatus, Physics, TickBook,
        };
        let json = include_str!("../../fixtures/kotak/quotes_neosymbol_depth.json");
        let mut depthbook = DepthBook::new();
        let snap = depth_snapshot_from_kotak_json(json, chrono::Utc::now()).unwrap();
        depthbook.upsert(snap);
        let envelope = extract_depth(&depthbook, "nse_cm|2885", Some(KOTAK_NEO_ADAPTER_ID));
        assert_eq!(envelope.status, DepthStatus::Success);
        assert_eq!(envelope.identity.physics, Physics::BoundedSnapshot);
        let data = depth_obtain_data(&envelope).expect("bounded snapshot");
        assert_eq!(data["identity"]["physics"], "bounded_snapshot");
        assert_ne!(data["identity"]["physics"], "ordered_state");
        assert_eq!(data["source"], "rest_snapshot");
        assert!(data.get("synced").is_none());
        assert!(TickBook::new()
            .get("kotak-nse-bse-cash", "nse_cm|2885")
            .is_none());
        assert!(depth_obtain_data(&extract_depth(
            &DepthBook::new(),
            "nse_cm|2885",
            Some(KOTAK_NEO_ADAPTER_ID)
        ))
        .is_none());
    }

    #[test]
    fn binance_obtain_history_succeeds_from_historybook_not_tickbook() {
        use crate::data::{
            apply_history_series, extract_licensed_history, history_obtain_data,
            series_from_klines_json, HistoryBook, HistoryStatus, Physics, TickBook, Transport,
        };
        let json = include_str!("../../fixtures/binance/klines.json");
        let mut book = HistoryBook::new();
        let series = series_from_klines_json(json, "BTCUSDT", "1m", Transport::Fixture).unwrap();
        apply_history_series(&mut book, series);
        let envelope = extract_licensed_history(&book, "btcusdt", Some("1m"), None);
        assert_eq!(envelope.status, HistoryStatus::Success);
        assert_eq!(envelope.identity.physics, Physics::HistoricalSeries);
        assert_eq!(envelope.provenance.adapter_id, "binance_com");
        let data = history_obtain_data(&envelope).expect("licensed series");
        assert_eq!(data["last_close"], "0.01590000");
        assert_eq!(data["source"], "binance_klines");
        assert!(TickBook::new().get("binance-com-spot", "btcusdt").is_none());
        assert!(history_obtain_data(&extract_licensed_history(
            &HistoryBook::new(),
            "btcusdt",
            Some("1m"),
            None
        ))
        .is_none());
    }

    #[test]
    fn obtain_book_alias_matches_adapter_for_spot() {
        use crate::data::first_party_s0_manifests;
        let manifests = first_party_s0_manifests();
        let by_adapter = resolve_manifest(&manifests, Some("binance_com"), None).unwrap();
        let by_book = resolve_manifest(&manifests, None, Some("binance-com-spot")).unwrap();
        let both =
            resolve_manifest(&manifests, Some("binance_com"), Some("binance-com-spot")).unwrap();
        assert_eq!(by_adapter.adapter_id, "binance_com");
        assert_eq!(by_adapter.book_id, "binance-com-spot");
        assert_eq!(by_book.adapter_id, by_adapter.adapter_id);
        assert_eq!(by_book.book_id, by_adapter.book_id);
        assert_eq!(both.adapter_id, by_adapter.adapter_id);
        assert_eq!(both.book_id, by_adapter.book_id);
        assert!(resolve_manifest(&manifests, None, Some("binance-com-usdm")).is_none());
        assert!(
            resolve_manifest(&manifests, Some("binance_com"), Some("kotak-nse-bse-cash")).is_none()
        );
        let nfo = resolve_manifest(&manifests, Some("kotak_neo"), Some("kotak-nse-nfo")).unwrap();
        assert_eq!(nfo.book_id, "kotak-nse-nfo");
        assert_eq!(nfo.manifest_id, "kotak_neo.nfo.v1");
        assert_eq!(
            resolve_manifest(&manifests, Some("kotak_neo"), None)
                .unwrap()
                .book_id,
            "kotak-nse-bse-cash"
        );
        let options =
            resolve_manifest(&manifests, Some("binance_com"), Some("binance-com-options")).unwrap();
        assert_eq!(options.book_id, "binance-com-options");
        assert_eq!(options.manifest_id, "binance_com.options.v1");
        assert_eq!(
            resolve_manifest(&manifests, None, Some("binance-com-options"))
                .unwrap()
                .book_id,
            "binance-com-options"
        );
        assert!(enricher("binance-com-spot", "optionchain").is_none());
        assert!(enricher("binance-com-usdm", "quotes").is_none());
        assert!(enricher("binance-com-options", "quotes").is_some());
        assert!(enricher("binance-com-options", "optionchain").is_some());
        assert!(enricher("binance-com-options", "open_interest").is_some());
        // The chain door never opens on spot or cash.
        assert!(enricher("binance-com-spot", "open_interest").is_none());
        assert!(enricher("kotak-nse-bse-cash", "optionchain").is_none());
        assert!(enricher("kotak-nse-nfo", "quotes").is_some());
        assert!(enricher("kotak-nse-nfo", "instruments").is_some());
        assert!(enricher("kotak-nse-nfo", "optionchain").is_some());
        let spot = manifests
            .iter()
            .find(|m| m.book_id == "binance-com-spot")
            .unwrap();
        assert_eq!(
            crate::data::obtain(spot, "optionchain").status,
            ObtainStatus::Unsupported
        );
        let options_m = manifests
            .iter()
            .find(|m| m.book_id == "binance-com-options")
            .unwrap();
        assert_eq!(
            crate::data::obtain(options_m, "optionchain").status,
            ObtainStatus::Unavailable
        );
        assert_eq!(
            crate::data::obtain(options_m, "open_interest").status,
            ObtainStatus::Unavailable
        );
        assert_eq!(
            crate::data::obtain(options_m, "quotes").status,
            ObtainStatus::Unavailable
        );
    }

    #[test]
    fn options_obtain_quotes_succeeds_only_with_last_in_that_slot() {
        use crate::data::{
            apply_quote, binance_com_quote_descriptor, extract_quote_for_book,
            quote_tick_from_options_ticker_json, TickBook, BINANCE_COM_OPTIONS_BOOK_ID,
        };
        let registry =
            crate::data::Registry::load(&[binance_com_quote_descriptor()]).expect("spot quote");
        let empty = TickBook::new();
        let missing = extract_quote_for_book(
            &registry,
            &empty,
            "BTC-200730-9000-C",
            chrono::Utc::now(),
            std::time::Duration::from_millis(1000),
            Some("binance_com"),
            Some(BINANCE_COM_OPTIONS_BOOK_ID),
        );
        assert_eq!(missing.status, QuoteStatus::Unavailable);
        assert!(tickbook_quote_obtain_data(&missing).is_none());
        assert!(missing.data.is_none());

        let mut book = TickBook::new();
        let json = include_str!("../../fixtures/binance/options_ticker.json");
        let tick = quote_tick_from_options_ticker_json(json, chrono::Utc::now()).unwrap();
        apply_quote(&registry, &mut book, tick).unwrap();
        let quote = extract_quote_for_book(
            &registry,
            &book,
            "BTC-200730-9000-C",
            chrono::Utc::now(),
            std::time::Duration::from_millis(1000),
            Some("binance_com"),
            Some(BINANCE_COM_OPTIONS_BOOK_ID),
        );
        let data = tickbook_quote_obtain_data(&quote).expect("options last is obtain success");
        assert_ne!(quote.status, QuoteStatus::Unavailable);
        assert_eq!(data["last"], "1.23");
        assert_ne!(data["last"], "0");
        assert_eq!(quote.instrument_id, "BTC-200730-9000-C");
        assert_eq!(quote.provenance.adapter_id, "binance_com");
        assert!(book.get("binance-com-spot", "BTC-200730-9000-C").is_none());
    }

    #[test]
    fn nfo_last_obtain_does_not_close_spot() {
        use crate::data::{
            apply_quote, binance_com_quote_descriptor, extract_quote_for_book,
            kotak_neo_nfo_manifest, kotak_neo_quote_descriptor, quote_tick_from_binance_json,
            quote_tick_from_kotak_json_for_book, TickBook, Transport, KOTAK_NSE_NFO_BOOK_ID,
        };
        let registry = crate::data::Registry::load(&[
            kotak_neo_quote_descriptor(),
            binance_com_quote_descriptor(),
        ])
        .expect("kotak + spot quote");
        let mut book = TickBook::new();
        let fo = r#"{"instrument_token":"12345","exchange_segment":"nse_fo","last_traded_price":"10.00"}"#;
        let nfo_tick =
            quote_tick_from_kotak_json_for_book(fo, chrono::Utc::now(), KOTAK_NSE_NFO_BOOK_ID)
                .expect("nfo tick");
        apply_quote(&registry, &mut book, nfo_tick).unwrap();
        let quote = extract_quote_for_book(
            &registry,
            &book,
            "nse_fo|12345",
            chrono::Utc::now(),
            std::time::Duration::from_millis(1000),
            Some(KOTAK_NEO_ADAPTER_ID),
            Some(KOTAK_NSE_NFO_BOOK_ID),
        );
        let data = tickbook_quote_obtain_data(&quote).expect("nfo last is obtain success");
        assert_eq!(data["quote_kind"], "last");
        let last: f64 = data["last"].as_str().unwrap().parse().unwrap();
        assert!(last > 0.0);
        assert_eq!(quote.provenance.adapter_id, "kotak_neo");

        let spot = quote_tick_from_binance_json(
            r#"{"e":"trade","E":1,"s":"BTCUSDT","p":"65000.00","T":1700000000000}"#,
            chrono::Utc::now(),
        )
        .expect("binance trade fixture");
        apply_quote(&registry, &mut book, spot).unwrap();
        assert_eq!(
            book.get("binance-com-spot", "btcusdt").unwrap().last,
            "65000.00"
        );
        let rest_spot = crate::data::QuoteTick {
            instrument_id: "btcusdt".to_string(),
            last: "65001.00".to_string(),
            as_of: chrono::Utc::now(),
            received_at: chrono::Utc::now(),
            age_unknown: true,
            transport: Transport::Rest,
            adapter_id: "binance_com".to_string(),
            book_id: "binance-com-spot".to_string(),
            session_ohlc: None,
        };
        apply_quote(&registry, &mut book, rest_spot).unwrap();
        assert_eq!(
            book.get("binance-com-spot", "btcusdt").unwrap().last,
            "65001.00"
        );

        let nfo = kotak_neo_nfo_manifest();
        assert_eq!(
            crate::data::obtain(&nfo, "optionchain").status,
            ObtainStatus::Unavailable
        );
        assert!(nfo.implemented.iter().any(|op| op == "optionchain"));
        assert!(nfo.implemented.iter().any(|op| op == "instruments"));
    }

    #[test]
    fn binance_obtain_quotes_succeeds_with_tickbook_last() {
        use crate::data::{
            apply_quote, binance_com_quote_descriptor, extract_quote_for,
            quote_tick_from_binance_json, TickBook,
        };
        let registry =
            crate::data::Registry::load(&[binance_com_quote_descriptor()]).expect("spot quote");
        let mut book = TickBook::new();
        let tick = quote_tick_from_binance_json(
            r#"{"e":"trade","E":1,"s":"BTCUSDT","p":"65000.00","T":1700000000000}"#,
            chrono::Utc::now(),
        )
        .expect("binance trade fixture");
        apply_quote(&registry, &mut book, tick).unwrap();
        let quote = extract_quote_for(
            &registry,
            &book,
            "btcusdt",
            chrono::Utc::now(),
            std::time::Duration::from_millis(1000),
            Some("binance_com"),
        );
        let data = tickbook_quote_obtain_data(&quote).expect("spot last is obtain success");
        assert_eq!(data["last"], "65000.00");
        assert_eq!(data["quote_kind"], "last");
        assert!(data["mark"].is_null());
        assert!(book.has_positive_tick_for("binance_com"));
    }

    #[test]
    fn selected_nfo_does_not_steal_cash_obtain_last() {
        use crate::data::{
            apply_quote, kotak_neo_nfo_manifest, kotak_neo_quote_descriptor,
            kotak_neo_s1k_manifest, quote_tick_from_kotak_json,
            quote_tick_from_kotak_json_for_book, TickBook, KOTAK_NSE_NFO_BOOK_ID,
        };
        let registry =
            crate::data::Registry::load(&[kotak_neo_quote_descriptor()]).expect("kotak quote");
        let mut book = TickBook::new();
        let cash_json = include_str!("../../fixtures/kotak/quotes_neosymbol.json");
        let cash_tick = quote_tick_from_kotak_json(cash_json, chrono::Utc::now()).unwrap();
        apply_quote(&registry, &mut book, cash_tick).unwrap();
        let nfo_json = include_str!("../../fixtures/kotak/quotes_neosymbol_nfo.json");
        let nfo_tick = quote_tick_from_kotak_json_for_book(
            nfo_json,
            chrono::Utc::now(),
            KOTAK_NSE_NFO_BOOK_ID,
        )
        .expect("nfo tick");
        apply_quote(&registry, &mut book, nfo_tick).unwrap();
        assert_eq!(
            book.get("kotak-nse-bse-cash", "nse_cm|2885").unwrap().last,
            "1400.50"
        );
        assert_eq!(
            book.get(KOTAK_NSE_NFO_BOOK_ID, "nse_fo|12345")
                .unwrap()
                .last,
            "10.00"
        );

        let freshness = std::time::Duration::from_millis(1000);
        let cash = enrich_tickbook_quotes_with(
            &registry,
            &book,
            Some("nse_fo|12345"),
            freshness,
            crate::data::obtain(&kotak_neo_s1k_manifest(), "quotes"),
        );
        assert_eq!(cash.status, ObtainStatus::Success);
        assert_eq!(cash.book_id, "kotak-nse-bse-cash");
        assert_eq!(cash.data.as_ref().unwrap()["last"], "1400.50");
        assert_ne!(cash.data.as_ref().unwrap()["last"], "10.00");

        let nfo = enrich_tickbook_quotes_with(
            &registry,
            &book,
            Some("nse_fo|12345"),
            freshness,
            crate::data::obtain(&kotak_neo_nfo_manifest(), "quotes"),
        );
        assert_eq!(nfo.status, ObtainStatus::Success);
        assert_eq!(nfo.book_id, "kotak-nse-nfo");
        assert_eq!(nfo.data.as_ref().unwrap()["last"], "10.00");
    }
}
