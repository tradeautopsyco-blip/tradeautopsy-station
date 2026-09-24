//! `groww` adapter component (ADR 0014 · B6 `groww`).
//!
//! Day fills via fan-out — there is no day trade-book (B6 rows 2/5):
//! `GET /v1/order/list` (paged, page_size ≤ 25) → filter filled/executed orders →,
//! `GET /v1/order/trades/{groww_order_id}` per order (page_size ≤ 50) →,
//! `payload.trade_list[]`. Auth headers (`Authorization`, `Accept`,
//! `X-API-VERSION: 1.0`) are attached outside Wasm (B6 row 10); the component
//! sends no auth headers at all (R6).
//!
//! First book: NSE/BSE cash, products CNC + MIS only. Non-`CASH` segments and
//! non-CNC/MIS products (INTRADAY/MARGIN/others — D5 partial, B6 row 8) are
//! refused per-row: the trade row is skipped, never defaulted. Unlike Dhan, the
//! fills path carries no per-trade charges, so `fee` is always `None` — never an
//! invented zero (B6 row 8).
//!
//! A 404/GA004 on a per-order call is a named `trades_gap` aborting the fetch
//! with the order id in the message — never a synthetic fill (B6 row 6; the
//! OpenAlgo `synthetic_{orderid}` fabrication is refused).
//!
//! Documented assumptions where B6 is silent (all fail-closed, dogfood owns):
//! - `segment=CASH` is sent on both order-list and per-order calls. The segment
//!   *code* `CASH` is B6-evidenced (rows 1/8); that the query param accepts it
//!   (vs. requiring another value or omission) is NOT SPECIFIED IN SOURCE. The
//!   mapper re-gates every trade row on `segment == CASH` regardless.
//! - Order-list pages start at `page=0` and continue while a page is full
//!   (payload carries no B6-evidenced page cursor). Page base 0 vs 1 is NOT
//!   SPECIFIED IN SOURCE.
//! - Order-row field names (`groww_order_id`, `order_status`, `filled_quantity`)
//!   follow the venue's snake_case vocabulary; B6 pins only `groww_order_id`
//!   (row 5) and `filled_quantity`/`remaining_quantity`/`average_fill_price`
//!   (row 8). Order *status* strings are NOT SPECIFIED IN SOURCE: an order
//!   qualifies for fan-out when `filled_quantity > 0` (B6-cited) or its status
//!   reads EXECUTED/FILLED. Anything else (open/rejected/unknown/missing) is
//!   skipped, never fanned out.
//! - `trade_date_time` is ISO 8601 (B6 row 8); the exact shape (offset vs naive)
//!   is NOT SPECIFIED IN SOURCE. An explicit offset/`Z` is honoured; a naive
//!   timestamp is read as IST (+05:30, India venue) — never UTC-by-accident.
//! - `fill_id` prefers `exchange_trade_id`, falls back to `groww_trade_id`,
//!   then to deterministic `{groww_order_id}#{trade index in that order's
//!   trade_list}` — unique per response without inventing a venue id.
//! - Page-loop guards (100 pages each) are anti-hang backstops, not venue
//!   claims: a real day book never reaches them.

#![allow(clippy::all)]
#![cfg_attr(test, allow(dead_code, unused_imports))]

wit_bindgen::generate!({
    world: "broker-adapter-data",
    path: "../../docs/contracts",
});

use crate::exports::tradeautopsy::ubi_data::adapter::Guest as AdapterGuest;
use crate::exports::tradeautopsy::ubi_data::data_adapter::Guest as DataAdapterGuest;
use crate::tradeautopsy::ubi_data::broker_http;
use crate::tradeautopsy::ubi_data::types::{
    AssetClass, BrokerHttpRequest, BrokerHttpResponse, FillCursor, FillEvent, HttpQueryParam,
    InstrumentClass,
};

const HOST: &str = "api.groww.in";
const ORDER_LIST_PATH: &str = "/v1/order/list";
const TRADES_PATH_PREFIX: &str = "/v1/order/trades/";
/// B6 row 4: order-list page_size is 100 (curl) vs 25 (SDK schema table) —
/// Station implements ≤ 25 until dogfood proves higher.
const ORDER_LIST_PAGE_SIZE: u32 = 25;
/// B6 row 4: trades page_size max 50 (both pages agree).
const TRADES_PAGE_SIZE: u32 = 50;
const SEGMENT_CASH: &str = "CASH";
/// Anti-hang loop guards (see module docs) — not venue claims.
const MAX_ORDER_PAGES: u32 = 100;
const MAX_TRADES_PAGES: u32 = 100;
/// Naive `trade_date_time` values are read as IST (see module docs).
const IST_OFFSET_SECONDS: i64 = 5 * 3600 + 30 * 60;

struct GrowwAdapter;

// `export!` + the `AdapterGuest` impl reference the `broker-http` Wasm import,
// which has no host-side symbol when unit tests link natively. Gate them out under
// `cfg(test)` so the pure fetch/mapper stays unit-testable; the Wasm contract
// (fetch, describe, obtain) is proven by `agent/tests/ubi_groww_component.rs`.
#[cfg(not(test))]
export!(GrowwAdapter);

#[cfg(not(test))]
impl AdapterGuest for GrowwAdapter {
    fn fetch_fills(cursor: FillCursor) -> Result<Vec<FillEvent>, String> {
        fetch_all_fills(&broker_http::broker_http_call, &cursor)
    }
}

impl DataAdapterGuest for GrowwAdapter {
    fn describe() -> Result<String, String> {
        Ok(describe_json())
    }

    fn obtain(request: String) -> Result<String, String> {
        obtain_json(&request)
    }
}

/// Which call a transport/envelope error belongs to — per-order failures name
/// the order so a gap is never anonymous.
#[derive(Clone, Copy)]
enum CallCtx<'a> {
    OrderList,
    OrderTrades(&'a str),
}

/// Full fills pipeline over an injected `broker_http_call`. In Wasm the caller
/// passes the host import; native tests pass stubs. Never synthesizes.
fn fetch_all_fills(
    call: &dyn Fn(&BrokerHttpRequest) -> Result<BrokerHttpResponse, String>,
    cursor: &FillCursor,
) -> Result<Vec<FillEvent>, String> {
    let mut fills = Vec::new();
    let mut page = 0u32;
    loop {
        let root = broker_get(
            call,
            ORDER_LIST_PATH,
            order_list_query(page),
            CallCtx::OrderList,
        )?;
        fail_closed_envelope(&root, CallCtx::OrderList)?;
        let orders = order_rows(&root)?;
        for order in orders {
            if !order_qualifies(order) {
                continue;
            }
            let order_id = string_number_field(order, "groww_order_id").ok_or_else(|| {
                "groww order row qualifies for fan-out but has no groww_order_id".to_string()
            })?;
            fetch_order_trades(call, &order_id, &mut fills)?;
        }
        page += 1;
        if orders.len() < ORDER_LIST_PAGE_SIZE as usize || page >= MAX_ORDER_PAGES {
            break;
        }
    }

    if let Some(since) = cursor.since_unix_ms {
        fills.retain(|f| f.filled_at_unix_ms >= since);
    }
    fills.sort_by_key(|f: &FillEvent| f.filled_at_unix_ms);
    Ok(fills)
}

/// Per-order fan-out: every page of `trade_list[]` for one order. A 404/GA004
/// aborts with a named `trades_gap` — the caller never sees partial success.
fn fetch_order_trades(
    call: &dyn Fn(&BrokerHttpRequest) -> Result<BrokerHttpResponse, String>,
    order_id: &str,
    fills: &mut Vec<FillEvent>,
) -> Result<(), String> {
    let path = format!("{TRADES_PATH_PREFIX}{order_id}");
    let mut page = 0u32;
    loop {
        let root = broker_get(call, &path, trades_query(page), CallCtx::OrderTrades(order_id))?;
        fail_closed_envelope(&root, CallCtx::OrderTrades(order_id))?;
        let trades = trade_rows(&root)?;
        for (index, trade) in trades.iter().enumerate() {
            if let Some(fill) = map_trade_row(trade, order_id, index)? {
                fills.push(fill);
            }
        }
        page += 1;
        if trades.len() < TRADES_PAGE_SIZE as usize || page >= MAX_TRADES_PAGES {
            break;
        }
    }
    Ok(())
}

fn order_list_query(page: u32) -> Vec<HttpQueryParam> {
    vec![
        query_param("segment", SEGMENT_CASH),
        query_param("page", &page.to_string()),
        query_param("page_size", &ORDER_LIST_PAGE_SIZE.to_string()),
    ]
}

fn trades_query(page: u32) -> Vec<HttpQueryParam> {
    vec![
        query_param("segment", SEGMENT_CASH),
        query_param("page", &page.to_string()),
        query_param("page_size", &TRADES_PAGE_SIZE.to_string()),
    ]
}

fn query_param(name: &str, value: &str) -> HttpQueryParam {
    HttpQueryParam {
        name: name.to_string(),
        value: value.to_string(),
    }
}

/// One authenticated GET through the host. The component sends no auth headers —
/// the host attaches `Authorization` + `Accept` + `X-API-VERSION` (B6 row 10).
fn broker_get(
    call: &dyn Fn(&BrokerHttpRequest) -> Result<BrokerHttpResponse, String>,
    path: &str,
    query: Vec<HttpQueryParam>,
    ctx: CallCtx,
) -> Result<serde_json::Value, String> {
    let response = call(&BrokerHttpRequest {
        method: "GET".to_string(),
        host: HOST.to_string(),
        path: path.to_string(),
        query,
        headers: vec![],
        body: None,
    })?;
    require_ok(&response, ctx)?;
    let root: serde_json::Value =
        serde_json::from_str(&response.body).map_err(|e| ctx_msg(ctx, &format!("json: {e}")))?;
    if is_session_code(&root) {
        return Err("groww session_expired".to_string());
    }
    Ok(root)
}

fn require_ok(response: &BrokerHttpResponse, ctx: CallCtx) -> Result<(), String> {
    if let Some(class) = response.error_class.as_deref() {
        if class == "session_expired" || class == "unauthorized" {
            return Err("groww session_expired".to_string());
        }
        // A 404 on a per-order call is the honest gap (B6 rows 4/6) — even when
        // the host classified it as a generic http_error. Name the order.
        if matches!(ctx, CallCtx::OrderTrades(_)) && response.status == 404 {
            return Err(gap_msg(ctx, "http 404"));
        }
        return Err(match ctx {
            CallCtx::OrderList => format!("groww {class} (http {})", response.status),
            CallCtx::OrderTrades(order_id) => {
                format!("groww trades_error (order {order_id}: {class} http {})", response.status)
            }
        });
    }
    if response.status == 401 || response.status == 403 {
        return Err("groww session_expired".to_string());
    }
    if matches!(ctx, CallCtx::OrderTrades(_)) && response.status == 404 {
        return Err(gap_msg(ctx, "http 404"));
    }
    if response.status != 200 {
        return Err(match ctx {
            CallCtx::OrderList => format!("groww http {}", response.status),
            CallCtx::OrderTrades(order_id) => {
                format!("groww trades_error (order {order_id}: http {})", response.status)
            }
        });
    }
    Ok(())
}

/// FAILURE envelopes are visible errors, never empty success (B6 row 4). On a
/// per-order call, GA004 (entity does not exist) is the named gap.
fn fail_closed_envelope(root: &serde_json::Value, ctx: CallCtx) -> Result<(), String> {
    let status = string_field(root, "status").unwrap_or_default();
    let failed = status.eq_ignore_ascii_case("failure") || status.eq_ignore_ascii_case("failed");
    let has_error = root.get("error").is_some();
    if !failed && !has_error {
        return Ok(());
    }
    // Session codes are handled in `broker_get` before this runs; re-check so a
    // direct caller still fails toward reconnect, never toward Kill.
    if is_session_code(root) {
        return Err("groww session_expired".to_string());
    }
    let detail = error_detail(root);
    if matches!(ctx, CallCtx::OrderTrades(_)) && is_gap_code(root) {
        return Err(gap_msg(ctx, &detail));
    }
    Err(match ctx {
        CallCtx::OrderList => format!("groww trades_error ({detail})"),
        CallCtx::OrderTrades(order_id) => {
            format!("groww trades_error (order {order_id}: {detail})")
        }
    })
}

/// Dead-token predicate: GA005 ("not authorised", B6 row 9 — the nearest code;
/// Z6 locks the exact predicate at dogfood) in the official `error.code`
/// position, so ids and timestamps elsewhere cannot false-positive.
fn is_session_code(root: &serde_json::Value) -> bool {
    error_code(root)
        .map(|code| code.to_ascii_uppercase().contains("GA005"))
        .unwrap_or(false)
}

/// GA004 (entity does not exist — B6 row 4) in the official `error.code`
/// position. Only meaningful on per-order calls; the order-list path never
/// treats it as a gap.
fn is_gap_code(root: &serde_json::Value) -> bool {
    error_code(root)
        .map(|code| code.to_ascii_uppercase().contains("GA004"))
        .unwrap_or(false)
}

fn error_code(root: &serde_json::Value) -> Option<String> {
    root.get("error")
        .and_then(|e| string_field(e, "code").or_else(|| string_number_field(e, "code")))
}

fn error_detail(root: &serde_json::Value) -> String {
    let code = error_code(root)
        .map(|c| format!("code {c}"))
        .unwrap_or_else(|| {
            format!(
                "status {}",
                string_field(root, "status").unwrap_or_else(|| "unknown".to_string())
            )
        });
    match root
        .get("error")
        .and_then(|e| string_field(e, "message"))
    {
        Some(message) => format!("{code} {message}"),
        None => code,
    }
}

fn gap_msg(ctx: CallCtx, detail: &str) -> String {
    match ctx {
        CallCtx::OrderList => format!("groww trades_gap ({detail})"),
        CallCtx::OrderTrades(order_id) => {
            format!("groww trades_gap (order {order_id}: {detail})")
        }
    }
}

fn ctx_msg(ctx: CallCtx, detail: &str) -> String {
    match ctx {
        CallCtx::OrderList => format!("groww order_list {detail}"),
        CallCtx::OrderTrades(order_id) => format!("groww trades (order {order_id}) {detail}"),
    }
}

/// `payload.order_list[]` on success. Missing/null `payload` on SUCCESS is a
/// quiet day — zero fills, never an error (sibling parity).
fn order_rows(root: &serde_json::Value) -> Result<&[serde_json::Value], String> {
    payload_list(root, "order_list", "order rows")
}

/// `payload.trade_list[]` on success. Missing/null on SUCCESS means this order
/// contributed no fills — continue, never gap (only 404/GA004 is a gap).
fn trade_rows(root: &serde_json::Value) -> Result<&[serde_json::Value], String> {
    payload_list(root, "trade_list", "trade rows")
}

fn payload_list<'a>(
    root: &'a serde_json::Value,
    key: &str,
    what: &str,
) -> Result<&'a [serde_json::Value], String> {
    let payload = match root.get("payload") {
        None | Some(serde_json::Value::Null) => return Ok(&[]),
        Some(payload) => payload,
    };
    match payload.get(key) {
        None | Some(serde_json::Value::Null) => Ok(&[]),
        Some(serde_json::Value::Array(rows)) => Ok(rows.as_slice()),
        _ => Err(format!("groww payload.{key} is not an array ({what})")),
    }
}

/// Fan-out filter: an order qualifies when `filled_quantity > 0` (B6 row 8) or
/// its status reads EXECUTED/FILLED. Order status strings are NOT SPECIFIED IN
/// SOURCE (see module docs) — open/rejected/unknown/missing never fan out.
fn order_qualifies(row: &serde_json::Value) -> bool {
    if let Some(qty) = row.get("filled_quantity").and_then(|v| parse_f64(Some(v)).ok()) {
        if qty > 0.0 {
            return true;
        }
    }
    let status = string_field(row, "order_status")
        .or_else(|| string_field(row, "status"))
        .unwrap_or_default()
        .to_ascii_uppercase();
    status == "EXECUTED" || status == "FILLED"
}

/// One Groww trade row → one cash `FillEvent`, or `None` when the row is refused
/// by the cash-book gates. Refusals are silent skips of that row (sibling
/// parity); malformed *accepted* rows fail loudly instead of inventing values.
fn map_trade_row(
    row: &serde_json::Value,
    order_id: &str,
    index: usize,
) -> Result<Option<FillEvent>, String> {
    // Gate 1 — cash segments only, stamped verbatim. FNO/CURRENCY/COMMODITY and
    // anything unmapped are refused (B6 rows 1/12/21), never defaulted.
    let exchange_segment = match string_field(row, "segment") {
        Some(seg) if seg.eq_ignore_ascii_case(SEGMENT_CASH) => seg,
        _ => return Ok(None),
    };

    // Gate 2 — cash products only, named explicitly. CNC→CNC, MIS→MIS; D5
    // partial: INTRADAY/MARGIN/others are REFUSED until dogfood evidences them
    // (B6 row 8). Missing product is refused, never defaulted.
    let product = match string_field(row, "product")
        .map(|p| p.to_ascii_uppercase())
        .as_deref()
    {
        Some("CNC") => "CNC".to_string(),
        Some("MIS") => "MIS".to_string(),
        _ => return Ok(None),
    };

    let symbol = string_field(row, "trading_symbol")
        .ok_or_else(|| "groww trade row missing trading_symbol".to_string())?;
    let side = match string_field(row, "transaction_type")
        .map(|s| s.to_ascii_uppercase())
        .as_deref()
    {
        Some("BUY") => "BUY",
        Some("SELL") => "SELL",
        other => {
            return Err(format!(
                "groww trade row has unknown transaction_type {:?}",
                other.unwrap_or("")
            ))
        }
    };
    let qty = parse_f64(row.get("quantity"))
        .map_err(|_| "groww trade row missing quantity".to_string())?;
    let price = parse_f64(row.get("price"))
        .map_err(|_| "groww trade row missing price".to_string())?;
    let filled_at_unix_ms = parse_trade_time(row)?;

    // fill_id: venue trade id preferred; fallback is deterministic and unique per
    // response (`{groww_order_id}#{row index}`) without inventing a venue id.
    let fill_id = string_number_field(row, "exchange_trade_id")
        .or_else(|| string_number_field(row, "groww_trade_id"))
        .unwrap_or_else(|| format!("{order_id}#{index}"));
    let trade_id = string_number_field(row, "exchange_order_id").or_else(|| Some(order_id.to_string()));

    Ok(Some(FillEvent {
        fill_id,
        broker_slug: "groww".to_string(),
        connection_id: String::new(),
        // Placeholders only: the host stamps asset/instrument axes from the
        // shipping book (ADR 0004) and this component never classifies. The
        // symbol stays the plain `trading_symbol` — product/segment are never
        // smuggled into the class axes or the symbol (host `instrument_type`
        // derivation reads the plain symbol).
        asset_class: AssetClass::Equity,
        instrument_class: InstrumentClass::Spot,
        is_inverse: false,
        symbol,
        side: side.to_string(),
        qty,
        price,
        // B6 row 7: all prices in rupees. No per-trade charges exist in the
        // fills path — fee stays None, never an invented zero.
        currency: "INR".to_string(),
        filled_at_unix_ms,
        fee_amount: None,
        fee_currency: None,
        exchange_segment: Some(exchange_segment),
        product: Some(product),
        trade_id,
    }))
}

/// `trade_date_time` only (ISO 8601, B6 row 8). `created_at` is never consulted
/// as the fill clock. Unparseable clocks fail loudly, never silent zero.
fn parse_trade_time(row: &serde_json::Value) -> Result<i64, String> {
    match string_field(row, "trade_date_time") {
        Some(raw) => parse_iso_timestamp(&raw),
        None => Err("groww trade row missing trade_date_time".to_string()),
    }
}

/// Minimal ISO-8601 reader: `YYYY-MM-DD[T ]HH:MM:SS[.frac][Z|±HH[:MM]|±HHMM]`.
/// An explicit zone is honoured; a naive timestamp is read as IST (module docs).
fn parse_iso_timestamp(raw: &str) -> Result<i64, String> {
    let text = raw.trim();
    let (date_part, rest) = text
        .split_once(['T', 't', ' '])
        .ok_or_else(|| format!("groww bad trade_date_time '{raw}' (no date/time separator)"))?;

    let mut date_parts = date_part.split('-');
    let year: i64 = next_num(&mut date_parts, "year", raw)?;
    let month: i64 = next_num(&mut date_parts, "month", raw)?;
    let day: i64 = next_num(&mut date_parts, "day", raw)?;

    // Split clock from zone suffix: trailing Z, or ±HH[:MM]/±HHMM at the end.
    let (clock, offset_seconds) = split_zone(rest, raw)?;
    let mut clock_parts = clock.split(':');
    let hour: i64 = next_num(&mut clock_parts, "hour", raw)?;
    let minute: i64 = next_num(&mut clock_parts, "minute", raw)?;
    let second: i64 = clock_parts
        .next()
        .map(|s| s.split('.').next().unwrap_or("0").parse().unwrap_or(-1))
        .unwrap_or(0);
    let millis: i64 = clock
        .split(':')
        .nth(2)
        .and_then(|s| s.split_once('.').map(|(_, frac)| frac_millis(frac)))
        .unwrap_or(0);

    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return Err(format!("groww bad trade_date_time '{raw}'"));
    }
    if !(0..=23).contains(&hour) || !(0..=59).contains(&minute) || !(0..=60).contains(&second) {
        return Err(format!("groww bad trade_date_time '{raw}'"));
    }

    let days = days_from_civil(year, month, day);
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second - offset_seconds;
    Ok(seconds * 1000 + millis)
}

fn split_zone(rest: &str, raw: &str) -> Result<(String, i64), String> {
    let trimmed = rest.trim();
    if let Some(clock) = trimmed
        .strip_suffix(['Z', 'z'])
        .map(str::trim)
    {
        return Ok((clock.to_string(), 0));
    }
    // Find the last + / - that starts a zone (clock itself has no +/-).
    let mut zone_at = None;
    for (index, byte) in trimmed.bytes().enumerate() {
        if byte == b'+' || byte == b'-' {
            zone_at = Some(index);
        }
    }
    match zone_at {
        None => Ok((trimmed.to_string(), IST_OFFSET_SECONDS)),
        Some(at) => {
            let clock = trimmed[..at].trim().to_string();
            let zone = trimmed[at..].replace(':', "");
            if zone.len() < 3 {
                return Err(format!("groww bad trade_date_time '{raw}'"));
            }
            let sign = if zone.starts_with('-') { -1 } else { 1 };
            let digits = &zone[1..];
            let zone_hour: i64 = digits
                .get(..2)
                .and_then(|h| h.parse().ok())
                .ok_or_else(|| format!("groww bad trade_date_time '{raw}'"))?;
            let zone_min: i64 = digits.get(2..4).and_then(|m| m.parse().ok()).unwrap_or(0);
            if zone_hour > 23 || zone_min > 59 {
                return Err(format!("groww bad trade_date_time '{raw}'"));
            }
            Ok((clock, sign * (zone_hour * 3600 + zone_min * 60)))
        }
    }
}

fn frac_millis(frac: &str) -> i64 {
    let digits: String = frac.chars().filter(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return 0;
    }
    let padded = format!("{digits:0<3}");
    padded[..3].parse().unwrap_or(0)
}

fn next_num<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
    label: &str,
    raw: &str,
) -> Result<i64, String> {
    parts
        .next()
        .ok_or_else(|| format!("groww bad trade_date_time '{raw}' (missing {label})"))?
        .trim()
        .parse()
        .map_err(|_| format!("groww bad trade_date_time '{raw}'"))
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn string_field(row: &serde_json::Value, key: &str) -> Option<String> {
    row.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn string_number_field(row: &serde_json::Value, key: &str) -> Option<String> {
    if let Some(s) = string_field(row, key) {
        return Some(s);
    }
    row.get(key).and_then(|v| match v {
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    })
}

fn parse_f64(value: Option<&serde_json::Value>) -> Result<f64, String> {
    match value {
        Some(serde_json::Value::Number(n)) => n.as_f64().ok_or_else(|| "number not f64".to_string()),
        Some(serde_json::Value::String(s)) => s
            .trim()
            .parse()
            .map_err(|e| format!("parse f64 '{s}': {e}")),
        _ => Err("missing number".to_string()),
    }
}

fn describe_json() -> String {
    serde_json::json!({
        "manifest_id": "tradeautopsy:groww-cash@0.1.0",
        "adapter_id": "groww",
        "implemented": ["tradebook"],
        "bindings": [
            {
                "operation": "tradebook",
                "adapter_id": "groww",
                "family": "account",
                "capability_id": "fills",
                "physics": "bounded_snapshot",
            },
        ],
    })
    .to_string()
}

fn obtain_json(request: &str) -> Result<String, String> {
    let req: serde_json::Value =
        serde_json::from_str(request).map_err(|e| format!("obtain json: {e}"))?;
    let family = json_str(&req, &["family"]).unwrap_or_default();
    let capability = json_str(&req, &["capability-id", "capability_id"]).unwrap_or_default();
    let physics = json_str(&req, &["physics"]).unwrap_or_default();
    let operation = json_str(&req, &["operation"]).unwrap_or_default();
    let is_history = operation == "history"
        || (family == "market" && capability == "ohlcv" && physics == "historical_series");
    if is_history {
        return Ok(serde_json::json!({
            "status": "unsupported",
            "operation": "history",
            "reason": "historical candles are not user trade history",
        })
        .to_string());
    }
    Err(format!(
        "groww obtain unsupported: {family}/{capability}/{physics}"
    ))
}

fn json_str(value: &serde_json::Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(s) = value.get(*key).and_then(|v| v.as_str()) {
            let trimmed = s.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    const ORDER_LIST: &str = include_str!("../../fixtures/groww/order_list_page.json");
    const TRADES_MULTI: &str = include_str!("../../fixtures/groww/trades_order_multi.json");
    const TRADES_SINGLE: &str = include_str!("../../fixtures/groww/trades_order_single.json");
    const TRADES_REFUSED_PRODUCT: &str =
        include_str!("../../fixtures/groww/trades_refused_product.json");
    const TRADES_REFUSED_SEGMENT: &str =
        include_str!("../../fixtures/groww/trades_refused_segment.json");

    /// 2026-09-24 10:15:30 +05:30.
    const FIRST_FILL_MS: i64 = 1_790_225_130_000;
    /// 2026-09-24 14:05:00 +05:30.
    const SINGLE_FILL_MS: i64 = 1_790_238_900_000;

    fn cursor_none() -> FillCursor {
        FillCursor {
            since_unix_ms: None,
            from_id: None,
            symbol: None,
        }
    }

    fn ok(body: &str) -> BrokerHttpResponse {
        BrokerHttpResponse {
            status: 200,
            headers: vec![],
            body: body.to_string(),
            error_class: None,
        }
    }

    /// Stub transport keyed by path; records every call for fan-out assertions.
    struct Stub {
        routes: HashMap<String, BrokerHttpResponse>,
        calls: RefCell<Vec<String>>,
    }

    impl Stub {
        fn call(&self, request: &BrokerHttpRequest) -> Result<BrokerHttpResponse, String> {
            assert_eq!(request.host, "api.groww.in");
            assert_eq!(request.method, "GET");
            assert!(
                request.headers.is_empty(),
                "component must send no auth headers"
            );
            self.calls.borrow_mut().push(request.path.clone());
            self.routes.get(&request.path).cloned().ok_or_else(|| {
                format!("no stub for path {} (honest gap in test wiring)", request.path)
            })
        }
    }

    fn main_stub() -> Stub {
        Stub {
            routes: HashMap::from([
                (ORDER_LIST_PATH.to_string(), ok(ORDER_LIST)),
                ("/v1/order/trades/GWKFILLMULTI01".to_string(), ok(TRADES_MULTI)),
                (
                    "/v1/order/trades/GWKFILLSINGLE02".to_string(),
                    ok(TRADES_SINGLE),
                ),
            ]),
            calls: RefCell::new(vec![]),
        }
    }

    fn sorted(mut fills: Vec<FillEvent>) -> Vec<FillEvent> {
        fills.sort_by_key(|f| f.filled_at_unix_ms);
        fills
    }

    #[test]
    fn fan_out_maps_filled_orders_and_skips_open_and_rejected() {
        let stub = main_stub();
        let fills = sorted(fetch_all_fills(&|r| stub.call(r), &cursor_none()).expect("fan-out maps"));

        // Multi order: 3 trades (20/50/30); single order: 1 trade. Open +
        // rejected orders are never fanned out.
        assert_eq!(fills.len(), 4);
        assert_eq!(
            *stub.calls.borrow(),
            vec![
                "/v1/order/list".to_string(),
                "/v1/order/trades/GWKFILLMULTI01".to_string(),
                "/v1/order/trades/GWKFILLSINGLE02".to_string(),
            ]
        );

        let first = &fills[0];
        assert_eq!(first.fill_id, "11000012345678");
        assert_eq!(first.broker_slug, "groww");
        assert_eq!(first.symbol, "RELIANCE");
        assert_eq!(first.side, "BUY");
        assert!((first.qty - 20.0).abs() < 1e-9);
        assert!((first.price - 1450.75).abs() < 1e-9);
        assert_eq!(first.currency, "INR");
        assert_eq!(first.exchange_segment.as_deref(), Some("CASH"));
        assert_eq!(first.product.as_deref(), Some("CNC"));
        assert_eq!(first.trade_id.as_deref(), Some("20000012345678"));
        assert_eq!(first.filled_at_unix_ms, FIRST_FILL_MS);
        // No per-trade charges in the fills path — never an invented fee.
        assert!(first.fee_amount.is_none());
        assert!(first.fee_currency.is_none());

        let single = &fills[3];
        assert_eq!(single.symbol, "SBIN");
        assert_eq!(single.side, "SELL");
        assert_eq!(single.product.as_deref(), Some("MIS"));
        assert_eq!(single.filled_at_unix_ms, SINGLE_FILL_MS);
    }

    #[test]
    fn order_list_paginates_while_pages_are_full() {
        let page_full: serde_json::Value = serde_json::from_str(ORDER_LIST).expect("fixture json");
        let _ = page_full;
        // Two logical pages via distinct stubs is path-keyed, so emulate a full
        // first page returning 25 qualifying rows inline and assert the loop
        // requests page=1 (query recorded through a bespoke stub).
        let mut orders = String::from(r#"{"status":"SUCCESS","payload":{"order_list":["#);
        for n in 0..25 {
            if n > 0 {
                orders.push(',');
            }
            orders.push_str(&format!(
                r#"{{"groww_order_id":"GWKPAGE{n:02}","order_status":"OPEN","filled_quantity":0}}"#
            ));
        }
        orders.push_str("]}}");
        let seen_pages = RefCell::new(vec![]);
        let empty = r#"{"status":"SUCCESS","payload":{"order_list":[]}}"#.to_string();
        let call = |request: &BrokerHttpRequest| {
            let page = request
                .query
                .iter()
                .find(|q| q.name == "page")
                .map(|q| q.value.clone())
                .unwrap_or_default();
            seen_pages.borrow_mut().push(page.clone());
            if page == "0" {
                Ok(ok(&orders))
            } else {
                Ok(ok(&empty))
            }
        };
        let fills = fetch_all_fills(&call, &cursor_none()).expect("paged fetch");
        assert!(fills.is_empty());
        assert_eq!(*seen_pages.borrow(), vec!["0".to_string(), "1".to_string()]);
    }

    #[test]
    fn per_order_404_is_a_named_gap_never_synthetic() {
        let stub = Stub {
            routes: HashMap::from([
                (
                    ORDER_LIST_PATH.to_string(),
                    ok(r#"{"status":"SUCCESS","payload":{"order_list":[
                        {"groww_order_id":"GWKGAP01","order_status":"EXECUTED","filled_quantity":10}
                    ]}}"#),
                ),
                (
                    "/v1/order/trades/GWKGAP01".to_string(),
                    BrokerHttpResponse {
                        status: 404,
                        headers: vec![],
                        body: r#"{"status":"FAILURE","error":{"code":"GA004","message":"Order does not exist"}}"#.to_string(),
                        error_class: Some("http_error".to_string()),
                    },
                ),
            ]),
            calls: RefCell::new(vec![]),
        };
        let err = fetch_all_fills(&|r| stub.call(r), &cursor_none()).expect_err("gap must abort");
        assert!(err.contains("trades_gap"), "unexpected: {err}");
        assert!(err.contains("GWKGAP01"), "gap must name the order: {err}");
        assert!(!err.contains("synthetic"), "never synthesize: {err}");
    }

    #[test]
    fn ga004_body_on_200_is_still_a_named_gap() {
        let stub = Stub {
            routes: HashMap::from([
                (
                    ORDER_LIST_PATH.to_string(),
                    ok(r#"{"status":"SUCCESS","payload":{"order_list":[
                        {"groww_order_id":"GWKGAP02","order_status":"FILLED","filled_quantity":5}
                    ]}}"#),
                ),
                (
                    "/v1/order/trades/GWKGAP02".to_string(),
                    ok(r#"{"status":"FAILURE","error":{"code":"GA004","message":"entity does not exist"}}"#),
                ),
            ]),
            calls: RefCell::new(vec![]),
        };
        let err = fetch_all_fills(&|r| stub.call(r), &cursor_none()).expect_err("GA004 gap");
        assert!(err.contains("trades_gap"), "unexpected: {err}");
        assert!(err.contains("GWKGAP02"), "unexpected: {err}");
    }

    #[test]
    fn non_gap_venue_errors_are_visible_not_empty_success() {
        let stub = Stub {
            routes: HashMap::from([(
                ORDER_LIST_PATH.to_string(),
                ok(r#"{"status":"FAILURE","error":{"code":"GA003","message":"unable to serve"}}"#),
            )]),
            calls: RefCell::new(vec![]),
        };
        let err = fetch_all_fills(&|r| stub.call(r), &cursor_none()).expect_err("venue error");
        assert!(err.contains("trades_error"), "unexpected: {err}");
        assert!(err.contains("GA003"), "unexpected: {err}");
    }

    #[test]
    fn refused_products_are_skipped_not_defaulted() {
        // B6 D5 partial: INTRADAY/MARGIN/unknown/missing never reach the book.
        let stub = Stub {
            routes: HashMap::from([
                (
                    ORDER_LIST_PATH.to_string(),
                    ok(r#"{"status":"SUCCESS","payload":{"order_list":[
                        {"groww_order_id":"GWKREF01","order_status":"EXECUTED","filled_quantity":4}
                    ]}}"#),
                ),
                (
                    "/v1/order/trades/GWKREF01".to_string(),
                    ok(TRADES_REFUSED_PRODUCT),
                ),
            ]),
            calls: RefCell::new(vec![]),
        };
        let fills = fetch_all_fills(&|r| stub.call(r), &cursor_none()).expect("refusals skip");
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].symbol, "INFY");
        assert_eq!(fills[0].product.as_deref(), Some("CNC"));
    }

    #[test]
    fn refused_segments_are_skipped() {
        let stub = Stub {
            routes: HashMap::from([
                (
                    ORDER_LIST_PATH.to_string(),
                    ok(r#"{"status":"SUCCESS","payload":{"order_list":[
                        {"groww_order_id":"GWKREF02","order_status":"EXECUTED","filled_quantity":2}
                    ]}}"#),
                ),
                (
                    "/v1/order/trades/GWKREF02".to_string(),
                    ok(TRADES_REFUSED_SEGMENT),
                ),
            ]),
            calls: RefCell::new(vec![]),
        };
        let fills = fetch_all_fills(&|r| stub.call(r), &cursor_none()).expect("refusals skip");
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].symbol, "TCS");
        assert_eq!(fills[0].exchange_segment.as_deref(), Some("CASH"));
    }

    #[test]
    fn fill_id_falls_back_through_groww_id_to_order_index() {
        let stub = Stub {
            routes: HashMap::from([
                (
                    ORDER_LIST_PATH.to_string(),
                    ok(r#"{"status":"SUCCESS","payload":{"order_list":[
                        {"groww_order_id":"GWKFALL01","order_status":"EXECUTED","filled_quantity":2}
                    ]}}"#),
                ),
                (
                    "/v1/order/trades/GWKFALL01".to_string(),
                    ok(r#"{"status":"SUCCESS","payload":{"trade_list":[
                        {"groww_trade_id":"GT-1","trading_symbol":"A","transaction_type":"BUY",
                         "product":"CNC","segment":"CASH","exchange":"NSE",
                         "quantity":1,"price":10.0,"trade_date_time":"2026-09-24T10:00:00+05:30"},
                        {"trading_symbol":"B","transaction_type":"SELL",
                         "product":"MIS","segment":"CASH","exchange":"NSE",
                         "quantity":2,"price":20.0,"trade_date_time":"2026-09-24T11:00:00+05:30"}
                    ]}}"#),
                ),
            ]),
            calls: RefCell::new(vec![]),
        };
        let fills = sorted(fetch_all_fills(&|r| stub.call(r), &cursor_none()).expect("maps"));
        assert_eq!(fills.len(), 2);
        assert_eq!(fills[0].fill_id, "GT-1");
        assert_eq!(fills[1].fill_id, "GWKFALL01#1");
    }

    #[test]
    fn clocks_and_numbers_fail_loudly_never_silent_zero() {
        for (label, trade) in [
            (
                "clock",
                r#"{"exchange_trade_id":"E1","trading_symbol":"X","transaction_type":"BUY","product":"CNC","segment":"CASH","exchange":"NSE","quantity":1,"price":1.0,"trade_date_time":"not-a-time"}"#,
            ),
            (
                "qty",
                r#"{"exchange_trade_id":"E1","trading_symbol":"X","transaction_type":"BUY","product":"CNC","segment":"CASH","exchange":"NSE","price":1.0,"trade_date_time":"2026-09-24T10:00:00+05:30"}"#,
            ),
            (
                "price",
                r#"{"exchange_trade_id":"E1","trading_symbol":"X","transaction_type":"BUY","product":"CNC","segment":"CASH","exchange":"NSE","quantity":1,"trade_date_time":"2026-09-24T10:00:00+05:30"}"#,
            ),
            (
                "side",
                r#"{"exchange_trade_id":"E1","trading_symbol":"X","transaction_type":"HOLD","product":"CNC","segment":"CASH","exchange":"NSE","quantity":1,"price":1.0,"trade_date_time":"2026-09-24T10:00:00+05:30"}"#,
            ),
            (
                "symbol",
                r#"{"exchange_trade_id":"E1","transaction_type":"BUY","product":"CNC","segment":"CASH","exchange":"NSE","quantity":1,"price":1.0,"trade_date_time":"2026-09-24T10:00:00+05:30"}"#,
            ),
        ] {
            let stub = Stub {
                routes: HashMap::from([
                    (
                        ORDER_LIST_PATH.to_string(),
                        ok(r#"{"status":"SUCCESS","payload":{"order_list":[
                            {"groww_order_id":"GWKLOUD01","order_status":"EXECUTED","filled_quantity":1}
                        ]}}"#),
                    ),
                    (
                        "/v1/order/trades/GWKLOUD01".to_string(),
                        ok(&format!(
                            r#"{{"status":"SUCCESS","payload":{{"trade_list":[{trade}]}}}}"#
                        )),
                    ),
                ]),
                calls: RefCell::new(vec![]),
            };
            fetch_all_fills(&|r| stub.call(r), &cursor_none())
                .expect_err(&format!("{label} must fail loudly"));
        }
    }

    #[test]
    fn iso_zones_agree_and_naive_reads_ist() {
        // Same instant three ways: Z, +05:30, naive-as-IST.
        let z = parse_iso_timestamp("2026-09-24T04:45:30Z").expect("Z");
        let offset = parse_iso_timestamp("2026-09-24T10:15:30+05:30").expect("offset");
        let naive = parse_iso_timestamp("2026-09-24T10:15:30").expect("naive");
        assert_eq!(z, FIRST_FILL_MS);
        assert_eq!(offset, FIRST_FILL_MS);
        assert_eq!(naive, FIRST_FILL_MS);
        assert_eq!(
            parse_iso_timestamp("2026-09-24T10:15:30.250+05:30").expect("millis"),
            FIRST_FILL_MS + 250
        );
    }

    #[test]
    fn empty_day_is_zero_fills() {
        for body in [
            r#"{"status":"SUCCESS","payload":{"order_list":[]}}"#,
            r#"{"status":"SUCCESS","payload":null}"#,
            r#"{"status":"SUCCESS"}"#,
        ] {
            let stub = Stub {
                routes: HashMap::from([(ORDER_LIST_PATH.to_string(), ok(body))]),
                calls: RefCell::new(vec![]),
            };
            let fills = fetch_all_fills(&|r| stub.call(r), &cursor_none())
                .unwrap_or_else(|e| panic!("empty is success ({body}): {e}"));
            assert!(fills.is_empty(), "body={body}");
        }
    }

    #[test]
    fn cursor_since_filters_and_output_sorts_by_time() {
        let stub = main_stub();
        let fills = fetch_all_fills(
            &|r| stub.call(r),
            &FillCursor {
                since_unix_ms: Some(SINGLE_FILL_MS),
                from_id: None,
                symbol: None,
            },
        )
        .expect("cursor fetch");
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].symbol, "SBIN");
    }

    #[test]
    fn session_signals_surface_as_session_expired() {
        // GA005 body on the order list.
        let stub = Stub {
            routes: HashMap::from([(
                ORDER_LIST_PATH.to_string(),
                ok(r#"{"status":"FAILURE","error":{"code":"GA005","message":"not authorised"}}"#),
            )]),
            calls: RefCell::new(vec![]),
        };
        let err = fetch_all_fills(&|r| stub.call(r), &cursor_none()).expect_err("GA005");
        assert!(err.contains("session_expired"), "unexpected: {err}");

        // 401-class transport on a per-order call.
        let stub = Stub {
            routes: HashMap::from([
                (
                    ORDER_LIST_PATH.to_string(),
                    ok(r#"{"status":"SUCCESS","payload":{"order_list":[
                        {"groww_order_id":"GWKSESS01","order_status":"EXECUTED","filled_quantity":1}
                    ]}}"#),
                ),
                (
                    "/v1/order/trades/GWKSESS01".to_string(),
                    BrokerHttpResponse {
                        status: 401,
                        headers: vec![],
                        body: "{}".to_string(),
                        error_class: Some("unauthorized".to_string()),
                    },
                ),
            ]),
            calls: RefCell::new(vec![]),
        };
        let err = fetch_all_fills(&|r| stub.call(r), &cursor_none()).expect_err("401");
        assert!(err.contains("session_expired"), "unexpected: {err}");
    }

    #[test]
    fn host_refusals_fail_closed_with_names() {
        let stub = Stub {
            routes: HashMap::from([(
                ORDER_LIST_PATH.to_string(),
                BrokerHttpResponse {
                    status: 0,
                    headers: vec![],
                    body: String::new(),
                    error_class: Some("host_blocked".to_string()),
                },
            )]),
            calls: RefCell::new(vec![]),
        };
        let err = fetch_all_fills(&|r| stub.call(r), &cursor_none()).expect_err("host_blocked");
        assert!(err.contains("host_blocked"), "unexpected: {err}");

        let stub = Stub {
            routes: HashMap::from([(
                ORDER_LIST_PATH.to_string(),
                BrokerHttpResponse {
                    status: 429,
                    headers: vec![],
                    body: String::new(),
                    error_class: Some("rate_limited".to_string()),
                },
            )]),
            calls: RefCell::new(vec![]),
        };
        let err = fetch_all_fills(&|r| stub.call(r), &cursor_none()).expect_err("rate_limited");
        assert!(err.contains("rate_limited"), "unexpected: {err}");
    }

    #[test]
    fn describe_claims_tradebook_only() {
        let json: serde_json::Value =
            serde_json::from_str(&describe_json()).expect("describe json");
        assert_eq!(json["adapter_id"], "groww");
        let implemented = json["implemented"].as_array().cloned().unwrap_or_default();
        assert_eq!(implemented.len(), 1);
        assert!(!implemented.iter().any(|v| v.as_str() == Some("history")));
    }

    #[test]
    fn obtain_history_is_unsupported() {
        let request = serde_json::json!({
            "family": "market",
            "capability-id": "ohlcv",
            "physics": "historical_series",
            "operation": "history",
        })
        .to_string();
        let json: serde_json::Value =
            serde_json::from_str(&obtain_json(&request).expect("obtain")).expect("json");
        assert_eq!(json["status"], "unsupported");
    }
}
