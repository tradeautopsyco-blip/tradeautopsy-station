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
        broker: Option<String>,
        filled_at_iso: Option<String>,
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
        let plan_snapshot = s1.map(|s1| {
            json!({
                "setup_label": s1.get("setup_type").and_then(Value::as_str).unwrap_or(""),
                "invalidation_line": s1.get("invalidation").and_then(Value::as_str).unwrap_or(""),
                "calm_scale": s1.get("mood_stress").and_then(Value::as_f64),
                "confidence_scale": s1.get("mood_impulse").and_then(Value::as_f64),
            })
        });
        let target = json_f64(body.get("target_price")).or_else(|| json_f64(body.get("target")));
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
            if let Some(ps) = plan_snapshot {
                pending
                    .as_object_mut()
                    .expect("pending object")
                    .insert("plan_snapshot".into(), ps);
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
                    "filledAtMs": filled_at_iso,
                    "broker": broker,
                }),
            );
        }
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
        });
        book.apply(LiveBookEvent::Fill {
            symbol: "reliance".into(),
            side: "buy".into(),
            qty: 10.0,
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
    fn fill_without_pending_is_undeclared_not_zero_pnl() {
        let book = LiveBook::new();
        book.apply(LiveBookEvent::Fill {
            symbol: "RELIANCE".into(),
            side: "BUY".into(),
            qty: 2.0,
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
    }
}
