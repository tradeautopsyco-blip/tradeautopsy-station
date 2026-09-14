//! Host-mediated Binance COM spot private reads for obtain(funds|orderbook).
//! Pipe A: async kick fetch → AccountBook slot → obtain enricher.

use crate::api::AppState;
use crate::binance_com_spot_client::{open_order_to_broker_open_order, BinanceComSpotClient};
use crate::broker_data_class::{BrokerBalancesSnapshot, BrokerHolding, BrokerOpenOrdersSnapshot};
use crate::data::{authorize_book_call, BINANCE_COM_SPOT_BOOK_ID};
use crate::ubi::CredentialBlob;
use chrono::Utc;

const SPOT_ACCOUNT_PATH: &str = "/api/v3/account";
const SPOT_OPEN_ORDERS_PATH: &str = "/api/v3/openOrders";
const SPOT_PRIVATE_MAX_AGE_MS: i64 = 5_000;
const SPOT_PRIVATE_ENV: &str = "prod";
const SPOT_PRIVATE_SLUG: &str = "binance_com";

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

fn spot_credential_handle(state: &AppState) -> Option<String> {
    let active = {
        let st = state
            .broker_status
            .lock()
            .expect("broker_status mutex poisoned");
        let slug = st.active_broker_slug.as_deref()?;
        if slug != SPOT_PRIVATE_SLUG {
            return None;
        }
        slug.to_string()
    };
    let map = state
        .broker_connections
        .lock()
        .expect("broker connections mutex poisoned");
    map.get(BINANCE_COM_SPOT_BOOK_ID)
        .or_else(|| map.get(&active))
        .and_then(|runtime| runtime.credential_handle.clone())
}

fn resolve_spot_client(state: &AppState) -> Option<BinanceComSpotClient> {
    let handle = spot_credential_handle(state)?;
    let connection_id = parse_connection_id_from_handle(&handle)?;
    let blob = state
        .broker_sync_control
        .credential_vault()
        .load(SPOT_PRIVATE_ENV, SPOT_PRIVATE_SLUG, connection_id)
        .ok()??;
    let CredentialBlob::HmacApiKeySecret {
        api_key,
        api_secret,
    } = blob
    else {
        return None;
    };
    if let Some(base) = state
        .binance_spot_base_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Some(BinanceComSpotClient::with_base_url(
            base, api_key, api_secret,
        ));
    }
    Some(BinanceComSpotClient::new(api_key, api_secret))
}

fn balances_from_account(
    account: crate::binance_com_spot_client::BinanceComAccountInfo,
) -> BrokerBalancesSnapshot {
    let holdings = account
        .balances
        .into_iter()
        .filter_map(|b| {
            let free: f64 = b.free.parse().ok()?;
            let locked: f64 = b.locked.parse().ok()?;
            if free + locked <= 0.0 {
                return None;
            }
            Some(BrokerHolding {
                asset: b.asset,
                free,
                locked,
            })
        })
        .collect();
    BrokerBalancesSnapshot {
        holdings,
        unrealized_pnl: None,
    }
}

pub async fn ensure_spot_account(state: &AppState) {
    {
        let book = state
            .account_book
            .lock()
            .expect("account_book mutex poisoned");
        if let Some(slot) = book.funds_slot(BINANCE_COM_SPOT_BOOK_ID) {
            if slot_fresh(slot.as_of_ms, SPOT_PRIVATE_MAX_AGE_MS) {
                return;
            }
        }
    }
    // Egress re-runs this fence before it charges; the early check refuses
    // fetches when the book/host/path tuple is not allowlisted.
    if authorize_book_call(
        BINANCE_COM_SPOT_BOOK_ID,
        "api.binance.com",
        "GET",
        SPOT_ACCOUNT_PATH,
        true,
    )
    .is_err()
    {
        return;
    }
    let Some(client) = resolve_spot_client(state) else {
        return;
    };
    let Ok(account) = client.fetch_account_info().await else {
        return;
    };
    let snapshot = balances_from_account(account);
    let as_of_ms = Utc::now().timestamp_millis();
    state
        .account_book
        .lock()
        .expect("account_book mutex poisoned")
        .replace_funds(
            BINANCE_COM_SPOT_BOOK_ID,
            snapshot,
            SPOT_ACCOUNT_PATH,
            as_of_ms,
        );
}

pub async fn ensure_spot_open_orders(state: &AppState) {
    {
        let book = state
            .account_book
            .lock()
            .expect("account_book mutex poisoned");
        if let Some(slot) = book.orders_slot(BINANCE_COM_SPOT_BOOK_ID) {
            if slot_fresh(slot.as_of_ms, SPOT_PRIVATE_MAX_AGE_MS) {
                return;
            }
        }
    }
    if authorize_book_call(
        BINANCE_COM_SPOT_BOOK_ID,
        "api.binance.com",
        "GET",
        SPOT_OPEN_ORDERS_PATH,
        true,
    )
    .is_err()
    {
        return;
    }
    let Some(client) = resolve_spot_client(state) else {
        return;
    };
    let Ok(rows) = client.fetch_open_orders(None).await else {
        return;
    };
    let orders: Vec<_> = rows.iter().map(open_order_to_broker_open_order).collect();
    let snapshot = BrokerOpenOrdersSnapshot { orders };
    let as_of_ms = Utc::now().timestamp_millis();
    state
        .account_book
        .lock()
        .expect("account_book mutex poisoned")
        .replace_orders(
            BINANCE_COM_SPOT_BOOK_ID,
            snapshot,
            SPOT_OPEN_ORDERS_PATH,
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
