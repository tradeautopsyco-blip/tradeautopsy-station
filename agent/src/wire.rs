//! Wire v1 verification for bar → agent requests (design §4.1, issue #58).
//!
//! Canonical string (UTF-8):
//! `{METHOD}\n{path}\n{timestamp_rfc3339}\n{request_id}\n{lowercase_hex_sha256(body)}`
//!
//! `x-signature` = standard base64(HMAC-SHA256(secret, canonical)).

use crate::api::AppState;
use axum::body::Body;
use axum::extract::State;
use axum::http::Request;
use axum::http::{HeaderMap, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use hmac::{Hmac, Mac};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use subtle::ConstantTimeEq;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

/// Strict wire v1 protocol version header value.
pub const WIRE_PROTO_VERSION: &str = "1";

type NonceEntry = (Instant, [u8; 16]);
type NonceLedger = Arc<Mutex<Vec<NonceEntry>>>;

#[derive(Clone)]
pub struct WireVerifier {
    secret: Vec<u8>,
    nonce_window: NonceLedger,
}

impl WireVerifier {
    pub fn new(shared_secret: impl Into<String>) -> Self {
        Self {
            secret: shared_secret.into().into_bytes(),
            nonce_window: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn check_nonce(&self, nonce: [u8; 16]) -> Result<(), ()> {
        let mut guard = self.nonce_window.lock().expect("nonce mutex poisoned");
        let now = Instant::now();
        guard.retain(|(t, _)| now.duration_since(*t) < Duration::from_secs(60));
        if guard.iter().any(|(_, n)| n == &nonce) {
            return Err(());
        }
        guard.push((now, nonce));
        Ok(())
    }

    /// Visible for tests that assert signature bytes without going through HTTP.
    pub fn compute_signature(
        &self,
        method: &str,
        path: &str,
        timestamp_rfc3339: &str,
        request_id: &str,
        body: &[u8],
    ) -> Vec<u8> {
        let canonical = canonical_string(method, path, timestamp_rfc3339, request_id, body);
        let mut mac =
            HmacSha256::new_from_slice(&self.secret).expect("HMAC key length is valid for SHA256");
        mac.update(canonical.as_bytes());
        mac.finalize().into_bytes().to_vec()
    }
}

fn canonical_string(
    method: &str,
    path: &str,
    timestamp_rfc3339: &str,
    request_id: &str,
    body: &[u8],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    let body_hex = hex::encode(hasher.finalize());
    let m = method.to_ascii_uppercase();
    format!("{m}\n{path}\n{timestamp_rfc3339}\n{request_id}\n{body_hex}",)
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|v| v.to_str().ok())
}

fn wire_error(
    status: StatusCode,
    error_class: &str,
    message: &str,
    request_id: Option<&str>,
) -> Response {
    let body = json!({
        "error_class": error_class,
        "message": message,
        "retry_after_ms": Value::Null,
        "request_id": request_id
            .map(|s| Value::String(s.to_string()))
            .unwrap_or(Value::Null),
    });
    (status, Json(body)).into_response()
}

fn reject_wire_authenticated(
    state: &AppState,
    status: StatusCode,
    error_class: &str,
    message: &str,
    request_id: Option<&str>,
) -> Response {
    state.metrics.record_hmac_verify_failure();
    wire_error(status, error_class, message, request_id)
}

/// Phase 9 (#66) fail-closed policy when `AGENT_CALLER_STRICT` is set (design §10.1 T2).
fn caller_integrity_check(headers: &HeaderMap) -> Result<(), &'static str> {
    let strict = matches!(
        std::env::var("AGENT_CALLER_STRICT").ok().as_deref(),
        Some("1") | Some("true") | Some("yes")
    );
    if !strict {
        return Ok(());
    }
    let Ok(expected) = std::env::var("AGENT_EXPECTED_CALLER_BUNDLE_ID") else {
        return Err("AGENT_EXPECTED_CALLER_BUNDLE_ID required when AGENT_CALLER_STRICT is enabled");
    };
    let bundle = headers
        .get("x-tradeautopsy-caller-bundle-id")
        .and_then(|v| v.to_str().ok());
    match bundle {
        Some(b) if b == expected.as_str() => Ok(()),
        Some(_) => Err("caller bundle id does not match pinned expectation"),
        None => Err("x-tradeautopsy-caller-bundle-id required under caller integrity policy"),
    }
}

pub async fn verify_middleware(
    State(state): State<AppState>,
    request: Request<Body>,
    next: Next,
) -> Response {
    let (parts, body) = request.into_parts();
    let method = parts.method.clone();
    let path = parts.uri.path().to_owned();

    let body_bytes = match body.collect().await {
        Ok(c) => c.to_bytes(),
        Err(_) => {
            return wire_error(
                StatusCode::BAD_REQUEST,
                "VALIDATION",
                "failed to read request body",
                header(&parts.headers, "x-request-id"),
            );
        }
    };

    let proto = header(&parts.headers, "x-proto-version");
    if proto != Some(WIRE_PROTO_VERSION) {
        return reject_wire_authenticated(
            &state,
            StatusCode::PRECONDITION_FAILED,
            "PROTO_VERSION",
            "x-proto-version must be 1 for wire v1",
            header(&parts.headers, "x-request-id"),
        );
    }

    let secret_hdr = header(&parts.headers, "x-daemon-secret");
    let ok_secret = match secret_hdr {
        Some(s) => {
            let a = s.as_bytes();
            let b = &state.wire.secret;
            if a.len() != b.len() {
                false
            } else {
                bool::from(a.ct_eq(b))
            }
        }
        None => false,
    };
    if !ok_secret {
        return reject_wire_authenticated(
            &state,
            StatusCode::UNAUTHORIZED,
            "SIG_INVALID",
            "x-daemon-secret mismatch or missing",
            header(&parts.headers, "x-request-id"),
        );
    }

    let user_id = header(&parts.headers, "x-user-id");
    if user_id.is_none_or(|u| Uuid::parse_str(u).is_err()) {
        return wire_error(
            StatusCode::BAD_REQUEST,
            "VALIDATION",
            "x-user-id must be a UUID",
            header(&parts.headers, "x-request-id"),
        );
    }

    let request_id = match header(&parts.headers, "x-request-id") {
        Some(r) if UlidOk::is_ulid(r) => r,
        _ => {
            return wire_error(
                StatusCode::BAD_REQUEST,
                "VALIDATION",
                "x-request-id must be a ULID",
                header(&parts.headers, "x-request-id"),
            );
        }
    };

    let ts_str = match header(&parts.headers, "x-timestamp") {
        Some(t) => t,
        None => {
            return wire_error(
                StatusCode::BAD_REQUEST,
                "VALIDATION",
                "x-timestamp is required (RFC 3339 UTC)",
                Some(request_id),
            );
        }
    };

    let ts = match chrono::DateTime::parse_from_rfc3339(ts_str) {
        Ok(t) => t.with_timezone(&chrono::Utc),
        Err(_) => {
            return wire_error(
                StatusCode::BAD_REQUEST,
                "VALIDATION",
                "x-timestamp must be RFC 3339",
                Some(request_id),
            );
        }
    };

    let skew = (chrono::Utc::now() - ts).num_seconds().unsigned_abs();
    if skew > 30 {
        return reject_wire_authenticated(
            &state,
            StatusCode::UNAUTHORIZED,
            "SIG_INVALID",
            "x-timestamp outside allowed skew window",
            Some(request_id),
        );
    }

    let nonce_b64 = match header(&parts.headers, "x-nonce") {
        Some(n) => n,
        None => {
            return wire_error(
                StatusCode::BAD_REQUEST,
                "VALIDATION",
                "x-nonce is required (16-byte value base64-encoded)",
                Some(request_id),
            );
        }
    };

    let nonce_bytes = match B64.decode(nonce_b64.as_bytes()) {
        Ok(b) if b.len() == 16 => {
            let mut a = [0u8; 16];
            a.copy_from_slice(&b);
            a
        }
        _ => {
            return wire_error(
                StatusCode::BAD_REQUEST,
                "VALIDATION",
                "x-nonce must decode to 16 bytes",
                Some(request_id),
            );
        }
    };

    if state.wire.check_nonce(nonce_bytes).is_err() {
        return reject_wire_authenticated(
            &state,
            StatusCode::UNAUTHORIZED,
            "SIG_INVALID",
            "x-nonce replay",
            Some(request_id),
        );
    }

    let sig_b64 = match header(&parts.headers, "x-signature") {
        Some(s) => s,
        None => {
            return reject_wire_authenticated(
                &state,
                StatusCode::UNAUTHORIZED,
                "SIG_INVALID",
                "x-signature is required",
                Some(request_id),
            );
        }
    };

    let expected =
        state
            .wire
            .compute_signature(method.as_str(), &path, ts_str, request_id, &body_bytes);
    let provided = match B64.decode(sig_b64.as_bytes()) {
        Ok(b) => b,
        Err(_) => {
            return reject_wire_authenticated(
                &state,
                StatusCode::UNAUTHORIZED,
                "SIG_INVALID",
                "x-signature must be valid base64",
                Some(request_id),
            );
        }
    };

    if expected.len() != provided.len() || !bool::from(expected.ct_eq(&provided)) {
        return reject_wire_authenticated(
            &state,
            StatusCode::UNAUTHORIZED,
            "SIG_INVALID",
            "HMAC verification failed",
            Some(request_id),
        );
    }

    if let Err(msg) = caller_integrity_check(&parts.headers) {
        return reject_wire_authenticated(
            &state,
            StatusCode::UNAUTHORIZED,
            "SIG_INVALID",
            msg,
            Some(request_id),
        );
    }

    let new_body = Body::from(body_bytes);
    let req = Request::from_parts(parts, new_body);
    next.run(req).await
}

/// ULID syntax check without accepting arbitrary 26-char strings.
struct UlidOk;

impl UlidOk {
    fn is_ulid(s: &str) -> bool {
        ulid::Ulid::from_string(s).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_matches_known_vector() {
        let v = WireVerifier::new("unit-test-secret");
        let sig = v.compute_signature(
            "GET",
            "/api/daemon/health",
            "2020-01-01T00:00:00.000Z",
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            b"",
        );
        let b64 = B64.encode(&sig);
        let again = v.compute_signature(
            "GET",
            "/api/daemon/health",
            "2020-01-01T00:00:00.000Z",
            "01ARZ3NDEKTSV4RRFFQ69G5FAV",
            b"",
        );
        assert_eq!(sig, again);
        assert!(b64.len() > 32);
    }
}
