//! Host-owned describe / obtain. Loopback like quote. No Wasm `describe`/`obtain`.

use crate::api::desk::{account_identity, quote_status_wire, reference_identity};
use crate::api::AppState;
use crate::data::{
    depth_obtain_data, describe, extract_depth, extract_licensed_history, extract_quote_for,
    history_obtain_data, obtain, DepthStatus, ObtainEnvelope, ObtainStatus, QuoteStatus,
    SourceManifest, DEFAULT_HISTORY_INTERVAL,
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
}

#[derive(Debug, Serialize)]
pub struct ManifestList {
    pub manifests: Vec<SourceManifest>,
}

#[derive(Debug, Deserialize)]
pub struct ObtainQuery {
    pub adapter: Option<String>,
    pub operation: Option<String>,
}

pub async fn manifest_handler(
    State(state): State<AppState>,
    Query(query): Query<ManifestQuery>,
) -> Json<ManifestList> {
    let adapter = query
        .adapter
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let manifests = match adapter {
        Some(slug) => state
            .source_manifests
            .iter()
            .filter(|manifest| manifest.adapter_id == slug)
            .filter(|manifest| describe(manifest).is_ok())
            .cloned()
            .collect(),
        None => state
            .source_manifests
            .iter()
            .filter(|manifest| describe(manifest).is_ok())
            .cloned()
            .collect(),
    };
    Json(ManifestList { manifests })
}

pub async fn obtain_handler(
    State(state): State<AppState>,
    Query(query): Query<ObtainQuery>,
) -> Json<ObtainEnvelope> {
    let adapter = query
        .adapter
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    let operation = query
        .operation
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or_default();
    if adapter.is_empty() || operation.is_empty() {
        return Json(ObtainEnvelope {
            adapter_id: adapter.to_string(),
            operation: operation.to_string(),
            status: ObtainStatus::Unsupported,
            data: None,
            provenance_adapter_id: None,
        });
    }
    let Some(manifest) = state
        .source_manifests
        .iter()
        .find(|manifest| manifest.adapter_id == adapter)
        .filter(|manifest| describe(manifest).is_ok())
    else {
        return Json(ObtainEnvelope {
            adapter_id: adapter.to_string(),
            operation: operation.to_string(),
            status: ObtainStatus::Unsupported,
            data: None,
            provenance_adapter_id: None,
        });
    };
    let mut envelope = obtain(manifest, operation);
    if envelope.status == ObtainStatus::Unavailable {
        envelope = enrich_obtain(&state, envelope);
    }
    Json(envelope)
}

fn enrich_obtain(state: &AppState, mut envelope: ObtainEnvelope) -> ObtainEnvelope {
    match envelope.operation.as_str() {
        "quotes" if envelope.adapter_id == "binance_com" || envelope.adapter_id == "kotak_neo" => {
            let adapter = envelope.adapter_id.clone();
            let mut instrument = state
                .selected_quote_instrument
                .lock()
                .expect("selected quote instrument poisoned")
                .clone()
                .unwrap_or_default();
            let book = state.tickbook.lock().expect("tickbook mutex poisoned");
            let selected_ok = !instrument.is_empty()
                && book
                    .get(&instrument)
                    .is_some_and(|row| row.adapter_id == adapter);
            if !selected_ok {
                if let Some((id, _)) = book.iter().find(|(_, row)| row.adapter_id == adapter) {
                    instrument = id.clone();
                } else {
                    instrument.clear();
                }
            }
            if instrument.is_empty() {
                return envelope;
            }
            let quote = extract_quote_for(
                state.quote_registry.as_ref(),
                &book,
                &instrument,
                Utc::now(),
                state.quote_freshness,
                Some(&adapter),
            );
            if let Some(data) = tickbook_quote_obtain_data(&quote) {
                envelope.status = ObtainStatus::Success;
                envelope.data = Some(data);
                envelope.provenance_adapter_id = Some(quote.provenance.adapter_id);
            }
        }
        "depth" if envelope.adapter_id == "kotak_neo" || envelope.adapter_id == "binance_com" => {
            let adapter = envelope.adapter_id.clone();
            let book = state.depthbook.lock().expect("depthbook mutex poisoned");
            let mut instrument = state
                .resolve_candidates()
                .first()
                .cloned()
                .unwrap_or_else(|| state.s1_desk_symbol.clone().unwrap_or_default());
            if adapter == "binance_com" {
                instrument = crate::data::normalize_quote_instrument(&instrument);
            }
            if instrument.is_empty() || book.get(&instrument).is_none() {
                if let Some((id, _)) = book
                    .iter()
                    .find(|(_, row)| row.adapter_id == adapter && row.completeness)
                {
                    instrument = id.clone();
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
        }
        "funds" if envelope.adapter_id == "binance_com" => {
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
        }
        "tradebook" => {
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
        }
        "instruments" if envelope.adapter_id == "binance_com" => {
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
        }
        "instruments" if envelope.adapter_id == "kotak_neo" => {
            let master = state
                .kotak_scrip_master
                .lock()
                .expect("kotak scrip master mutex poisoned");
            if let Some(data) = kotak_instruments_data(&master) {
                envelope.status = ObtainStatus::Success;
                envelope.data = Some(data);
            }
        }
        "history" if envelope.adapter_id == "binance_com" => {
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
            // Empty id may use first_for_adapter inside extract (boot/CI).
            // A selected/desk id that misses stays unavailable — never another pair.
            let history =
                extract_licensed_history(&book, &instrument, Some(DEFAULT_HISTORY_INTERVAL), None);
            if let Some(data) = history_obtain_data(&history) {
                envelope.status = ObtainStatus::Success;
                envelope.data = Some(data);
                envelope.provenance_adapter_id = Some(history.provenance.adapter_id);
            }
        }
        _ => {}
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
        assert!(TickBook::new().get("nse_cm|2885").is_none());
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
        assert!(TickBook::new().get("btcusdt").is_none());
        assert!(history_obtain_data(&extract_licensed_history(
            &HistoryBook::new(),
            "btcusdt",
            Some("1m"),
            None
        ))
        .is_none());
    }
}
