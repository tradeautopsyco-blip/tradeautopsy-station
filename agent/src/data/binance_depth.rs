//! Binance.com spot depth: REST `GET /api/v3/depth` snapshot + `@depth` managed book.
//!
//! Physics stays `market/order_book/bounded_snapshot` (5000-level bound). Sequence
//! validation drives gap → Unusable, not a flip to `ordered_state`.
//!
//! Source: `docs/reference/crypto/binance-global/spot/WEBSOCKET.md` (reconstruction
//! steps 1–7 and per-event update procedure) and REST.md `GET /api/v3/depth`
//! (Security NONE). Depth events must not enter TickBook.

use super::binance_public::normalize_quote_instrument;
use super::depthbook::DepthBook;
use super::descriptor::BINANCE_COM_ADAPTER_ID;
use super::host_policy::authorize_inferred_call;
use super::kotak_depth::{DepthLevel, DepthSnapshot};
use super::tick::Transport;
use chrono::{DateTime, Utc};
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

/// COM depth host only. Not `api.binance.us`. REST.md: `https://api.binance.com`.
pub const DEPTH_COM_HOST: &str = "api.binance.com";
pub const DEPTH_PATH: &str = "/api/v3/depth";
/// WEBSOCKET.md step 3: `limit=5000`.
pub const DEPTH_SNAPSHOT_LIMIT: u32 = 5000;
/// WEBSOCKET.md step 4 may refetch; cap GET `/api/v3/depth?limit=5000` attempts.
const DEPTH_RESNAPSHOT_MAX: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthSyncPhase {
    AwaitingFirstValid,
    Live,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepthDeltaDecision {
    /// `u <= lastUpdateId` (buffer) or `u < local` (live).
    Discard,
    /// First: `lastUpdateId ∈ [U, u]`; later: contiguous.
    Accept,
    /// First event lastUpdateId not in `[U, u]`, or live `U > local + 1`.
    Gap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DepthDelta {
    pub first_update_id: u64,
    pub final_update_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepthLevelChange {
    pub price: String,
    pub quantity: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepthUpdate {
    pub instrument_id: String,
    pub first_update_id: u64,
    pub final_update_id: u64,
    pub bids: Vec<DepthLevelChange>,
    pub asks: Vec<DepthLevelChange>,
}

impl DepthUpdate {
    pub fn delta(&self) -> DepthDelta {
        DepthDelta {
            first_update_id: self.first_update_id,
            final_update_id: self.final_update_id,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManagedBookError {
    NeedResnapshot,
    Gap,
}

pub fn binance_public_depth_stream_url(instrument: &str) -> String {
    format!(
        "wss://stream.binance.com:9443/ws/{}@depth",
        normalize_quote_instrument(instrument)
    )
}

pub fn depth_snapshot_url(symbol: &str) -> String {
    format!(
        "https://api.binance.com/api/v3/depth?symbol={}&limit={DEPTH_SNAPSHOT_LIMIT}",
        symbol.trim().to_ascii_uppercase()
    )
}

/// WEBSOCKET.md:141–151. Do not invent extra rules.
pub fn validate_depth_delta(
    snapshot_or_local_seq: u64,
    delta: &DepthDelta,
    phase: DepthSyncPhase,
) -> DepthDeltaDecision {
    match phase {
        DepthSyncPhase::AwaitingFirstValid => {
            if delta.final_update_id <= snapshot_or_local_seq {
                return DepthDeltaDecision::Discard;
            }
            if delta.first_update_id <= snapshot_or_local_seq
                && snapshot_or_local_seq <= delta.final_update_id
            {
                DepthDeltaDecision::Accept
            } else {
                DepthDeltaDecision::Gap
            }
        }
        DepthSyncPhase::Live => {
            if delta.final_update_id < snapshot_or_local_seq {
                return DepthDeltaDecision::Discard;
            }
            if delta.first_update_id > snapshot_or_local_seq.saturating_add(1) {
                return DepthDeltaDecision::Gap;
            }
            DepthDeltaDecision::Accept
        }
    }
}

fn json_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|n| u64::try_from(n).ok()))
}

fn parse_price_qty(value: &Value) -> Option<(String, String)> {
    let arr = value.as_array()?;
    let price = arr.first()?.as_str()?.to_string();
    let qty = arr.get(1)?.as_str()?.to_string();
    if price.is_empty() {
        return None;
    }
    Some((price, qty))
}

fn qty_is_zero(qty: &str) -> bool {
    qty.parse::<f64>().map(|q| q == 0.0).unwrap_or(false)
}

fn qty_is_positive(qty: &str) -> bool {
    qty.parse::<f64>().map(|q| q > 0.0).unwrap_or(false)
}

/// Parse REST `GET /api/v3/depth` JSON (`lastUpdateId`, `bids`/`asks` as `[price, qty]`).
pub fn depth_snapshot_from_binance_json(
    raw: &str,
    symbol: &str,
    received_at: DateTime<Utc>,
) -> Option<DepthSnapshot> {
    let value: Value = serde_json::from_str(raw).ok()?;
    let last_update_id = json_u64(value.get("lastUpdateId")?)?;
    let bids = levels_from_pairs(value.get("bids"), false);
    let asks = levels_from_pairs(value.get("asks"), false);
    if bids.is_empty() && asks.is_empty() {
        return None;
    }
    let bound_levels = bids.len().max(asks.len());
    Some(DepthSnapshot {
        instrument_id: normalize_quote_instrument(symbol),
        adapter_id: BINANCE_COM_ADAPTER_ID.to_string(),
        bids,
        asks,
        completeness: true,
        bound_levels,
        as_of: received_at,
        transport: Transport::Rest,
        sequence: Some(last_update_id),
    })
}

fn levels_from_pairs(value: Option<&Value>, keep_zero: bool) -> Vec<DepthLevel> {
    let Some(arr) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|row| {
            let (price, quantity) = parse_price_qty(row)?;
            if keep_zero {
                Some(DepthLevel {
                    price,
                    quantity,
                    orders: None,
                })
            } else if qty_is_positive(&quantity) {
                Some(DepthLevel {
                    price,
                    quantity,
                    orders: None,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Parse a `depthUpdate` event. Qty 0 is kept as a change (remove on apply).
pub fn depth_update_from_binance_json(raw: &str) -> Option<DepthUpdate> {
    let value: Value = serde_json::from_str(raw).ok()?;
    if value.get("e").and_then(Value::as_str) != Some("depthUpdate") {
        return None;
    }
    let symbol = value.get("s").and_then(Value::as_str)?;
    let first_update_id = json_u64(value.get("U")?)?;
    let final_update_id = json_u64(value.get("u")?)?;
    Some(DepthUpdate {
        instrument_id: normalize_quote_instrument(symbol),
        first_update_id,
        final_update_id,
        bids: level_changes(value.get("b")),
        asks: level_changes(value.get("a")),
    })
}

fn level_changes(value: Option<&Value>) -> Vec<DepthLevelChange> {
    let Some(arr) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|row| {
            let (price, quantity) = parse_price_qty(row)?;
            Some(DepthLevelChange { price, quantity })
        })
        .collect()
}

/// WEBSOCKET.md: set quantity; quantity = 0 → remove level.
pub fn apply_level_changes(levels: &mut Vec<DepthLevel>, changes: &[DepthLevelChange]) {
    for change in changes {
        if qty_is_zero(&change.quantity) {
            levels.retain(|level| level.price != change.price);
            continue;
        }
        if !qty_is_positive(&change.quantity) {
            continue;
        }
        if let Some(existing) = levels.iter_mut().find(|level| level.price == change.price) {
            existing.quantity = change.quantity.clone();
        } else {
            levels.push(DepthLevel {
                price: change.price.clone(),
                quantity: change.quantity.clone(),
                orders: None,
            });
        }
    }
}

pub fn apply_accepted_delta(book: &mut DepthSnapshot, update: &DepthUpdate) {
    apply_level_changes(&mut book.bids, &update.bids);
    apply_level_changes(&mut book.asks, &update.asks);
    book.sequence = Some(update.final_update_id);
    book.transport = Transport::Stream;
    book.bound_levels = book.bids.len().max(book.asks.len());
}

/// WEBSOCKET.md step 4: snapshot `lastUpdateId` < first buffered `U`.
/// Same dialect as `apply_managed_snapshot` → `NeedResnapshot`.
fn snapshot_behind_first_u(snapshot: &DepthSnapshot, buffer: &[DepthUpdate]) -> bool {
    let Some(last_id) = snapshot.sequence else {
        return true;
    };
    buffer
        .first()
        .is_some_and(|first| last_id < first.first_update_id)
}

/// WEBSOCKET.md step 5: discard buffered events where `u <= lastUpdateId`.
fn discard_events_at_or_before(buffer: &mut Vec<DepthUpdate>, last_update_id: u64) {
    buffer.retain(|update| update.final_update_id > last_update_id);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResnapshotAction {
    FetchAgain,
    Apply,
    GiveUp,
}

/// `attempt` is 1-based and counts the snapshot just checked.
fn next_resnapshot_action(behind: bool, attempt: u32) -> ResnapshotAction {
    if !behind {
        ResnapshotAction::Apply
    } else if attempt < DEPTH_RESNAPSHOT_MAX {
        ResnapshotAction::FetchAgain
    } else {
        ResnapshotAction::GiveUp
    }
}

/// WEBSOCKET.md steps 4–7 as a pure apply. No socket.
pub fn apply_managed_snapshot(
    mut snapshot: DepthSnapshot,
    buffer: &[DepthUpdate],
) -> Result<DepthSnapshot, ManagedBookError> {
    if snapshot_behind_first_u(&snapshot, buffer) {
        return Err(ManagedBookError::NeedResnapshot);
    }
    let last_id = snapshot.sequence.ok_or(ManagedBookError::NeedResnapshot)?;
    let mut phase = DepthSyncPhase::AwaitingFirstValid;
    let mut local = last_id;
    for update in buffer {
        match validate_depth_delta(local, &update.delta(), phase) {
            DepthDeltaDecision::Discard => continue,
            DepthDeltaDecision::Gap => return Err(ManagedBookError::Gap),
            DepthDeltaDecision::Accept => {
                apply_accepted_delta(&mut snapshot, update);
                local = update.final_update_id;
                phase = DepthSyncPhase::Live;
            }
        }
    }
    Ok(snapshot)
}

/// Starts a `@depth` stream once per instrument. Subsequent calls are no-ops.
pub fn ensure_binance_com_depth_stream(
    depthbook: Arc<Mutex<DepthBook>>,
    spawned: &Arc<Mutex<HashSet<String>>>,
    symbol: &str,
) {
    let instrument = normalize_quote_instrument(symbol);
    if instrument.is_empty() {
        return;
    }
    {
        let mut guard = spawned.lock().expect("depth stream set poisoned");
        if !guard.insert(instrument.clone()) {
            return;
        }
    }
    spawn_binance_com_depth_loop(depthbook, instrument);
}

pub fn spawn_binance_com_depth_loop(book: Arc<Mutex<DepthBook>>, symbol: String) {
    tokio::spawn(async move {
        let url = binance_public_depth_stream_url(&symbol);
        tracing::info!(instrument = %symbol, url = %url, "s1 desk: binance_com depth stream");
        loop {
            if let Err(err) = run_one_depth_connection(&book, &url, &symbol).await {
                tracing::warn!(instrument = %symbol, error = %err, "s1 desk: depth stream ended");
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    });
}

async fn fetch_binance_depth_snapshot(symbol: &str) -> anyhow::Result<DepthSnapshot> {
    authorize_inferred_call(DEPTH_COM_HOST, "GET", DEPTH_PATH, false)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()?;
    let body = client
        .get(depth_snapshot_url(symbol))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    depth_snapshot_from_binance_json(&body, symbol, Utc::now())
        .ok_or_else(|| anyhow::anyhow!("depth snapshot unusable"))
}

/// Keep reading `@depth` events into `buffer` while GET snapshot runs (WEBSOCKET.md 1–3).
/// `None` means the stream closed before the snapshot arrived.
async fn fetch_snapshot_while_buffering(
    symbol: &str,
    read: &mut (impl Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin),
    buffer: &mut Vec<DepthUpdate>,
) -> Result<Option<DepthSnapshot>, anyhow::Error> {
    let fetch = fetch_binance_depth_snapshot(symbol);
    tokio::pin!(fetch);
    let mut snapshot = None;
    loop {
        tokio::select! {
            snap = &mut fetch, if snapshot.is_none() => {
                snapshot = Some(snap?);
            }
            msg = read.next() => {
                let Some(msg) = msg else {
                    return Ok(snapshot);
                };
                let text = match msg? {
                    Message::Text(text) => text.to_string(),
                    Message::Ping(_) | Message::Pong(_) | Message::Binary(_) => continue,
                    Message::Close(_) | Message::Frame(_) => return Ok(snapshot),
                };
                let Some(update) = depth_update_from_binance_json(&text) else {
                    continue;
                };
                if update.instrument_id != symbol {
                    continue;
                }
                buffer.push(update);
            }
        }
        if snapshot.is_some() {
            break;
        }
    }
    Ok(snapshot)
}

/// Land Unusable even when no prior row exists. Prefer the snapshot we already
/// hold (keep its levels); otherwise insert a Binance-tagged placeholder.
fn stamp_gap_unusable(
    book: &Arc<Mutex<DepthBook>>,
    instrument: &str,
    incomplete: Option<DepthSnapshot>,
) {
    let mut guard = book.lock().expect("depthbook mutex poisoned");
    if let Some(mut snap) = incomplete {
        snap.completeness = false;
        guard.upsert(snap);
    } else {
        guard.invalidate(instrument, BINANCE_COM_ADAPTER_ID);
    }
}

async fn run_one_depth_connection(
    book: &Arc<Mutex<DepthBook>>,
    url: &str,
    symbol: &str,
) -> Result<(), anyhow::Error> {
    let (ws, _response) = connect_async(url).await?;
    let (_write, mut read) = ws.split();
    let mut buffer: Vec<DepthUpdate> = Vec::new();
    let mut snapshot: Option<DepthSnapshot> = None;

    let fetch = fetch_binance_depth_snapshot(symbol);
    tokio::pin!(fetch);

    loop {
        tokio::select! {
            snap = &mut fetch, if snapshot.is_none() => {
                snapshot = Some(snap?);
            }
            msg = read.next() => {
                let Some(msg) = msg else { return Ok(()); };
                let text = match msg? {
                    Message::Text(text) => text.to_string(),
                    Message::Ping(_) | Message::Pong(_) | Message::Binary(_) => continue,
                    Message::Close(_) | Message::Frame(_) => return Ok(()),
                };
                let Some(update) = depth_update_from_binance_json(&text) else {
                    continue;
                };
                if update.instrument_id != symbol {
                    continue;
                }
                buffer.push(update);
            }
        }
        if snapshot.is_some() && !buffer.is_empty() {
            break;
        }
    }

    let mut snap = snapshot.expect("select exited with snapshot");
    let mut attempt = 1u32;
    loop {
        match next_resnapshot_action(snapshot_behind_first_u(&snap, &buffer), attempt) {
            ResnapshotAction::Apply => {
                if let Some(last_id) = snap.sequence {
                    discard_events_at_or_before(&mut buffer, last_id);
                }
                break;
            }
            ResnapshotAction::FetchAgain => {
                match fetch_snapshot_while_buffering(symbol, &mut read, &mut buffer).await? {
                    Some(next) => snap = next,
                    None => return Ok(()),
                }
                attempt = attempt.saturating_add(1);
            }
            ResnapshotAction::GiveUp => {
                stamp_gap_unusable(book, symbol, Some(snap));
                return Err(anyhow::anyhow!("depth snapshot behind buffer"));
            }
        }
    }

    match apply_managed_snapshot(snap.clone(), &buffer) {
        Ok(applied) => {
            let mut guard = book.lock().expect("depthbook mutex poisoned");
            guard.upsert(applied);
        }
        Err(ManagedBookError::NeedResnapshot) => {
            stamp_gap_unusable(book, symbol, Some(snap));
            return Err(anyhow::anyhow!("depth snapshot behind buffer"));
        }
        Err(ManagedBookError::Gap) => {
            stamp_gap_unusable(book, symbol, Some(snap));
            return Err(anyhow::anyhow!("depth sequence gap"));
        }
    }

    let mut local = {
        let guard = book.lock().expect("depthbook mutex poisoned");
        guard
            .get(symbol)
            .and_then(|row| row.sequence)
            .ok_or_else(|| anyhow::anyhow!("depth book missing sequence after apply"))?
    };

    while let Some(msg) = read.next().await {
        let text = match msg? {
            Message::Text(text) => text.to_string(),
            Message::Ping(_) | Message::Pong(_) | Message::Binary(_) => continue,
            Message::Close(_) | Message::Frame(_) => break,
        };
        let Some(update) = depth_update_from_binance_json(&text) else {
            continue;
        };
        if update.instrument_id != symbol {
            continue;
        }
        match validate_depth_delta(local, &update.delta(), DepthSyncPhase::Live) {
            DepthDeltaDecision::Discard => {}
            DepthDeltaDecision::Accept => {
                let mut guard = book.lock().expect("depthbook mutex poisoned");
                if let Some(row) = guard.get(symbol).cloned() {
                    let mut next = row;
                    apply_accepted_delta(&mut next, &update);
                    local = update.final_update_id;
                    guard.upsert(next);
                }
            }
            DepthDeltaDecision::Gap => {
                stamp_gap_unusable(book, symbol, None);
                return Err(anyhow::anyhow!("depth sequence gap"));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::depthbook::DepthBook;
    use crate::data::descriptor::KOTAK_NEO_ADAPTER_ID;
    use crate::data::kotak_depth::{
        depth_snapshot_from_kotak_json, extract_depth, DepthLevel, DepthStatus,
    };
    use chrono::TimeZone;

    fn received() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 28, 12, 0, 0).unwrap()
    }

    const REST_FIXTURE: &str = r#"{
        "lastUpdateId": 1027024,
        "bids": [
            ["4.00000000", "431.00000000"],
            ["3.90000000", "10.00000000"]
        ],
        "asks": [
            ["4.00000200", "12.00000000"]
        ]
    }"#;

    fn delta(u_first: u64, u_final: u64) -> DepthDelta {
        DepthDelta {
            first_update_id: u_first,
            final_update_id: u_final,
        }
    }

    fn rest_snapshot(last_update_id: u64) -> DepthSnapshot {
        DepthSnapshot {
            instrument_id: "btcusdt".into(),
            adapter_id: BINANCE_COM_ADAPTER_ID.into(),
            bids: vec![DepthLevel {
                price: "100.00".into(),
                quantity: "1".into(),
                orders: None,
            }],
            asks: vec![DepthLevel {
                price: "101.00".into(),
                quantity: "1".into(),
                orders: None,
            }],
            completeness: true,
            bound_levels: 1,
            as_of: received(),
            transport: Transport::Rest,
            sequence: Some(last_update_id),
        }
    }

    fn update(
        u_first: u64,
        u_final: u64,
        bids: Vec<DepthLevelChange>,
        asks: Vec<DepthLevelChange>,
    ) -> DepthUpdate {
        DepthUpdate {
            instrument_id: "btcusdt".into(),
            first_update_id: u_first,
            final_update_id: u_final,
            bids,
            asks,
        }
    }

    #[test]
    fn rest_snapshot_parses_last_update_id_as_sequence() {
        let snap = depth_snapshot_from_binance_json(REST_FIXTURE, "BTCUSDT", received())
            .expect("fixture depth");
        assert_eq!(snap.instrument_id, "btcusdt");
        assert_eq!(snap.adapter_id, BINANCE_COM_ADAPTER_ID);
        assert_eq!(snap.transport, Transport::Rest);
        assert_eq!(snap.sequence, Some(1027024));
        assert!(snap.completeness);
        assert_eq!(snap.bids.len(), 2);
        assert_eq!(snap.asks.len(), 1);
        assert_eq!(snap.bids[0].price, "4.00000000");
        assert_eq!(snap.bids[0].quantity, "431.00000000");
        assert!(snap.bids[0].orders.is_none());
    }

    #[test]
    fn kotak_rest_snapshot_sequence_stays_none() {
        let kotak = r#"{
            "message": [{
                "instrument_token": "2885",
                "exchange_segment": "nse_cm",
                "trading_symbol": "RELIANCE-EQ",
                "depth": {
                    "buy": [{"price": "1400.00", "quantity": "120"}],
                    "sell": [{"price": "1400.50", "quantity": "90"}]
                }
            }]
        }"#;
        let snap = depth_snapshot_from_kotak_json(kotak, received()).expect("kotak");
        assert!(snap.sequence.is_none());
        assert_eq!(snap.adapter_id, KOTAK_NEO_ADAPTER_ID);
        let mut book = DepthBook::new();
        book.upsert(snap);
        let envelope = extract_depth(&book, "nse_cm|2885", Some(KOTAK_NEO_ADAPTER_ID));
        assert_eq!(envelope.status, DepthStatus::Success);
    }

    #[test]
    fn buffer_discard_when_u_at_or_before_snapshot() {
        assert_eq!(
            validate_depth_delta(100, &delta(90, 95), DepthSyncPhase::AwaitingFirstValid),
            DepthDeltaDecision::Discard
        );
    }

    #[test]
    fn first_valid_when_last_update_id_in_range() {
        assert_eq!(
            validate_depth_delta(100, &delta(98, 105), DepthSyncPhase::AwaitingFirstValid),
            DepthDeltaDecision::Accept
        );
    }

    #[test]
    fn first_event_gap_when_last_update_id_not_in_range() {
        assert_eq!(
            validate_depth_delta(100, &delta(101, 110), DepthSyncPhase::AwaitingFirstValid),
            DepthDeltaDecision::Gap
        );
    }

    #[test]
    fn live_contiguous_is_accept() {
        assert_eq!(
            validate_depth_delta(105, &delta(106, 110), DepthSyncPhase::Live),
            DepthDeltaDecision::Accept
        );
    }

    #[test]
    fn live_gap_when_u_first_skips_local_plus_one() {
        assert_eq!(
            validate_depth_delta(105, &delta(108, 110), DepthSyncPhase::Live),
            DepthDeltaDecision::Gap
        );
    }

    #[test]
    fn live_discard_when_u_before_local() {
        assert_eq!(
            validate_depth_delta(105, &delta(100, 104), DepthSyncPhase::Live),
            DepthDeltaDecision::Discard
        );
    }

    #[test]
    fn gap_invalidates_complete_book_extract_unusable() {
        let mut book = DepthBook::new();
        book.upsert(rest_snapshot(100));
        assert_eq!(
            extract_depth(&book, "btcusdt", Some(BINANCE_COM_ADAPTER_ID)).status,
            DepthStatus::Success
        );
        // Step 4: first buffered U=90 is not ahead of lastUpdateId=100, so no refetch.
        // After discard (u<=100), remaining first event 100 ∉ [101, 110] → Gap.
        let err = apply_managed_snapshot(
            rest_snapshot(100),
            &[
                update(90, 95, Vec::new(), Vec::new()),
                update(101, 110, Vec::new(), Vec::new()),
            ],
        );
        assert_eq!(err, Err(ManagedBookError::Gap));
        book.invalidate("btcusdt", BINANCE_COM_ADAPTER_ID);
        let envelope = extract_depth(&book, "btcusdt", Some(BINANCE_COM_ADAPTER_ID));
        assert_eq!(envelope.status, DepthStatus::Unusable);
        assert_ne!(envelope.status, DepthStatus::Success);
        assert!(envelope.data.is_none());
    }

    #[test]
    fn stamp_gap_on_empty_book_extracts_unusable_not_unavailable() {
        let book = std::sync::Arc::new(std::sync::Mutex::new(DepthBook::new()));
        stamp_gap_unusable(&book, "btcusdt", None);
        let envelope = extract_depth(
            &book.lock().expect("depthbook mutex poisoned"),
            "btcusdt",
            Some(BINANCE_COM_ADAPTER_ID),
        );
        assert_eq!(envelope.status, DepthStatus::Unusable);
        assert_ne!(envelope.status, DepthStatus::Unavailable);
        assert!(envelope.data.is_none());
    }

    #[test]
    fn apply_managed_snapshot_discards_then_accepts_first_valid() {
        let applied = apply_managed_snapshot(
            rest_snapshot(100),
            &[
                update(90, 95, Vec::new(), Vec::new()),
                update(
                    98,
                    105,
                    vec![DepthLevelChange {
                        price: "100.00".into(),
                        quantity: "2".into(),
                    }],
                    Vec::new(),
                ),
            ],
        )
        .expect("first valid");
        assert_eq!(applied.sequence, Some(105));
        assert_eq!(applied.transport, Transport::Stream);
        assert_eq!(applied.bids[0].quantity, "2");
    }

    #[test]
    fn qty_zero_removes_level() {
        let applied = apply_managed_snapshot(
            rest_snapshot(100),
            &[update(
                98,
                105,
                vec![DepthLevelChange {
                    price: "100.00".into(),
                    quantity: "0".into(),
                }],
                Vec::new(),
            )],
        )
        .expect("apply");
        assert!(applied.bids.is_empty());
        assert_eq!(applied.asks.len(), 1);
        assert_eq!(applied.sequence, Some(105));
    }

    #[test]
    fn depth_update_parses_u_and_levels() {
        let json = r#"{
            "e":"depthUpdate","E":1,"s":"BTCUSDT",
            "U":157,"u":160,
            "b":[["0.0024","10"],["0.0023","0"]],
            "a":[["0.0026","100"]]
        }"#;
        let update = depth_update_from_binance_json(json).expect("depthUpdate");
        assert_eq!(update.instrument_id, "btcusdt");
        assert_eq!(update.first_update_id, 157);
        assert_eq!(update.final_update_id, 160);
        assert_eq!(update.bids.len(), 2);
        assert_eq!(update.bids[1].quantity, "0");
        assert_eq!(update.asks[0].price, "0.0026");
    }

    #[test]
    fn snapshot_behind_first_u_needs_refetch() {
        let snap = rest_snapshot(50);
        let buf = [update(90, 95, Vec::new(), Vec::new())];
        assert!(snapshot_behind_first_u(&snap, &buf));
        let err = apply_managed_snapshot(snap, &buf);
        assert_eq!(err, Err(ManagedBookError::NeedResnapshot));
    }

    #[test]
    fn snapshot_not_behind_when_last_update_id_at_or_after_first_u() {
        assert!(!snapshot_behind_first_u(
            &rest_snapshot(100),
            &[update(90, 95, Vec::new(), Vec::new())]
        ));
    }

    #[test]
    fn resnapshot_attempt_3_still_behind_gives_up() {
        assert_eq!(
            next_resnapshot_action(true, 1),
            ResnapshotAction::FetchAgain
        );
        assert_eq!(
            next_resnapshot_action(true, 2),
            ResnapshotAction::FetchAgain
        );
        assert_eq!(
            next_resnapshot_action(true, DEPTH_RESNAPSHOT_MAX),
            ResnapshotAction::GiveUp
        );
        assert_eq!(
            next_resnapshot_action(false, DEPTH_RESNAPSHOT_MAX),
            ResnapshotAction::Apply
        );
    }

    #[test]
    fn discard_events_at_or_before_then_first_valid_applies() {
        let mut buffer = vec![
            update(90, 95, Vec::new(), Vec::new()),
            update(
                98,
                105,
                vec![DepthLevelChange {
                    price: "100.00".into(),
                    quantity: "2".into(),
                }],
                Vec::new(),
            ),
        ];
        discard_events_at_or_before(&mut buffer, 100);
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer[0].first_update_id, 98);
        assert_eq!(buffer[0].final_update_id, 105);
        let applied = apply_managed_snapshot(rest_snapshot(100), &buffer).expect("first valid");
        assert_eq!(applied.sequence, Some(105));
        assert_eq!(applied.bids[0].quantity, "2");
    }

    #[test]
    fn depth_stream_url_is_public_symbol_lowercased() {
        assert_eq!(
            binance_public_depth_stream_url("BTCUSDT"),
            "wss://stream.binance.com:9443/ws/btcusdt@depth"
        );
    }

    #[test]
    fn extract_binance_rest_snapshot_is_success_with_sequence() {
        let snap = depth_snapshot_from_binance_json(REST_FIXTURE, "BTCUSDT", received()).unwrap();
        let mut book = DepthBook::new();
        book.upsert(snap);
        let envelope = extract_depth(&book, "btcusdt", Some(BINANCE_COM_ADAPTER_ID));
        assert_eq!(envelope.status, DepthStatus::Success);
        assert_eq!(envelope.provenance.transport, Some(Transport::Rest));
        assert_eq!(envelope.provenance.adapter_id, BINANCE_COM_ADAPTER_ID);
    }
}
