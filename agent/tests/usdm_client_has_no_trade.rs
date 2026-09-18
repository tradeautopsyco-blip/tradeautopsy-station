//! USDM thin client must not grow a TRADE place surface.
//!
//! Lock: `issues/compliance/locks/binance-com-usdm.md` — refuse `POST /fapi/v1/order`.
//! Scan executable source (not `//!` / `//` comments). `/fapi/v1/forceOrders` is USER_DATA GET,
//! not the place-order path.

use std::path::{Path, PathBuf};

const CLIENT: &str = "src/binance_com_usdm_client.rs";

fn agent_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn strip_line_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn path_tokens(code: &str) -> Vec<&str> {
    code.split(|c: char| !(c.is_ascii_alphanumeric() || c == '/' || c == '_' || c == '.'))
        .filter(|tok| !tok.is_empty())
        .collect()
}

fn mentions_place_order_path(code: &str) -> bool {
    path_tokens(code).iter().any(|tok| *tok == "/fapi/v1/order")
}

fn mentions_http_post(code: &str) -> bool {
    code.contains("Method::POST")
        || code.contains(".post(")
        || code.contains("\"POST\"")
        || code.contains("new_order")
}

#[test]
fn the_detector_flags_place_order_and_spares_force_orders() {
    assert!(mentions_place_order_path(
        r#"client.signed_post("/fapi/v1/order", &[])"#
    ));
    assert!(!mentions_place_order_path(
        r#"self.signed_get("/fapi/v1/forceOrders", &[])"#
    ));
    assert!(mentions_http_post("reqwest::Method::POST"));
    assert!(!mentions_http_post("self.signed_get(path, extra)"));
}

#[test]
fn usdm_client_has_no_trade_place_surface() {
    let path = agent_root().join(CLIENT);
    let source = std::fs::read_to_string(&path).unwrap_or_else(|err| {
        panic!("failed to read {CLIENT}: {err}");
    });
    let code = strip_line_comments(&source);
    assert!(
        code.contains("fn fetch_balance"),
        "public USER_DATA surface: fetch_balance"
    );
    assert!(
        code.contains("fn fetch_positions"),
        "public USER_DATA surface: fetch_positions"
    );
    assert!(
        code.contains("fn fetch_force_orders"),
        "public USER_DATA surface: fetch_force_orders"
    );
    assert!(
        code.contains("fn fetch_income"),
        "public USER_DATA surface: fetch_income (REALIZED_PNL, not TRADE)"
    );
    assert!(
        !mentions_place_order_path(&code),
        "{CLIENT} must not dial /fapi/v1/order (forceOrders GET is a different path)"
    );
    assert!(
        !mentions_http_post(&code),
        "{CLIENT} must not POST / grow new_order"
    );
}
