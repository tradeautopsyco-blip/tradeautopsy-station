//! Phase 4 crypto spot host fences and Start factory checks (offline · no live keys).
//!
//! ## Offline G7 command list
//!
//! From the `agent/` workspace root (no API keys required):
//!
//! ```text
//! cargo test p4_crypto
//! cargo test --test ubi_bybit_component
//! cargo test --test ubi_okx_component
//! cargo test --test ubi_kraken_component
//! cargo test --test ubi_coinbase_advanced_component
//! ```
//!
//! If release Wasm artifacts are missing, build all four adapters once:
//!
//! ```text
//! cargo build --release --target wasm32-wasip2 \
//!   -p ubi-bybit-adapter -p ubi-okx-adapter -p ubi-kraken-adapter -p ubi-coinbase-advanced-adapter
//! ```

mod ubi_support;

use serial_test::serial;
use tradeautopsy_agent::{
    authorize_book_call, authorize_book_fence, build_runtime_adapter, host_allowed, uses_wasm_component,
    BYBIT_BOOK_ID, CredentialBlob, HostRefuse, KRAKEN_BOOK_ID, OKX_API_HOST, OKX_COM_SPOT_BOOK_ID,
};

const COINBASE_ADVANCED_SPOT_BOOK_ID: &str = "coinbase-advanced-spot";
const COINBASE_FILLS_PATH: &str = "/api/v3/brokerage/orders/historical/fills";

#[test]
fn p4_crypto_spot_fence_bybit_cannot_use_okx_host() {
    assert!(host_allowed(OKX_API_HOST), "okx prod host is globally allowlisted");
    assert_eq!(
        authorize_book_fence(BYBIT_BOOK_ID, OKX_API_HOST, "/v5/execution/list"),
        Err(HostRefuse::HostNotAllowed),
        "bybit book must not authorize calls on okx host"
    );
    assert_eq!(
        authorize_book_call(BYBIT_BOOK_ID, OKX_API_HOST, "GET", "/v5/execution/list", true),
        Err(HostRefuse::HostNotAllowed)
    );
}

#[test]
fn p4_crypto_spot_fence_kraken_refuses_futures_host() {
    assert!(
        !host_allowed("futures.kraken.com"),
        "futures host must not appear on the global broker allowlist"
    );
    assert_eq!(
        authorize_book_fence(KRAKEN_BOOK_ID, "futures.kraken.com", "/0/private/TradesHistory"),
        Err(HostRefuse::HostNotAllowed)
    );
    assert_eq!(
        authorize_book_call(
            KRAKEN_BOOK_ID,
            "futures.kraken.com",
            "POST",
            "/0/private/TradesHistory",
            true
        ),
        Err(HostRefuse::HostNotAllowed)
    );
}

#[test]
fn p4_crypto_spot_fence_okx_refuses_us_and_eea_hosts() {
    for sibling in ["us.okx.com", "eea.okx.com"] {
        assert!(!host_allowed(sibling), "{sibling} must not be allowlisted");
        assert_eq!(
            authorize_book_fence(OKX_COM_SPOT_BOOK_ID, sibling, "/api/v5/trade/fills"),
            Err(HostRefuse::HostNotAllowed),
            "okx book fence must refuse {sibling}"
        );
        assert_eq!(
            authorize_book_call(OKX_COM_SPOT_BOOK_ID, sibling, "GET", "/api/v5/trade/fills", true),
            Err(HostRefuse::HostNotAllowed)
        );
    }
}

#[test]
fn p4_crypto_spot_fence_coinbase_refuses_sandbox_host() {
    assert!(!host_allowed("api-sandbox.coinbase.com"));
    assert_eq!(
        authorize_book_fence(
            COINBASE_ADVANCED_SPOT_BOOK_ID,
            "api-sandbox.coinbase.com",
            COINBASE_FILLS_PATH
        ),
        Err(HostRefuse::HostNotAllowed)
    );
    assert_eq!(
        authorize_book_call(
            COINBASE_ADVANCED_SPOT_BOOK_ID,
            "api-sandbox.coinbase.com",
            "GET",
            COINBASE_FILLS_PATH,
            true
        ),
        Err(HostRefuse::HostNotAllowed)
    );
}

fn bybit_hmac_blob() -> CredentialBlob {
    CredentialBlob::hmac("TA_FAKE_BYBIT_KEY", "secret")
}

fn kraken_hmac_blob() -> CredentialBlob {
    CredentialBlob::hmac("TA_FAKE_KRAKEN_KEY", "c2VjcmV0")
}

fn okx_session_blob() -> CredentialBlob {
    CredentialBlob::OkxPassphraseSession {
        api_key: "TA_FAKE_OKX_KEY".into(),
        api_secret: "secret".into(),
        passphrase: "pass".into(),
    }
}

fn coinbase_jwt_blob() -> CredentialBlob {
    CredentialBlob::CoinbaseJwtEs256Session {
        api_key: "organizations/test/apiKeys/TA_FAKE_COINBASE".into(),
        pem_private_key: "-----BEGIN EC PRIVATE KEY-----\nMHcCAQEEIBdummy\n-----END EC PRIVATE KEY-----\n"
            .into(),
    }
}

#[test]
fn p4_crypto_uses_wasm_component_for_all_four_slugs() {
    for slug in ["bybit", "okx_com", "kraken", "coinbase_advanced"] {
        assert!(
            uses_wasm_component(slug),
            "{slug} must route through Wasm (ADR 0001)"
        );
    }
}

#[test]
#[serial]
fn p4_crypto_build_runtime_adapter_accepts_correct_credential_shapes() {
    let _ = ubi_support::component_wasm("bybit");
    let _ = ubi_support::component_wasm("okx_com");
    let _ = ubi_support::component_wasm("kraken");
    let _ = ubi_support::component_wasm("coinbase_advanced");

    let bybit = build_runtime_adapter("bybit", "conn-p4-bybit", &bybit_hmac_blob())
        .expect("bybit HMAC + TA_FAKE_*");
    assert_eq!(bybit.name(), "ubi_wasm");

    let okx = build_runtime_adapter("okx_com", "conn-p4-okx", &okx_session_blob())
        .expect("okx OkxPassphraseSession");
    assert_eq!(okx.name(), "ubi_wasm");

    let kraken = build_runtime_adapter("kraken", "conn-p4-kraken", &kraken_hmac_blob())
        .expect("kraken HMAC");
    assert_eq!(kraken.name(), "ubi_wasm");

    let coinbase =
        build_runtime_adapter("coinbase_advanced", "conn-p4-coinbase", &coinbase_jwt_blob())
            .expect("coinbase CoinbaseJwtEs256Session");
    assert_eq!(coinbase.name(), "ubi_wasm");
}

#[test]
fn p4_crypto_build_runtime_adapter_rejects_wrong_credential_shapes() {
    let hmac = CredentialBlob::hmac("TA_FAKE_KEY", "secret");
    for (slug, blob) in [
        ("okx_com", hmac.clone()),
        ("coinbase_advanced", hmac.clone()),
        ("bybit", okx_session_blob()),
    ] {
        let msg = match build_runtime_adapter(slug, "conn-p4-shape", &blob) {
            Err(e) => e.to_string(),
            Ok(_) => panic!("{slug} must reject wrong credential shape"),
        };
        assert!(
            msg.contains("unsupported broker slug or credential shape")
                || msg.contains("session vault incomplete"),
            "{slug}: {msg}"
        );
    }
}
