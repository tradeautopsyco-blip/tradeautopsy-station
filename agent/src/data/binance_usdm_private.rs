//! Host-mediated Binance USDM USER_DATA for obtain(funds|positionbook|forceorder)
//! plus `ensure_usdm_realized_income` (not an obtain operation this slice).

use crate::api::AppState;
use crate::binance_com_usdm_client::BinanceComUsdmClient;
use crate::broker_data_class::BrokerPositionsSnapshot;
use crate::data::{authorize_book_call, observation_from_rest, BINANCE_COM_USDM_BOOK_ID};
use crate::ubi::CredentialBlob;
use crate::usdm_realized_pnl::UsdmRealizedSlot;
use chrono::Utc;

pub use crate::usdm_realized_pnl::usdm_income_call;

const USDM_BALANCE_PATH: &str = "/fapi/v3/balance";
const USDM_POSITION_PATH: &str = "/fapi/v3/positionRisk";
const USDM_FORCE_ORDERS_PATH: &str = "/fapi/v1/forceOrders";
const USDM_PRIVATE_MAX_AGE_MS: i64 = 5_000;
const USDM_PRIVATE_ENV: &str = "prod";
const USDM_PRIVATE_SLUG: &str = "binance_com";

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

fn usdm_credential_handle(state: &AppState) -> Option<String> {
    let active = {
        let st = state
            .broker_status
            .lock()
            .expect("broker_status mutex poisoned");
        let slug = st.active_broker_slug.as_deref()?;
        if slug != USDM_PRIVATE_SLUG {
            return None;
        }
        slug.to_string()
    };
    let map = state
        .broker_connections
        .lock()
        .expect("broker connections mutex poisoned");
    map.get(BINANCE_COM_USDM_BOOK_ID)
        .or_else(|| map.get(&active))
        .and_then(|runtime| runtime.credential_handle.clone())
}

fn resolve_usdm_client(state: &AppState) -> Option<BinanceComUsdmClient> {
    let handle = usdm_credential_handle(state)?;
    let connection_id = parse_connection_id_from_handle(&handle)?;
    let blob = state
        .broker_sync_control
        .credential_vault()
        .load(USDM_PRIVATE_ENV, USDM_PRIVATE_SLUG, connection_id)
        .ok()??;
    let CredentialBlob::HmacApiKeySecret {
        api_key,
        api_secret,
    } = blob
    else {
        return None;
    };
    if let Some(base) = state
        .binance_usdm_base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Some(BinanceComUsdmClient::with_base_url(
            base, api_key, api_secret,
        ));
    }
    Some(BinanceComUsdmClient::new(api_key, api_secret))
}

pub async fn ensure_usdm_balance(state: &AppState) {
    {
        let book = state
            .account_book
            .lock()
            .expect("account_book mutex poisoned");
        if book
            .funds_slot(BINANCE_COM_USDM_BOOK_ID)
            .is_some_and(|slot| slot_fresh(slot.as_of_ms, USDM_PRIVATE_MAX_AGE_MS))
        {
            return;
        }
    }
    if authorize_book_call(
        BINANCE_COM_USDM_BOOK_ID,
        "fapi.binance.com",
        "GET",
        USDM_BALANCE_PATH,
        true,
    )
    .is_err()
    {
        return;
    }
    let Some(client) = resolve_usdm_client(state) else {
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
            BINANCE_COM_USDM_BOOK_ID,
            snapshot,
            USDM_BALANCE_PATH,
            as_of_ms,
        );
}

pub async fn ensure_usdm_positions(state: &AppState) {
    {
        let book = state
            .account_book
            .lock()
            .expect("account_book mutex poisoned");
        if book
            .positions_slot(BINANCE_COM_USDM_BOOK_ID)
            .is_some_and(|slot| slot_fresh(slot.as_of_ms, USDM_PRIVATE_MAX_AGE_MS))
        {
            return;
        }
    }
    if authorize_book_call(
        BINANCE_COM_USDM_BOOK_ID,
        "fapi.binance.com",
        "GET",
        USDM_POSITION_PATH,
        true,
    )
    .is_err()
    {
        return;
    }
    let Some(client) = resolve_usdm_client(state) else {
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
            BINANCE_COM_USDM_BOOK_ID,
            BrokerPositionsSnapshot { positions },
            USDM_POSITION_PATH,
            as_of_ms,
        );
}

pub async fn ensure_usdm_force_orders(state: &AppState) {
    if authorize_book_call(
        BINANCE_COM_USDM_BOOK_ID,
        "fapi.binance.com",
        "GET",
        USDM_FORCE_ORDERS_PATH,
        true,
    )
    .is_err()
    {
        return;
    }
    let Some(client) = resolve_usdm_client(state) else {
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
        .replace(BINANCE_COM_USDM_BOOK_ID, envelope);
}

/// Fetch venue `REALIZED_PNL` income into `AppState.usdm_realized`.
/// Not wired to obtain. Force-order is not this sum. Parent may kick later.
pub async fn ensure_usdm_realized_income(state: &AppState) {
    {
        let slot = state
            .usdm_realized
            .lock()
            .expect("usdm_realized mutex poisoned");
        if slot
            .as_ref()
            .is_some_and(|s| slot_fresh(s.as_of_ms, USDM_PRIVATE_MAX_AGE_MS))
        {
            return;
        }
    }
    let call = usdm_income_call();
    if authorize_book_call(call.book_id, call.host, call.method, call.path, true).is_err() {
        return;
    }
    let Some(client) = resolve_usdm_client(state) else {
        return;
    };
    let Ok(body) = client.fetch_income().await else {
        return;
    };
    let as_of_ms = Utc::now().timestamp_millis();
    let slot = UsdmRealizedSlot::from_income_json(&body, as_of_ms);
    *state
        .usdm_realized
        .lock()
        .expect("usdm_realized mutex poisoned") = Some(slot.clone());
    crate::m1_income_share::share_usdm_realized_income_today(&state.fact_outbox, &slot);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn income_call_is_not_force_order_or_trade() {
        let call = usdm_income_call();
        assert_eq!(call.book_id, BINANCE_COM_USDM_BOOK_ID);
        assert_eq!(call.host, "fapi.binance.com");
        assert_eq!(call.method, "GET");
        assert_eq!(call.path, "/fapi/v1/income");
        assert_eq!(call.query_income_type, "REALIZED_PNL");
        assert_ne!(call.path, USDM_FORCE_ORDERS_PATH);
        assert_ne!(call.path, "/fapi/v1/order");
    }
}
