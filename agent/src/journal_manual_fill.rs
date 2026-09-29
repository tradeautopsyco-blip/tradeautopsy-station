//! Founder manual fill → local recent-trades row + toolbar-capture outbox (broker miss lane).

use crate::broker::BrokerFill;
use chrono::{TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const JOURNAL_FILL_SOURCE_MANUAL: &str = "manual";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManualFillAcceptBody {
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub price: f64,
    pub filled_at_ms: i64,
    #[serde(default)]
    pub pre_trade_declaration_id: Option<String>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    /// Console `trades.id` when the founder picked “Link to today’s trade”.
    /// Absent → pending escrow (`explicitPending: true`, `tradeId: null`).
    #[serde(default)]
    pub console_trade_id: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ManualFillValidationError {
    Message(String),
}

impl ManualFillValidationError {
    pub(crate) fn message(self) -> String {
        match self {
            Self::Message(m) => m,
        }
    }
}

pub fn validate_manual_fill(body: &ManualFillAcceptBody) -> Result<(), ManualFillValidationError> {
    let symbol = body.symbol.trim();
    if symbol.is_empty() || symbol.len() > 64 {
        return Err(ManualFillValidationError::Message(
            "symbol required (max 64)".into(),
        ));
    }
    let side = normalize_side(&body.side)
        .ok_or_else(|| ManualFillValidationError::Message("side must be BUY or SELL".into()))?;
    if !body.quantity.is_finite() || body.quantity <= 0.0 {
        return Err(ManualFillValidationError::Message(
            "quantity must be positive".into(),
        ));
    }
    if !body.price.is_finite() || body.price <= 0.0 {
        return Err(ManualFillValidationError::Message(
            "price must be positive".into(),
        ));
    }
    if body.filled_at_ms <= 0 {
        return Err(ManualFillValidationError::Message(
            "filledAtMs must be positive".into(),
        ));
    }
    if let Some(id) = body.pre_trade_declaration_id.as_deref() {
        let t = id.trim();
        if !t.is_empty() && uuid::Uuid::parse_str(t).is_err() {
            return Err(ManualFillValidationError::Message(
                "preTradeDeclarationId must be UUID".into(),
            ));
        }
    }
    if body.idempotency_key.as_ref().is_some_and(|s| s.len() > 256) {
        return Err(ManualFillValidationError::Message(
            "idempotencyKey too long".into(),
        ));
    }
    if let Some(id) = body.console_trade_id.as_deref() {
        let t = id.trim();
        if !t.is_empty() && uuid::Uuid::parse_str(t).is_err() {
            return Err(ManualFillValidationError::Message(
                "consoleTradeId must be a Console trades UUID".into(),
            ));
        }
    }
    let _ = side;
    Ok(())
}

/// Broker-miss link: pending unless the caller passed a Console `trades.id`.
/// Never uses the Station-local fill id.
pub struct ManualFillLink {
    pub trade_id: Option<String>,
    pub explicit_pending: bool,
}

pub fn manual_fill_link_strategy(console_trade_id: Option<&str>) -> ManualFillLink {
    match console_trade_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(id) => ManualFillLink {
            trade_id: Some(id.to_string()),
            explicit_pending: false,
        },
        None => ManualFillLink {
            trade_id: None,
            explicit_pending: true,
        },
    }
}

fn normalize_side(raw: &str) -> Option<&'static str> {
    match raw.trim().to_uppercase().as_str() {
        "BUY" => Some("BUY"),
        "SELL" => Some("SELL"),
        _ => None,
    }
}

pub struct ManualFillPersisted {
    pub trade_id: String,
    pub fill_id: String,
    pub broker_fill: BrokerFill,
}

pub fn persist_manual_fill(
    body: &ManualFillAcceptBody,
) -> Result<ManualFillPersisted, ManualFillValidationError> {
    validate_manual_fill(body)?;
    let side = normalize_side(&body.side).expect("validated");
    let trade_id = uuid::Uuid::new_v4().to_string();
    let fill_id = format!("manual-{trade_id}");
    let filled_at = Utc
        .timestamp_millis_opt(body.filled_at_ms)
        .single()
        .ok_or_else(|| ManualFillValidationError::Message("filledAtMs out of range".into()))?;
    let broker_fill = BrokerFill {
        fill_id: fill_id.clone(),
        trade_id: trade_id.clone(),
        symbol: body.symbol.trim().to_uppercase(),
        side: side.to_string(),
        qty: body.quantity,
        price: body.price,
        filled_at,
        broker: JOURNAL_FILL_SOURCE_MANUAL.to_string(),
        fee_amount: None,
        fee_asset: None,
        currency: None,
        product: None,
        exchange_segment: None,
        instrument_type: None,
        lot: None,
    };
    Ok(ManualFillPersisted {
        trade_id,
        fill_id,
        broker_fill,
    })
}

/// Human + machine journal draft (Console persists `draft_text` as-is).
pub fn build_manual_fill_draft_text(
    symbol: &str,
    side: &str,
    quantity: f64,
    price: f64,
    filled_at_ms: i64,
    trade_id: &str,
    declaration_id: Option<&str>,
) -> String {
    let human = format!(
        "manual fill · {symbol} {side} {} @ {price:.2}",
        format_manual_qty(quantity),
        symbol = symbol.trim(),
        side = side.trim().to_uppercase(),
        price = price
    );
    let mut payload = json!({
        "journalFillSource": JOURNAL_FILL_SOURCE_MANUAL,
        "v": 1,
        "symbol": symbol.trim().to_uppercase(),
        "side": side.trim().to_uppercase(),
        "quantity": quantity,
        "price": price,
        "filledAtMs": filled_at_ms,
        "stationFillId": trade_id,
    });
    if let Some(d) = declaration_id.map(str::trim).filter(|s| !s.is_empty()) {
        payload["declarationId"] = json!(d);
    }
    format!("{human}\n{}", payload)
}

fn format_manual_qty(q: f64) -> String {
    if (q - q.round()).abs() < 1e-9 {
        format!("{}", q.round() as i64)
    } else {
        format!("{q:.4}")
    }
}

pub fn build_manual_fill_capture_body(
    persisted: &ManualFillPersisted,
    body: &ManualFillAcceptBody,
) -> Value {
    let side = normalize_side(&body.side).expect("validated");
    let idempotency_key = body
        .idempotency_key
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| format!("manual-fill-{}", ulid::Ulid::new()));
    let declaration_id = body
        .pre_trade_declaration_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let draft_text = build_manual_fill_draft_text(
        &body.symbol,
        side,
        body.quantity,
        body.price,
        body.filled_at_ms,
        &persisted.trade_id,
        declaration_id,
    );
    let link = manual_fill_link_strategy(body.console_trade_id.as_deref());
    let trade_id_json = match &link.trade_id {
        Some(id) => json!(id),
        None => Value::Null,
    };
    let mut capture = json!({
        "draftText": draft_text,
        "tradeId": trade_id_json,
        "explicitPending": link.explicit_pending,
        "idempotencyKey": idempotency_key,
        "journalFillSource": JOURNAL_FILL_SOURCE_MANUAL,
    });
    if let Some(d) = declaration_id {
        capture["preTradeDeclarationId"] = json!(d);
    }
    capture
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_labels_manual_in_json_line() {
        let text = build_manual_fill_draft_text(
            "PNB",
            "BUY",
            100.0,
            95.5,
            1_757_000_000_000,
            "11111111-2222-4333-8444-555555555555",
            Some("0491f631-78d1-496b-b191-4ade1dacb0df"),
        );
        assert!(text.starts_with("manual fill · PNB BUY 100 @ 95.50"));
        assert!(text.contains("journalFillSource"));
        assert!(text.contains("manual"));
        assert!(text.contains("stationFillId"));
        assert!(text.contains("0491f631-78d1-496b-b191-4ade1dacb0df"));
    }

    fn sample_body(
        console_trade_id: Option<&str>,
        declaration: Option<&str>,
    ) -> ManualFillAcceptBody {
        ManualFillAcceptBody {
            symbol: "PNB".into(),
            side: "buy".into(),
            quantity: 100.0,
            price: 95.5,
            filled_at_ms: 1_757_000_000_000,
            pre_trade_declaration_id: declaration.map(str::to_string),
            idempotency_key: Some("manual-fill-link-1".into()),
            console_trade_id: console_trade_id.map(str::to_string),
        }
    }

    #[test]
    fn pending_link_omits_local_fill_id() {
        let decl = "0491f631-78d1-496b-b191-4ade1dacb0df";
        let body = sample_body(None, Some(decl));
        let persisted = persist_manual_fill(&body).expect("persist");
        let cap = build_manual_fill_capture_body(&persisted, &body);
        assert_eq!(cap["explicitPending"], true);
        assert!(cap["tradeId"].is_null());
        assert_ne!(cap["tradeId"].as_str(), Some(persisted.trade_id.as_str()));
        assert_eq!(cap["preTradeDeclarationId"], decl);
        assert!(cap["draftText"].as_str().unwrap().contains(decl));
        assert!(!cap["draftText"].as_str().unwrap().contains("\"tradeId\""));
    }

    #[test]
    fn linked_console_trade_sends_that_uuid() {
        let console = "bbbbbbbb-bbbb-4ccc-8ddd-eeeeeeeeeeee";
        let body = sample_body(Some(console), None);
        let persisted = persist_manual_fill(&body).expect("persist");
        let cap = build_manual_fill_capture_body(&persisted, &body);
        assert_eq!(cap["explicitPending"], false);
        assert_eq!(cap["tradeId"], console);
        assert_ne!(cap["tradeId"].as_str(), Some(persisted.trade_id.as_str()));
        assert!(cap.get("preTradeDeclarationId").is_none());
    }

    #[test]
    fn local_fill_uuid_is_not_a_console_trade_id() {
        let body = sample_body(None, None);
        let persisted = persist_manual_fill(&body).expect("persist");
        assert!(uuid::Uuid::parse_str(&persisted.trade_id).is_ok());
        let cap = build_manual_fill_capture_body(&persisted, &body);
        assert!(cap["tradeId"].is_null());
        assert_eq!(cap["explicitPending"], true);
    }
}
