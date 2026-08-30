use crate::api::desk::quote_status_wire;
use crate::api::AppState;
use crate::data::{
    extract_quote, extract_quote_for, quote_subscription_for, QuoteStatus, QuoteSubscription,
};
use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;

pub async fn search_instruments(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let q = params.get("q").cloned().unwrap_or_default();
    let master_status = state.instrument_master_wire();
    if q.len() < 2 {
        if state.is_binance_com_desk() || state.is_kotak_neo_desk() {
            state.maybe_kick_instrument_master_refresh();
        }
        return Json(json!({ "symbols": [], "master_status": master_status }));
    }

    if state.is_binance_com_desk() {
        let symbols = binance_master_symbols(&state, &q);
        if symbols.is_empty() {
            state.maybe_kick_instrument_master_refresh();
        }
        return Json(
            json!({ "symbols": symbols, "master_status": state.instrument_master_wire() }),
        );
    }

    if state.is_kotak_neo_desk() {
        let symbols = kotak_master_symbols(&state, &q);
        if symbols.is_empty() {
            state.maybe_kick_instrument_master_refresh();
        }
        return Json(
            json!({ "symbols": symbols, "master_status": state.instrument_master_wire() }),
        );
    }

    if !crate::zerodha_instruments_enabled() {
        return Json(json!({ "symbols": [], "master_status": master_status }));
    }

    let symbols = match state.instruments.search(&q, 10) {
        Ok(results) => results
            .into_iter()
            .map(|r| {
                json!({
                    "trading_symbol": r.trading_symbol,
                    "name": r.name,
                    "exchange": r.exchange,
                    "segment": r.segment,
                    "instrument_token": r.instrument_token,
                    "last_price": r.last_price,
                })
            })
            .collect::<Vec<_>>(),
        Err(e) => {
            tracing::error!("instrument search error: {}", e);
            Vec::new()
        }
    };

    Json(json!({ "symbols": symbols, "master_status": master_status }))
}

fn kotak_master_symbols(state: &AppState, q: &str) -> Vec<serde_json::Value> {
    let master = state
        .kotak_scrip_master
        .lock()
        .expect("kotak scrip master mutex poisoned");
    let hits = master.search(q, 10);
    drop(master);
    hits.into_iter()
        .map(|hit| {
            let instrument_id =
                crate::kotak_scrip_master::instrument_id(&hit.segment, hit.instrument_token);
            let last_price = {
                let book = state.tickbook.lock().expect("tickbook mutex poisoned");
                let env = extract_quote_for(
                    state.quote_registry.as_ref(),
                    &book,
                    &instrument_id,
                    Utc::now(),
                    state.quote_freshness,
                    Some(crate::data::KOTAK_NEO_ADAPTER_ID),
                );
                env.data
                    .and_then(|d| d.last.parse::<f64>().ok())
                    .filter(|p| *p > 0.0)
                    .unwrap_or(0.0)
            };
            json!({
                "trading_symbol": hit.trading_symbol,
                "name": hit.name,
                "exchange": hit.exchange,
                "segment": hit.segment,
                "instrument_token": hit.instrument_token,
                "last_price": last_price,
            })
        })
        .collect()
}

fn binance_master_symbols(state: &AppState, q: &str) -> Vec<serde_json::Value> {
    let master = state
        .instrument_master
        .lock()
        .expect("instrument master mutex poisoned");
    let mut hits = master.search_trading(q, 10);
    drop(master);
    if hits.is_empty() {
        hits = state
            .resolve_candidates()
            .into_iter()
            .filter(|id| crate::data::resolve_among(q, std::iter::once(id.as_str())).is_some())
            .map(|id| crate::exchange_info::InstrumentSearchHit {
                symbol: id.to_ascii_uppercase(),
                base_asset: id.to_ascii_uppercase(),
                quote_asset: String::new(),
            })
            .take(10)
            .collect();
    }
    hits.into_iter()
        .map(|hit| {
            let instrument_id = crate::data::normalize_quote_instrument(&hit.symbol);
            let last_price = {
                let book = state.tickbook.lock().expect("tickbook mutex poisoned");
                let env = extract_quote(
                    state.quote_registry.as_ref(),
                    &book,
                    &instrument_id,
                    Utc::now(),
                    state.quote_freshness,
                );
                env.data
                    .and_then(|d| d.last.parse::<f64>().ok())
                    .filter(|p| *p > 0.0)
                    .unwrap_or(0.0)
            };
            json!({
                "trading_symbol": hit.symbol,
                "name": hit.base_asset,
                "exchange": "binance_com",
                "segment": "SPOT",
                "instrument_token": 0,
                "last_price": last_price,
            })
        })
        .collect()
}

pub async fn get_ltp(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let symbol = params.get("symbol").cloned().unwrap_or_default();
    let exchange = params
        .get("exchange")
        .cloned()
        .unwrap_or_else(|| "NSE".to_string());
    let segment = params
        .get("segment")
        .cloned()
        .unwrap_or_else(|| "NSE".to_string());

    if symbol.is_empty() {
        return Json(json!({ "ltp": null, "source": "none" }));
    }

    let tick_id = state.validated_quote_id(&symbol).unwrap_or_default();
    if !tick_id.is_empty() {
        match quote_subscription_for(&tick_id, state.should_subscribe_quote(&tick_id)) {
            QuoteSubscription::OptionsQuote => state.bind_spot_market(&tick_id),
            QuoteSubscription::Desk => state.subscribe_instrument(&tick_id),
            QuoteSubscription::None => {}
        }
    }
    if !tick_id.is_empty() {
        let book = state.tickbook.lock().expect("tickbook mutex poisoned");
        let env = extract_quote_for(
            state.quote_registry.as_ref(),
            &book,
            &tick_id,
            Utc::now(),
            state.quote_freshness,
            state.active_adapter_id().as_deref(),
        );
        if env.status != QuoteStatus::Unavailable {
            if let Some(data) = env.data {
                if let Ok(ltp) = data.last.parse::<f64>() {
                    if ltp > 0.0 {
                        return Json(json!({
                            "ltp": ltp,
                            "source": "tickbook",
                            "quote_status": quote_status_wire(env.status),
                        }));
                    }
                }
            }
        }
    }

    if state.is_binance_com_desk()
        || state.is_binance_path(&tick_id)
        || exchange.eq_ignore_ascii_case("binance_com")
    {
        return Json(json!({
            "ltp": null,
            "source": "tickbook",
            "quote_status": "unavailable",
        }));
    }

    if state.is_kotak_neo_desk() || exchange.eq_ignore_ascii_case("kotak_neo") {
        return Json(json!({
            "ltp": null,
            "source": "tickbook",
            "quote_status": "unavailable",
        }));
    }

    if !crate::zerodha_instruments_enabled() {
        return Json(json!({ "ltp": null, "source": "none" }));
    }

    if let Ok(Some(price)) = state.instruments.get_last_price(&symbol) {
        if price > 0.0 {
            return Json(json!({ "ltp": price, "source": "cache" }));
        }
    }

    // Wire middleware already authenticated the loopback caller; upstream is Bearer only.
    // Kotak/Zerodha cache path only — never Console LTP for the TickBook desk.
    let url = format!("{}/api/bar/v1/broker/ltp", state.upstream.config.base_url);
    let body = json!({
        "trading_symbol": symbol,
        "exchange": exchange,
        "segment": segment,
    });

    let req = match state
        .upstream
        .authorize_brain(state.upstream.http.post(&url).json(&body))
    {
        Ok(r) => r,
        Err(err) => {
            return Json(json!({
                "ltp": null,
                "source": "none",
                "error": err.to_string(),
            }));
        }
    };

    match req.send().await {
        Ok(resp) => {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                if let Some(ltp) = data["ltp"].as_f64() {
                    if ltp > 0.0 {
                        let _ = state.instruments.upsert_last_price(&symbol, ltp);
                        return Json(json!({ "ltp": ltp, "source": "broker" }));
                    }
                }
                if let Some(err) = data.get("error").and_then(|v| v.as_str()) {
                    return Json(json!({
                        "ltp": null,
                        "source": "none",
                        "error": err,
                    }));
                }
            }
            Json(json!({ "ltp": null, "source": "none" }))
        }
        Err(e) => {
            tracing::debug!("broker ltp upstream failed: {e}");
            Json(json!({ "ltp": null, "source": "none" }))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::data::normalize_quote_instrument;
    use crate::exchange_info::ExchangeInfoSymbolCache;

    #[test]
    fn master_search_does_not_invent_binance_public_last() {
        let json = r#"{
            "symbols": [
                { "symbol": "BTCUSDT", "baseAsset": "BTC", "quoteAsset": "USDT", "status": "TRADING" }
            ]
        }"#;
        let cache = ExchangeInfoSymbolCache::from_exchange_info_json(json).unwrap();
        let hits = cache.search_trading("btc", 10);
        assert_eq!(hits[0].symbol, "BTCUSDT");
        assert_ne!(hits[0].base_asset, "Binance public last");
        assert_eq!(normalize_quote_instrument(&hits[0].symbol), "btcusdt");
    }

    #[test]
    fn kotak_search_json_is_reliance_not_nse_or_binance() {
        use crate::kotak_scrip_master::KotakScripMaster;
        let csv = include_str!("../../fixtures/kotak/nse_cm_cash.csv");
        let master = KotakScripMaster::from_csv_bytes(csv.as_bytes(), None).unwrap();
        let hit = master
            .search("RELIANCE", 10)
            .into_iter()
            .find(|h| h.segment == "nse_cm")
            .expect("nse_cm RELIANCE");
        let row = serde_json::json!({
            "trading_symbol": hit.trading_symbol,
            "name": hit.name,
            "exchange": hit.exchange,
            "segment": hit.segment,
            "instrument_token": hit.instrument_token,
            "last_price": 0.0,
        });
        assert_eq!(row["trading_symbol"], "RELIANCE");
        assert_eq!(row["exchange"], "kotak_neo");
        assert_eq!(row["segment"], "nse_cm");
        assert_eq!(row["instrument_token"], 2885);
        assert_ne!(row["exchange"], "NSE");
        assert_ne!(row["exchange"], "BINANCE");
        assert_ne!(row["exchange"], "binance_com");
        assert_eq!(
            crate::kotak_scrip_master::instrument_id("nse_cm", 2885),
            "nse_cm|2885"
        );
    }
}
