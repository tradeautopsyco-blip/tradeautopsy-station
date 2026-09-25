//! M1 cited-PnL envelope — shared contract for Console consume (`lib/station/m1-envelope.ts`).
//!
//! Signal type stays `station_cited_pnl`. Payload `v: 2` adds `cite_kind`.
//! DualNoBlend: one settlement currency per envelope; no FX blend keys.

use crate::coinm_realized_pnl::CoinmIncomeRow;
use crate::fx_cds_realized_pnl::FxCdsRoundTrip;
use crate::inr_cash_wac::InrCashRoundTrip;
use crate::mcx_realized_pnl::McxRoundTrip;
use crate::money_matrix::{owner_for_book, MoneyOwner};
use crate::nfo_realized_pnl::NfoRoundTrip;
use crate::round_trip_engine::RoundTrip;
use crate::usdm_realized_pnl::UsdmIncomeRow;
use chrono::{TimeZone, Utc};
use serde_json::{json, Value};

pub const ENVELOPE_V2: i64 = 2;
pub const ENVELOPE_V1: i64 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CiteKind {
    InrCashWac,
    NfoRealizedInr,
    FxCdsRealizedInr,
    McxFutureRealizedInr,
    CryptoSpotWacUsd,
    CryptoIncomeRealizedUsd,
}

impl CiteKind {
    pub fn as_str(self) -> &'static str {
        match self {
            CiteKind::InrCashWac => "inr_cash_wac",
            CiteKind::NfoRealizedInr => "nfo_realized_inr",
            CiteKind::FxCdsRealizedInr => "fx_cds_realized_inr",
            CiteKind::McxFutureRealizedInr => "mcx_future_realized_inr",
            CiteKind::CryptoSpotWacUsd => "crypto_spot_wac_usd",
            CiteKind::CryptoIncomeRealizedUsd => "crypto_income_realized_usd",
        }
    }

    pub fn settlement_currency(self) -> &'static str {
        match self {
            CiteKind::InrCashWac
            | CiteKind::NfoRealizedInr
            | CiteKind::FxCdsRealizedInr
            | CiteKind::McxFutureRealizedInr => "INR",
            CiteKind::CryptoSpotWacUsd | CiteKind::CryptoIncomeRealizedUsd => "USD",
        }
    }
}

pub fn cite_kind_for_book(book_id: &str) -> Option<CiteKind> {
    let owner = owner_for_book(book_id)?;
    match owner {
        MoneyOwner::ExplicitNone(_) => None,
        MoneyOwner::Engine(path) => cite_kind_for_owner(path),
    }
}

fn cite_kind_for_owner(owner_path: &str) -> Option<CiteKind> {
    match owner_path {
        crate::inr_cash_wac::OWNER_PATH => Some(CiteKind::InrCashWac),
        crate::nfo_realized_pnl::OWNER_PATH => Some(CiteKind::NfoRealizedInr),
        crate::fx_cds_realized_pnl::OWNER_PATH => Some(CiteKind::FxCdsRealizedInr),
        crate::mcx_realized_pnl::OWNER_PATH => Some(CiteKind::McxFutureRealizedInr),
        "agent/src/round_trip_engine.rs" => Some(CiteKind::CryptoSpotWacUsd),
        crate::usdm_realized_pnl::OWNER_PATH => Some(CiteKind::CryptoIncomeRealizedUsd),
        crate::coinm_realized_pnl::OWNER_PATH => Some(CiteKind::CryptoIncomeRealizedUsd),
        _ => None,
    }
}

fn envelope_shell(
    cite_kind: CiteKind,
    book_id: &str,
    owner: &str,
    trips: Vec<Value>,
) -> Value {
    json!({
        "v": ENVELOPE_V2,
        "cite_kind": cite_kind.as_str(),
        "book_id": book_id,
        "currency": cite_kind.settlement_currency(),
        "owner": owner,
        "source": "station",
        "trips": trips,
    })
}

/// INR cash WAC batch (all `*-nse-bse-cash` books share owner file).
pub fn cited_inr_cash_payload(book_id: &str, owner: &str, trips: &[InrCashRoundTrip]) -> Value {
    let cited: Vec<Value> = trips
        .iter()
        .filter(|t| t.realized_pnl_inr.is_some() && !t.unknown_basis)
        .map(|t| {
            json!({
                "trip_id": inr_cash_trip_id(t),
                "symbol": t.symbol,
                "qty": t.qty,
                "avg_entry_price": t.avg_entry_price,
                "avg_exit_price": t.avg_exit_price,
                "opened_at": t.opened_at.to_rfc3339(),
                "closed_at": t.closed_at.to_rfc3339(),
                "realized_pnl_inr": t.realized_pnl_inr,
                "fees_inr": t.fees_inr,
                "product": t.product,
                "book_id": book_id,
                "currency": "INR",
                "owner": owner,
                "source": "station",
            })
        })
        .collect();
    envelope_shell(CiteKind::InrCashWac, book_id, owner, cited)
}

fn inr_cash_trip_id(t: &InrCashRoundTrip) -> String {
    format!(
        "inr-{}-{}-{}",
        t.symbol,
        t.closed_at.timestamp_millis(),
        (t.qty * 1e6).round() as i64
    )
}

/// NFO `(exit−entry)×qty×lot` batch — book id is broker-specific (`dhan-nse-nfo`, etc.).
pub fn cited_nfo_payload(book_id: &str, owner: &str, trips: &[NfoRoundTrip]) -> Value {
    let cited: Vec<Value> = trips
        .iter()
        .filter(|t| t.realized_pnl_inr.is_some() && !t.unknown_basis)
        .map(|t| {
            json!({
                "trip_id": nfo_trip_id(book_id, t),
                "symbol": t.symbol,
                "qty": t.qty,
                "lot": t.lot,
                "avg_entry_price": t.avg_entry_price,
                "avg_exit_price": t.avg_exit_price,
                "opened_at": t.opened_at.to_rfc3339(),
                "closed_at": t.closed_at.to_rfc3339(),
                "realized_pnl_inr": t.realized_pnl_inr,
                "product": t.product,
                "book_id": book_id,
                "currency": "INR",
                "owner": owner,
                "source": "station",
            })
        })
        .collect();
    envelope_shell(CiteKind::NfoRealizedInr, book_id, owner, cited)
}

fn nfo_trip_id(book_id: &str, t: &NfoRoundTrip) -> String {
    format!(
        "nfo-{}-{}-{}-{}",
        book_id,
        t.symbol,
        t.closed_at.timestamp_millis(),
        (t.qty * 1e6).round() as i64
    )
}

fn derivative_inr_trip_json(
    prefix: &str,
    book_id: &str,
    owner: &str,
    symbol: &str,
    qty: f64,
    lot: i64,
    entry: f64,
    exit: f64,
    opened_at: chrono::DateTime<Utc>,
    closed_at: chrono::DateTime<Utc>,
    pnl: f64,
    product: &str,
) -> Value {
    json!({
        "trip_id": format!(
            "{}-{}-{}-{}-{}",
            prefix,
            book_id,
            symbol,
            closed_at.timestamp_millis(),
            (qty * 1e6).round() as i64
        ),
        "symbol": symbol,
        "qty": qty,
        "lot": lot,
        "avg_entry_price": entry,
        "avg_exit_price": exit,
        "opened_at": opened_at.to_rfc3339(),
        "closed_at": closed_at.to_rfc3339(),
        "realized_pnl_inr": pnl,
        "product": product,
        "book_id": book_id,
        "currency": "INR",
        "owner": owner,
        "source": "station",
    })
}

pub fn cited_fx_cds_payload(book_id: &str, owner: &str, trips: &[FxCdsRoundTrip]) -> Value {
    let cited: Vec<Value> = trips
        .iter()
        .filter(|t| t.realized_pnl_inr.is_some() && !t.unknown_basis)
        .map(|t| {
            derivative_inr_trip_json(
                "cds",
                book_id,
                owner,
                &t.symbol,
                t.qty,
                t.lot,
                t.avg_entry_price,
                t.avg_exit_price,
                t.opened_at,
                t.closed_at,
                t.realized_pnl_inr.unwrap(),
                &t.product,
            )
        })
        .collect();
    envelope_shell(CiteKind::FxCdsRealizedInr, book_id, owner, cited)
}

pub fn cited_mcx_payload(book_id: &str, owner: &str, trips: &[McxRoundTrip]) -> Value {
    let cited: Vec<Value> = trips
        .iter()
        .filter(|t| t.realized_pnl_inr.is_some() && !t.unknown_basis)
        .map(|t| {
            derivative_inr_trip_json(
                "mcx",
                book_id,
                owner,
                &t.symbol,
                t.qty,
                t.lot,
                t.avg_entry_price,
                t.avg_exit_price,
                t.opened_at,
                t.closed_at,
                t.realized_pnl_inr.unwrap(),
                &t.product,
            )
        })
        .collect();
    envelope_shell(CiteKind::McxFutureRealizedInr, book_id, owner, cited)
}

const CRYPTO_SPOT_OWNER: &str = "agent/src/round_trip_engine.rs";

/// COM USD spot WAC round trips (`binance-com-spot`, `bybit-com-spot`, …).
pub fn cited_crypto_spot_payload(book_id: &str, trips: &[RoundTrip]) -> Value {
    let cited: Vec<Value> = trips
        .iter()
        .filter(|t| t.realized_pnl_usd.is_some() && !t.unknown_basis && !t.quote_not_usd)
        .map(|t| {
            json!({
                "trip_id": format!(
                    "spot-{}-{}-{}",
                    book_id,
                    t.symbol,
                    t.closed_at.timestamp_millis()
                ),
                "symbol": t.symbol,
                "qty": t.qty,
                "avg_entry_price": t.avg_entry_price,
                "avg_exit_price": t.avg_exit_price,
                "opened_at": t.opened_at.to_rfc3339(),
                "closed_at": t.closed_at.to_rfc3339(),
                "realized_pnl_usd": t.realized_pnl_usd,
                "fees_usd": t.fees_usd,
                "book_id": book_id,
                "currency": "USD",
                "owner": CRYPTO_SPOT_OWNER,
                "source": "station",
            })
        })
        .collect();
    envelope_shell(CiteKind::CryptoSpotWacUsd, book_id, CRYPTO_SPOT_OWNER, cited)
}

fn income_trip_json(
    book_id: &str,
    owner: &str,
    prefix: &str,
    symbol: &str,
    closed_at_ms: i64,
    realized_pnl_usd: f64,
) -> Value {
    let closed_at = Utc
        .timestamp_millis_opt(closed_at_ms)
        .single()
        .unwrap_or_else(Utc::now);
    json!({
        "trip_id": format!("{}-{}-{}-{}", prefix, book_id, symbol, closed_at_ms),
        "symbol": symbol,
        "closed_at": closed_at.to_rfc3339(),
        "realized_pnl_usd": realized_pnl_usd,
        "book_id": book_id,
        "currency": "USD",
        "owner": owner,
        "source": "station",
        "income_type": "REALIZED_PNL",
    })
}

fn filter_income_rows<T, F>(rows: &[T], mut row_ok: F) -> Vec<Value>
where
    F: FnMut(&T) -> Option<Value>,
{
    rows.iter().filter_map(|r| row_ok(r)).collect()
}

pub fn cited_usdm_income_payload(book_id: &str, rows: &[UsdmIncomeRow]) -> Value {
    let cited = filter_income_rows(rows, |row| {
        if row.income_type.as_deref() != Some("REALIZED_PNL") {
            return None;
        }
        let raw = row.income.as_deref()?;
        let amount: f64 = raw.parse().ok()?;
        if !amount.is_finite() {
            return None;
        }
        let symbol = row.symbol.clone().unwrap_or_else(|| "USDM".into());
        let ts = row.time.unwrap_or(0);
        Some(income_trip_json(
            book_id,
            crate::usdm_realized_pnl::OWNER_PATH,
            "usdm",
            &symbol,
            ts,
            amount,
        ))
    });
    envelope_shell(
        CiteKind::CryptoIncomeRealizedUsd,
        book_id,
        crate::usdm_realized_pnl::OWNER_PATH,
        cited,
    )
}

pub fn cited_coinm_income_payload(book_id: &str, rows: &[CoinmIncomeRow]) -> Value {
    let cited = filter_income_rows(rows, |row| {
        if row.income_type.as_deref() != Some("REALIZED_PNL") {
            return None;
        }
        let raw = row.income.as_deref()?;
        let amount: f64 = raw.parse().ok()?;
        if !amount.is_finite() {
            return None;
        }
        let symbol = row.symbol.clone().unwrap_or_else(|| "COINM".into());
        let ts = row.time.unwrap_or(0);
        Some(income_trip_json(
            book_id,
            crate::coinm_realized_pnl::OWNER_PATH,
            "coinm",
            &symbol,
            ts,
            amount,
        ))
    });
    envelope_shell(
        CiteKind::CryptoIncomeRealizedUsd,
        book_id,
        crate::coinm_realized_pnl::OWNER_PATH,
        cited,
    )
}

pub fn envelope_has_cited_trips(payload: &Value) -> bool {
    payload
        .get("trips")
        .and_then(|t| t.as_array())
        .is_some_and(|a| !a.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use crate::inr_cash_wac::{InrCashRoundTrip, OWNER_PATH as CASH_OWNER};
    use crate::nfo_realized_pnl::{NfoRoundTrip, OWNER_PATH as NFO_OWNER};

    #[test]
    fn v2_inr_cash_dual_no_blend() {
        let trip = InrCashRoundTrip {
            symbol: "RELIANCE".into(),
            opened_at: Utc.with_ymd_and_hms(2026, 9, 21, 4, 0, 0).unwrap(),
            closed_at: Utc.with_ymd_and_hms(2026, 9, 21, 5, 0, 0).unwrap(),
            avg_entry_price: 105.0,
            avg_exit_price: 120.0,
            qty: 2.0,
            realized_pnl_inr: Some(30.0),
            fees_inr: None,
            unknown_basis: false,
            product: "CNC".into(),
        };
        let payload = cited_inr_cash_payload("kotak-nse-bse-cash", CASH_OWNER, &[trip]);
        assert_eq!(payload["v"], ENVELOPE_V2);
        assert_eq!(payload["cite_kind"], "inr_cash_wac");
        let text = payload.to_string();
        assert!(!text.contains("exchange_rate"));
        assert!(!text.contains("realized_pnl_usd"));
    }

    #[test]
    fn v2_nfo_includes_lot() {
        let trip = NfoRoundTrip {
            symbol: "NIFTY24SEP25000CE".into(),
            opened_at: Utc.with_ymd_and_hms(2026, 9, 21, 4, 0, 0).unwrap(),
            closed_at: Utc.with_ymd_and_hms(2026, 9, 21, 5, 0, 0).unwrap(),
            avg_entry_price: 10.0,
            avg_exit_price: 11.0,
            qty: 2.0,
            lot: 65,
            realized_pnl_inr: Some(130.0),
            product: "NRML".into(),
            unknown_basis: false,
        };
        let payload = cited_nfo_payload("dhan-nse-nfo", NFO_OWNER, &[trip]);
        assert_eq!(payload["cite_kind"], "nfo_realized_inr");
        assert_eq!(payload["trips"][0]["lot"], 65);
        assert_eq!(payload["trips"][0]["realized_pnl_inr"], 130.0);
    }

    #[test]
    fn cite_kind_maps_shipping_books() {
        assert_eq!(
            cite_kind_for_book("binance-com-spot"),
            Some(CiteKind::CryptoSpotWacUsd)
        );
        assert_eq!(cite_kind_for_book("binance-com-options"), None);
    }

    #[test]
    fn crypto_spot_payload_usd_only() {
        let trip = RoundTrip {
            symbol: "BTCUSDT".into(),
            opened_at: Utc.with_ymd_and_hms(2026, 9, 21, 4, 0, 0).unwrap(),
            closed_at: Utc.with_ymd_and_hms(2026, 9, 21, 5, 0, 0).unwrap(),
            avg_entry_price: 100.0,
            avg_exit_price: 110.0,
            qty: 1.0,
            realized_pnl_usd: Some(10.0),
            fees_usd: Some(0.1),
            unknown_basis: false,
            fee_unhandled: false,
            quote_not_usd: false,
        };
        let payload = cited_crypto_spot_payload("binance-com-spot", &[trip]);
        assert_eq!(payload["cite_kind"], "crypto_spot_wac_usd");
        assert!(!payload.to_string().contains("realized_pnl_inr"));
    }
}
