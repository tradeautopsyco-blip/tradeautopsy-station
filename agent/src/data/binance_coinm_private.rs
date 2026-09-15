//! Host-mediated Binance Coin-M USER_DATA for obtain(funds|positionbook|forceorder).

use crate::api::AppState;
use crate::binance_com_coinm_client::BinanceComCoinmClient;
use crate::broker_data_class::BrokerPositionsSnapshot;
use crate::data::{authorize_book_call, observation_from_rest, BINANCE_COM_COINM_BOOK_ID};
use crate::ubi::CredentialBlob;
use chrono::Utc;

const COINM_BALANCE_PATH: &str = "/dapi/v1/balance";
const COINM_POSITION_PATH: &str = "/dapi/v1/positionRisk";
const COINM_FORCE_ORDERS_PATH: &str = "/dapi/v1/forceOrders";
const COINM_PRIVATE_MAX_AGE_MS: i64 = 5_000;
const COINM_PRIVATE_ENV: &str = "prod";
const COINM_PRIVATE_SLUG: &str = "binance_com";

fn slot_fresh(as_of_ms: i64, max_age_ms: i64) -> bool {
    Utc::now().timestamp_millis().saturating_sub(as_of_ms) < max_age_ms
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

fn coinm_credential_handle(state: &AppState) -> Option<String> {
    let active = {
        let st = state
            .broker_status
            .lock()
            .expect("broker_status mutex poisoned");
        let slug = st.active_broker_slug.as_deref()?;
        if slug != COINM_PRIVATE_SLUG {
            return None;
        }
        slug.to_string()
    };
    let map = state
        .broker_connections
        .lock()
        .expect("broker connections mutex poisoned");
    map.get(BINANCE_COM_COINM_BOOK_ID)
        .or_else(|| map.get(&active))
        .and_then(|runtime| runtime.credential_handle.clone())
}

fn resolve_coinm_client(state: &AppState) -> Option<BinanceComCoinmClient> {
    let handle = coinm_credential_handle(state)?;
    let connection_id = parse_connection_id_from_handle(&handle)?;
    let blob = state
        .broker_sync_control
        .credential_vault()
        .load(COINM_PRIVATE_ENV, COINM_PRIVATE_SLUG, connection_id)
        .ok()??;
    let CredentialBlob::HmacApiKeySecret {
        api_key,
        api_secret,
    } = blob
    else {
        return None;
    };
    if let Some(base) = state
        .binance_coinm_base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Some(BinanceComCoinmClient::with_base_url(
            base, api_key, api_secret,
        ));
    }
    Some(BinanceComCoinmClient::new(api_key, api_secret))
}

pub async fn ensure_coinm_balance(state: &AppState) {
    {
        let book = state
            .account_book
            .lock()
            .expect("account_book mutex poisoned");
        if book
            .funds_slot(BINANCE_COM_COINM_BOOK_ID)
            .is_some_and(|slot| slot_fresh(slot.as_of_ms, COINM_PRIVATE_MAX_AGE_MS))
        {
            return;
        }
    }
    if authorize_book_call(
        BINANCE_COM_COINM_BOOK_ID,
        "dapi.binance.com",
        "GET",
        COINM_BALANCE_PATH,
        true,
    )
    .is_err()
    {
        return;
    }
    let Some(client) = resolve_coinm_client(state) else {
        return;
    };
    let Ok(snapshot) = client.fetch_balance().await else {
        return;
    };
    let as_of_ms = Utc::now().timestamp_millis();
    state
        .account_book
        .lock()
        .expect("account_book mutex poisoned")
        .replace_funds(
            BINANCE_COM_COINM_BOOK_ID,
            snapshot,
            COINM_BALANCE_PATH,
            as_of_ms,
        );
}

pub async fn ensure_coinm_positions(state: &AppState) {
    {
        let book = state
            .account_book
            .lock()
            .expect("account_book mutex poisoned");
        if book
            .positions_slot(BINANCE_COM_COINM_BOOK_ID)
            .is_some_and(|slot| slot_fresh(slot.as_of_ms, COINM_PRIVATE_MAX_AGE_MS))
        {
            return;
        }
    }
    if authorize_book_call(
        BINANCE_COM_COINM_BOOK_ID,
        "dapi.binance.com",
        "GET",
        COINM_POSITION_PATH,
        true,
    )
    .is_err()
    {
        return;
    }
    let Some(client) = resolve_coinm_client(state) else {
        return;
    };
    let Ok(positions) = client.fetch_positions().await else {
        return;
    };
    let as_of_ms = Utc::now().timestamp_millis();
    state
        .account_book
        .lock()
        .expect("account_book mutex poisoned")
        .replace_positions(
            BINANCE_COM_COINM_BOOK_ID,
            BrokerPositionsSnapshot { positions },
            COINM_POSITION_PATH,
            as_of_ms,
        );
}

pub async fn ensure_coinm_force_orders(state: &AppState) {
    if authorize_book_call(
        BINANCE_COM_COINM_BOOK_ID,
        "dapi.binance.com",
        "GET",
        COINM_FORCE_ORDERS_PATH,
        true,
    )
    .is_err()
    {
        return;
    }
    let Some(client) = resolve_coinm_client(state) else {
        return;
    };
    let Ok(events) = client.fetch_force_orders().await else {
        return;
    };
    let envelope = observation_from_rest("binance_com", events);
    state
        .force_order_book
        .lock()
        .expect("force_order_book mutex poisoned")
        .replace(BINANCE_COM_COINM_BOOK_ID, envelope);
}
