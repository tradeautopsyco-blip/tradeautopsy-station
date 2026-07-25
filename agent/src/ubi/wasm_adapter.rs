//! `BrokerAdapter` backed by a sandboxed Wasm component (Phase 3).
//!
//! The poll loop keeps its existing shape; every broker call now goes through
//! `run_fetch_fills` → component → host `broker_http_call`, so credentials stay in the
//! Enforcer and the adapter binary has no ambient network authority (ADR 0001).

use crate::broker::{BrokerAdapter, BrokerError, BrokerFill};
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
    asset_class: String,
    credentials: HostCredentialBlob,
    transport: Arc<dyn BrokerHttpTransport>,
    name: &'static str,
}

impl WasmBrokerAdapter {
    pub fn new(
        broker_slug: &str,
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
            Err(err) => Err(classify_adapter_error(&err.to_string())),
        }
    }
}

/// Components signal venue conditions through the error string the host classified for
/// them (`rate_limited`, `session_expired`, …) — never through credential-derived detail.
fn classify_adapter_error(message: &str) -> BrokerError {
    if message.contains("rate_limited") {
        return BrokerError::RateLimited {
            retry_after_ms: Some(60_000),
        };
    }
    BrokerError::Http(message.to_string())
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

    #[test]
    fn fill_event_maps_onto_poll_loop_fill() {
        let fill = fill_event_to_broker_fill(&sample_event());
        assert_eq!(fill.fill_id, "28457");
        assert_eq!(fill.trade_id, "28457");
        assert_eq!(fill.broker, "binance_com");
        assert_eq!(fill.fee_asset.as_deref(), Some("BNB"));
        assert_eq!(fill.filled_at.timestamp_millis(), 1_499_865_549_590);
    }

    #[test]
    fn rate_limited_component_error_maps_to_backoff() {
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
    fn adapter_names_are_stable_per_slug() {
        assert_eq!(static_adapter_name("binance_com"), "binance_com_wasm");
        assert_eq!(static_adapter_name("kotak_neo"), "kotak_neo_wasm");
    }
}
