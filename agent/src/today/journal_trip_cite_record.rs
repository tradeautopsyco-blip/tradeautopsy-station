//! Record Station-owned Journal trip cites from Console test inject responses only.
//!
//! Does not flip declaration `status` locally and does not invent `matched` in sqlite.
//! Production `FILL_MATCHED` on real fill ingest remains a Console ticket (other repo).

use chrono::{DateTime, Local, NaiveDate, Utc};
use serde_json::Value;

use super::store::TodayStore;

/// When Console returns `test_only` + `status: matched`, persist optional `trip_cite` for Wave J.
pub fn try_record_console_test_fill_matched_cite(
    store: &TodayStore,
    request_body: &Value,
    response_body: &Value,
) -> bool {
    if !is_console_test_matched(response_body) {
        return false;
    }
    let Some(declaration_id) = declaration_id_from_request(request_body) else {
        return false;
    };
    let Some((net, currency)) = parse_trip_cite(response_body) else {
        return false;
    };
    let close_date = local_close_date_from_response(response_body);
    store
        .upsert_journal_trip_cite(&declaration_id, net, &currency, close_date)
        .is_ok()
}

fn is_console_test_matched(response: &Value) -> bool {
    if response.get("test_only").and_then(|v| v.as_bool()) != Some(true) {
        return false;
    }
    matches!(
        response.get("status").and_then(|v| v.as_str()),
        Some("matched")
    )
}

fn declaration_id_from_request(body: &Value) -> Option<String> {
    body.get("declaration_id")
        .or_else(|| body.get("declarationId"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn parse_trip_cite(response: &Value) -> Option<(f64, String)> {
    for key in ["trip_cite", "tripCite", "journal_trip_cite", "journalTripCite"] {
        if let Some(obj) = response.get(key) {
            if let Some(parsed) = net_currency_from_object(obj) {
                return Some(parsed);
            }
        }
    }
    if let Some(trip) = response.get("trip") {
        if let Some(parsed) = net_currency_from_trip_object(trip) {
            return Some(parsed);
        }
    }
    net_currency_from_trip_object(response)
}

fn net_currency_from_object(obj: &Value) -> Option<(f64, String)> {
    let net = obj
        .get("net")
        .or_else(|| obj.get("net_pnl"))
        .or_else(|| obj.get("netPnl"))
        .and_then(as_finite_f64)?;
    let currency = obj
        .get("currency")
        .or_else(|| obj.get("quote_currency"))
        .or_else(|| obj.get("quoteCurrency"))
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_ascii_uppercase())
        .filter(|s| !s.is_empty())?;
    Some((net, currency))
}

fn net_currency_from_trip_object(obj: &Value) -> Option<(f64, String)> {
    if let Some(parsed) = net_currency_from_object(obj) {
        return Some(parsed);
    }
    if let Some(net) = obj
        .get("net_pnl_usd")
        .or_else(|| obj.get("netPnlUsd"))
        .or_else(|| obj.get("realized_pnl_usd"))
        .or_else(|| obj.get("realizedPnlUsd"))
        .and_then(as_finite_f64)
    {
        return Some((net, "USD".to_string()));
    }
    if let Some(net) = obj
        .get("net_pnl_inr")
        .or_else(|| obj.get("netPnlInr"))
        .or_else(|| obj.get("realized_pnl_inr"))
        .or_else(|| obj.get("realizedPnlInr"))
        .and_then(as_finite_f64)
    {
        return Some((net, "INR".to_string()));
    }
    None
}

fn as_finite_f64(v: &Value) -> Option<f64> {
    v.as_f64().filter(|n| n.is_finite())
}

fn local_close_date_from_response(response: &Value) -> NaiveDate {
    let ms = response
        .get("matched_at_ms")
        .or_else(|| response.get("matchedAtMs"))
        .and_then(|v| v.as_i64());
    if let Some(ms) = ms {
        let secs = ms / 1000;
        let nsecs = ((ms % 1000) * 1_000_000) as u32;
        if let Some(dt) = DateTime::<Utc>::from_timestamp(secs, nsecs) {
            return dt.with_timezone(&Local).date_naive();
        }
    }
    Local::now().date_naive()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn skips_non_test_only_response() {
        let store = temp_store();
        let req = json!({ "declaration_id": "d-1" });
        let resp = json!({ "status": "matched", "trip_cite": { "net": 1.0, "currency": "USD" } });
        assert!(!try_record_console_test_fill_matched_cite(&store, &req, &resp));
    }

    #[test]
    fn records_trip_cite_from_console_test_inject() {
        let store = temp_store();
        let req = json!({ "declaration_id": "d-abc" });
        let resp = json!({
            "test_only": true,
            "status": "matched",
            "matched_at_ms": 1_704_067_200_000i64,
            "trip_cite": { "net": 12.5, "currency": "usd" }
        });
        assert!(try_record_console_test_fill_matched_cite(&store, &req, &resp));
        let rows = store
            .fetch_journal_trip_cites_between(
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2030, 1, 1).unwrap(),
            )
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].declaration_id, "d-abc");
        assert_eq!(rows[0].net, 12.5);
        assert_eq!(rows[0].currency, "USD");
    }

    #[test]
    fn matched_without_trip_cite_does_not_write_sqlite() {
        let store = temp_store();
        let req = json!({ "declaration_id": "d-2" });
        let resp = json!({
            "test_only": true,
            "status": "matched",
            "fidelity": { "ok": true }
        });
        assert!(!try_record_console_test_fill_matched_cite(&store, &req, &resp));
        let rows = store
            .fetch_journal_trip_cites_between(
                NaiveDate::from_ymd_opt(2024, 1, 1).unwrap(),
                NaiveDate::from_ymd_opt(2030, 1, 1).unwrap(),
            )
            .unwrap();
        assert!(rows.is_empty());
    }

    fn temp_store() -> TodayStore {
        let dir = std::env::temp_dir().join(format!(
            "rta-journal-trip-cite-record-{}-{}",
            std::process::id(),
            ulid::Ulid::new()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        TodayStore::open(&dir.join("today.db")).unwrap()
    }
}
