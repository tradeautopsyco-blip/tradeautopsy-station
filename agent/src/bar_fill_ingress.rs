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
        let default_product = std::env::var("AGENT_BAR_DEFAULT_PRODUCT")
            .unwrap_or_else(|_| "MIS".to_string())
            .to_uppercase();
        if !matches!(default_product.as_str(), "MIS" | "NRML" | "CNC") {
            tracing::warn!(
                product = %default_product,
                "AGENT_BAR_DEFAULT_PRODUCT must be MIS | NRML | CNC — defaulting to MIS"
            );
            return Some(Self {
                user_id,
                broker_connection_id,
                default_product: "MIS".to_string(),
            });
        }
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
    product: &'a str,
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

/// Builds the JSON body matching `app/api/internal/bar/v1/broker-ingest/fill/route.ts` `bodySchema`.
pub(crate) fn build_bar_broker_fill_ingress_json(
    fill: &BrokerFill,
    cfg: &BarBrokerFillIngressConfig,
    source: BarFillIngestSource,
) -> Result<serde_json::Value, &'static str> {
    let side = normalize_side(&fill.side).ok_or("side must be BUY or SELL")?;
    if !fill.qty.is_finite() || fill.qty <= 0.0 {
        return Err("quantity must be finite and positive");
    }
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
            product: cfg.default_product.as_str(),
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
    let resp = match client
        .http
        .post(&url)
        .header("x-daemon-secret", &client.config.daemon_secret)
        .header("x-user-id", &cfg.user_id)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .json(&body)
        .send()
        .await
    {
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
    async fn post_sends_secret_and_user_headers_and_json_body() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path(INGEST_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "inserted": true,
                "fingerprint": "test-fp"
            })))
            .mount(&server)
            .await;

        let client = UpstreamClient {
            config: crate::UpstreamConfig {
                base_url: server.uri(),
                daemon_secret: "sec-for-test".to_string(),
            },
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .expect("client"),
        };
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
                .get("x-daemon-secret")
                .and_then(|h| h.to_str().ok()),
            Some("sec-for-test")
        );
        assert_eq!(
            requests[0]
                .headers
                .get("x-user-id")
                .and_then(|h| h.to_str().ok()),
            Some(TEST_USER)
        );
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
    }
}
