//! Options chain = `market/option_chain` BoundedSnapshot.
//! Open interest = `market/open_interest` LatestState.
//!
//! Not contracts, not depth-as-chain, not greeks. Rebuild from the named book's
//! slice-2 master + TickBook last — do not upsert chain rows into TickBook.
//!
//! NFO lock: no Kotak `/optionchain`. Chain = FO master rows for
//! (`pSymbolName` underlying, expiry) + optional last from TickBook.
//! Quote JSON does not name an `oi` field → OI stays Unavailable.
//!
//! Glance `status` uses [`GlanceStatus`] (depth-like). Success is not a fifth
//! [`super::honesty::HonestyStatus`].

use super::descriptor::{
    BINANCE_COM_ADAPTER_ID, BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID,
    KOTAK_NEO_ADAPTER_ID, KOTAK_NSE_BSE_CASH_BOOK_ID, KOTAK_NSE_NFO_BOOK_ID,
};
use super::honesty::HonestyStatus;
use super::identity::{CapabilityId, Family, Identity, Physics};
use super::provenance::ProvenanceLine;
use super::tickbook::TickBook;
use serde::Serialize;

/// Extract status on the glance wire. Not [`HonestyStatus`] — Success is live data.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GlanceStatus {
    Success,
    Unavailable,
    Empty,
    Unusable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GlanceEnvelope {
    pub identity: Identity,
    pub instrument_id: String,
    pub status: GlanceStatus,
    pub data: Option<serde_json::Value>,
    pub provenance: ProvenanceLine,
    pub ineligible: Vec<String>,
    pub canonical: bool,
    pub persist_canonical: bool,
}

/// One chain row. Strike/expiry stay raw. Last is optional per lock.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChainRow {
    pub instrument_id: String,
    pub lot: i64,
    pub trading_symbol: String,
    pub segment: String,
    pub instrument_type: String,
    pub option_type: String,
    pub strike_raw: String,
    pub expiry_raw: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last: Option<String>,
}

pub const CASH_IS_NOT_CHAIN: &str = "cash_is_not_option_chain";
pub const SPOT_IS_NOT_CHAIN: &str = "spot_is_not_option_chain";
pub const BOOK_REQUIRED: &str = "book_id_required";
pub const FO_MASTER_UNSPECIFIED: &str = "nfo_scrip_master_refused";
pub const OI_FIELD_UNSPECIFIED: &str = "oi_field_unspecified";

fn chain_identity() -> Identity {
    Identity::new(
        Family::Market,
        CapabilityId::new("option_chain").expect("canonical option_chain id"),
        Physics::BoundedSnapshot,
    )
}

fn oi_identity() -> Identity {
    Identity::new(
        Family::Market,
        CapabilityId::new("open_interest").expect("open_interest id"),
        Physics::LatestState,
    )
}

fn display_instrument(book_id: Option<&str>, raw: &str) -> String {
    let trimmed = raw.trim();
    match book_id.map(str::trim).filter(|id| !id.is_empty()) {
        Some(id) if id == KOTAK_NSE_NFO_BOOK_ID => {
            trimmed.to_string()
        }
        _ => trimmed.to_ascii_lowercase(),
    }
}

fn dark_chain(book_id: Option<&str>, instrument: &str, ineligible: &str) -> GlanceEnvelope {
    let identity = chain_identity();
    GlanceEnvelope {
        identity: identity.clone(),
        instrument_id: display_instrument(book_id, instrument),
        status: GlanceStatus::Unavailable,
        data: None,
        provenance: ProvenanceLine::raw_hole(identity),
        ineligible: vec![ineligible.to_string()],
        canonical: false,
        persist_canonical: false,
    }
}

fn dark_oi(book_id: Option<&str>, instrument: &str, ineligible: &str) -> GlanceEnvelope {
    let identity = oi_identity();
    GlanceEnvelope {
        identity: identity.clone(),
        instrument_id: display_instrument(book_id, instrument),
        status: GlanceStatus::Unavailable,
        data: None,
        provenance: ProvenanceLine::raw_hole(identity),
        ineligible: vec![ineligible.to_string()],
        canonical: false,
        persist_canonical: false,
    }
}

fn adapter_for_book(book_id: &str) -> &'static str {
    match book_id {
        id if id == KOTAK_NSE_NFO_BOOK_ID => KOTAK_NEO_ADAPTER_ID,
        id if id == BINANCE_COM_OPTIONS_BOOK_ID => BINANCE_COM_ADAPTER_ID,
        id if id == KOTAK_NSE_BSE_CASH_BOOK_ID => KOTAK_NEO_ADAPTER_ID,
        id if id == BINANCE_COM_SPOT_BOOK_ID => BINANCE_COM_ADAPTER_ID,
        _ => "",
    }
}

/// Hole unless `book_id` names a chain book with rows.
pub fn extract_chain(book_id: Option<&str>, underlying_or_instrument: &str) -> GlanceEnvelope {
    extract_chain_from(book_id, underlying_or_instrument, None, None)
}

/// Rebuild from caller-supplied master rows + optional TickBook last.
/// Empty store → Unavailable. Matching rows → Success even when last is missing
/// (NFO lock allows structure-without-last).
pub fn extract_chain_from(
    book_id: Option<&str>,
    underlying_or_instrument: &str,
    rows: Option<&[ChainRow]>,
    tickbook: Option<&TickBook>,
) -> GlanceEnvelope {
    let identity = chain_identity();
    let Some(book) = book_id.map(str::trim).filter(|id| !id.is_empty()) else {
        return dark_chain(book_id, underlying_or_instrument, BOOK_REQUIRED);
    };
    let instrument = display_instrument(Some(book), underlying_or_instrument);

    match book {
        id if id == KOTAK_NSE_BSE_CASH_BOOK_ID => {
            dark_chain(Some(book), underlying_or_instrument, CASH_IS_NOT_CHAIN)
        }
        id if id == BINANCE_COM_SPOT_BOOK_ID => {
            dark_chain(Some(book), underlying_or_instrument, SPOT_IS_NOT_CHAIN)
        }
        id if id == BINANCE_COM_OPTIONS_BOOK_ID => {
            dark_chain(Some(book), underlying_or_instrument, FO_MASTER_UNSPECIFIED)
        }
        id if id == KOTAK_NSE_NFO_BOOK_ID => match rows {
            None => dark_chain(Some(book), underlying_or_instrument, FO_MASTER_UNSPECIFIED),
            Some([]) => dark_chain(Some(book), underlying_or_instrument, FO_MASTER_UNSPECIFIED),
            Some(rows) => {
                let overlaid: Vec<ChainRow> = rows
                    .iter()
                    .map(|row| overlay_last(book, row, tickbook))
                    .collect();
                lit_chain(identity, book, &instrument, &overlaid)
            }
        },
        _ => dark_chain(Some(book), underlying_or_instrument, BOOK_REQUIRED),
    }
}

fn overlay_last(book_id: &str, row: &ChainRow, tickbook: Option<&TickBook>) -> ChainRow {
    let mut out = row.clone();
    if out.last.is_none() {
        if let Some(stored) = tickbook.and_then(|book| book.get(book_id, &row.instrument_id)) {
            if !stored.last.trim().is_empty() {
                out.last = Some(stored.last.clone());
            }
        }
    }
    out
}

fn lit_chain(
    identity: Identity,
    book_id: &str,
    instrument: &str,
    rows: &[ChainRow],
) -> GlanceEnvelope {
    let data = serde_json::json!({
        "identity": identity,
        "underlying": instrument,
        "row_count": rows.len(),
        "rows": rows,
    });
    GlanceEnvelope {
        identity: identity.clone(),
        instrument_id: instrument.to_string(),
        status: GlanceStatus::Success,
        data: Some(data),
        provenance: ProvenanceLine {
            identity,
            model: "raw".to_string(),
            input_at: None,
            adapter_id: adapter_for_book(book_id).to_string(),
        },
        ineligible: Vec::new(),
        canonical: false,
        persist_canonical: false,
    }
}

/// Hole unless `book_id` + a named OI snapshot. NFO quote JSON does not name `oi`.
pub fn extract_open_interest(book_id: Option<&str>, instrument_id: &str) -> GlanceEnvelope {
    extract_open_interest_from(book_id, instrument_id)
}

pub fn extract_open_interest_from(book_id: Option<&str>, instrument_id: &str) -> GlanceEnvelope {
    let Some(book) = book_id.map(str::trim).filter(|id| !id.is_empty()) else {
        return dark_oi(book_id, instrument_id, BOOK_REQUIRED);
    };

    match book {
        id if id == KOTAK_NSE_BSE_CASH_BOOK_ID => {
            dark_oi(Some(book), instrument_id, CASH_IS_NOT_CHAIN)
        }
        id if id == BINANCE_COM_SPOT_BOOK_ID => {
            dark_oi(Some(book), instrument_id, SPOT_IS_NOT_CHAIN)
        }
        id if id == KOTAK_NSE_NFO_BOOK_ID || id == BINANCE_COM_OPTIONS_BOOK_ID => {
            // NFO quote JSON does not name `oi`. eapi OI is not in this commit.
            dark_oi(Some(book), instrument_id, OI_FIELD_UNSPECIFIED)
        }
        _ => dark_oi(Some(book), instrument_id, BOOK_REQUIRED),
    }
}

/// Lit chain for greeks inherit: Success with `data`.
pub fn chain_input_honesty(envelope: &GlanceEnvelope) -> super::honesty::InputHonesty {
    if envelope.status == GlanceStatus::Success && envelope.data.is_some() {
        super::honesty::InputHonesty::Lit
    } else {
        super::honesty::InputHonesty::Dark(HonestyStatus::Unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::identity::Physics;
    use crate::data::tick::{QuoteTick, Transport};
    use crate::data::{apply_quote, kotak_neo_quote_descriptor, Registry};
    use chrono::{TimeZone, Utc};

    fn lock_nfo_row() -> ChainRow {
        ChainRow {
            instrument_id: "nse_fo|56526".to_string(),
            lot: 65,
            trading_symbol: "NIFTY2692221000PE".to_string(),
            segment: "nse_fo".to_string(),
            instrument_type: "OPTIDX".to_string(),
            option_type: "PE".to_string(),
            strike_raw: "2.1e+06".to_string(),
            expiry_raw: "1474554600".to_string(),
            last: None,
        }
    }

    #[test]
    fn chain_identity_is_option_chain_bounded_snapshot() {
        let chain = extract_chain(None, "BANKNIFTY");
        assert_eq!(chain.identity.family, Family::Market);
        assert_eq!(chain.identity.capability_id.as_str(), "option_chain");
        assert_eq!(chain.identity.physics, Physics::BoundedSnapshot);
        assert_ne!(chain.identity.capability_id.as_str(), "order_book");
        assert_ne!(
            chain.identity.capability_id.as_str(),
            "derivative_contracts"
        );
    }

    #[test]
    fn oi_identity_is_open_interest_latest_state() {
        let oi = extract_open_interest(None, "BTC");
        assert_eq!(oi.identity.capability_id.as_str(), "open_interest");
        assert_eq!(oi.identity.physics, Physics::LatestState);
        assert_ne!(oi.identity.capability_id.as_str(), "order_book");
    }

    #[test]
    fn extract_chain_without_book_is_unavailable() {
        let chain = extract_chain(None, "BANKNIFTY");
        assert_eq!(chain.status, GlanceStatus::Unavailable);
        assert!(chain.data.is_none());
        assert!(chain.ineligible.iter().any(|s| s == BOOK_REQUIRED));
        assert_eq!(chain.instrument_id, "banknifty");
    }

    #[test]
    fn empty_master_is_unavailable_data_null() {
        let none = extract_chain(Some(KOTAK_NSE_NFO_BOOK_ID), "NIFTY");
        assert_eq!(none.status, GlanceStatus::Unavailable);
        assert!(none.data.is_none());
        let empty = extract_chain_from(Some(KOTAK_NSE_NFO_BOOK_ID), "NIFTY", Some(&[]), None);
        assert_eq!(empty.status, GlanceStatus::Unavailable);
        assert!(empty.data.is_none());
    }

    #[test]
    fn fixture_master_without_last_is_success() {
        let rows = [lock_nfo_row()];
        let chain = extract_chain_from(Some(KOTAK_NSE_NFO_BOOK_ID), "NIFTY", Some(&rows), None);
        assert_eq!(chain.status, GlanceStatus::Success);
        assert!(chain.data.is_some());
        assert_eq!(chain.instrument_id, "NIFTY");
        let data = chain.data.as_ref().unwrap();
        assert_eq!(data["row_count"], 1);
        assert_eq!(data["rows"][0]["instrument_id"], "nse_fo|56526");
        assert_eq!(data["rows"][0]["lot"], 65);
        assert!(data["rows"][0].get("last").is_none() || data["rows"][0]["last"].is_null());
        assert_ne!(data["rows"][0]["strike_raw"], "57500");
        assert_eq!(chain.provenance.adapter_id, KOTAK_NEO_ADAPTER_ID);
    }

    #[test]
    fn tickbook_last_overlays_nfo_row_without_upserting_chain() {
        let registry = Registry::load(&[kotak_neo_quote_descriptor()]).unwrap();
        let mut book = TickBook::new();
        let tick = QuoteTick {
            instrument_id: "nse_fo|56526".to_string(),
            last: "10.00".to_string(),
            as_of: Utc.with_ymd_and_hms(2026, 8, 28, 12, 0, 0).unwrap(),
            received_at: Utc.with_ymd_and_hms(2026, 8, 28, 12, 0, 0).unwrap(),
            age_unknown: true,
            transport: Transport::Rest,
            adapter_id: KOTAK_NEO_ADAPTER_ID.to_string(),
            book_id: KOTAK_NSE_NFO_BOOK_ID.to_string(),
            session_ohlc: None,
        };
        apply_quote(&registry, &mut book, tick).unwrap();
        let rows = [lock_nfo_row()];
        let chain = extract_chain_from(
            Some(KOTAK_NSE_NFO_BOOK_ID),
            "NIFTY",
            Some(&rows),
            Some(&book),
        );
        assert_eq!(chain.status, GlanceStatus::Success);
        assert_eq!(chain.data.as_ref().unwrap()["rows"][0]["last"], "10.00");
        assert!(book.get(KOTAK_NSE_NFO_BOOK_ID, "NIFTY").is_none());
    }

    #[test]
    fn cash_and_spot_chain_stay_unavailable() {
        let cash = extract_chain(Some(KOTAK_NSE_BSE_CASH_BOOK_ID), "RELIANCE");
        assert_eq!(cash.status, GlanceStatus::Unavailable);
        assert!(cash.ineligible.iter().any(|s| s == CASH_IS_NOT_CHAIN));
        let spot = extract_chain(Some(BINANCE_COM_SPOT_BOOK_ID), "BTCUSDT");
        assert_eq!(spot.status, GlanceStatus::Unavailable);
        assert!(spot.ineligible.iter().any(|s| s == SPOT_IS_NOT_CHAIN));
    }

    #[test]
    fn nfo_oi_stays_unavailable_without_named_json_field() {
        let oi = extract_open_interest(Some(KOTAK_NSE_NFO_BOOK_ID), "NIFTY");
        assert_eq!(oi.status, GlanceStatus::Unavailable);
        assert!(oi.data.is_none());
        assert!(oi.ineligible.iter().any(|s| s == OI_FIELD_UNSPECIFIED));
    }

    #[test]
    fn depth_success_is_not_chain_success() {
        use crate::data::{extract_depth, DepthBook, DepthStatus};
        let depth = extract_depth(&DepthBook::new(), "nse_cm|11536", None);
        assert_eq!(depth.status, DepthStatus::Unavailable);
        assert_ne!(depth.identity.capability_id.as_str(), "option_chain");
        let rows = [lock_nfo_row()];
        let chain = extract_chain_from(Some(KOTAK_NSE_NFO_BOOK_ID), "NIFTY", Some(&rows), None);
        assert_eq!(chain.status, GlanceStatus::Success);
        assert_eq!(chain.identity.capability_id.as_str(), "option_chain");
    }

    #[test]
    fn chain_wire_keeps_unavailable_without_book() {
        let json = serde_json::to_value(extract_chain(None, "btcusdt")).unwrap();
        assert_eq!(json["status"], "unavailable");
        assert!(json["data"].is_null());
        assert!(json.get("honesty").is_none());
        assert_eq!(json["provenance"]["model"], "raw");
    }

    #[test]
    fn success_wire_is_success_not_honesty_lit() {
        let rows = [lock_nfo_row()];
        let json = serde_json::to_value(extract_chain_from(
            Some(KOTAK_NSE_NFO_BOOK_ID),
            "NIFTY",
            Some(&rows),
            None,
        ))
        .unwrap();
        assert_eq!(json["status"], "success");
        assert!(json["data"].is_object());
        assert!(json.get("honesty").is_none());
    }
}
