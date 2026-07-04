//! Redaction boundary before behavioral/upload egress (Backend Box v1 #18).

use serde_json::Value;

const FORBIDDEN_KEYS: &[&str] = &[
    "api_key",
    "apiKey",
    "api_secret",
    "apiSecret",
    "passphrase",
    "session_token",
    "sessionToken",
    "oauth_token",
    "oauthToken",
    "cookie",
    "hmac",
    "authorization",
    "signed_url",
    "signedUrl",
    "raw_request",
    "raw_response",
    "raw_error",
    "keychain_item",
    "keychainItem",
    "macos_username",
    "local_path",
    "machine_serial",
];

pub struct RedactionBoundary;

impl RedactionBoundary {
    pub fn contains_forbidden_material(value: &Value, secret_literals: &[&str]) -> bool {
        if value_contains_forbidden_key(value) {
            return true;
        }
        let serialized = value.to_string();
        secret_literals
            .iter()
            .any(|secret| !secret.is_empty() && serialized.contains(secret))
    }

    pub fn redact_for_upload(mut value: Value, secret_literals: &[&str]) -> Option<Value> {
        strip_forbidden_keys(&mut value);
        let serialized = value.to_string();
        if secret_literals
            .iter()
            .any(|secret| !secret.is_empty() && serialized.contains(secret))
        {
            return None;
        }
        Some(value)
    }
}

fn value_contains_forbidden_key(value: &Value) -> bool {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if FORBIDDEN_KEYS
                    .iter()
                    .any(|f| key.eq_ignore_ascii_case(f))
                {
                    return true;
                }
                if value_contains_forbidden_key(child) {
                    return true;
                }
            }
            false
        }
        Value::Array(items) => items.iter().any(value_contains_forbidden_key),
        _ => false,
    }
}

fn strip_forbidden_keys(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|key, _| {
                !FORBIDDEN_KEYS
                    .iter()
                    .any(|f| key.eq_ignore_ascii_case(f))
            });
            for child in map.values_mut() {
                strip_forbidden_keys(child);
            }
        }
        Value::Array(items) => {
            for item in items {
                strip_forbidden_keys(item);
            }
        }
        _ => {}
    }
}
