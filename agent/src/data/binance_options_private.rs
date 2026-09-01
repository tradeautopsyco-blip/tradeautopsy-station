//! Host-mediated Binance COM options private reads for obtain(tradebook).
//! Pipe A: async kick fetch → AccountBook slot → obtain enricher.

use crate::api::AppState;
use crate::binance_com_options_client::{user_trade_to_broker_fill, BinanceComOptionsClient};
use crate::data::{authorize_book_call, BINANCE_COM_OPTIONS_BOOK_ID};
use crate::ubi::CredentialBlob;
use chrono::Utc;

const OPTIONS_USER_TRADES_PATH: &str = "/eapi/v1/userTrades";
const OPTIONS_PRIVATE_MAX_AGE_MS: i64 = 5_000;
const OPTIONS_PRIVATE_ENV: &str = "prod";
const OPTIONS_PRIVATE_SLUG: &str = "binance_com";

fn slot_fresh(as_of_ms: i64, max_age_ms: i64) -> bool {
    let now = Utc::now().timestamp_millis();
    now.saturating_sub(as_of_ms) < max_age_ms
}

fn parse_connection_id_from_handle(handle: &str) -> Option<&str> {
    let rest = handle.strip_prefix("keychain:")?;
    let (_service, connection_id) = rest.rsplit_once(':')?;
    if connection_id.is_empty() {
        None
    } else {
        Some(connection_id)
    }
}

fn options_credential_handle(state: &AppState) -> Option<String> {
    let active = {
        let st = state
            .broker_status
            .lock()
            .expect("broker_status mutex poisoned");
        let slug = st.active_broker_slug.as_deref()?;
        if slug != OPTIONS_PRIVATE_SLUG {
            return None;
        }
        slug.to_string()
    };
    let map = state
        .broker_connections
        .lock()
        .expect("broker connections mutex poisoned");
    map.get(BINANCE_COM_OPTIONS_BOOK_ID)
        .or_else(|| map.get(&active))
        .and_then(|runtime| runtime.credential_handle.clone())
}

fn resolve_options_client(state: &AppState) -> Option<BinanceComOptionsClient> {
    let handle = options_credential_handle(state)?;
    let connection_id = parse_connection_id_from_handle(&handle)?;
    let blob = state
        .broker_sync_control
        .credential_vault()
        .load(OPTIONS_PRIVATE_ENV, OPTIONS_PRIVATE_SLUG, connection_id)
        .ok()??;
    let CredentialBlob::HmacApiKeySecret {
        api_key,
        api_secret,
    } = blob
    else {
        return None;
    };
    Some(BinanceComOptionsClient::new(api_key, api_secret))
}

pub async fn ensure_options_user_trades(state: &AppState, symbol: &str) {
    if symbol.trim().is_empty() {
        return;
    }
    {
        let book = state
            .account_book
            .lock()
            .expect("account_book mutex poisoned");
        if let Some(slot) = book.fills_slot(BINANCE_COM_OPTIONS_BOOK_ID) {
            if slot_fresh(slot.as_of_ms, OPTIONS_PRIVATE_MAX_AGE_MS) {
                return;
            }
        }
    }
    if authorize_book_call(
        BINANCE_COM_OPTIONS_BOOK_ID,
        "eapi.binance.com",
        "GET",
        OPTIONS_USER_TRADES_PATH,
        true,
    )
    .is_err()
    {
        return;
    }
    let Some(client) = resolve_options_client(state) else {
        return;
    };
    let Ok(rows) = client.fetch_user_trades(symbol).await else {
        return;
    };
    let fills: Vec<_> = rows.iter().map(user_trade_to_broker_fill).collect();
    let as_of_ms = Utc::now().timestamp_millis();
    state
        .account_book
        .lock()
        .expect("account_book mutex poisoned")
        .replace_fills(
            BINANCE_COM_OPTIONS_BOOK_ID,
            fills,
            OPTIONS_USER_TRADES_PATH,
            as_of_ms,
        );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_connection_id_from_keychain_handle() {
        assert_eq!(
            parse_connection_id_from_handle(
                "keychain:in.tradeautopsy.station.broker-credentials:00000000-0000-4000-8000-000000000001"
            ),
            Some("00000000-0000-4000-8000-000000000001")
        );
        assert!(parse_connection_id_from_handle("keychain:binance").is_none());
    }
}
