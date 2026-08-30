//! Normalized broker behavioral events with opt-out gate (Backend Box v1 #18).

use crate::broker_redaction::RedactionBoundary;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct BrokerConnectionIdentityFields {
    pub broker_connection_id: String,
    pub broker_slug: String,
    pub asset_class: String,
    pub environment: String,
}

impl BrokerConnectionIdentityFields {
    pub fn inject(&self, mut payload: Value) -> Value {
        if let Value::Object(ref mut map) = payload {
            map.insert(
                "broker_connection_id".into(),
                Value::String(self.broker_connection_id.clone()),
            );
            map.insert(
                "broker_slug".into(),
                Value::String(self.broker_slug.clone()),
            );
            map.insert(
                "asset_class".into(),
                Value::String(self.asset_class.clone()),
            );
            map.insert(
                "environment".into(),
                Value::String(self.environment.clone()),
            );
        }
        payload
    }
}

#[derive(Clone, Default)]
pub struct BrokerBehavioralRecorder {
    opted_out: Arc<AtomicBool>,
    recorded: Arc<Mutex<Vec<Value>>>,
    secret_literals: Arc<Mutex<Vec<String>>>,
}

impl BrokerBehavioralRecorder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_opted_out(&self, opted_out: bool) {
        self.opted_out.store(opted_out, Ordering::Relaxed);
    }

    pub fn is_opted_out(&self) -> bool {
        self.opted_out.load(Ordering::Relaxed)
    }

    pub fn track_secret_literal(&self, secret: String) {
        if secret.is_empty() {
            return;
        }
        self.secret_literals
            .lock()
            .expect("secret literals")
            .push(secret);
    }

    pub fn record_normalized(
        &self,
        identity: &BrokerConnectionIdentityFields,
        event_type: &str,
        payload: Value,
        completeness: Option<Value>,
    ) -> bool {
        if self.is_opted_out() {
            return false;
        }

        let secrets: Vec<String> = self.secret_literals.lock().expect("secrets").clone();
        let secret_refs: Vec<&str> = secrets.iter().map(String::as_str).collect();

        let mut body = json!({
            "event_type": event_type,
            "payload": payload,
        });
        if let Some(flags) = completeness {
            if let Value::Object(ref mut map) = body {
                map.insert("completeness".into(), flags);
            }
        }

        let with_identity = identity.inject(body);
        let Some(redacted) = RedactionBoundary::redact_for_upload(with_identity, &secret_refs)
        else {
            return false;
        };

        self.recorded.lock().expect("recorded").push(redacted);
        true
    }

    pub fn recorded_events(&self) -> Vec<Value> {
        self.recorded.lock().expect("recorded").clone()
    }

    pub fn clear(&self) {
        self.recorded.lock().expect("recorded").clear();
    }
}
