//! S1 desk: Binance.com public last price into TickBook. No depth, no Neon, no keys.
//!
//! CI parses fixture JSON only. Live `wss://` follows runtime subscriptions.

use super::apply::apply_quote;
use super::descriptor::BINANCE_COM_ADAPTER_ID;
use super::registry::Registry;
use super::tick::{QuoteTick, Transport};
use super::tickbook::TickBook;
use chrono::{DateTime, Utc};
use futures::StreamExt;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

pub fn normalize_quote_instrument(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

pub fn binance_public_trade_stream_url(instrument: &str) -> String {
    format!(
        "wss://stream.binance.com:9443/ws/{}@trade",
        normalize_quote_instrument(instrument)
    )
}

/// Map a public Binance JSON payload to a quote tick. Depth / force-order → `None`.
pub fn quote_tick_from_binance_json(raw: &str, received_at: DateTime<Utc>) -> Option<QuoteTick> {
    let value: Value = serde_json::from_str(raw).ok()?;
    if value.get("e").and_then(Value::as_str) == Some("depthUpdate")
        || value.get("e").and_then(Value::as_str) == Some("forceOrder")
    {
        return None;
    }

    if value.get("e").and_then(Value::as_str) == Some("trade") {
        let symbol = value.get("s").and_then(Value::as_str)?;
        let last = value.get("p").and_then(Value::as_str)?.to_string();
        let (as_of, age_unknown) = match value.get("T").and_then(Value::as_i64) {
            Some(ms) => match DateTime::from_timestamp_millis(ms) {
                Some(ts) => (ts, false),
                None => (received_at, true),
            },
            None => (received_at, true),
        };
        return Some(QuoteTick {
            instrument_id: normalize_quote_instrument(symbol),
            last,
            as_of,
            received_at,
            age_unknown,
            transport: Transport::Stream,
            adapter_id: BINANCE_COM_ADAPTER_ID.to_string(),
            session_ohlc: None,
        });
    }

    if value.get("e").and_then(Value::as_str) == Some("24hrMiniTicker") {
        let symbol = value.get("s").and_then(Value::as_str)?;
        let last = value.get("c").and_then(Value::as_str)?.to_string();
        let (as_of, age_unknown) = match value.get("E").and_then(Value::as_i64) {
            Some(ms) => match DateTime::from_timestamp_millis(ms) {
                Some(ts) => (ts, false),
                None => (received_at, true),
            },
            None => (received_at, true),
        };
        return Some(QuoteTick {
            instrument_id: normalize_quote_instrument(symbol),
            last,
            as_of,
            received_at,
            age_unknown,
            transport: Transport::Stream,
            adapter_id: BINANCE_COM_ADAPTER_ID.to_string(),
            session_ohlc: None,
        });
    }

    let symbol = value.get("symbol").and_then(Value::as_str)?;
    let last = value.get("price").and_then(Value::as_str)?.to_string();
    Some(QuoteTick {
        instrument_id: normalize_quote_instrument(symbol),
        last,
        as_of: received_at,
        received_at,
        age_unknown: true,
        transport: Transport::Rest,
        adapter_id: BINANCE_COM_ADAPTER_ID.to_string(),
        session_ohlc: None,
    })
}

/// Subscribe the instrument and keep a public trade stream applied as `Transport::Stream`.
/// Does not fall back to REST on disconnect. No keys on the socket.
pub fn spawn_binance_com_trade_loop(
    registry: Arc<Registry>,
    book: Arc<Mutex<TickBook>>,
    symbol: String,
) {
    tokio::spawn(async move {
        let instrument = normalize_quote_instrument(&symbol);
        {
            let mut guard = book.lock().expect("tickbook mutex poisoned");
            guard.subscribe(BINANCE_COM_ADAPTER_ID, &instrument);
        }
        let url = binance_public_trade_stream_url(&instrument);
        tracing::info!(instrument = %instrument, url = %url, "s1 desk: binance_com trade stream");
        loop {
            if let Err(err) = run_one_connection(registry.as_ref(), &book, &url).await {
                tracing::warn!(instrument = %instrument, error = %err, "s1 desk: stream ended");
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

/// Starts a stream once per instrument. Subsequent calls are no-ops.
pub fn ensure_binance_com_trade_stream(
    registry: Arc<Registry>,
    book: Arc<Mutex<TickBook>>,
    spawned: &Arc<Mutex<std::collections::HashSet<String>>>,
    symbol: &str,
) {
    let instrument = normalize_quote_instrument(symbol);
    if instrument.is_empty() {
        return;
    }
    {
        let mut guard = spawned.lock().expect("quote stream set poisoned");
        if !guard.insert(instrument.clone()) {
            return;
        }
    }
    spawn_binance_com_trade_loop(registry, book, instrument);
}

async fn run_one_connection(
    registry: &Registry,
    book: &Arc<Mutex<TickBook>>,
    url: &str,
) -> Result<(), anyhow::Error> {
    let (ws, _response) = connect_async(url).await?;
    let (_write, mut read) = ws.split();
    while let Some(msg) = read.next().await {
        let msg = msg?;
        let text = match msg {
            Message::Text(text) => text.to_string(),
            Message::Ping(_) | Message::Pong(_) | Message::Binary(_) => continue,
            Message::Close(_) | Message::Frame(_) => break,
        };
        let received_at = Utc::now();
        let Some(tick) = quote_tick_from_binance_json(&text, received_at) else {
            continue;
        };
        if tick.transport != Transport::Stream {
            continue;
        }
        let mut guard = book.lock().expect("tickbook mutex poisoned");
        match apply_quote(registry, &mut guard, tick) {
            Ok(_) => {}
            Err(err) => tracing::debug!(error = %err, "s1 desk: apply refused"),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::apply::{apply_quote, ApplyError};
    use crate::data::descriptor::binance_com_quote_descriptor;
    use crate::data::extract::{extract_quote, QuoteStatus};
    use chrono::{TimeZone, Utc};
    use std::time::Duration as StdDuration;

    fn received() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 25, 12, 0, 0).unwrap()
    }

    fn desk_registry() -> Registry {
        Registry::load(&[binance_com_quote_descriptor()]).expect("binance_com quote loads")
    }

    #[test]
    fn parses_public_trade_as_stream_quote() {
        let json = r#"{
            "e":"trade","E":1672515782136,"s":"BTCUSDT","t":1,
            "p":"96450.12","q":"0.01","T":1672515782136,"m":true,"M":true
        }"#;
        let tick = quote_tick_from_binance_json(json, received()).unwrap();
        assert_eq!(tick.instrument_id, "btcusdt");
        assert_eq!(tick.last, "96450.12");
        assert_eq!(tick.transport, Transport::Stream);
        assert_eq!(tick.adapter_id, BINANCE_COM_ADAPTER_ID);
        assert!(!tick.age_unknown);
        assert_eq!(
            tick.as_of,
            DateTime::from_timestamp_millis(1672515782136).unwrap()
        );
    }

    #[test]
    fn parses_rest_ticker_as_rest_transport() {
        let json = r#"{"symbol":"BTCUSDT","price":"96450.12"}"#;
        let tick = quote_tick_from_binance_json(json, received()).unwrap();
        assert_eq!(tick.transport, Transport::Rest);
        assert!(tick.age_unknown);
        assert_eq!(tick.instrument_id, "btcusdt");
    }

    #[test]
    fn ignores_depth_and_force_order() {
        let depth = r#"{"e":"depthUpdate","E":1,"s":"BTCUSDT","U":1,"u":2,"b":[],"a":[]}"#;
        let force = r#"{"e":"forceOrder","E":1,"o":{}}"#;
        assert!(quote_tick_from_binance_json(depth, received()).is_none());
        assert!(quote_tick_from_binance_json(force, received()).is_none());
    }

    #[test]
    fn stream_applies_and_rest_closes_when_subscribed() {
        let registry = desk_registry();
        let mut book = TickBook::new();
        book.subscribe(BINANCE_COM_ADAPTER_ID, "btcusdt");

        let trade = quote_tick_from_binance_json(
            r#"{"e":"trade","E":1,"s":"BTCUSDT","p":"100.00","T":1672515782136}"#,
            received(),
        )
        .unwrap();
        apply_quote(&registry, &mut book, trade).unwrap();
        assert_eq!(
            book.get(BINANCE_COM_ADAPTER_ID, "btcusdt")
                .map(|row| row.last.as_str()),
            Some("100.00")
        );

        let rest =
            quote_tick_from_binance_json(r#"{"symbol":"BTCUSDT","price":"99.00"}"#, received())
                .unwrap();
        let err = apply_quote(&registry, &mut book, rest).unwrap_err();
        assert_eq!(
            err,
            ApplyError::RestClosed {
                instrument_id: "btcusdt".to_string()
            }
        );
        assert_eq!(
            book.get(BINANCE_COM_ADAPTER_ID, "btcusdt")
                .map(|row| row.last.as_str()),
            Some("100.00")
        );

        let as_of = DateTime::from_timestamp_millis(1672515782136).unwrap();
        let envelope = extract_quote(
            &registry,
            &book,
            "btcusdt",
            as_of,
            StdDuration::from_millis(1000),
        );
        assert_eq!(envelope.status, QuoteStatus::Fresh);
        assert_eq!(
            envelope.data.as_ref().map(|d| d.last.as_str()),
            Some("100.00")
        );
        assert_eq!(envelope.provenance.adapter_id, BINANCE_COM_ADAPTER_ID);
        assert!(!envelope.canonical);
        assert!(!envelope.persist_canonical);
    }

    #[test]
    fn trade_stream_url_is_public_and_symbol_lowercased() {
        assert_eq!(
            binance_public_trade_stream_url("BTCUSDT"),
            "wss://stream.binance.com:9443/ws/btcusdt@trade"
        );
    }
}
