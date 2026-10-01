//! In-memory LiveBook — Station live-read authority after one hosted snapshot.
//!
//! N1: declare / cancel / protective / fill **apply** here. Console is write-behind.
//! This module does not compute PnL, charges, or notional.

use chrono::{SecondsFormat, Utc};
use serde_json::{json, Map, Value};
use std::sync::Mutex;
use std::time::Instant;

#[derive(Default)]
struct Inner {
    value: Option<Value>,
    hydrated_at: Option<Instant>,
}

#[derive(Default)]
pub struct LiveBook {
    inner: Mutex<Inner>,
}

/// Events the book can project. Callers must not patch JSON themselves.
#[derive(Debug, Clone)]
pub enum LiveBookEvent {
    Declare {
        local_id: String,
        symbol: String,
        side: String,
        quantity: f64,
        declaration_kind: String,
        stop_loss: Option<f64>,
        target: Option<f64>,
        protective_sl_consent: bool,
        plan_snapshot: Option<Value>,
        book_id: Option<String>,
        ticket_intent: Option<Value>,
    },
    /// After Console 2xx — pending id becomes the archive id (no second arm).
    ReconcileArchive {
        local_id: String,
        server_id: String,
    },
    Cancel {
        declaration_id: String,
    },
    Protective {
        declaration_id: Option<String>,
        stop_loss: Option<f64>,
    },
    /// Join a fill onto pending (matched id) or mark undeclared. Never writes PnL.
    Fill {
        symbol: String,
        side: String,
        qty: f64,
        price: Option<f64>,
        broker: Option<String>,
        filled_at_iso: Option<String>,
    },
    /// Wave 2 — freeze last vs invalidation/target on the declaration snapshot when the session ends.
    CaptureWorkingCondition {
        last: Option<f64>,
        last_status: String,
    },
}

impl LiveBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hydrate(&self, value: Value) {
        let mut g = self.inner.lock().expect("live book");
        g.value = Some(value);
        g.hydrated_at = Some(Instant::now());
    }

    pub fn snapshot(&self) -> Option<Value> {
        self.inner.lock().expect("live book").value.clone()
    }

    pub fn apply(&self, event: LiveBookEvent) {
        let mut g = self.inner.lock().expect("live book");
        let book = ensure_book(&mut g);
        apply_event(book, event);
    }
}

impl LiveBookEvent {
    pub fn declare_from_body(body: &Value, local_id: String) -> Self {
        let payload = body.get("declaration_payload");
        let consent = payload
            .and_then(|p| p.get("protective_sl_consent"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let s1 = payload.and_then(|p| p.get("s1"));
        let plan_snapshot = s1.map(|s1| plan_snapshot_from_s1(s1, body));
        let target = json_f64(body.get("target_price")).or_else(|| json_f64(body.get("target")));
        let book_id = json_string(body.get("book_id")).filter(|id| !id.is_empty());
        let ticket_intent = payload
            .and_then(|p| p.get("ticket"))
            .cloned()
            .filter(|v| v.is_object());
        Self::Declare {
            local_id,
            symbol: json_string(body.get("symbol")).unwrap_or_default(),
            side: json_string(body.get("side"))
                .unwrap_or_default()
                .to_ascii_uppercase(),
            quantity: json_f64(body.get("quantity")).unwrap_or(0.0),
            declaration_kind: json_string(body.get("declaration_kind"))
                .unwrap_or_else(|| "intraday".into()),
            stop_loss: json_f64(body.get("stop_loss")),
            target,
            protective_sl_consent: consent,
            plan_snapshot,
            book_id,
            ticket_intent,
        }
    }
}

fn ensure_book(inner: &mut Inner) -> &mut Value {
    if inner.value.is_none() {
        inner.value = Some(empty_book());
        inner.hydrated_at = Some(Instant::now());
    }
    inner.value.as_mut().expect("live book value")
}

fn empty_book() -> Value {
    json!({
        "schemaVersion": 1,
        "barFeaturesActive": true,
        "notch": {
            "plan_state": "",
            "sync_state": "GREEN",
            "pending_declaration": Value::Null
        }
    })
}

fn apply_event(book: &mut Value, event: LiveBookEvent) {
    match event {
        LiveBookEvent::Declare {
            local_id,
            symbol,
            side,
            quantity,
            declaration_kind,
            stop_loss,
            target,
            protective_sl_consent,
            plan_snapshot,
            book_id,
            ticket_intent,
        } => {
            let created = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
            let mut pending = json!({
                "id": local_id,
                "status": "PENDING",
                "created_at": created,
                "symbol": symbol,
                "side": side,
                "quantity": quantity,
                "declaration_kind": declaration_kind,
                "protective_sl_consent": protective_sl_consent,
                "stop_loss": stop_loss,
                "target": target,
            });
            if let Some(book) = book_id.filter(|id| !id.is_empty()) {
                pending
                    .as_object_mut()
                    .expect("pending object")
                    .insert("book_id".into(), json!(book));
            }
            if let Some(ps) = plan_snapshot {
                pending
                    .as_object_mut()
                    .expect("pending object")
                    .insert("plan_snapshot".into(), ps);
            }
            if let Some(ticket) = ticket_intent {
                pending
                    .as_object_mut()
                    .expect("pending object")
                    .insert("ticket".into(), ticket);
            }
            notch_map(book).insert("pending_declaration".into(), pending);
        }
        LiveBookEvent::ReconcileArchive {
            local_id,
            server_id,
        } => {
            if server_id.is_empty() {
                return;
            }
            if pending_id(book).as_deref() == Some(local_id.as_str()) {
                if let Some(obj) = pending_map(book) {
                    obj.insert("id".into(), json!(server_id));
                }
            }
        }
        LiveBookEvent::Cancel { declaration_id } => {
            if pending_id(book).as_deref() == Some(declaration_id.as_str()) {
                notch_map(book).insert("pending_declaration".into(), Value::Null);
            }
        }
        LiveBookEvent::Protective {
            declaration_id,
            stop_loss,
        } => {
            let matches = match declaration_id.as_deref() {
                None | Some("") => pending_id(book).is_some(),
                Some(id) => pending_id(book).as_deref() == Some(id),
            };
            if !matches {
                return;
            }
            if let Some(sl) = stop_loss {
                if let Some(obj) = pending_map(book) {
                    obj.insert("stop_loss".into(), json!(sl));
                }
            }
        }
        LiveBookEvent::Fill {
            symbol,
            side,
            qty,
            price,
            broker,
            filled_at_iso,
        } => {
            let pending_sym = pending_string(book, "symbol");
            let pending_side = pending_string(book, "side");
            let symbol_ok = pending_sym
                .as_ref()
                .is_some_and(|s| s.eq_ignore_ascii_case(&symbol));
            let side_ok = pending_side
                .as_ref()
                .is_some_and(|s| s.eq_ignore_ascii_case(&side));
            if symbol_ok && side_ok {
                let prev_qty = pending_f64(book, "filled_qty").unwrap_or(0.0);
                let prev_avg = pending_f64(book, "avg_fill");
                let new_qty = prev_qty + qty;
                let new_avg = match (prev_avg, price, qty > 0.0, new_qty > 0.0) {
                    (_, Some(px), true, true) if prev_qty <= 0.0 => Some(px),
                    (Some(avg), Some(px), true, true) => {
                        Some((avg * prev_qty + px * qty) / new_qty)
                    }
                    (avg, None, _, _) => avg,
                    (_, Some(px), _, _) => Some(px),
                };
                if let Some(obj) = pending_map(book) {
                    obj.insert("fill_symbol".into(), json!(symbol));
                    obj.insert("fill_side".into(), json!(side.to_ascii_uppercase()));
                    obj.insert("filled_qty".into(), json!(new_qty));
                    if let Some(avg) = new_avg {
                        obj.insert("avg_fill".into(), json!(avg));
                    }
                }
                if let Some(id) = pending_id(book) {
                    notch_map(book).insert("matched_declaration_id".into(), json!(id));
                }
                return;
            }
            let qty_int = if qty.fract() == 0.0 {
                json!(qty as i64)
            } else {
                json!(qty)
            };
            notch_map(book).insert(
                "undeclared_position".into(),
                json!({
                    "symbol": symbol,
                    "side": side,
                    "quantity": qty_int,
                    "price": price,
                    "filledAtMs": filled_at_iso,
                    "broker": broker,
                }),
            );
        }
        LiveBookEvent::CaptureWorkingCondition { last, last_status } => {
            let Some(obj) = pending_map(book) else {
                return;
            };
            let Some(plan) = obj.get_mut("plan_snapshot").and_then(Value::as_object_mut) else {
                return;
            };
            if plan.contains_key("condition_at_close") {
                return;
            }
            let side_buy = pending_string(book, "side")
                .map(|s| !s.to_ascii_uppercase().contains("SELL"))
                .unwrap_or(true);
            let inv_kind = plan
                .get("invalidation_kind")
                .or_else(|| plan.get("invalidation").and_then(|i| i.get("kind")))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string);
            let inv_price = json_f64(plan.get("invalidation_price"))
                .or_else(|| {
                    plan.get("invalidation")
                        .and_then(|i| json_f64(i.get("price")))
                });
            let target = json_f64(obj.get("target")).or_else(|| json_f64(plan.get("target_price")));
            let status = last_status.trim().to_ascii_lowercase();
            let captured = status != "not_captured"
                && last.is_some_and(|v| v.is_finite() && v > 0.0)
                && working_last_bound(&status);
            let inv_state = if captured {
                working_vs_invalidation(side_buy, last, &status, inv_kind.as_deref(), inv_price)
            } else {
                "not_captured".to_string()
            };
            let tgt_state = if captured {
                working_vs_target(side_buy, last, &status, target)
            } else {
                "not_captured".to_string()
            };
            plan.insert(
                "condition_at_close".into(),
                json!({
                    "last": last.filter(|v| v.is_finite() && *v > 0.0),
                    "last_status": if captured { status } else { "not_captured" },
                    "invalidation_state": inv_state,
                    "target_state": tgt_state,
                }),
            );
        }
    }
}

fn working_last_bound(status: &str) -> bool {
    !matches!(
        status,
        "unavailable" | "unsupported" | "empty" | "quotes_http" | "not_connected"
    )
}

fn working_vs_invalidation(
    side_buy: bool,
    last: Option<f64>,
    status: &str,
    kind: Option<&str>,
    price: Option<f64>,
) -> String {
    let k = kind.unwrap_or("").trim().to_ascii_lowercase();
    if matches!(k.as_str(), "time" | "behaviour" | "behavior" | "context") {
        return "waiting".into();
    }
    let is_price = k == "price" || (k.is_empty() && price.is_some());
    if !is_price {
        return "waiting".into();
    }
    let bound = working_last_bound(status);
    let (Some(last), Some(inv)) = (last.filter(|v| v.is_finite() && *v > 0.0), price.filter(|p| *p > 0.0))
    else {
        return if bound { "waiting".into() } else { "dark".into() };
    };
    if side_buy {
        return if last <= inv { "breached".into() } else { "intact".into() };
    }
    if last >= inv {
        "breached".into()
    } else {
        "intact".into()
    }
}

fn working_vs_target(
    side_buy: bool,
    last: Option<f64>,
    status: &str,
    target: Option<f64>,
) -> String {
    let bound = working_last_bound(status);
    let (Some(last), Some(tgt)) = (
        last.filter(|v| v.is_finite() && *v > 0.0),
        target.filter(|t| *t > 0.0),
    ) else {
        return if bound { "waiting".into() } else { "dark".into() };
    };
    if side_buy {
        return if last >= tgt { "touched".into() } else { "intact".into() };
    }
    if last <= tgt {
        "touched".into()
    } else {
        "intact".into()
    }
}

fn notch_map(book: &mut Value) -> &mut Map<String, Value> {
    let root = book.as_object_mut().expect("live book object");
    let notch = root.entry("notch").or_insert_with(|| json!({}));
    if !notch.is_object() {
        *notch = json!({});
    }
    notch.as_object_mut().expect("notch object")
}

fn pending_map(book: &mut Value) -> Option<&mut Map<String, Value>> {
    notch_map(book)
        .get_mut("pending_declaration")
        .and_then(Value::as_object_mut)
}

fn pending_id(book: &Value) -> Option<String> {
    book.get("notch")
        .and_then(|n| n.get("pending_declaration"))
        .and_then(|p| p.get("id"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn pending_string(book: &Value, key: &str) -> Option<String> {
    book.get("notch")
        .and_then(|n| n.get("pending_declaration"))
        .and_then(|p| p.get(key))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn pending_f64(book: &Value, key: &str) -> Option<f64> {
    json_f64(
        book.get("notch")
            .and_then(|n| n.get("pending_declaration"))
            .and_then(|p| p.get(key)),
    )
}

fn json_string(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn json_f64(v: Option<&Value>) -> Option<f64> {
    let v = v?;
    v.as_f64()
        .or_else(|| v.as_i64().map(|i| i as f64))
        .or_else(|| v.as_u64().map(|i| i as f64))
}

fn plan_snapshot_from_s1(s1: &Value, body: &Value) -> Value {
    let spec = s1.get("invalidation_spec");
    let kind = spec
        .and_then(|s| json_string(s.get("kind")))
        .or_else(|| json_string(s1.get("invalidation_type")))
        .unwrap_or_default();
    let price = spec
        .and_then(|s| json_f64(s.get("price")))
        .or_else(|| json_f64(s1.get("invalidation_price")));
    let line = spec
        .and_then(|s| json_string(s.get("line")))
        .or_else(|| json_string(s1.get("invalidation")))
        .unwrap_or_default();
    let calm = json_f64(s1.get("mood_stress"));
    let confidence = json_f64(s1.get("mood_impulse"));
    let frustration = json_f64(s1.get("mood_frustration"));
    let excitement = json_f64(s1.get("mood_excitement"));
    let target = json_f64(body.get("target_price")).or_else(|| json_f64(body.get("target")));
    json!({
        "setup_label": json_string(s1.get("setup_type")).unwrap_or_default(),
        "intent": json_string(s1.get("intent")).unwrap_or_default(),
        "stance": json_string(s1.get("stance")).unwrap_or_default(),
        "invalidation_line": line,
        "invalidation_kind": kind,
        "invalidation_price": price,
        "calm_scale": calm,
        "confidence_scale": confidence,
        "frustration_scale": frustration,
        "excitement_scale": excitement,
        "product": json_string(s1.get("product")),
        "target_price": target,
        "symbol": json_string(body.get("symbol")).unwrap_or_default(),
        "side": json_string(body.get("side")).unwrap_or_default().to_ascii_uppercase(),
        "quantity": json_f64(body.get("quantity")),
        "stop_loss": json_f64(body.get("stop_loss")),
        "book_id": json_string(body.get("book_id")),
        "entry_price": json_f64(body.get("entry_price")),
        "emotion_in": {
            "calm": calm,
            "confidence": confidence,
            "frustration": frustration,
            "excitement": excitement,
        },
        "invalidation": {
            "kind": kind,
            "price": price,
            "line": line,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending<'a>(book: &'a Value) -> &'a Value {
        &book["notch"]["pending_declaration"]
    }

    #[test]
    fn declare_puts_pending_on_empty_book() {
        let book = LiveBook::new();
        book.apply(LiveBookEvent::Declare {
            local_id: "local-1".into(),
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            quantity: 10.0,
            declaration_kind: "intraday".into(),
            stop_loss: Some(1400.0),
            target: Some(1500.0),
            protective_sl_consent: true,
            plan_snapshot: None,
            book_id: None,
            ticket_intent: None,
        });
        let snap = book.snapshot().expect("book");
        assert_eq!(pending(&snap)["id"], "local-1");
        assert_eq!(pending(&snap)["status"], "PENDING");
        assert_eq!(pending(&snap)["symbol"], "RELIANCE");
        assert_eq!(pending(&snap)["side"], "BUY");
        assert_eq!(pending(&snap)["quantity"], 10.0);
        assert_eq!(pending(&snap)["stop_loss"], 1400.0);
        assert!(snap["notch"].get("unrealized_pnl").is_none());
        assert!(snap["notch"].get("declared_max_loss_inr").is_none());
    }

    #[test]
    fn reconcile_replaces_local_id_not_a_second_arm() {
        let book = LiveBook::new();
        book.apply(LiveBookEvent::Declare {
            local_id: "local-1".into(),
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            quantity: 1.0,
            declaration_kind: "intraday".into(),
            stop_loss: None,
            target: None,
            protective_sl_consent: false,
            plan_snapshot: None,
            book_id: None,
            ticket_intent: None,
        });
        book.apply(LiveBookEvent::ReconcileArchive {
            local_id: "local-1".into(),
            server_id: "server-9".into(),
        });
        let snap = book.snapshot().expect("book");
        assert_eq!(pending(&snap)["id"], "server-9");
        assert_eq!(pending(&snap)["status"], "PENDING");
    }

    #[test]
    fn cancel_clears_matching_pending() {
        let book = LiveBook::new();
        book.apply(LiveBookEvent::Declare {
            local_id: "d1".into(),
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            quantity: 1.0,
            declaration_kind: "intraday".into(),
            stop_loss: None,
            target: None,
            protective_sl_consent: false,
            plan_snapshot: None,
            book_id: None,
            ticket_intent: None,
        });
        book.apply(LiveBookEvent::Cancel {
            declaration_id: "d1".into(),
        });
        let snap = book.snapshot().expect("book");
        assert!(pending(&snap).is_null());
    }

    #[test]
    fn cancel_unknown_id_leaves_pending() {
        let book = LiveBook::new();
        book.apply(LiveBookEvent::Declare {
            local_id: "d1".into(),
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            quantity: 1.0,
            declaration_kind: "intraday".into(),
            stop_loss: None,
            target: None,
            protective_sl_consent: false,
            plan_snapshot: None,
            book_id: None,
            ticket_intent: None,
        });
        book.apply(LiveBookEvent::Cancel {
            declaration_id: "other".into(),
        });
        assert_eq!(pending(&book.snapshot().unwrap())["id"], "d1");
    }

    #[test]
    fn protective_patches_stop_on_current_pending() {
        let book = LiveBook::new();
        book.apply(LiveBookEvent::Declare {
            local_id: "d1".into(),
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            quantity: 1.0,
            declaration_kind: "intraday".into(),
            stop_loss: Some(1400.0),
            target: None,
            protective_sl_consent: true,
            plan_snapshot: None,
            book_id: None,
            ticket_intent: None,
        });
        book.apply(LiveBookEvent::Protective {
            declaration_id: None,
            stop_loss: Some(1390.0),
        });
        assert_eq!(pending(&book.snapshot().unwrap())["stop_loss"], 1390.0);
    }

    #[test]
    fn fill_matching_pending_sets_matched_id_without_pnl() {
        let book = LiveBook::new();
        book.apply(LiveBookEvent::Declare {
            local_id: "d1".into(),
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            quantity: 10.0,
            declaration_kind: "intraday".into(),
            stop_loss: Some(1400.0),
            target: None,
            protective_sl_consent: false,
            plan_snapshot: None,
            book_id: None,
            ticket_intent: None,
        });
        book.apply(LiveBookEvent::Fill {
            symbol: "reliance".into(),
            side: "buy".into(),
            qty: 10.0,
            price: Some(1420.0),
            broker: Some("kotak_neo".into()),
            filled_at_iso: Some("2026-09-19T01:00:00.000Z".into()),
        });
        let snap = book.snapshot().unwrap();
        assert_eq!(snap["notch"]["matched_declaration_id"], "d1");
        assert_eq!(pending(&snap)["status"], "PENDING");
        assert!(snap["notch"].get("unrealized_pnl").is_none());
        assert!(snap["notch"].get("undeclared_position").is_none());
    }

    #[test]
    fn fill_price_accumulates_qty_weighted_avg() {
        let book = LiveBook::new();
        book.apply(LiveBookEvent::Declare {
            local_id: "d1".into(),
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            quantity: 2.0,
            declaration_kind: "intraday".into(),
            stop_loss: Some(1400.0),
            target: Some(1500.0),
            protective_sl_consent: false,
            plan_snapshot: None,
            book_id: None,
            ticket_intent: None,
        });
        book.apply(LiveBookEvent::Fill {
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            qty: 1.0,
            price: Some(100.0),
            broker: None,
            filled_at_iso: None,
        });
        book.apply(LiveBookEvent::Fill {
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            qty: 1.0,
            price: Some(102.0),
            broker: None,
            filled_at_iso: None,
        });
        let snap = book.snapshot().unwrap();
        let p = pending(&snap);
        assert_eq!(p["filled_qty"], 2.0);
        assert_eq!(p["avg_fill"], 101.0);
        assert_eq!(p["fill_symbol"], "RELIANCE");
    }

    #[test]
    fn fill_without_pending_is_undeclared_not_zero_pnl() {
        let book = LiveBook::new();
        book.apply(LiveBookEvent::Fill {
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            qty: 2.0,
            price: Some(1410.0),
            broker: Some("kotak_neo".into()),
            filled_at_iso: None,
        });
        let snap = book.snapshot().unwrap();
        assert_eq!(snap["notch"]["undeclared_position"]["symbol"], "RELIANCE");
        assert_eq!(snap["notch"]["undeclared_position"]["quantity"], 2);
        assert!(snap["notch"].get("unrealized_pnl").is_none());
        assert!(pending(&snap).is_null());
    }

    #[test]
    fn archive_failure_does_not_unarm() {
        let book = LiveBook::new();
        book.apply(LiveBookEvent::Declare {
            local_id: "local-1".into(),
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            quantity: 1.0,
            declaration_kind: "intraday".into(),
            stop_loss: None,
            target: None,
            protective_sl_consent: false,
            plan_snapshot: None,
            book_id: None,
            ticket_intent: None,
        });
        // No ReconcileArchive — Console 5xx. Book stays armed.
        assert_eq!(pending(&book.snapshot().unwrap())["id"], "local-1");
        assert_eq!(pending(&book.snapshot().unwrap())["status"], "PENDING");
    }

    #[test]
    fn declare_from_body_copies_s1_into_plan_snapshot() {
        let body = json!({
            "symbol": "BTCUSDT",
            "side": "BUY",
            "quantity": 0.002,
            "stop_loss": 64000.0,
            "target_price": 0.75,
            "declaration_kind": "intraday",
            "declaration_payload": {
                "v": 1,
                "protective_sl_consent": false,
                "s1": {
                    "setup_type": "breakout",
                    "invalidation": "Last through invalidation.",
                    "mood_stress": 2.0,
                    "mood_impulse": 4.0
                }
            }
        });
        let book = LiveBook::new();
        book.apply(LiveBookEvent::declare_from_body(&body, "local-usdm".into()));
        let snap = book.snapshot().unwrap();
        let pending = pending(&snap);
        assert_eq!(pending["symbol"], "BTCUSDT");
        assert_eq!(pending["stop_loss"], 64000.0);
        assert_eq!(pending["plan_snapshot"]["setup_label"], "breakout");
        assert_eq!(
            pending["plan_snapshot"]["invalidation_line"],
            "Last through invalidation."
        );
        assert_eq!(pending["plan_snapshot"]["calm_scale"], 2.0);
        assert_ne!(pending["plan_snapshot"]["setup_label"], Value::Null);
        assert!(pending.get("book_id").is_none());
    }

    #[test]
    fn declare_from_body_copies_wave2_snapshot_fields() {
        let body = json!({
            "symbol": "RELIANCE",
            "side": "BUY",
            "quantity": 10.0,
            "stop_loss": 1400.0,
            "target_price": 1500.0,
            "entry_price": 1420.0,
            "book_id": "kotak-nse-bse-cash",
            "declaration_kind": "intraday",
            "declaration_payload": {
                "v": 1,
                "protective_sl_consent": true,
                "s1": {
                    "setup_type": "Breakout",
                    "intent": "Range break, volume confirmed.",
                    "stance": "planned",
                    "mood_stress": 2.0,
                    "mood_impulse": 3.0,
                    "mood_frustration": 1.0,
                    "mood_excitement": 2.0,
                    "invalidation_type": "price",
                    "invalidation_price": 1390.0,
                    "invalidation": "Last through 1390.",
                    "product": "MIS"
                }
            }
        });
        let book = LiveBook::new();
        book.apply(LiveBookEvent::declare_from_body(&body, "local-cash".into()));
        let book_snap = book.snapshot().unwrap();
        let pending = pending(&book_snap);
        let snap = &pending["plan_snapshot"];
        assert_eq!(snap["intent"], "Range break, volume confirmed.");
        assert_eq!(snap["stance"], "planned");
        assert_eq!(snap["invalidation"]["kind"], "price");
        assert_eq!(snap["invalidation"]["price"], 1390.0);
        assert_eq!(snap["product"], "MIS");
        assert_eq!(snap["target_price"], 1500.0);
        assert_eq!(snap["emotion_in"]["frustration"], 1.0);
        assert_eq!(pending["quantity"], 10.0);
    }

    #[test]
    fn usdm_declare_from_body_names_the_usdm_book() {
        let body = json!({
            "symbol": "CATIUSDT",
            "side": "BUY",
            "quantity": 2.0,
            "stop_loss": 0.04,
            "book_id": "binance-com-usdm",
            "declaration_kind": "intraday",
            "declaration_payload": {
                "v": 1,
                "protective_sl_consent": false,
                "s1": {
                    "setup_type": "breakout",
                    "invalidation": "Last through invalidation.",
                    "mood_stress": 2.0,
                    "mood_impulse": 4.0
                }
            }
        });
        let book = LiveBook::new();
        book.apply(LiveBookEvent::declare_from_body(
            &body,
            "local-usdm-book".into(),
        ));
        let snap = book.snapshot().unwrap();
        let pending = pending(&snap);
        assert_eq!(pending["symbol"], "CATIUSDT");
        assert_eq!(pending["quantity"], 2.0);
        assert_ne!(pending["quantity"], 1.0);
        assert_eq!(pending["book_id"], "binance-com-usdm");
        assert_ne!(pending["book_id"], "binance-com-spot");
    }

    #[test]
    fn capture_working_condition_freezes_plan_snapshot_once() {
        let body = json!({
            "symbol": "RELIANCE",
            "side": "BUY",
            "quantity": 10.0,
            "stop_loss": 1400.0,
            "target_price": 1500.0,
            "declaration_kind": "intraday",
            "declaration_payload": {
                "v": 1,
                "protective_sl_consent": true,
                "s1": {
                    "setup_type": "Breakout",
                    "invalidation_type": "price",
                    "invalidation_price": 1410.0
                }
            }
        });
        let book = LiveBook::new();
        book.apply(LiveBookEvent::declare_from_body(&body, "d1".into()));
        book.apply(LiveBookEvent::CaptureWorkingCondition {
            last: Some(1405.0),
            last_status: "fresh".into(),
        });
        let snap = book.snapshot().unwrap();
        let at_close = &pending(&snap)["plan_snapshot"]["condition_at_close"];
        assert_eq!(at_close["last"], 1405.0);
        assert_eq!(at_close["last_status"], "fresh");
        assert_eq!(at_close["invalidation_state"], "breached");
        assert_eq!(at_close["target_state"], "intact");
        book.apply(LiveBookEvent::CaptureWorkingCondition {
            last: Some(9999.0),
            last_status: "fresh".into(),
        });
        assert_eq!(
            pending(&book.snapshot().unwrap())["plan_snapshot"]["condition_at_close"]["last"],
            1405.0
        );
    }

    #[test]
    fn capture_working_condition_not_captured_when_last_dark() {
        let body = json!({
            "symbol": "RELIANCE",
            "side": "BUY",
            "quantity": 10.0,
            "declaration_kind": "intraday",
            "declaration_payload": {
                "v": 1,
                "protective_sl_consent": false,
                "s1": {
                    "invalidation_type": "price",
                    "invalidation_price": 1410.0
                }
            }
        });
        let book = LiveBook::new();
        book.apply(LiveBookEvent::declare_from_body(&body, "d1".into()));
        book.apply(LiveBookEvent::CaptureWorkingCondition {
            last: Some(1405.0),
            last_status: "unavailable".into(),
        });
        let at_close = &pending(&book.snapshot().unwrap())["plan_snapshot"]["condition_at_close"];
        assert_eq!(at_close["last_status"], "not_captured");
        assert_eq!(at_close["invalidation_state"], "not_captured");
    }

    #[test]
    fn usdm_ticket_intent_lands_on_pending_and_is_not_a_venue_order() {
        let body = json!({
            "symbol": "CATIUSDT",
            "side": "BUY",
            "quantity": 2.0,
            "stop_loss": 0.04,
            "book_id": "binance-com-usdm",
            "declaration_kind": "intraday",
            "declaration_payload": {
                "v": 1,
                "protective_sl_consent": false,
                "s1": {
                    "setup_type": "breakout",
                    "invalidation": "Last through invalidation.",
                    "mood_stress": 2.0,
                    "mood_impulse": 4.0
                },
                "ticket": {
                    "type": "CONDITIONAL",
                    "tif": "GTC",
                    "reduce_only": true,
                    "path": "/fapi/v1/algoOrder"
                }
            }
        });
        let book = LiveBook::new();
        book.apply(LiveBookEvent::declare_from_body(
            &body,
            "local-ticket".into(),
        ));
        let snap = book.snapshot().unwrap();
        let pending = pending(&snap);
        assert_eq!(pending["ticket"]["type"], "CONDITIONAL");
        assert_eq!(pending["ticket"]["reduce_only"], true);
        assert_eq!(pending["ticket"]["path"], "/fapi/v1/algoOrder");
        assert_ne!(pending["ticket"]["path"], "/fapi/v1/order");
        assert_eq!(pending["book_id"], "binance-com-usdm");
        assert_eq!(pending["status"], "PENDING");
    }
}
