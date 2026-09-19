//! `BrokerAdapter` backed by a sandboxed Wasm component (Phase 3).
//!
//! The poll loop keeps its existing shape; every broker call now goes through
//! `run_fetch_fills` → component → host `broker_http_call`, so credentials stay in the
//! Enforcer and the adapter binary has no ambient network authority (ADR 0001).

use crate::broker::{slug_to_slot, BrokerAdapter, BrokerError, BrokerFill};
use crate::ubi::components::component_path_for_slug;
use crate::ubi::host::{run_fetch_fills, FillCursor, FillEvent, UbiHostConfig, UbiHostState};
use crate::ubi::http::{BrokerHttpTransport, HostCredentialBlob};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::path::PathBuf;
use std::sync::Arc;

pub struct WasmBrokerAdapter {
    component_path: PathBuf,
    connection_id: String,
    broker_slug: String,
    book_id: String,
    asset_class: String,
    credentials: HostCredentialBlob,
    transport: Arc<dyn BrokerHttpTransport>,
    name: &'static str,
}

impl WasmBrokerAdapter {
    pub fn new(
        broker_slug: &str,
        book_id: impl Into<String>,
        connection_id: impl Into<String>,
        asset_class: impl Into<String>,
        credentials: HostCredentialBlob,
        transport: Arc<dyn BrokerHttpTransport>,
    ) -> Result<Self> {
        let component_path = component_path_for_slug(broker_slug).ok_or_else(|| {
            anyhow!(
                "no adapter component found for {broker_slug} \
                 (build it for wasm32-wasip2 or set TRADEAUTOPSY_UBI_COMPONENT_DIR)"
            )
        })?;
        Ok(Self {
            component_path,
            connection_id: connection_id.into(),
            broker_slug: broker_slug.to_string(),
            book_id: book_id.into(),
            asset_class: asset_class.into(),
            credentials,
            transport,
            name: static_adapter_name(broker_slug),
        })
    }

    fn host_state(&self) -> UbiHostState {
        UbiHostState::live(
            UbiHostConfig {
                connection_id: self.connection_id.clone(),
                broker_slug: self.broker_slug.clone(),
                book_id: self.book_id.clone(),
                asset_class: self.asset_class.clone(),
                credentials: self.credentials.clone(),
            },
            self.transport.clone(),
        )
    }
}

fn static_adapter_name(slug: &str) -> &'static str {
    match slug {
        "binance_com" => "binance_com_wasm",
        "kotak_neo" => "kotak_neo_wasm",
        _ => "ubi_wasm",
    }
}

#[async_trait]
impl BrokerAdapter for WasmBrokerAdapter {
    fn name(&self) -> &'static str {
        self.name
    }

    async fn poll_fills(
        &self,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<BrokerFill>, BrokerError> {
        // Time cursor only. Official paging is per-symbol `fromId` (adapter pages a
        // full 500-row window). The poller still does not pin symbol/fromId — B6
        // incremental-from-last-id stays a remaining FAIL until per-symbol cursors.
        let cursor = FillCursor {
            since_unix_ms: since.map(|t| t.timestamp_millis()),
            from_id: None,
            symbol: None,
        };
        let path = self.component_path.clone();
        let state = self.host_state();
        let broker = self.broker_slug.clone();

        let result = tokio::task::spawn_blocking(move || run_fetch_fills(&path, state, cursor))
            .await
            .map_err(|e| BrokerError::Http(format!("{broker} component task: {e}")))?;

        match result {
            Ok((fills, _state)) => Ok(fills.iter().map(fill_event_to_broker_fill).collect()),
            Err(err) => Err(classify_adapter_error_for(&err.to_string(), self.name)),
        }
    }
}

/// Components signal venue conditions through the error string the host classified for
/// them (`rate_limited`, `session_expired`, …) — never through credential-derived detail.
/// The engine owns the clock: `VenueStopped.until_ms` stays None here; RateLimited
/// prefers `posture_for` remaining-window over a hardcoded 60s.
fn classify_adapter_error_for(message: &str, adapter_name: &str) -> BrokerError {
    if message.contains("venue_banned") || message.contains("venue_frozen") {
        return BrokerError::VenueStopped { until_ms: None };
    }
    if message.contains("rate_limited") {
        let retry_after_ms = slug_to_slot(adapter_name)
            .and_then(|slot| crate::egress::shared_engine().posture_for(slot))
            .and_then(|p| p.until_ms)
            .map(|until| {
                let now = chrono::Utc::now().timestamp_millis();
                (until - now).max(0)
            })
            .or(Some(60_000));
        return BrokerError::RateLimited { retry_after_ms };
    }
    BrokerError::Http(message.to_string())
}

#[cfg(test)]
fn classify_adapter_error(message: &str) -> BrokerError {
    classify_adapter_error_for(message, message)
}

pub fn fill_event_to_broker_fill(fill: &FillEvent) -> BrokerFill {
    BrokerFill {
        fill_id: fill.fill_id.clone(),
        trade_id: fill
            .trade_id
            .clone()
            .unwrap_or_else(|| fill.fill_id.clone()),
        symbol: fill.symbol.clone(),
        side: fill.side.clone(),
        qty: fill.qty,
        price: fill.price,
        filled_at: DateTime::<Utc>::from_timestamp_millis(fill.filled_at_unix_ms)
            .unwrap_or_else(Utc::now),
        broker: fill.broker_slug.clone(),
        fee_amount: fill.fee_amount,
        fee_asset: fill.fee_currency.clone(),
        // I-N3: copy venue cash fields — dropping INR / nse_cm / CNC|MIS is a never-again.
        currency: nonempty_owned(&fill.currency),
        product: fill.product.clone().filter(|p| !p.is_empty()),
        exchange_segment: fill.exchange_segment.clone().filter(|s| !s.is_empty()),
        instrument_type: nfo_ce_pe_fut_from_symbol(fill.exchange_segment.as_deref(), &fill.symbol),
        // Lot is slice-2 master-owned. Never invent 1. Never copy onto COM/spot.
        lot: None,
    }
}

/// CE/PE/FUT from the fill's trading symbol on `nse_fo` only — never product, never `"NSE"`.
fn nfo_ce_pe_fut_from_symbol(segment: Option<&str>, symbol: &str) -> Option<String> {
    let seg = segment?.trim().to_ascii_lowercase();
    if seg != "nse_fo" {
        return None;
    }
    let upper = symbol.trim().to_ascii_uppercase();
    for suffix in ["CE", "PE", "FUT"] {
        if upper.ends_with(suffix) {
            return Some(suffix.to_string());
        }
    }
    None
}

fn nonempty_owned(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_event() -> FillEvent {
        FillEvent {
            fill_id: "28457".into(),
            broker_slug: "binance_com".into(),
            connection_id: "conn-1".into(),
            asset_class: "crypto_spot".into(),
            symbol: "BTCUSDT".into(),
            side: "BUY".into(),
            qty: 12.0,
            price: 4.000001,
            currency: "USDT".into(),
            filled_at_unix_ms: 1_499_865_549_590,
            fee_amount: Some(0.012),
            fee_currency: Some("BNB".into()),
            exchange_segment: None,
            product: None,
            trade_id: None,
        }
    }

    fn kotak_cash_event(product: &str) -> FillEvent {
        FillEvent {
            fill_id: "FILL-1".into(),
            broker_slug: "kotak_neo".into(),
            connection_id: "conn-1".into(),
            asset_class: "equities".into(),
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            qty: 1.0,
            price: 2500.0,
            currency: "INR".into(),
            filled_at_unix_ms: 1_784_968_500_000,
            fee_amount: None,
            fee_currency: None,
            exchange_segment: Some("nse_cm".into()),
            product: Some(product.into()),
            trade_id: Some("NSE998877".into()),
        }
    }

    #[test]
    fn fill_event_maps_onto_poll_loop_fill() {
        let fill = fill_event_to_broker_fill(&sample_event());
        assert_eq!(fill.fill_id, "28457");
        assert_eq!(fill.trade_id, "28457");
        assert_eq!(fill.broker, "binance_com");
        assert_eq!(fill.fee_asset.as_deref(), Some("BNB"));
        assert_eq!(fill.filled_at.timestamp_millis(), 1_499_865_549_590);
        assert_eq!(fill.currency.as_deref(), Some("USDT"));
    }

    #[test]
    fn fill_event_does_not_drop_inr_cash_fields() {
        // I-N3: INR + nse_cm + CNC/MIS must survive FillEvent → BrokerFill.
        let cnc = fill_event_to_broker_fill(&kotak_cash_event("CNC"));
        assert_eq!(cnc.currency.as_deref(), Some("INR"));
        assert_eq!(cnc.exchange_segment.as_deref(), Some("nse_cm"));
        assert_eq!(cnc.product.as_deref(), Some("CNC"));

        let mis = fill_event_to_broker_fill(&kotak_cash_event("MIS"));
        assert_eq!(mis.currency.as_deref(), Some("INR"));
        assert_eq!(mis.exchange_segment.as_deref(), Some("nse_cm"));
        assert_eq!(mis.product.as_deref(), Some("MIS"));
    }

    #[test]
    fn fill_event_preserves_nfo_segment_and_does_not_map_to_cash() {
        let fill = fill_event_to_broker_fill(&FillEvent {
            fill_id: "NFO-1".into(),
            broker_slug: "kotak_neo".into(),
            connection_id: "conn-1".into(),
            asset_class: "nfo".into(),
            symbol: "NIFTY".into(),
            side: "BUY".into(),
            qty: 15.0,
            price: 200.0,
            currency: "INR".into(),
            filled_at_unix_ms: 1_784_968_500_000,
            fee_amount: None,
            fee_currency: None,
            exchange_segment: Some("nse_fo".into()),
            product: Some("NRML".into()),
            trade_id: Some("NFO998877".into()),
        });
        assert_eq!(fill.exchange_segment.as_deref(), Some("nse_fo"));
        assert_eq!(fill.product.as_deref(), Some("NRML"));
        assert_eq!(fill.qty, 15.0);
        assert_eq!(fill.currency.as_deref(), Some("INR"));
        assert_ne!(fill.exchange_segment.as_deref(), Some("nse_cm"));
        assert_ne!(fill.product.as_deref(), Some("MIS"));
        assert!(fill.lot.is_none(), "lot comes from master, not FillEvent");
        assert!(
            fill.instrument_type.is_none(),
            "symbol NIFTY has no CE/PE/FUT suffix"
        );
    }

    #[test]
    fn fill_event_nfo_symbol_suffix_is_ce_pe_fut_not_product() {
        let fill = fill_event_to_broker_fill(&FillEvent {
            fill_id: "NFO-PE".into(),
            broker_slug: "kotak_neo".into(),
            connection_id: "conn-1".into(),
            asset_class: "nfo".into(),
            symbol: "NIFTY2692221000PE".into(),
            side: "BUY".into(),
            qty: 2.0,
            price: 10.0,
            currency: "INR".into(),
            filled_at_unix_ms: 1_784_968_500_000,
            fee_amount: None,
            fee_currency: None,
            exchange_segment: Some("nse_fo".into()),
            product: Some("NRML".into()),
            trade_id: Some("NFO998877".into()),
        });
        assert_eq!(fill.instrument_type.as_deref(), Some("PE"));
        assert_ne!(fill.instrument_type.as_deref(), Some("NRML"));
        assert_ne!(fill.instrument_type.as_deref(), Some("NSE"));
        assert!(fill.lot.is_none());
    }

    #[test]
    fn classify_adapter_error_rate_limited_still_rate_limited() {
        assert!(matches!(
            classify_adapter_error("adapter: binance_com http 429: rate_limited"),
            BrokerError::RateLimited { .. }
        ));
        assert!(matches!(
            classify_adapter_error("adapter: kotak_neo session_expired"),
            BrokerError::Http(_)
        ));
    }

    #[test]
    fn classify_adapter_error_venue_banned_is_venue_stopped() {
        assert!(matches!(
            classify_adapter_error("binance_com account http 0: venue_banned"),
            BrokerError::VenueStopped { until_ms: None }
        ));
    }

    #[test]
    fn classify_adapter_error_venue_frozen_is_venue_stopped() {
        assert!(matches!(
            classify_adapter_error("kotak_neo account http 0: venue_frozen"),
            BrokerError::VenueStopped { until_ms: None }
        ));
    }

    #[test]
    fn classify_adapter_error_ordinary_http_stays_http() {
        assert!(matches!(
            classify_adapter_error("binance_com account http 500: upstream"),
            BrokerError::Http(_)
        ));
    }

    #[test]
    fn adapter_names_are_stable_per_slug() {
        assert_eq!(static_adapter_name("binance_com"), "binance_com_wasm");
        assert_eq!(static_adapter_name("kotak_neo"), "kotak_neo_wasm");
    }
}
