//! Slice D: Kotak Neo REST `quote_type=depth` → bounded order-book snapshot.
//!
//! Same documented GET as Slice C:
//! `{baseUrl}/script-details/1.0/quotes/neosymbol/{neo_symbols}/{quote_type}`
//! with `quote_type=depth` (SDK Quotes.md / `PROD_URL["quotes_neo_symbol"]`).
//!
//! Physics is `market/order_book/bounded_snapshot` only. Never `ordered_state`,
//! never `synced`, never HSM `isDepth=true`. Cash `nse_cm` / `bse_cm` and named
//! NFO `nse_fo` on REST `quote_type=depth`. Depth JSON is not a TickBook last.

use super::descriptor::KOTAK_NEO_ADAPTER_ID;
use super::identity::{CapabilityId, Family, Identity, Physics};
use super::rights::Rights;
use super::kotak_quotes::{
    depth_book_and_instrument_from_quote_object, json_string, quote_objects,
};
use super::source_manifest::shipping_book_id_for_slug;
use super::tick::Transport;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DepthStatus {
    Success,
    Unavailable,
    Unusable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DepthLevel {
    pub price: String,
    pub quantity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orders: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DepthData {
    pub bids: Vec<DepthLevel>,
    pub asks: Vec<DepthLevel>,
    pub completeness: bool,
    pub bound_levels: usize,
    pub as_of: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DepthProvenance {
    pub adapter_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<Transport>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DepthEnvelope {
    pub identity: Identity,
    pub instrument_id: String,
    pub status: DepthStatus,
    pub data: Option<DepthData>,
    pub provenance: DepthProvenance,
    /// S3 desk grant. Depth extracts always carry `desk_display` on shipping books.
    pub rights: Rights,
}

/// Stored REST/stream ladder. Completeness/bounds are metadata, not a delta replica.
/// `sequence` is Binance `lastUpdateId` when present. Kotak REST has no sequence field
/// (NOT SPECIFIED IN SOURCE) → `None`. A missing sequence is a valid bounded snapshot,
/// not Unusable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepthSnapshot {
    pub instrument_id: String,
    /// Login identity / provenance. Distinct from `book_id`.
    pub adapter_id: String,
    /// DepthBook slot prefix. Two books can share a slug without overwrite.
    pub book_id: String,
    pub bids: Vec<DepthLevel>,
    pub asks: Vec<DepthLevel>,
    pub completeness: bool,
    pub bound_levels: usize,
    pub as_of: DateTime<Utc>,
    pub transport: Transport,
    pub sequence: Option<u64>,
}

pub fn order_book_bounded_snapshot() -> Identity {
    Identity::new(
        Family::Market,
        CapabilityId::new("order_book").expect("canonical order_book id"),
        Physics::BoundedSnapshot,
    )
}

/// First usable cash depth snapshot from a REST `quote_type=depth` body.
/// Empty / F&O / no valid levels → `None` (unusable, not an empty success).
pub fn depth_snapshot_from_kotak_json(
    raw: &str,
    received_at: DateTime<Utc>,
) -> Option<DepthSnapshot> {
    depth_snapshots_from_kotak_json(raw, received_at)
        .into_iter()
        .next()
}

pub fn depth_snapshots_from_kotak_json(
    raw: &str,
    received_at: DateTime<Utc>,
) -> Vec<DepthSnapshot> {
    let Ok(value) = serde_json::from_str::<Value>(raw) else {
        return Vec::new();
    };
    quote_objects(&value)
        .into_iter()
        .filter_map(|obj| snapshot_from_object(obj, received_at))
        .collect()
}

fn snapshot_from_object(value: &Value, received_at: DateTime<Utc>) -> Option<DepthSnapshot> {
    let (instrument_id, book_id) = depth_book_and_instrument_from_quote_object(value)?;
    let depth = value.get("depth").unwrap_or(value);
    let bids = levels_from(
        depth
            .get("buy")
            .or_else(|| depth.get("bids"))
            .or_else(|| depth.get("bid")),
    );
    let asks = levels_from(
        depth
            .get("sell")
            .or_else(|| depth.get("asks"))
            .or_else(|| depth.get("ask")),
    );
    if bids.is_empty() && asks.is_empty() {
        return None;
    }
    let bound_levels = bids.len().max(asks.len());
    Some(DepthSnapshot {
        instrument_id,
        adapter_id: KOTAK_NEO_ADAPTER_ID.to_string(),
        book_id,
        bids,
        asks,
        completeness: true,
        bound_levels,
        as_of: received_at,
        transport: Transport::Rest,
        sequence: None,
    })
}

fn levels_from(value: Option<&Value>) -> Vec<DepthLevel> {
    let Some(arr) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter().filter_map(level_from_value).collect()
}

fn level_from_value(value: &Value) -> Option<DepthLevel> {
    let price = json_string(
        value
            .get("price")
            .or_else(|| value.get("p"))
            .or_else(|| value.get("bp"))
            .or_else(|| value.get("sp"))?,
    )?;
    let quantity = json_string(
        value
            .get("quantity")
            .or_else(|| value.get("qty"))
            .or_else(|| value.get("q"))
            .or_else(|| value.get("volume"))
            .or_else(|| value.get("bq"))
            .or_else(|| value.get("sq"))?,
    )?;
    let price_n: f64 = price.parse().ok()?;
    let qty_n: f64 = quantity.parse().ok()?;
    if price_n <= 0.0 || qty_n <= 0.0 {
        return None;
    }
    let orders = value
        .get("orders")
        .or_else(|| value.get("o"))
        .or_else(|| value.get("no"))
        .and_then(json_string);
    Some(DepthLevel {
        price,
        quantity,
        orders,
    })
}

/// Slot prefix for DepthBook lookup. Shipping slugs map to their book; fixtures
/// and already-book ids keep the passed string (`shipping_book_id_for_slug` is None).
fn depth_slot(id: &str) -> String {
    shipping_book_id_for_slug(id).unwrap_or_else(|| id.to_string())
}

/// Adapter-keyed lookup: the Start **slug** picks its shipping book.
///
/// This resolves `binance_com` to `binance-com-spot`, so it cannot address a
/// second book on the same slug. Obtain must use [`extract_depth_on_book`]
/// instead — see the note there.
pub fn extract_depth(
    book: &super::depthbook::DepthBook,
    instrument_id: &str,
    adapter_id: Option<&str>,
) -> DepthEnvelope {
    let wanted = adapter_id.unwrap_or(KOTAK_NEO_ADAPTER_ID);
    let slot = depth_slot(wanted);
    let fallback = (slot != wanted).then_some(wanted);
    extract_depth_from_slot(book, instrument_id, &slot, fallback, wanted)
}

/// Book-keyed lookup — the only one obtain may use.
///
/// `binance_com` is the login slug for **two** books: spot and the named options
/// book. `shipping_book_id_for_slug("binance_com")` answers `binance-com-spot`,
/// so an adapter-keyed read on the options envelope would serve the spot ladder
/// for `btcusdt` as if it were the contract's own book. Here the slot is exactly
/// the caller's `book_id`, with no slug mapping and no adapter fallback: an empty
/// options slot reads Unavailable, never a cross-book paint.
pub fn extract_depth_on_book(
    book: &super::depthbook::DepthBook,
    instrument_id: &str,
    book_id: &str,
) -> DepthEnvelope {
    extract_depth_from_slot(book, instrument_id, book_id, None, book_id)
}

fn extract_depth_from_slot(
    book: &super::depthbook::DepthBook,
    instrument_id: &str,
    slot: &str,
    fallback_slot: Option<&str>,
    provenance_id: &str,
) -> DepthEnvelope {
    let identity = order_book_bounded_snapshot();
    let wanted = provenance_id;
    let Some(row) = book
        .get(slot, instrument_id)
        .or_else(|| fallback_slot.and_then(|alt| book.get(alt, instrument_id)))
    else {
        return DepthEnvelope {
            identity,
            instrument_id: instrument_id.to_string(),
            status: DepthStatus::Unavailable,
            data: None,
            provenance: DepthProvenance {
                adapter_id: wanted.to_string(),
                transport: None,
            },
            rights: Rights::desk_display(),
        };
    };
    if !row.completeness || (row.bids.is_empty() && row.asks.is_empty()) {
        return DepthEnvelope {
            identity,
            instrument_id: instrument_id.to_string(),
            status: DepthStatus::Unusable,
            data: None,
            provenance: DepthProvenance {
                adapter_id: row.adapter_id.clone(),
                transport: Some(row.transport),
            },
            rights: Rights::desk_display(),
        };
    }
    DepthEnvelope {
        identity,
        instrument_id: instrument_id.to_string(),
        status: DepthStatus::Success,
        data: Some(DepthData {
            bids: row.bids.clone(),
            asks: row.asks.clone(),
            completeness: true,
            bound_levels: row.bound_levels,
            as_of: row.as_of.to_rfc3339_opts(SecondsFormat::Millis, true),
        }),
        provenance: DepthProvenance {
            adapter_id: row.adapter_id.clone(),
            transport: Some(row.transport),
        },
        rights: Rights::desk_display(),
    }
}

/// Obtain(depth) succeeds only with a complete bounded snapshot — never synced.
pub fn depth_obtain_data(envelope: &DepthEnvelope) -> Option<serde_json::Value> {
    if envelope.status != DepthStatus::Success {
        return None;
    }
    let data = envelope.data.as_ref()?;
    if !data.completeness || (data.bids.is_empty() && data.asks.is_empty()) {
        return None;
    }
    Some(serde_json::json!({
        "identity": envelope.identity,
        "instrument_id": envelope.instrument_id,
        "bids": data.bids,
        "asks": data.asks,
        "completeness": data.completeness,
        "bound_levels": data.bound_levels,
        "as_of": data.as_of,
        "source": "rest_snapshot",
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::depthbook::DepthBook;
    use crate::data::kotak_quotes::quote_tick_from_kotak_json;
    use crate::data::KOTAK_NSE_NFO_BOOK_ID;
    use chrono::TimeZone;

    fn received() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 26, 12, 0, 0).unwrap()
    }

    const FIXTURE: &str = r#"{
        "message": [{
            "instrument_token": "2885",
            "exchange_segment": "nse_cm",
            "trading_symbol": "RELIANCE-EQ",
            "depth": {
                "buy": [
                    {"price": "1400.00", "quantity": "120", "orders": "3"},
                    {"price": "1399.50", "quantity": "80", "orders": "2"}
                ],
                "sell": [
                    {"price": "1400.50", "quantity": "90", "orders": "4"},
                    {"price": "1401.00", "quantity": "150", "orders": "5"}
                ]
            }
        }]
    }"#;

    #[test]
    fn fixture_depth_is_bounded_snapshot_not_tickbook() {
        let snap = depth_snapshot_from_kotak_json(FIXTURE, received()).expect("fixture depth");
        assert_eq!(snap.instrument_id, "nse_cm|2885");
        assert_eq!(snap.adapter_id, KOTAK_NEO_ADAPTER_ID);
        assert_eq!(
            snap.book_id,
            shipping_book_id_for_slug(KOTAK_NEO_ADAPTER_ID).expect("kotak cash book")
        );
        assert_eq!(snap.transport, Transport::Rest);
        assert!(snap.sequence.is_none());
        assert!(snap.completeness);
        assert!(!snap.bids.is_empty());
        assert!(!snap.asks.is_empty());
        assert_eq!(snap.bound_levels, snap.bids.len().max(snap.asks.len()));
        assert!(quote_tick_from_kotak_json(FIXTURE, received()).is_none());

        let mut book = DepthBook::new();
        book.upsert(snap);
        let envelope = extract_depth(&book, "nse_cm|2885", Some(KOTAK_NEO_ADAPTER_ID));
        assert_eq!(envelope.status, DepthStatus::Success);
        assert_eq!(envelope.identity, order_book_bounded_snapshot());
        assert_eq!(envelope.identity.physics, Physics::BoundedSnapshot);
        assert_ne!(envelope.identity.physics, Physics::OrderedState);
        let data = depth_obtain_data(&envelope).expect("complete snapshot");
        assert_eq!(data["identity"]["physics"], "bounded_snapshot");
        assert_ne!(data["identity"]["physics"], "ordered_state");
        assert_eq!(data["identity"]["capability_id"], "order_book");
        assert_eq!(data["source"], "rest_snapshot");
        assert!(data.get("synced").is_none());
        assert_ne!(data["source"], "synced");
    }

    #[test]
    fn empty_depth_is_unusable() {
        let empty = r#"{"instrument_token":"2885","exchange_segment":"nse_cm","depth":{"buy":[],"sell":[]}}"#;
        assert!(depth_snapshot_from_kotak_json(empty, received()).is_none());
        let book = DepthBook::new();
        let missing = extract_depth(&book, "nse_cm|2885", Some(KOTAK_NEO_ADAPTER_ID));
        assert_eq!(missing.status, DepthStatus::Unavailable);
        assert!(missing.data.is_none());
        assert!(depth_obtain_data(&missing).is_none());
    }

    #[test]
    fn nfo_depth_uses_exchange_token_and_nfo_book() {
        let json = r#"[{
            "exchange": "nse_fo",
            "exchange_token": "56526",
            "depth": {
                "buy": [{"price": "10.50", "quantity": "120", "orders": "3"}],
                "sell": [{"price": "11.00", "quantity": "90", "orders": "4"}]
            }
        }]"#;
        let snap = depth_snapshot_from_kotak_json(json, received()).expect("nfo depth");
        assert_eq!(snap.instrument_id, "nse_fo|56526");
        assert_eq!(snap.book_id, KOTAK_NSE_NFO_BOOK_ID);
        assert!(!snap.bids.is_empty());
        assert_eq!(snap.bids[0].orders.as_deref(), Some("3"));
    }

    #[test]
    fn live_nfo_depth_fixture_with_zero_levels_is_unusable() {
        let json = include_str!("../../fixtures/kotak/quotes_neosymbol_nfo_depth.json");
        assert!(depth_snapshot_from_kotak_json(json, received()).is_none());
    }

    #[test]
    fn live_root_array_depth_uses_exchange_token() {
        let json = r#"[{
            "exchange": "nse_cm",
            "exchange_token": "11536",
            "depth": {
                "buy": [{"price": "3224.00", "quantity": "10"}],
                "sell": [{"price": "3225.00", "quantity": "12"}]
            }
        }]"#;
        let snap = depth_snapshot_from_kotak_json(json, received()).expect("live array depth");
        assert_eq!(snap.instrument_id, "nse_cm|11536");
        assert_eq!(snap.adapter_id, KOTAK_NEO_ADAPTER_ID);
        assert!(!snap.bids.is_empty());
    }

    /// The load-bearing regression. `binance_com` is one slug over two books;
    /// obtain must read the slot it was asked for, not the slug's shipping book.
    #[test]
    fn a_book_keyed_read_never_serves_the_other_book_on_the_same_slug() {
        use crate::data::binance_options_depth::depth_snapshot_from_eapi_json;
        use crate::data::descriptor::{BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID};

        const EAPI: &str = include_str!("../../fixtures/binance/options_depth.json");
        let mut book = DepthBook::new();
        book.upsert(
            depth_snapshot_from_eapi_json(EAPI, "BTC-200730-9000-C", received()).expect("options"),
        );

        let options =
            extract_depth_on_book(&book, "BTC-200730-9000-C", BINANCE_COM_OPTIONS_BOOK_ID);
        assert_eq!(options.status, DepthStatus::Success);
        assert_eq!(options.provenance.adapter_id, "binance_com");
        let data = depth_obtain_data(&options).expect("bounded snapshot");
        assert_eq!(data["identity"]["physics"], "bounded_snapshot");
        assert_ne!(data["identity"]["physics"], "ordered_state");
        assert_eq!(data["source"], "rest_snapshot");
        assert!(data.get("synced").is_none());
        // Positional levels carry no order count.
        assert!(data["bids"][0].get("orders").is_none());

        // The same ladder must not be reachable as spot depth, by book or by id.
        assert_eq!(
            extract_depth_on_book(&book, "BTC-200730-9000-C", BINANCE_COM_SPOT_BOOK_ID).status,
            DepthStatus::Unavailable
        );
        assert_eq!(
            extract_depth_on_book(&book, "btcusdt", BINANCE_COM_SPOT_BOOK_ID).status,
            DepthStatus::Unavailable
        );
        // A lowercased contract is a different id on this book — no case smash.
        assert_eq!(
            extract_depth_on_book(&book, "btc-200730-9000-c", BINANCE_COM_OPTIONS_BOOK_ID).status,
            DepthStatus::Unavailable
        );
        // And the adapter-keyed helper still resolves the slug to spot, which is
        // exactly why obtain may not use it for this book.
        assert_eq!(
            extract_depth(&book, "BTC-200730-9000-C", Some("binance_com")).status,
            DepthStatus::Unavailable
        );
    }

    /// A spot ladder in the book must not paint an empty options slot.
    #[test]
    fn a_resident_spot_ladder_leaves_the_options_slot_unavailable() {
        use crate::data::descriptor::{
            BINANCE_COM_ADAPTER_ID, BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID,
        };
        let mut book = DepthBook::new();
        book.upsert(DepthSnapshot {
            instrument_id: "btcusdt".into(),
            adapter_id: BINANCE_COM_ADAPTER_ID.into(),
            book_id: BINANCE_COM_SPOT_BOOK_ID.into(),
            bids: vec![DepthLevel {
                price: "60000.00".into(),
                quantity: "1".into(),
                orders: None,
            }],
            asks: vec![DepthLevel {
                price: "60001.00".into(),
                quantity: "1".into(),
                orders: None,
            }],
            completeness: true,
            bound_levels: 1,
            as_of: received(),
            transport: Transport::Stream,
            sequence: Some(9),
        });
        assert_eq!(
            extract_depth_on_book(&book, "btcusdt", BINANCE_COM_SPOT_BOOK_ID).status,
            DepthStatus::Success
        );
        // Same adapter, other book: no fallback, no repaint.
        assert_eq!(
            extract_depth_on_book(&book, "BTC-200730-9000-C", BINANCE_COM_OPTIONS_BOOK_ID).status,
            DepthStatus::Unavailable
        );
        assert_eq!(
            extract_depth_on_book(&book, "btcusdt", BINANCE_COM_OPTIONS_BOOK_ID).status,
            DepthStatus::Unavailable
        );
    }

    #[test]
    fn complete_snapshot_with_no_sequence_is_success_not_unusable() {
        let snap = depth_snapshot_from_kotak_json(FIXTURE, received()).expect("fixture depth");
        assert!(snap.sequence.is_none());
        assert!(snap.completeness);
        let mut book = DepthBook::new();
        book.upsert(snap);
        let envelope = extract_depth(&book, "nse_cm|2885", Some(KOTAK_NEO_ADAPTER_ID));
        assert_eq!(envelope.status, DepthStatus::Success);
        assert_ne!(envelope.status, DepthStatus::Unusable);
        assert!(envelope.data.is_some());
    }
}
