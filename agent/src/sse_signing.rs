//! Phase 9 (#66) — Ed25519 signing for SSE event envelopes (design §4.4).

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::sync::Arc;

fn canonical_sign_message(event_id: &str, event_type: &str, payload: &Value) -> String {
    let payload_bytes = serde_json::to_vec(payload).unwrap_or_else(|_| "{}".as_bytes().to_vec());
    let digest = Sha256::digest(&payload_bytes);
    let digest_hex = hex::encode(digest);
    format!("{event_id}\n{event_type}\n{digest_hex}")
}

#[derive(Clone)]
pub struct SseSigner {
    key: Arc<SigningKey>,
}

impl SseSigner {
    pub fn generate() -> Self {
        use rand::rngs::OsRng;
        let mut csprng = OsRng;
        let key = SigningKey::generate(&mut csprng);
        Self { key: Arc::new(key) }
    }

    /// Deterministic key for tests (`AGENT_SSE_SIGNING_SEED_HEX` in harness).
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self {
            key: Arc::new(SigningKey::from_bytes(&seed)),
        }
    }

    pub fn from_env_or_generate() -> Self {
        let Ok(hex_seed) = std::env::var("AGENT_SSE_SIGNING_SEED_HEX") else {
            return Self::generate();
        };
        let bytes = match hex::decode(hex_seed.trim()) {
            Ok(b) if b.len() == 32 => b,
            _ => return Self::generate(),
        };
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes);
        Self::from_seed(seed)
    }

    pub fn sign_event(&self, event_id: &str, event_type: &str, payload: &Value) -> String {
        let msg = canonical_sign_message(event_id, event_type, payload);
        let sig: Signature = self.key.sign(msg.as_bytes());
        B64.encode(sig.to_bytes())
    }

    pub fn public_key_for_pinning(&self) -> SseSigningPubKey {
        SseSigningPubKey(self.key.verifying_key())
    }

    pub fn public_key_b64(&self) -> String {
        B64.encode(self.key.verifying_key().as_bytes())
    }
}

/// Bar-side pinned verifying key (parsed from health `sse_signing_pubkey_b64`).
#[derive(Clone)]
pub struct SseSigningPubKey(VerifyingKey);

impl SseSigningPubKey {
    pub fn from_standard_b64(b64: &str) -> Result<Self, ()> {
        let raw = B64.decode(b64.trim().as_bytes()).map_err(|_| ())?;
        if raw.len() != 32 {
            return Err(());
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&raw);
        VerifyingKey::from_bytes(&arr)
            .map(SseSigningPubKey)
            .map_err(|_| ())
    }

    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.0
    }
}

pub fn verify_sse_event_signature(
    pubkey: &SseSigningPubKey,
    event_id: &str,
    event_type: &str,
    payload: &Value,
    sig_b64: &str,
) -> bool {
    let Ok(raw) = B64.decode(sig_b64.trim().as_bytes()) else {
        return false;
    };
    if raw.len() != 64 {
        return false;
    };
    let Ok(sig_arr) = raw.try_into() else {
        return false;
    };
    let sig = Signature::from_bytes(&sig_arr);
    let msg = canonical_sign_message(event_id, event_type, payload);
    pubkey.verifying_key().verify(msg.as_bytes(), &sig).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_rejects_empty_and_truncated_signatures() {
        let signer = SseSigner::from_seed([9u8; 32]);
        let pubkey = signer.public_key_for_pinning();
        let payload = serde_json::json!({"x": 1});
        let event_id = "01HZYDTSJJTGPAKFYZVNPMEBNG";
        let sig = signer.sign_event(event_id, "agent_health", &payload);

        assert!(
            !verify_sse_event_signature(&pubkey, event_id, "agent_health", &payload, ""),
            "empty sig must fail closed"
        );
        assert!(
            !verify_sse_event_signature(&pubkey, event_id, "agent_health", &payload, "AAA="),
            "truncated sig must fail closed"
        );
        assert!(
            verify_sse_event_signature(&pubkey, event_id, "agent_health", &payload, &sig),
            "valid sig round-trip"
        );
    }
}
