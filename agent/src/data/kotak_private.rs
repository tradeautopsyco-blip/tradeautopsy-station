//! Host-mediated Kotak Neo private account reads (orders, holdings, positions,
//! limits, check-margin). Pipe A: async kick → AccountBook → obtain enricher.

use crate::egress::Lane;
use crate::kotak_scrip_master::KOTAK_NEO;
use crate::ubi::{
    kotak_base_host, prepare_request, BrokerCredentialVault, HostCredentialBlob,
    PreparedHttpRequest,
};
use serde_json::Value;
use std::time::Duration;

use super::descriptor::{
    KOTAK_MCX_FUTURE_BOOK_ID, KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_CDS_BOOK_ID,
    KOTAK_NSE_NFO_BOOK_ID,
};

pub const KOTAK_ORDERS_PATH: &str = "/quick/user/orders";
pub const KOTAK_POSITIONS_PATH: &str = "/quick/user/positions";
pub const KOTAK_HOLDINGS_PATH: &str = "/portfolio/v1/holdings";
pub const KOTAK_LIMITS_PATH: &str = "/quick/user/limits";

/// Kotak Neo SDK `rest.py`: `application/x-www-form-urlencoded` POSTs wrap JSON in
/// form field `jData` (see `limits_api.py` / `margin_api.py`). Raw `seg=…&exch=…`
/// bodies 500 on live gateways.
fn url_encode_form_component(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

pub fn kotak_jdata_form_body(fields: &[(&str, &str)]) -> String {
    let mut map = serde_json::Map::new();
    for (key, value) in fields {
        map.insert((*key).to_string(), Value::String((*value).to_string()));
    }
    let json = serde_json::to_string(&Value::Object(map)).expect("jData json");
    format!("jData={}", url_encode_form_component(&json))
}

/// Limits.md default segment/exchange/product for the cash book (re-fetched 2026-09-15 IST).
pub fn kotak_cash_limits_jdata_body() -> String {
    kotak_jdata_form_body(&[("seg", "ALL"), ("exch", "ALL"), ("prod", "ALL")])
}

/// Limits.md `segment` enum **FO** for the NFO book (re-fetched 2026-09-15 IST).
pub fn kotak_nfo_limits_jdata_body() -> String {
    kotak_jdata_form_body(&[("seg", "FO"), ("exch", "ALL"), ("prod", "ALL")])
}

#[cfg(test)]
const KOTAK_CHECK_MARGIN_PATH: &str = "/quick/user/check-margin";

#[cfg(test)]
fn kotak_private_book_id(segment: &str) -> Option<&'static str> {
    let seg = segment.trim().to_ascii_lowercase();
    if seg == "nse_fo" {
        Some(KOTAK_NSE_NFO_BOOK_ID)
    } else if seg == "cde_fo" {
        Some(KOTAK_NSE_CDS_BOOK_ID)
    } else if seg == "mcx_fo" {
        Some(KOTAK_MCX_FUTURE_BOOK_ID)
    } else if matches!(seg.as_str(), "nse_cm" | "bse_cm") {
        Some(KOTAK_NSE_BSE_CASH_BOOK_ID)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KotakPrivateFetchError {
    Session,
    Http(u16),
}

pub async fn fetch_kotak_private_json(
    vault: &dyn BrokerCredentialVault,
    environment: &str,
    connection_id: &str,
    book_id: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
    direct_base: Option<&str>,
) -> Result<String, KotakPrivateFetchError> {
    let blob = vault
        .load(environment, KOTAK_NEO, connection_id)
        .map_err(|_| KotakPrivateFetchError::Session)?
        .ok_or(KotakPrivateFetchError::Session)?;
    let creds = HostCredentialBlob::from(&blob);
    let base_url = match &creds {
        HostCredentialBlob::KotakSession { base_url, .. } => base_url.clone(),
        HostCredentialBlob::Hmac { .. }
        | HostCredentialBlob::KiteSession { .. }
        | HostCredentialBlob::UpstoxSession { .. }
        | HostCredentialBlob::FyersSession { .. }
        | HostCredentialBlob::GrowwSession { .. }
        | HostCredentialBlob::DhanSession { .. }
        | HostCredentialBlob::OkxSession { .. }
        | HostCredentialBlob::CoinbaseJwt { .. } => {
            return Err(KotakPrivateFetchError::Session)
        }
    };
    let host = kotak_base_host(&base_url).ok_or(KotakPrivateFetchError::Http(0))?;
    let mut headers = vec![];
    if body.is_some() {
        headers.push((
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        ));
    }
    let prepared = prepare_request(method, &host, path, &[], &headers, body, &creds, 0);
    send_private(book_id, &prepared, direct_base).await
}

fn rewrite_prepared_url(prepared_url: &str, direct_base: &str) -> String {
    let base = direct_base.trim_end_matches('/');
    let path_and_query = prepared_url
        .find("://")
        .and_then(|scheme| {
            prepared_url[scheme + 3..]
                .find('/')
                .map(|slash| &prepared_url[scheme + 3 + slash..])
        })
        .unwrap_or("/");
    format!("{base}{path_and_query}")
}

async fn send_private(
    book_id: &str,
    prepared: &PreparedHttpRequest,
    direct_base: Option<&str>,
) -> Result<String, KotakPrivateFetchError> {
    if let Some(base) = direct_base.map(str::trim).filter(|s| !s.is_empty()) {
        let url = rewrite_prepared_url(&prepared.url, base);
        let client = crate::egress::shared_client();
        let mut req = match prepared.method.to_ascii_uppercase().as_str() {
            "POST" => client.post(&url),
            _ => client.get(&url),
        };
        for (name, value) in &prepared.headers {
            req = req.header(name, value);
        }
        if let Some(body) = &prepared.body {
            req = req.body(body.clone());
        }
        let response = req
            .send()
            .await
            .map_err(|_| KotakPrivateFetchError::Http(0))?;
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|_| KotakPrivateFetchError::Http(0))?;
        if matches!(status, 401 | 403) {
            return Err(KotakPrivateFetchError::Session);
        }
        if !(200..300).contains(&status) {
            return Err(KotakPrivateFetchError::Http(status));
        }
        return Ok(body);
    }
    let resp = crate::egress::shared()
        .send_prepared(
            book_id,
            Lane::PrivateRead,
            prepared,
            Duration::from_secs(20),
        )
        .await
        .map_err(|_| KotakPrivateFetchError::Http(0))?;
    if matches!(resp.status, 401 | 403) {
        return Err(KotakPrivateFetchError::Session);
    }
    if !resp.is_success() {
        return Err(KotakPrivateFetchError::Http(resp.status));
    }
    Ok(resp.body)
}

const KOTAK_PRIVATE_MAX_AGE_MS: i64 = 5_000;

use crate::api::AppState;
use crate::broker_data_class::{
    BrokerBalancesSnapshot, BrokerHolding, BrokerHoldingsSnapshot, BrokerOpenOrder,
    BrokerOpenOrdersSnapshot, BrokerPortfolioHolding, BrokerPositionRow, BrokerPositionsSnapshot,
};
use crate::kotak_nfo_scrip::KotakNfoScripMaster;
use chrono::Utc;
use std::collections::HashMap;

use super::account_split::{
    split_kotak_holdings_by_book, split_kotak_orders_by_book, split_kotak_positions_by_book,
};
use super::authorize_book_call;

fn slot_fresh(as_of_ms: i64, max_age_ms: i64) -> bool {
    let now = Utc::now().timestamp_millis();
    now.saturating_sub(as_of_ms) < max_age_ms
}

fn string_field(row: &Value, key: &str) -> Option<String> {
    row.get(key).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

fn parse_f64(value: Option<&Value>) -> Option<f64> {
    value.and_then(|v| match v {
        Value::String(s) => s.trim().parse().ok(),
        Value::Number(n) => n.as_f64(),
        _ => None,
    })
}

fn parse_i64(value: Option<&Value>) -> Option<i64> {
    value.and_then(|v| match v {
        Value::String(s) => s.trim().parse().ok(),
        Value::Number(n) => n.as_i64(),
        _ => None,
    })
}

fn kotak_side(trns_tp: Option<&str>) -> Option<String> {
    match trns_tp.map(|s| s.trim()) {
        Some("B") | Some("b") => Some("BUY".into()),
        Some("S") | Some("s") => Some("SELL".into()),
        _ => None,
    }
}

fn strip_equity_suffix(symbol: &str) -> String {
    let trimmed = symbol.trim();
    for suffix in ["-EQ", "-eq", "-BE", "-be"] {
        if let Some(base) = trimmed.strip_suffix(suffix) {
            return base.to_string();
        }
    }
    trimmed.to_string()
}

fn kotak_open_order_from_row(row: &Value) -> Option<BrokerOpenOrder> {
    let unfilled = parse_i64(row.get("unFldSz")).unwrap_or(0);
    if unfilled <= 0 {
        return None;
    }
    let segment = string_field(row, "exSeg")?;
    let product = string_field(row, "prod")?;
    let side = kotak_side(string_field(row, "trnsTp").as_deref())?;
    let nfo = segment.eq_ignore_ascii_case("nse_fo");
    let raw_symbol = string_field(row, "trdSym").or_else(|| string_field(row, "sym"))?;
    let symbol = if nfo {
        raw_symbol
    } else {
        strip_equity_suffix(&raw_symbol)
    };
    Some(BrokerOpenOrder {
        order_id: string_field(row, "nOrdNo").unwrap_or_default(),
        symbol,
        side,
        qty: parse_f64(row.get("qty")).unwrap_or(0.0),
        price: parse_f64(row.get("prc")),
        product: Some(product),
        exchange_segment: Some(segment),
        status: string_field(row, "ordSt").or_else(|| string_field(row, "stat")),
        unfilled_qty: Some(unfilled as f64),
    })
}

pub fn parse_kotak_orders_json(body: &str) -> Result<Vec<BrokerOpenOrder>, String> {
    let root: Value = serde_json::from_str(body).map_err(|e| format!("orders json: {e}"))?;
    if root.get("stCode").and_then(|v| v.as_i64()) == Some(1003) {
        return Err("kotak_neo session_expired".into());
    }
    if let Some(stat) = string_field(&root, "stat") {
        if !stat.eq_ignore_ascii_case("ok") {
            return Err(format!("kotak_neo orders_not_ok ({stat})"));
        }
    }
    let rows = match root.get("data") {
        None | Some(Value::Null) => &[][..],
        Some(Value::Array(rows)) => rows.as_slice(),
        _ => return Err("orders missing data[]".into()),
    };
    Ok(rows.iter().filter_map(kotak_open_order_from_row).collect())
}

fn kotak_position_from_row(row: &Value) -> Option<BrokerPositionRow> {
    let segment = string_field(row, "exSeg")?;
    let product = string_field(row, "prod")?;
    let fl_buy = parse_f64(row.get("flBuyQty")).unwrap_or(0.0);
    let fl_sell = parse_f64(row.get("flSellQty")).unwrap_or(0.0);
    let cf_buy = parse_f64(row.get("cfBuyQty")).unwrap_or(0.0);
    let cf_sell = parse_f64(row.get("cfSellQty")).unwrap_or(0.0);
    let net_qty = (fl_buy + cf_buy) - (fl_sell + cf_sell);
    if net_qty.abs() < f64::EPSILON {
        return None;
    }
    let nfo = segment.eq_ignore_ascii_case("nse_fo");
    let raw_symbol = string_field(row, "trdSym").or_else(|| string_field(row, "sym"))?;
    let symbol = if nfo {
        raw_symbol.clone()
    } else {
        strip_equity_suffix(&raw_symbol)
    };
    Some(BrokerPositionRow {
        symbol,
        exchange_segment: segment,
        product,
        net_qty,
        trading_symbol: raw_symbol,
    })
}

/// Official empty snapshot: HTTP 200 + `stCode` 5203 / `errMsg` "No Data" (trade book parity).
fn is_kotak_no_data_response(root: &Value) -> bool {
    if root.get("stCode").and_then(|v| v.as_i64()) == Some(5203) {
        return true;
    }
    string_field(root, "errMsg").is_some_and(|m| m.eq_ignore_ascii_case("No Data"))
}

pub fn parse_kotak_positions_json(body: &str) -> Result<Vec<BrokerPositionRow>, String> {
    let root: Value = serde_json::from_str(body).map_err(|e| format!("positions json: {e}"))?;
    if root.get("stCode").and_then(|v| v.as_i64()) == Some(1003) {
        return Err("kotak_neo session_expired".into());
    }
    if is_kotak_no_data_response(&root) {
        return Ok(Vec::new());
    }
    if let Some(stat) = string_field(&root, "stat") {
        if !stat.eq_ignore_ascii_case("ok") {
            return Err(format!("kotak_neo positions_not_ok ({stat})"));
        }
    }
    let rows = match root.get("data") {
        None | Some(Value::Null) => &[][..],
        Some(Value::Array(rows)) => rows.as_slice(),
        _ => return Err("positions missing data[]".into()),
    };
    Ok(rows.iter().filter_map(kotak_position_from_row).collect())
}

fn kotak_holding_from_row(row: &Value) -> Option<BrokerPortfolioHolding> {
    let segment = string_field(row, "exchangeSegment")?;
    if !matches!(segment.as_str(), "nse_cm" | "bse_cm") {
        return None;
    }
    let quantity = parse_f64(row.get("quantity"))?;
    if quantity <= 0.0 {
        return None;
    }
    Some(BrokerPortfolioHolding {
        symbol: string_field(row, "symbol").or_else(|| string_field(row, "displaySymbol"))?,
        exchange_segment: segment,
        quantity,
        sellable_quantity: parse_f64(row.get("sellableQuantity")).unwrap_or(quantity),
        average_price: parse_f64(row.get("averagePrice")).unwrap_or(0.0),
        market_value: parse_f64(row.get("mktValue")).unwrap_or(0.0),
        instrument_type: string_field(row, "instrumentType").unwrap_or_default(),
    })
}

pub fn parse_kotak_holdings_json(body: &str) -> Result<Vec<BrokerPortfolioHolding>, String> {
    let root: Value = serde_json::from_str(body).map_err(|e| format!("holdings json: {e}"))?;
    let rows = match root.get("data") {
        None | Some(Value::Null) => &[][..],
        Some(Value::Array(rows)) => rows.as_slice(),
        _ => return Err("holdings missing data[]".into()),
    };
    Ok(rows.iter().filter_map(kotak_holding_from_row).collect())
}

/// Maps Limits.md `Net`/`MarginUsed` onto AccountBook INR funds.
///
/// See `docs/reference/india/kotak-neo/FUNDS-LIMITS.md`. Copy those keys;
/// do not derive free from `CollateralValue - MarginUsed`. Do not parse
/// SPAN into `margin_estimate`.
pub fn parse_kotak_limits_json(body: &str) -> Result<BrokerBalancesSnapshot, String> {
    let root: Value = serde_json::from_str(body).map_err(|e| format!("limits json: {e}"))?;
    if root.get("stCode").and_then(|v| v.as_i64()) == Some(1003) {
        return Err("kotak_neo session_expired".into());
    }
    if let Some(stat) = string_field(&root, "stat") {
        if !stat.eq_ignore_ascii_case("ok") {
            return Err(format!("kotak_neo limits_not_ok ({stat})"));
        }
    }
    let free = parse_f64(root.get("Net")).ok_or_else(|| "kotak limits missing Net".to_string())?;
    let locked = parse_f64(root.get("MarginUsed"))
        .ok_or_else(|| "kotak limits missing MarginUsed".to_string())?;
    let holdings = if free + locked <= 0.0 {
        Vec::new()
    } else {
        vec![BrokerHolding {
            asset: "INR".to_string(),
            free,
            locked,
        }]
    };
    Ok(BrokerBalancesSnapshot {
        holdings,
        unrealized_pnl: None,
    })
}

fn nfo_master(state: &AppState) -> KotakNfoScripMaster {
    state
        .kotak_nfo_scrip_master
        .lock()
        .expect("kotak nfo scrip master mutex poisoned")
        .clone()
}

async fn fetch_and_plant_orders(
    state: &AppState,
    environment: &str,
    connection_id: &str,
) -> Result<(), KotakPrivateFetchError> {
    let body = fetch_kotak_private_json(
        state.broker_sync_control.credential_vault().as_ref(),
        environment,
        connection_id,
        KOTAK_NSE_BSE_CASH_BOOK_ID,
        "GET",
        KOTAK_ORDERS_PATH,
        None,
        state.kotak_private_base_url.as_deref(),
    )
    .await?;
    let orders = parse_kotak_orders_json(&body).map_err(|_| KotakPrivateFetchError::Http(0))?;
    let master = nfo_master(state);
    let split = split_kotak_orders_by_book(orders, &master);
    plant_orders_split(state, split, KOTAK_ORDERS_PATH);
    Ok(())
}

async fn fetch_and_plant_positions(
    state: &AppState,
    environment: &str,
    connection_id: &str,
) -> Result<(), KotakPrivateFetchError> {
    let body = fetch_kotak_private_json(
        state.broker_sync_control.credential_vault().as_ref(),
        environment,
        connection_id,
        KOTAK_NSE_BSE_CASH_BOOK_ID,
        "GET",
        KOTAK_POSITIONS_PATH,
        None,
        state.kotak_private_base_url.as_deref(),
    )
    .await?;
    let positions =
        parse_kotak_positions_json(&body).map_err(|_| KotakPrivateFetchError::Http(0))?;
    let master = nfo_master(state);
    let split = split_kotak_positions_by_book(positions, &master);
    plant_positions_split(state, split, KOTAK_POSITIONS_PATH);
    Ok(())
}

async fn fetch_and_plant_holdings(
    state: &AppState,
    environment: &str,
    connection_id: &str,
) -> Result<(), KotakPrivateFetchError> {
    let body = fetch_kotak_private_json(
        state.broker_sync_control.credential_vault().as_ref(),
        environment,
        connection_id,
        KOTAK_NSE_BSE_CASH_BOOK_ID,
        "GET",
        KOTAK_HOLDINGS_PATH,
        None,
        state.kotak_private_base_url.as_deref(),
    )
    .await?;
    let holdings = parse_kotak_holdings_json(&body).map_err(|_| KotakPrivateFetchError::Http(0))?;
    let split = split_kotak_holdings_by_book(holdings);
    let as_of_ms = Utc::now().timestamp_millis();
    if let Some(rows) = split.get(KOTAK_NSE_BSE_CASH_BOOK_ID) {
        state
            .account_book
            .lock()
            .expect("account_book mutex poisoned")
            .replace_holdings(
                KOTAK_NSE_BSE_CASH_BOOK_ID,
                BrokerHoldingsSnapshot {
                    holdings: rows.clone(),
                },
                KOTAK_HOLDINGS_PATH,
                as_of_ms,
            );
    }
    Ok(())
}

async fn fetch_and_plant_funds(
    state: &AppState,
    environment: &str,
    connection_id: &str,
    book_id: &str,
) -> Result<(), KotakPrivateFetchError> {
    let body = match book_id {
        KOTAK_NSE_NFO_BOOK_ID => kotak_nfo_limits_jdata_body(),
        _ => kotak_cash_limits_jdata_body(),
    };
    let json = fetch_kotak_private_json(
        state.broker_sync_control.credential_vault().as_ref(),
        environment,
        connection_id,
        book_id,
        "POST",
        KOTAK_LIMITS_PATH,
        Some(&body),
        state.kotak_private_base_url.as_deref(),
    )
    .await?;
    let snapshot = parse_kotak_limits_json(&json).map_err(|_| KotakPrivateFetchError::Http(0))?;
    let as_of_ms = Utc::now().timestamp_millis();
    state
        .account_book
        .lock()
        .expect("account_book mutex poisoned")
        .replace_funds(book_id, snapshot, KOTAK_LIMITS_PATH, as_of_ms);
    Ok(())
}

fn plant_orders_split(state: &AppState, split: HashMap<String, Vec<BrokerOpenOrder>>, path: &str) {
    let as_of_ms = Utc::now().timestamp_millis();
    let mut book = state
        .account_book
        .lock()
        .expect("account_book mutex poisoned");
    for (book_id, orders) in split {
        book.replace_orders(
            &book_id,
            BrokerOpenOrdersSnapshot { orders },
            path,
            as_of_ms,
        );
    }
}

fn plant_positions_split(
    state: &AppState,
    split: HashMap<String, Vec<BrokerPositionRow>>,
    path: &str,
) {
    let as_of_ms = Utc::now().timestamp_millis();
    let mut book = state
        .account_book
        .lock()
        .expect("account_book mutex poisoned");
    for (book_id, positions) in split {
        book.replace_positions(
            &book_id,
            BrokerPositionsSnapshot { positions },
            path,
            as_of_ms,
        );
    }
}

fn kotak_session(state: &AppState) -> Option<(String, String)> {
    state
        .kotak_session_locator
        .lock()
        .expect("kotak session locator poisoned")
        .clone()
}

fn orders_fresh(state: &AppState, book_id: &str) -> bool {
    let book = state
        .account_book
        .lock()
        .expect("account_book mutex poisoned");
    book.orders_slot(book_id)
        .is_some_and(|slot| slot_fresh(slot.as_of_ms, KOTAK_PRIVATE_MAX_AGE_MS))
}

pub async fn ensure_kotak_orders(state: &AppState, book_id: &str) {
    if orders_fresh(state, book_id) {
        return;
    }
    if authorize_book_call(
        book_id,
        "cis.kotaksecurities.com",
        "GET",
        KOTAK_ORDERS_PATH,
        true,
    )
    .is_err()
    {
        return;
    }
    let Some((environment, connection_id)) = kotak_session(state) else {
        return;
    };
    let _ = fetch_and_plant_orders(state, &environment, &connection_id).await;
}

pub async fn ensure_kotak_positions(state: &AppState, book_id: &str) {
    {
        let book = state
            .account_book
            .lock()
            .expect("account_book mutex poisoned");
        if book
            .positions_slot(book_id)
            .is_some_and(|slot| slot_fresh(slot.as_of_ms, KOTAK_PRIVATE_MAX_AGE_MS))
        {
            return;
        }
    }
    if authorize_book_call(
        book_id,
        "cis.kotaksecurities.com",
        "GET",
        KOTAK_POSITIONS_PATH,
        true,
    )
    .is_err()
    {
        return;
    }
    let Some((environment, connection_id)) = kotak_session(state) else {
        return;
    };
    let _ = fetch_and_plant_positions(state, &environment, &connection_id).await;
}

pub async fn ensure_kotak_holdings(state: &AppState) {
    {
        let book = state
            .account_book
            .lock()
            .expect("account_book mutex poisoned");
        if book
            .holdings_slot(KOTAK_NSE_BSE_CASH_BOOK_ID)
            .is_some_and(|slot| slot_fresh(slot.as_of_ms, KOTAK_PRIVATE_MAX_AGE_MS))
        {
            return;
        }
    }
    if authorize_book_call(
        KOTAK_NSE_BSE_CASH_BOOK_ID,
        "cis.kotaksecurities.com",
        "GET",
        KOTAK_HOLDINGS_PATH,
        true,
    )
    .is_err()
    {
        return;
    }
    let Some((environment, connection_id)) = kotak_session(state) else {
        return;
    };
    let _ = fetch_and_plant_holdings(state, &environment, &connection_id).await;
}

pub async fn ensure_kotak_funds(state: &AppState, book_id: &str) {
    {
        let book = state
            .account_book
            .lock()
            .expect("account_book mutex poisoned");
        if book
            .funds_slot(book_id)
            .is_some_and(|slot| slot_fresh(slot.as_of_ms, KOTAK_PRIVATE_MAX_AGE_MS))
        {
            return;
        }
    }
    if authorize_book_call(
        book_id,
        "cis.kotaksecurities.com",
        "POST",
        KOTAK_LIMITS_PATH,
        true,
    )
    .is_err()
    {
        return;
    }
    let Some((environment, connection_id)) = kotak_session(state) else {
        return;
    };
    let _ = fetch_and_plant_funds(state, &environment, &connection_id, book_id).await;
}

#[cfg(test)]
pub(crate) async fn fetch_kotak_private_debug(
    vault: &dyn BrokerCredentialVault,
    environment: &str,
    connection_id: &str,
    book_id: &str,
    method: &str,
    path: &str,
    body: Option<&str>,
) -> Result<(u16, String), KotakPrivateFetchError> {
    let blob = vault
        .load(environment, KOTAK_NEO, connection_id)
        .map_err(|_| KotakPrivateFetchError::Session)?
        .ok_or(KotakPrivateFetchError::Session)?;
    let creds = HostCredentialBlob::from(&blob);
    let base_url = match &creds {
        HostCredentialBlob::KotakSession { base_url, .. } => base_url.clone(),
        HostCredentialBlob::Hmac { .. }
        | HostCredentialBlob::KiteSession { .. }
        | HostCredentialBlob::UpstoxSession { .. }
        | HostCredentialBlob::FyersSession { .. }
        | HostCredentialBlob::GrowwSession { .. }
        | HostCredentialBlob::DhanSession { .. }
        | HostCredentialBlob::OkxSession { .. }
        | HostCredentialBlob::CoinbaseJwt { .. } => {
            return Err(KotakPrivateFetchError::Session)
        }
    };
    let host = kotak_base_host(&base_url).ok_or(KotakPrivateFetchError::Http(0))?;
    let mut headers = vec![];
    if body.is_some() {
        headers.push((
            "Content-Type".to_string(),
            "application/x-www-form-urlencoded".to_string(),
        ));
    }
    let prepared = prepare_request(method, &host, path, &[], &headers, body, &creds, 0);
    if std::env::var("KOTAK_PRIVATE_PROBE").is_ok() {
        if let HostCredentialBlob::KotakSession {
            base_url,
            hs_server_id,
            ..
        } = &creds
        {
            println!("probe base_url: {base_url} hs_server_id: {hs_server_id:?}");
        }
        println!("probe url: {}", prepared.url);
        if let Some(b) = &prepared.body {
            println!("probe body: {b}");
        }
    }
    let resp = crate::egress::shared()
        .send_prepared(
            book_id,
            Lane::PrivateRead,
            &prepared,
            Duration::from_secs(20),
        )
        .await
        .map_err(|_| KotakPrivateFetchError::Http(0))?;
    Ok((resp.status, resp.body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::host_policy::{authorize_inferred_call, AuthMode, HostRefuse};
    use crate::ubi::KeyringBrokerCredentialVault;
    use serde_json::Value;

    fn root_shape(value: &Value) -> String {
        match value {
            Value::Null => "null".into(),
            Value::Bool(_) => "bool".into(),
            Value::Number(_) => "number".into(),
            Value::String(_) => "string".into(),
            Value::Array(a) => format!("array(len={})", a.len()),
            Value::Object(m) => format!("object(keys={})", m.len()),
        }
    }

    fn first_row<'a>(value: &'a Value) -> Option<&'a Value> {
        if let Some(arr) = value.as_array().and_then(|a| a.first()) {
            return Some(arr);
        }
        value
            .get("data")
            .and_then(|d| d.as_array())
            .and_then(|a| a.first())
            .or_else(|| value.get("data"))
    }

    fn type_of(value: &Value) -> &'static str {
        match value {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        }
    }

    async fn probe_endpoint(label: &str, method: &str, path: &str, body: Option<&str>) {
        let vault = KeyringBrokerCredentialVault::new();
        let connection_id = std::env::var("KOTAK_PRIVATE_PROBE_CONNECTION")
            .unwrap_or_else(|_| "00000000-0000-4000-8000-000000000003".to_string());
        println!("\n───── {label} · {method} {path}");
        match authorize_inferred_call("cis.kotaksecurities.com", method, path, true) {
            Ok((cap, auth)) => println!("host_policy: capability={cap:?} auth={auth:?}"),
            Err(e) => {
                println!("host_policy REFUSED: {e:?} — fix allowlist before live probe");
                return;
            }
        }
        let body = match fetch_kotak_private_debug(
            &vault,
            "prod",
            &connection_id,
            KOTAK_NSE_BSE_CASH_BOOK_ID,
            method,
            path,
            body,
        )
        .await
        {
            Ok((status, body)) if (200..300).contains(&status) => body,
            Ok((status, body)) => {
                println!(
                    "HTTP {status} body prefix: {}",
                    body.chars().take(200).collect::<String>()
                );
                return;
            }
            Err(err) => {
                println!("FETCH failed: {err:?}");
                return;
            }
        };
        let Ok(value) = serde_json::from_str::<Value>(&body) else {
            println!("body did not parse as JSON (len={})", body.len());
            return;
        };
        println!("shape: {}", root_shape(&value));
        if let Some(stat) = value.get("stat").and_then(|v| v.as_str()) {
            println!("stat: {stat:?}");
        }
        if let Some(code) = value.get("stCode").and_then(|v| v.as_i64()) {
            println!("stCode: {code}");
        }
        if let Some(row) = first_row(&value) {
            if let Value::Object(map) = row {
                let mut keys: Vec<&str> = map.keys().map(String::as_str).collect();
                keys.sort_unstable();
                println!("first-row keys: {keys:?}");
                let redacted: Vec<String> = keys
                    .iter()
                    .map(|key| format!("\"{key}\": \"<{}>\"", type_of(&map[*key])))
                    .collect();
                println!("redacted row: {{{}}}", redacted.join(", "));
            }
        }
        let fixture = match label {
            "orders" => "fixtures/kotak/quick_user_orders.json",
            "positions" => "fixtures/kotak/quick_user_positions.json",
            "holdings" => "fixtures/kotak/portfolio_holdings.json",
            "limits" => "fixtures/kotak/quick_user_limits.json",
            "check-margin" => "fixtures/kotak/quick_user_check_margin.json",
            _ => return,
        };
        if std::env::var("KOTAK_PRIVATE_CAPTURE").is_ok() {
            let path = format!("{}/{}", env!("CARGO_MANIFEST_DIR"), fixture);
            if let Err(err) = std::fs::write(&path, &body) {
                println!("fixture write failed: {err}");
            } else {
                println!("wrote fixture: {path}");
            }
        }
    }

    /// Live probe for PR A5 — orders, holdings, positions, limits, check-margin.
    ///
    /// ```text
    /// KOTAK_PRIVATE_PROBE=1 cargo test --lib kotak_private_account_probe -- --ignored --nocapture
    /// KOTAK_PRIVATE_CAPTURE=1 …  # also writes fixtures/kotak/*.json
    /// ```
    #[tokio::test]
    #[ignore = "live authenticated Kotak private reads"]
    async fn kotak_private_account_probe() {
        if std::env::var("KOTAK_PRIVATE_PROBE").is_err() {
            println!("set KOTAK_PRIVATE_PROBE=1 to run live Kotak private account probe");
            return;
        }
        probe_endpoint("orders", "GET", KOTAK_ORDERS_PATH, None).await;
        probe_endpoint("positions", "GET", KOTAK_POSITIONS_PATH, None).await;
        probe_endpoint("holdings", "GET", KOTAK_HOLDINGS_PATH, None).await;
        probe_endpoint(
            "limits",
            "POST",
            KOTAK_LIMITS_PATH,
            Some(&kotak_cash_limits_jdata_body()),
        )
        .await;
        probe_endpoint(
            "check-margin",
            "POST",
            KOTAK_CHECK_MARGIN_PATH,
            Some(
                "exSeg=nse_cm&prc=90&prcTp=MKT&prod=CNC&qty=1&tok=1476&trnsTp=B&trgPrc=0&brkName=&brnchId=&slAbsOrTks=&slVal=&sqrOffAbsOrTks=&sqrOffVal=&trailSL=&tSLTks=",
            ),
        )
        .await;
    }

    #[test]
    fn kotak_private_paths_allowlisted() {
        authorize_inferred_call("cis.kotaksecurities.com", "GET", KOTAK_ORDERS_PATH, true)
            .expect("orders");
        authorize_inferred_call("cis.kotaksecurities.com", "GET", KOTAK_POSITIONS_PATH, true)
            .expect("positions");
        authorize_inferred_call("cis.kotaksecurities.com", "GET", KOTAK_HOLDINGS_PATH, true)
            .expect("holdings");
        let (limits_cap, limits_mode) =
            authorize_inferred_call("cis.kotaksecurities.com", "POST", KOTAK_LIMITS_PATH, true)
                .expect("limits");
        assert_eq!(limits_cap, "funds");
        assert_eq!(limits_mode, AuthMode::PrivateRead);
        authorize_inferred_call(
            "cis.kotaksecurities.com",
            "POST",
            KOTAK_CHECK_MARGIN_PATH,
            true,
        )
        .expect("check-margin");
    }

    #[test]
    fn kotak_place_order_still_refused() {
        assert!(matches!(
            authorize_inferred_call(
                "cis.kotaksecurities.com",
                "POST",
                "/quick/order/rule/ms/place",
                true
            ),
            Err(HostRefuse::MutationForbidden)
        ));
    }

    #[test]
    fn fixture_orders_parses_open_rows_only() {
        let body = include_str!("../../fixtures/kotak/quick_user_orders.json");
        let orders = parse_kotak_orders_json(body).expect("orders parse");
        assert_eq!(
            orders.len(),
            0,
            "fixture day book has no open unFldSz>0 rows"
        );
    }

    #[test]
    fn kotak_limits_jdata_body_matches_sdk_rest_py() {
        assert!(kotak_cash_limits_jdata_body().starts_with("jData="));
        assert!(kotak_nfo_limits_jdata_body().starts_with("jData="));
        let cash = kotak_jdata_form_body(&[("seg", "ALL"), ("exch", "ALL"), ("prod", "ALL")]);
        assert_eq!(cash, kotak_cash_limits_jdata_body());
    }

    #[test]
    fn positions_5203_no_data_is_empty_success() {
        let body = r#"{"stat":"Not_Ok","stCode":5203,"errMsg":"No Data"}"#;
        assert!(parse_kotak_positions_json(body).expect("5203").is_empty());
    }

    #[test]
    fn fixture_positions_parses_net_qty() {
        let body = include_str!("../../fixtures/kotak/quick_user_positions.json");
        let rows = parse_kotak_positions_json(body).expect("positions parse");
        assert!(!rows.is_empty());
        assert!(rows.iter().all(|r| r.net_qty.abs() > f64::EPSILON));
    }

    #[test]
    fn fixture_holdings_parses_cash_rows() {
        let body = include_str!("../../fixtures/kotak/portfolio_holdings.json");
        let rows = parse_kotak_holdings_json(body).expect("holdings parse");
        assert_eq!(rows.len(), 3);
        assert!(rows
            .iter()
            .all(|h| matches!(h.exchange_segment.as_str(), "nse_cm" | "bse_cm")));
    }

    #[test]
    fn limits_fixture_maps_net_and_margin_used_to_inr() {
        let body = include_str!("../../fixtures/kotak/quick_user_limits.json");
        let payload: Value = serde_json::from_str(body).expect("limits fixture parses");
        let snapshot = parse_kotak_limits_json(body).expect("limits parse");
        assert_eq!(snapshot.holdings.len(), 1);
        assert_eq!(snapshot.holdings[0].asset, "INR");
        assert_eq!(snapshot.holdings[0].free, 19.409999999999997);
        assert_eq!(snapshot.holdings[0].locked, 18.78);
        assert!(snapshot.unrealized_pnl.is_none());
        assert!(payload.get("SpanMarginPrsnt").is_some());
        assert!(payload.get("CollateralValue").is_some());
    }

    #[test]
    fn limits_zero_net_and_used_drops_inr_row() {
        let body = r#"{"stat":"Ok","stCode":200,"Net":"0","MarginUsed":"0"}"#;
        let snapshot = parse_kotak_limits_json(body).expect("zero limits parse");
        assert!(snapshot.holdings.is_empty());
    }

    #[test]
    fn rewrite_prepared_url_keeps_path_and_query() {
        assert_eq!(
            rewrite_prepared_url(
                "https://cis.kotaksecurities.com/portfolio/v1/holdings?sId=server4",
                "http://127.0.0.1:9",
            ),
            "http://127.0.0.1:9/portfolio/v1/holdings?sId=server4"
        );
    }

    #[test]
    fn kotak_private_book_id_routes_segments() {
        assert_eq!(kotak_private_book_id("nse_fo"), Some(KOTAK_NSE_NFO_BOOK_ID));
        assert_eq!(
            kotak_private_book_id("nse_cm"),
            Some(KOTAK_NSE_BSE_CASH_BOOK_ID)
        );
        assert_eq!(
            kotak_private_book_id("mcx_fo"),
            Some(KOTAK_MCX_FUTURE_BOOK_ID)
        );
    }
}
