//! Bar v1 — POST normalized fills to hosted `broker-ingest/fill` (issue #88 producer).

use crate::broker::BrokerFill;
use crate::UpstreamClient;
use serde::Serialize;

const INGEST_PATH: &str = "/api/internal/bar/v1/broker-ingest/fill";

/// When `AGENT_BAR_BROKER_FILL_INGEST=1`, the agent POSTs each new local fill to the app.
#[derive(Clone, Debug)]
pub struct BarBrokerFillIngressConfig {
    pub user_id: String,
    pub broker_connection_id: String,
    pub default_product: String,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BarFillIngestSource {
    WebSocket,
    Reconciliation,
    Manual,
}

impl BarBrokerFillIngressConfig {
    /// Returns `Some` only when enabled **and** required UUID env vars are present and valid.
    pub fn from_env() -> Option<Self> {
        if std::env::var("AGENT_BAR_BROKER_FILL_INGEST")
            .ok()
            .as_deref()
            != Some("1")
        {
            return None;
        }
        let user_id = std::env::var("AGENT_BAR_INGEST_USER_ID").ok()?;
        let broker_connection_id = std::env::var("AGENT_BAR_BROKER_CONNECTION_ID").ok()?;
        if uuid::Uuid::parse_str(&user_id).is_err()
            || uuid::Uuid::parse_str(&broker_connection_id).is_err()
        {
            tracing::warn!(
                "AGENT_BAR_BROKER_FILL_INGEST=1 but AGENT_BAR_INGEST_USER_ID or AGENT_BAR_BROKER_CONNECTION_ID is not a valid UUID — bar fill ingress disabled"
            );
            return None;
        }
        let raw_product =
            std::env::var("AGENT_BAR_DEFAULT_PRODUCT").unwrap_or_else(|_| "MIS".to_string());
        // I-N4: v1 cash lock — CNC|MIS only. NRML/CO/FO must not re-enter via default_product.
        let default_product = match v1_cash_product(&raw_product) {
            Some(product) => product.to_string(),
            None => {
                tracing::warn!(
                    product = %raw_product,
                    "AGENT_BAR_DEFAULT_PRODUCT must be CNC | MIS (v1 cash lock) — refusing NRML/CO/FO"
                );
                return None;
            }
        };
        Some(Self {
            user_id,
            broker_connection_id,
            default_product,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FingerprintPartsJson<'a> {
    broker_order_id: &'a str,
    event_type: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    fill_sequence: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    quantity_slice: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exchange_trade_id: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BarFillBodyJson<'a> {
    user_id: &'a str,
    broker_connection_id: &'a str,
    broker: &'a str,
    broker_order_id: &'a str,
    symbol: &'a str,
    side: &'a str,
    quantity: f64,
    fill_price: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    product: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    instrument_type: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    lot: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    currency: Option<&'a str>,
    filled_at_ms: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    fee_amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fee_asset: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    declaration_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scalper_session_id: Option<&'a str>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BarBrokerFillIngressBody<'a> {
    user_id: &'a str,
    fingerprint_parts: FingerprintPartsJson<'a>,
    order_timestamp_ms: i64,
    payload: serde_json::Value,
    source: BarFillIngestSource,
    fill: BarFillBodyJson<'a>,
}

/// I-N4: Station v1 cash lock (locks/kotak-nse-bse-cash.md, fetch 2026-08-22 IST).
/// NRML/CO/BO/FO are not a default and must not re-enter.
pub(crate) fn v1_cash_product(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "CNC" => Some("CNC"),
        "MIS" => Some("MIS"),
        _ => None,
    }
}

pub(crate) fn bar_broker_fill_ingest_url(base_url: &str) -> String {
    format!("{}{}", base_url.trim_end_matches('/'), INGEST_PATH)
}

fn normalize_side(side: &str) -> Option<&'static str> {
    match side.trim().to_ascii_uppercase().as_str() {
        "BUY" => Some("BUY"),
        "SELL" => Some("SELL"),
        _ => None,
    }
}

fn finite_fill_price(price: f64) -> Option<f64> {
    if !price.is_finite() {
        return None;
    }
    Some(price)
}

fn is_com_spot_fill(fill: &BrokerFill) -> bool {
    fill.broker.trim() == "binance_com" && !is_fo_segment(fill.exchange_segment.as_deref())
}

fn is_fo_segment(segment: Option<&str>) -> bool {
    matches!(
        segment.map(|s| s.trim().to_ascii_lowercase()).as_deref(),
        Some("nse_fo" | "bse_fo" | "cde_fo" | "mcx_fo")
    )
}

/// Named NFO book is `nse_fo` only. `bse_fo` / CDS / MCX stay unsigned.
fn is_nse_fo_segment(segment: Option<&str>) -> bool {
    segment
        .map(|s| s.trim().eq_ignore_ascii_case("nse_fo"))
        .unwrap_or(false)
}

enum FillProductLane {
    Spot,
    Cash,
    Nfo,
}

/// Lane is adapter + `exchange_segment`, not a global cash lock.
fn fill_product_lane(fill: &BrokerFill) -> FillProductLane {
    if is_com_spot_fill(fill) {
        return FillProductLane::Spot;
    }
    if is_nse_fo_segment(fill.exchange_segment.as_deref()) {
        return FillProductLane::Nfo;
    }
    FillProductLane::Cash
}

/// Spot has no CNC/MIS. Pass through only a non-cash, non-FO label.
fn spot_legal_product(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    match trimmed.to_ascii_uppercase().as_str() {
        "CNC" | "MIS" | "NRML" | "CO" | "BO" | "FO" => None,
        _ => Some(trimmed),
    }
}

/// NFO product allowlist. `docs/reference/india/kotak-neo/NFO-SCRIP-MASTER.md`
/// Products allowed **NRML**. **MIS** scoped — keep None. CNC/CO/BO stay None
/// (CNC on `nse_fo` is NOT SPECIFIED: do not enable; do not invent a dedicated refuse).
pub(crate) fn v1_nfo_product(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_uppercase().as_str() {
        "NRML" => Some("NRML"),
        _ => None,
    }
}

/// Builds the JSON body matching `app/api/internal/bar/v1/broker-ingest/fill/route.ts` `bodySchema`.
///
/// D3 owner. Locks: locks/kotak-nse-bse-cash.md + locks/binance-com-spot.md (fetch 2026-08-22 IST)
/// + `docs/reference/india/kotak-neo/NFO-SCRIP-MASTER.md`. Cash CNC|MIS is Kotak cash
/// (`nse_cm`/`bse_cm` / missing segment) only. NFO (`nse_fo`) product comes from the
/// fill (NRML), never `AGENT_BAR_DEFAULT_PRODUCT`. COM fills must not inherit MIS.
pub(crate) fn build_bar_broker_fill_ingress_json(
    fill: &BrokerFill,
    cfg: &BarBrokerFillIngressConfig,
    source: BarFillIngestSource,
) -> Result<serde_json::Value, &'static str> {
    let side = normalize_side(&fill.side).ok_or("side must be BUY or SELL")?;
    if !fill.qty.is_finite() || fill.qty <= 0.0 {
        return Err("quantity must be finite and positive");
    }
    if is_fo_segment(fill.exchange_segment.as_deref())
        && !is_nse_fo_segment(fill.exchange_segment.as_deref())
    {
        return Err("nfo lock: FO segment not signed (nse_fo only)");
    }
    let product = match fill_product_lane(fill) {
        FillProductLane::Spot => fill.product.as_deref().and_then(spot_legal_product),
        FillProductLane::Cash => {
            let default = v1_cash_product(&cfg.default_product)
                .ok_or("v1 cash lock: default_product must be CNC or MIS")?;
            if let Some(fill_product) = fill.product.as_deref() {
                if v1_cash_product(fill_product).is_none() {
                    return Err("v1 cash lock: fill product NRML/CO/FO refused");
                }
            }
            Some(default)
        }
        FillProductLane::Nfo => {
            let Some(fill_product) = fill.product.as_deref() else {
                return Err("nfo lock: no signed NFO product; refusing cash MIS default");
            };
            Some(
                v1_nfo_product(fill_product)
                    .ok_or("nfo lock: fill product not a signed NFO product")?,
            )
        }
    };
    let nfo_lane = matches!(fill_product_lane(fill), FillProductLane::Nfo);
    // NFO identity stays off cash/spot (do not copy lot onto Binance options / COM).
    let instrument_type = if nfo_lane {
        fill.instrument_type.as_deref()
    } else {
        None
    };
    let lot = if nfo_lane { fill.lot } else { None };
    // DualNoBlend: INR at insert for IND NFO. Never netPnL * exchange_rate.
    let currency = if nfo_lane {
        fill.currency.as_deref().or(Some("INR"))
    } else {
        fill.currency.as_deref()
    };
    let filled_at_ms = fill.filled_at.timestamp_millis();
    let broker_order_id = fill.fill_id.as_str();
    let body = BarBrokerFillIngressBody {
        user_id: cfg.user_id.as_str(),
        fingerprint_parts: FingerprintPartsJson {
            broker_order_id,
            event_type: "fill",
            fill_sequence: None,
            quantity_slice: None,
            exchange_trade_id: Some(fill.trade_id.as_str()),
        },
        order_timestamp_ms: filled_at_ms,
        payload: serde_json::json!({
            "agentFillId": fill.fill_id,
            "exchangeTradeId": fill.trade_id,
        }),
        source,
        fill: BarFillBodyJson {
            user_id: cfg.user_id.as_str(),
            broker_connection_id: cfg.broker_connection_id.as_str(),
            broker: fill.broker.as_str(),
            broker_order_id,
            symbol: fill.symbol.as_str(),
            side,
            quantity: fill.qty,
            fill_price: finite_fill_price(fill.price),
            product,
            instrument_type,
            lot,
            currency,
            filled_at_ms,
            fee_amount: fill.fee_amount,
            fee_asset: fill.fee_asset.as_deref(),
            declaration_id: None,
            scalper_session_id: None,
        },
    };
    serde_json::to_value(&body).map_err(|_| "serialize bar ingest body")
}

/// POST fill to hosted ingest. **404** (route disabled server-side) is treated as success with a debug log.
pub async fn post_bar_broker_fill_ingress(
    client: &UpstreamClient,
    cfg: &BarBrokerFillIngressConfig,
    fill: &BrokerFill,
    source: BarFillIngestSource,
) {
    let body = match build_bar_broker_fill_ingress_json(fill, cfg, source) {
        Ok(v) => v,
        Err(reason) => {
            tracing::warn!(
                reason,
                fill_id = %fill.fill_id,
                "skip bar broker fill ingest — validation"
            );
            return;
        }
    };

    let url = bar_broker_fill_ingest_url(&client.config.base_url);
    let req = match client.authorize_brain(
        client
            .http
            .post(&url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&body),
    ) {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(
                error = %err,
                fill_id = %fill.fill_id,
                "bar broker fill ingest skipped — Station Bearer missing"
            );
            return;
        }
    };
    let resp = match req.send().await {
        Ok(r) => r,
        Err(err) => {
            tracing::warn!(
                error = %err,
                fill_id = %fill.fill_id,
                "bar broker fill ingest transport failed"
            );
            return;
        }
    };

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();

    if status == 404 {
        tracing::debug!(
            fill_id = %fill.fill_id,
            "bar broker fill ingest returned 404 — BAR_INTERNAL_BROKER_FILL_INGEST likely off on server"
        );
        return;
    }

    if !status.is_success() {
        tracing::warn!(
            status = %status,
            fill_id = %fill.fill_id,
            body = %text.chars().take(512).collect::<String>(),
            "bar broker fill ingest HTTP error"
        );
        return;
    }

    let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) else {
        tracing::warn!(
            fill_id = %fill.fill_id,
            body_prefix = %text.chars().take(256).collect::<String>(),
            "bar broker fill ingest: non-JSON 200 body"
        );
        return;
    };

    let inserted = val.get("inserted").and_then(|v| v.as_bool()) == Some(true);
    let deduped = val.get("deduplicated").and_then(|v| v.as_bool()) == Some(true);
    if inserted || deduped {
        tracing::info!(
            fill_id = %fill.fill_id,
            inserted,
            deduplicated = deduped,
            fingerprint = ?val.get("fingerprint"),
            "bar_broker_fill_ingest_ok"
        );
        return;
    }

    tracing::warn!(
        fill_id = %fill.fill_id,
        body = %text.chars().take(512).collect::<String>(),
        "bar broker fill ingest: unexpected 200 response shape"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use reqwest::header::CONTENT_TYPE;
    use serial_test::serial;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    static TEST_USER: &str = "ac35ef44-6366-40d6-89d4-95530e8e3dbf";
    static TEST_CONN: &str = "b1b1b1b1-b1b1-b1b1-b1b1-b1b1b1b1b1b1";

    fn fixture_cfg() -> BarBrokerFillIngressConfig {
        BarBrokerFillIngressConfig {
            user_id: TEST_USER.to_string(),
            broker_connection_id: TEST_CONN.to_string(),
            default_product: "MIS".to_string(),
        }
    }

    fn fixture_fill() -> BrokerFill {
        BrokerFill {
            fill_id: "ord-123".to_string(),
            trade_id: "ex-999".to_string(),
            symbol: "RELIANCE".to_string(),
            side: "buy".to_string(),
            qty: 1.0,
            price: 2500.5,
            filled_at: Utc::now(),
            broker: "kotak".to_string(),
            fee_amount: None,
            fee_asset: None,
            ..Default::default()
        }
    }

    #[test]
    fn bar_broker_fill_ingest_url_joins_base() {
        assert_eq!(
            bar_broker_fill_ingest_url("https://tradeautopsy.in"),
            "https://tradeautopsy.in/api/internal/bar/v1/broker-ingest/fill"
        );
        assert_eq!(
            bar_broker_fill_ingest_url("https://localhost:3000/"),
            "https://localhost:3000/api/internal/bar/v1/broker-ingest/fill"
        );
    }

    #[test]
    fn build_json_matches_route_contract() {
        let cfg = fixture_cfg();
        let fill = fixture_fill();
        let v =
            build_bar_broker_fill_ingress_json(&fill, &cfg, BarFillIngestSource::Reconciliation)
                .expect("ok");

        assert_eq!(v["userId"], cfg.user_id);
        assert_eq!(v["fill"]["userId"], cfg.user_id);
        assert_eq!(v["fill"]["brokerConnectionId"], cfg.broker_connection_id);
        assert_eq!(v["fill"]["side"], "BUY");
        assert_eq!(v["fill"]["quantity"], 1.0);
        assert_eq!(v["fill"]["product"], "MIS");
        assert!(v["fill"].get("feeAmount").is_none());
        assert!(v["fill"].get("feeAsset").is_none());
        assert_eq!(v["fingerprintParts"]["brokerOrderId"], "ord-123");
        assert_eq!(v["fingerprintParts"]["eventType"], "fill");
        assert_eq!(v["fingerprintParts"]["exchangeTradeId"], "ex-999");
        assert_eq!(v["source"], "reconciliation");
        assert!(v["orderTimestampMs"].is_number());
        assert_eq!(v["orderTimestampMs"], v["fill"]["filledAtMs"]);
        assert_eq!(
            v["payload"]["agentFillId"],
            serde_json::Value::String("ord-123".to_string())
        );

        let keys = v
            .as_object()
            .unwrap()
            .keys()
            .map(|k| k.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            keys,
            [
                "fill",
                "fingerprintParts",
                "orderTimestampMs",
                "payload",
                "source",
                "userId",
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn build_json_includes_fee_fields_only_when_fill_supplies_them() {
        let cfg = fixture_cfg();
        let mut fill = fixture_fill();
        fill.fee_amount = Some(0.42);
        fill.fee_asset = Some("USDT".to_string());

        let v =
            build_bar_broker_fill_ingress_json(&fill, &cfg, BarFillIngestSource::Reconciliation)
                .expect("ok");

        assert_eq!(v["fill"]["feeAmount"], 0.42);
        assert_eq!(v["fill"]["feeAsset"], "USDT");
    }

    #[tokio::test]
    #[serial]
    async fn post_sends_bearer_authorization_and_json_body_not_daemon_identity() {
        std::env::set_var("STATION_ACCESS_TOKEN", "station.test.jwt");
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(INGEST_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "inserted": true,
                "fingerprint": "test-fp"
            })))
            .mount(&server)
            .await;

        let client = UpstreamClient::new(crate::UpstreamConfig {
            base_url: server.uri(),
            daemon_secret: "sec-for-test".to_string(),
        })
        .expect("upstream");
        let cfg = fixture_cfg();
        post_bar_broker_fill_ingress(
            &client,
            &cfg,
            &fixture_fill(),
            BarFillIngestSource::Reconciliation,
        )
        .await;

        let requests = server.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            requests[0]
                .headers
                .get("authorization")
                .and_then(|h| h.to_str().ok()),
            Some("Bearer station.test.jwt")
        );
        assert!(requests[0].headers.get("x-daemon-secret").is_none());
        assert!(requests[0].headers.get("x-user-id").is_none());
        assert_eq!(
            requests[0]
                .headers
                .get(CONTENT_TYPE)
                .and_then(|h| h.to_str().ok()),
            Some("application/json")
        );
        let body: serde_json::Value = serde_json::from_slice(&requests[0].body).unwrap();
        assert_eq!(body["userId"], TEST_USER);
        assert_eq!(body["fill"]["side"], "BUY");
        assert_eq!(body["fingerprintParts"]["eventType"], "fill");
        std::env::remove_var("STATION_ACCESS_TOKEN");
    }

    #[test]
    fn v1_cash_lock_allows_cnc_and_mis_only() {
        // I-N4
        assert_eq!(v1_cash_product("CNC"), Some("CNC"));
        assert_eq!(v1_cash_product("mis"), Some("MIS"));
        assert_eq!(v1_cash_product("NRML"), None);
        assert_eq!(v1_cash_product("CO"), None);
        assert_eq!(v1_cash_product("BO"), None);
        assert_eq!(v1_cash_product("FO"), None);
    }

    #[test]
    fn bar_fill_ingress_refuses_nrml_default_product() {
        // I-N4: NRML must not re-enter Station via default_product (same as FO).
        let mut cfg = fixture_cfg();
        cfg.default_product = "NRML".to_string();
        let err = build_bar_broker_fill_ingress_json(
            &fixture_fill(),
            &cfg,
            BarFillIngestSource::Reconciliation,
        )
        .expect_err("NRML default must be refused");
        assert!(err.contains("CNC or MIS"), "{err}");
    }

    #[test]
    fn bar_fill_ingress_refuses_nrml_fill_product() {
        // I-N4
        let mut fill = fixture_fill();
        fill.product = Some("NRML".to_string());
        let err = build_bar_broker_fill_ingress_json(
            &fill,
            &fixture_cfg(),
            BarFillIngestSource::Reconciliation,
        )
        .expect_err("NRML fill product must be refused");
        assert!(err.contains("NRML") || err.contains("refused"), "{err}");
    }

    #[tokio::test]
    #[serial]
    async fn from_env_refuses_nrml_default_product() {
        // I-N4
        std::env::set_var("AGENT_BAR_BROKER_FILL_INGEST", "1");
        std::env::set_var("AGENT_BAR_INGEST_USER_ID", TEST_USER);
        std::env::set_var("AGENT_BAR_BROKER_CONNECTION_ID", TEST_CONN);
        std::env::set_var("AGENT_BAR_DEFAULT_PRODUCT", "NRML");
        let cfg = BarBrokerFillIngressConfig::from_env();
        std::env::remove_var("AGENT_BAR_BROKER_FILL_INGEST");
        std::env::remove_var("AGENT_BAR_INGEST_USER_ID");
        std::env::remove_var("AGENT_BAR_BROKER_CONNECTION_ID");
        std::env::remove_var("AGENT_BAR_DEFAULT_PRODUCT");
        assert!(cfg.is_none(), "NRML default_product must disable ingress");
    }

    #[test]
    fn com_spot_fill_builds_json_without_cnc_or_mis() {
        // locks/binance-com-spot.md (fetch 2026-08-22 IST): spot has no CNC.
        // AGENT_BAR_DEFAULT_PRODUCT=MIS must not leak onto COM fills.
        let cfg = fixture_cfg();
        assert_eq!(cfg.default_product, "MIS");
        let fill = BrokerFill {
            fill_id: "com-1".to_string(),
            trade_id: "ex-1".to_string(),
            symbol: "BTCUSDT".to_string(),
            side: "BUY".to_string(),
            qty: 0.01,
            price: 65000.0,
            filled_at: Utc::now(),
            broker: "binance_com".to_string(),
            fee_amount: Some(0.1),
            fee_asset: Some("USDT".to_string()),
            currency: Some("USD".to_string()),
            product: None,
            exchange_segment: None,
            instrument_type: None,
            lot: None,
        };
        let v =
            build_bar_broker_fill_ingress_json(&fill, &cfg, BarFillIngestSource::Reconciliation)
                .expect("COM fill without CNC must still build");
        assert_eq!(v["fill"]["broker"], "binance_com");
        assert!(v["fill"].get("product").is_none());
        assert_ne!(v["fill"]["product"], "MIS");
        assert_ne!(v["fill"]["product"], "CNC");
    }

    #[test]
    fn v1_nfo_product_refuses_nrml_and_cash_labels() {
        assert_eq!(v1_nfo_product("NRML"), Some("NRML"));
        assert_eq!(v1_nfo_product("CNC"), None);
        assert_eq!(v1_nfo_product("MIS"), None);
        assert_eq!(v1_nfo_product("CO"), None);
        assert_eq!(v1_nfo_product("BO"), None);
        assert_eq!(v1_nfo_product("FO"), None);
    }

    fn kotak_cash_fill() -> BrokerFill {
        BrokerFill {
            fill_id: "cash-1".to_string(),
            trade_id: "ex-cash".to_string(),
            symbol: "RELIANCE".to_string(),
            side: "BUY".to_string(),
            qty: 1.0,
            price: 2500.0,
            filled_at: Utc::now(),
            broker: "kotak_neo".to_string(),
            fee_amount: None,
            fee_asset: None,
            currency: Some("INR".to_string()),
            product: Some("CNC".to_string()),
            exchange_segment: Some("nse_cm".to_string()),
            instrument_type: None,
            lot: None,
        }
    }

    fn kotak_nfo_fill(product: Option<&str>, qty: f64) -> BrokerFill {
        BrokerFill {
            fill_id: "nfo-1".to_string(),
            trade_id: "ex-nfo".to_string(),
            symbol: "NIFTY".to_string(),
            side: "BUY".to_string(),
            qty,
            price: 200.0,
            filled_at: Utc::now(),
            broker: "kotak_neo".to_string(),
            fee_amount: None,
            fee_asset: None,
            currency: Some("INR".to_string()),
            product: product.map(str::to_string),
            exchange_segment: Some("nse_fo".to_string()),
            instrument_type: None,
            lot: None,
        }
    }

    /// Lock sample `NIFTY2692221000PE` — lot 65 from slice-2 master, not invented 1/50.
    fn kotak_nfo_fill_from_lock_master(product: Option<&str>, qty: f64, price: f64) -> BrokerFill {
        let master = crate::kotak_nfo_scrip::KotakNfoScripMaster::from_csv_bytes(
            include_str!("../fixtures/kotak/nse_fo_header.csv").as_bytes(),
        )
        .expect("lock nse_fo fixture");
        let row = master.get(56526).expect("lock sample token 56526");
        assert_eq!(row.lot, 65, "lock sample lot is 65, not 1");
        let opt = row.option_type.trim().to_ascii_uppercase();
        let instrument_type = match opt.as_str() {
            "CE" | "PE" | "FUT" => Some(opt),
            _ => None,
        };
        let mut fill = kotak_nfo_fill(product, qty);
        fill.symbol = row.trading_symbol.clone();
        fill.price = price;
        fill.lot = Some(row.lot);
        fill.instrument_type = instrument_type;
        fill
    }

    #[test]
    fn kotak_neo_cash_segment_still_uses_cnc_mis_lock() {
        let cfg = fixture_cfg();
        let fill = kotak_cash_fill();
        let v =
            build_bar_broker_fill_ingress_json(&fill, &cfg, BarFillIngestSource::Reconciliation)
                .expect("cash nse_cm CNC must ingest");
        assert_eq!(v["fill"]["product"], "MIS");
        assert_eq!(v["fill"]["broker"], "kotak_neo");

        let mut missing_seg = fill.clone();
        missing_seg.exchange_segment = None;
        build_bar_broker_fill_ingress_json(&missing_seg, &cfg, BarFillIngestSource::Reconciliation)
            .expect("kotak_neo with missing segment stays cash");

        let mut bse = fill.clone();
        bse.exchange_segment = Some("bse_cm".to_string());
        build_bar_broker_fill_ingress_json(&bse, &cfg, BarFillIngestSource::Reconciliation)
            .expect("bse_cm stays cash");
    }

    #[test]
    fn nse_fo_nrml_fill_ingests_as_nrml_not_mis() {
        let cfg = fixture_cfg();
        assert_eq!(cfg.default_product, "MIS");
        let fill = kotak_nfo_fill_from_lock_master(Some("NRML"), 1.0, 200.0);
        let v =
            build_bar_broker_fill_ingress_json(&fill, &cfg, BarFillIngestSource::Reconciliation)
                .expect("nse_fo + NRML must ingest");
        assert_eq!(v["fill"]["product"], "NRML");
        assert_ne!(v["fill"]["product"], "MIS");
        assert_ne!(v["fill"]["product"], "CNC");
        assert_eq!(v["fill"]["currency"], "INR");
        assert_eq!(v["fill"]["lot"], 65);
        assert_eq!(v["fill"]["instrumentType"], "PE");
        assert_ne!(v["fill"]["instrumentType"], "NRML");
        assert_ne!(v["fill"]["instrumentType"], "NSE");
    }

    #[test]
    fn nse_fo_options_qty_gt1_is_not_ingested_as_cash() {
        let cfg = fixture_cfg();
        let fill = kotak_nfo_fill_from_lock_master(Some("NRML"), 15.0, 200.0);
        let v =
            build_bar_broker_fill_ingress_json(&fill, &cfg, BarFillIngestSource::Reconciliation)
                .expect("qty>1 nse_fo NRML must ingest as NFO, not cash MIS");
        assert_eq!(v["fill"]["product"], "NRML");
        assert_ne!(v["fill"]["product"], "MIS");
        assert_ne!(v["fill"]["product"], "CNC");
        assert_eq!(v["fill"]["quantity"], 15.0);
        assert_eq!(v["fill"]["lot"], 65);

        let no_product = kotak_nfo_fill(None, 15.0);
        let err = build_bar_broker_fill_ingress_json(
            &no_product,
            &cfg,
            BarFillIngestSource::Reconciliation,
        )
        .expect_err("nse_fo without product must not inherit cash MIS");
        assert!(err.contains("nfo lock"), "{err}");
        assert!(err.contains("MIS") || err.contains("signed"), "{err}");
    }

    #[test]
    fn bse_fo_and_other_unsigned_fo_segments_refuse() {
        let cfg = fixture_cfg();
        for segment in ["bse_fo", "cde_fo", "mcx_fo"] {
            let mut fill = kotak_nfo_fill_from_lock_master(Some("NRML"), 1.0, 200.0);
            fill.exchange_segment = Some(segment.to_string());
            let err = build_bar_broker_fill_ingress_json(
                &fill,
                &cfg,
                BarFillIngestSource::Reconciliation,
            )
            .expect_err("unsigned FO must not ingest as cash or NFO");
            assert!(err.contains("nse_fo"), "{segment}: {err}");
            assert!(!err.contains("MIS default"), "{segment}: {err}");
        }
    }

    #[test]
    fn nfo_qty2_lot65_ingress_carries_identity_not_pnl() {
        // Lock golden: qty=2, lot=65. PnL formula NOT SPECIFIED — do not assert a rupee number.
        let cfg = fixture_cfg();
        let entry = kotak_nfo_fill_from_lock_master(Some("NRML"), 2.0, 10.0);
        assert!(entry.side.eq_ignore_ascii_case("BUY"));
        let entry_body =
            build_bar_broker_fill_ingress_json(&entry, &cfg, BarFillIngestSource::Reconciliation)
                .expect("entry");
        let mut exit = kotak_nfo_fill_from_lock_master(Some("NRML"), 2.0, 11.0);
        exit.side = "SELL".to_string();
        exit.fill_id = "nfo-exit".to_string();
        let exit_body =
            build_bar_broker_fill_ingress_json(&exit, &cfg, BarFillIngestSource::Reconciliation)
                .expect("exit");

        assert_eq!(entry_body["fill"]["quantity"], 2.0);
        assert_eq!(entry_body["fill"]["lot"], 65);
        assert_eq!(entry_body["fill"]["fillPrice"], 10.0);
        assert_eq!(exit_body["fill"]["quantity"], 2.0);
        assert_eq!(exit_body["fill"]["lot"], 65);
        assert_eq!(exit_body["fill"]["fillPrice"], 11.0);
        assert!(entry_body["fill"].get("realizedPnl").is_none());
        assert!(entry_body["fill"].get("netPnl").is_none());
        assert!(exit_body["fill"].get("realizedPnlUsd").is_none());
    }

    #[test]
    fn nfo_dual_no_blend_is_inr_not_fx_multiplied() {
        let cfg = fixture_cfg();
        let fill = kotak_nfo_fill_from_lock_master(Some("NRML"), 2.0, 10.0);
        let v =
            build_bar_broker_fill_ingress_json(&fill, &cfg, BarFillIngestSource::Reconciliation)
                .expect("nfo inr");
        const EXCHANGE_RATE: f64 = 83.0;
        assert_eq!(v["fill"]["currency"], "INR");
        assert_eq!(v["fill"]["quantity"], 2.0);
        assert_eq!(v["fill"]["fillPrice"], 10.0);
        assert_eq!(v["fill"]["lot"], 65);
        assert_ne!(v["fill"]["quantity"], 2.0 * EXCHANGE_RATE);
        assert_ne!(v["fill"]["fillPrice"], 10.0 * EXCHANGE_RATE);
        assert!(v["fill"].get("exchangeRate").is_none());
        assert!(v["fill"].get("netPnl").is_none());
    }

    #[test]
    fn com_spot_does_not_inherit_nfo_lot_or_cash_product() {
        let cfg = fixture_cfg();
        let mut fill = BrokerFill {
            fill_id: "com-opt".to_string(),
            trade_id: "ex-opt".to_string(),
            symbol: "BTC-240927-60000-C".to_string(),
            side: "BUY".to_string(),
            qty: 1.0,
            price: 100.0,
            filled_at: Utc::now(),
            broker: "binance_com".to_string(),
            fee_amount: None,
            fee_asset: None,
            currency: Some("USD".to_string()),
            product: None,
            exchange_segment: None,
            instrument_type: Some("CE".to_string()),
            lot: Some(65),
        };
        let v =
            build_bar_broker_fill_ingress_json(&fill, &cfg, BarFillIngestSource::Reconciliation)
                .expect("com spot");
        assert!(v["fill"].get("lot").is_none());
        assert!(v["fill"].get("instrumentType").is_none());
        assert!(v["fill"].get("product").is_none());
        fill.product = Some("NRML".to_string());
        let v =
            build_bar_broker_fill_ingress_json(&fill, &cfg, BarFillIngestSource::Reconciliation)
                .expect("spot still builds with NRML label stripped");
        assert!(v["fill"].get("product").is_none());
    }
}
