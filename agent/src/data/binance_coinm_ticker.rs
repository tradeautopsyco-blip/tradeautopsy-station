//! S1 desk: unsigned `GET /dapi/v1/ticker/price?symbol=` on `binance-com-coinm`.
//!
//! Lock (`binance-com-coinm.md`): public last is REST field `price` (string),
//! host `dapi.binance.com`, no HMAC. Never fapi. Never spot. Never eapi `lastPrice`.
//! TickBook key preserves venue case (`BTCUSD_PERP`). Underscore is legal. TRADE off.

use super::apply::apply_quote;
use super::binance_options_public::is_dated_option_contract;
use super::descriptor::{BINANCE_COM_ADAPTER_ID, BINANCE_COM_COINM_BOOK_ID};
use super::registry::Registry;
use super::tick::{QuoteTick, Transport};
use super::tickbook::TickBook;
use crate::egress::{EgressCall, EgressError, EgressResponse, Lane};
use crate::kotak_rest_quotes::{QuoteFetchErrorClass, QuoteFetchErrorMap};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

const TICKER_MAX_AGE_MS: i64 = 1_000;

pub const COINM_DAPI_HOST: &str = "dapi.binance.com";
pub const COINM_TICKER_PATH: &str = "/dapi/v1/ticker/price";

pub fn normalize_coinm_instrument(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

fn is_coinm_pair(id: &str) -> bool {
    !id.is_empty()
        && !id.contains('|')
        && !is_dated_option_contract(id)
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
}

pub fn binance_coinm_ticker_call(instrument_id: &str) -> Option<EgressCall> {
    let id = normalize_coinm_instrument(instrument_id);
    if !is_coinm_pair(&id) {
        return None;
    }
    Some(
        EgressCall::get(
            BINANCE_COM_COINM_BOOK_ID,
            COINM_DAPI_HOST,
            COINM_TICKER_PATH,
            Lane::MarketData,
        )
        .with_query_pairs(&[("symbol", id.as_str())])
        .with_max_age_ms(TICKER_MAX_AGE_MS),
    )
}

pub fn should_prime_binance_coinm_ticker(book: &TickBook, instrument_id: &str) -> bool {
    let id = normalize_coinm_instrument(instrument_id);
    if binance_coinm_ticker_call(&id).is_none() {
        return false;
    }
    book.get(BINANCE_COM_COINM_BOOK_ID, &id).is_none()
}

pub async fn await_binance_coinm_ticker(
    registry: Arc<Registry>,
    book: Arc<Mutex<TickBook>>,
    inflight: Arc<Mutex<HashSet<String>>>,
    quote_fetch_error: QuoteFetchErrorMap,
    instrument_id: &str,
) {
    let id = normalize_coinm_instrument(instrument_id);
    {
        let guard = book.lock().expect("tickbook mutex poisoned");
        if !should_prime_binance_coinm_ticker(&guard, &id) {
            return;
        }
    }
    let Some(call) = binance_coinm_ticker_call(&id) else {
        return;
    };
    let we_own = {
        let mut guard = inflight.lock().expect("coinm ticker inflight poisoned");
        guard.insert(id.clone())
    };
    if !we_own {
        wait_for_inflight_coinm_ticker(&book, &inflight, &id).await;
        return;
    }

    let result = crate::egress::shared().send(&call).await;
    inflight
        .lock()
        .expect("coinm ticker inflight poisoned")
        .remove(&id);

    apply_coinm_ticker_result(registry, book, quote_fetch_error, &id, result);
}

async fn wait_for_inflight_coinm_ticker(
    book: &Arc<Mutex<TickBook>>,
    inflight: &Arc<Mutex<HashSet<String>>>,
    instrument_id: &str,
) {
    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        {
            let guard = book.lock().expect("tickbook mutex poisoned");
            if guard.get(BINANCE_COM_COINM_BOOK_ID, instrument_id).is_some() {
                return;
            }
        }
        if !inflight
            .lock()
            .expect("coinm ticker inflight poisoned")
            .contains(instrument_id)
        {
            return;
        }
    }
}

fn apply_coinm_ticker_result(
    registry: Arc<Registry>,
    book: Arc<Mutex<TickBook>>,
    quote_fetch_error: QuoteFetchErrorMap,
    id: &str,
    result: Result<EgressResponse, EgressError>,
) {
    let class = match result {
        Ok(resp) if resp.is_success() => {
            match quote_tick_from_coinm_ticker_json_for_symbol(&resp.body, id, Utc::now()) {
                Some(tick) => {
                    let mut guard = book.lock().expect("tickbook mutex poisoned");
                    match apply_quote(registry.as_ref(), &mut guard, tick) {
                        Ok(_) => {
                            drop(guard);
                            quote_fetch_error
                                .lock()
                                .expect("quote fetch error poisoned")
                                .remove(id);
                            return;
                        }
                        Err(_) => None,
                    }
                }
                None => Some(QuoteFetchErrorClass::QuotesUnusable),
            }
        }
        Ok(_) => Some(QuoteFetchErrorClass::QuotesHttp),
        Err(_) => Some(QuoteFetchErrorClass::QuotesHttp),
    };
    if let Some(class) = class {
        quote_fetch_error
            .lock()
            .expect("quote fetch error poisoned")
            .insert(id.to_string(), class.as_str().to_string());
    }
}

fn tick_from_ticker_object(value: &Value, received_at: DateTime<Utc>) -> Option<QuoteTick> {
    let symbol = value.get("symbol").and_then(Value::as_str)?;
    let instrument_id = symbol.trim().to_string();
    if !is_coinm_pair(&normalize_coinm_instrument(&instrument_id)) {
        return None;
    }
    let last = value.get("price").and_then(Value::as_str)?.to_string();
    let last_n: f64 = last.parse().ok()?;
    if last_n <= 0.0 {
        return None;
    }
    Some(QuoteTick {
        instrument_id,
        last,
        as_of: received_at,
        received_at,
        age_unknown: true,
        transport: Transport::Rest,
        adapter_id: BINANCE_COM_ADAPTER_ID.to_string(),
        book_id: BINANCE_COM_COINM_BOOK_ID.to_string(),
        session_ohlc: None,
    })
}

pub fn quote_tick_from_coinm_ticker_json(
    raw: &str,
    received_at: DateTime<Utc>,
) -> Option<QuoteTick> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let obj = if value.is_object() {
        &value
    } else {
        return None;
    };
    tick_from_ticker_object(obj, received_at)
}

pub fn quote_tick_from_coinm_ticker_json_for_symbol(
    raw: &str,
    symbol: &str,
    received_at: DateTime<Utc>,
) -> Option<QuoteTick> {
    let want = normalize_coinm_instrument(symbol);
    if !is_coinm_pair(&want) {
        return None;
    }
    let tick = quote_tick_from_coinm_ticker_json(raw, received_at)?;
    (normalize_coinm_instrument(&tick.instrument_id) == want).then_some(tick)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::egress::Lane;

    #[test]
    fn coinm_pair_builds_unsigned_dapi_ticker_get() {
        let call = binance_coinm_ticker_call("BTCUSD_PERP").expect("coinm id builds a call");
        assert_eq!(call.method, "GET");
        assert_eq!(call.host, "dapi.binance.com");
        assert_eq!(call.path, "/dapi/v1/ticker/price");
        assert_eq!(call.query, "symbol=BTCUSD_PERP");
        assert_eq!(call.lane, Lane::MarketData);
        assert_eq!(call.book_id, "binance-com-coinm");
        assert!(call.headers.is_empty());
        assert_ne!(call.path, "/fapi/v1/ticker/price");
        assert_ne!(call.host, "fapi.binance.com");
        assert_ne!(call.book_id, "binance-com-usdm");
    }

    #[test]
    fn leftover_usdm_letters_still_build_a_call_but_never_share_fapi() {
        // BTCUSDT letters are legal alphanumeric; matching is book_id.
        let call = binance_coinm_ticker_call("BTCUSDT").expect("letters are a pair shape");
        assert_eq!(call.book_id, "binance-com-coinm");
        assert_eq!(call.host, "dapi.binance.com");
    }

    #[test]
    fn dated_or_kotak_never_builds_a_call() {
        assert!(binance_coinm_ticker_call("BTC-200730-9000-C").is_none());
        assert!(binance_coinm_ticker_call("nse_cm|2885").is_none());
        assert!(binance_coinm_ticker_call("").is_none());
    }

    #[test]
    fn last_is_price_not_last_price() {
        let tick = quote_tick_from_coinm_ticker_json(
            r#"{"symbol":"BTCUSD_PERP","ps":"BTCUSD","price":"65000.10","time":1}"#,
            Utc::now(),
        )
        .expect("parses");
        assert_eq!(tick.instrument_id, "BTCUSD_PERP");
        assert_eq!(tick.last, "65000.10");
        assert_eq!(tick.book_id, "binance-com-coinm");
        assert!(quote_tick_from_coinm_ticker_json(
            r#"{"symbol":"BTCUSD_PERP","lastPrice":"1","time":1}"#,
            Utc::now(),
        )
        .is_none());
    }
}
