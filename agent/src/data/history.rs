//! S2 history extract — widget-first. No Neon. No ingestSignal.
//!
//! Licensed series is Binance.com public klines (`historical_series`).
//! Yahoo is research_segment + `rights_forbid_canonical` and is never persisted.

use super::binance_coinm_klines::{validate_coinm_kline_request, DEFAULT_COINM_HISTORY_INTERVAL};
use super::binance_coinm_ticker::normalize_coinm_instrument;
use super::binance_klines::{validate_kline_request, HistorySeries, DEFAULT_HISTORY_INTERVAL};
use super::binance_options_klines::{
    validate_options_kline_request, DEFAULT_OPTIONS_HISTORY_INTERVAL,
};
use super::binance_options_public::{is_dated_option_contract, normalize_options_instrument};
use super::binance_usdm_klines::{validate_usdm_kline_request, DEFAULT_USDM_HISTORY_INTERVAL};
use super::binance_usdm_ticker::normalize_usdm_instrument;
use super::descriptor::{
    BINANCE_COM_ADAPTER_ID, BINANCE_COM_COINM_BOOK_ID, BINANCE_COM_USDM_BOOK_ID,
};
use super::historybook::HistoryBook;
use super::identity::{CapabilityId, Family, Identity, Physics};
use super::rights::Rights;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HistoryStatus {
    Success,
    Unavailable,
    ResearchSegment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HistoryEnvelope {
    pub identity: Identity,
    pub instrument_id: String,
    pub status: HistoryStatus,
    pub data: Option<serde_json::Value>,
    pub rights: Rights,
    pub ineligible: Vec<String>,
    pub research: bool,
    pub canonical: bool,
    pub persist_canonical: bool,
    pub provenance: HistoryProvenance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HistoryProvenance {
    pub adapter_id: String,
}

fn ohlcv_identity() -> Identity {
    Identity::new(
        Family::Market,
        CapabilityId::new("ohlcv").expect("canonical ohlcv id"),
        Physics::HistoricalSeries,
    )
}

fn persist_canonical(rights: Rights) -> bool {
    rights.store
}

fn empty_licensed(
    instrument_id: String,
    adapter_id: &str,
    ineligible: Vec<String>,
) -> HistoryEnvelope {
    let rights = Rights::research_fetch_only();
    HistoryEnvelope {
        identity: ohlcv_identity(),
        instrument_id,
        status: HistoryStatus::Unavailable,
        data: None,
        rights,
        ineligible,
        research: true,
        canonical: false,
        persist_canonical: persist_canonical(rights),
        provenance: HistoryProvenance {
            adapter_id: adapter_id.to_string(),
        },
    }
}

/// Yahoo-shaped (or unnamed free chart) never becomes canonical product history.
pub fn extract_history(instrument_id: &str, source: Option<&str>) -> HistoryEnvelope {
    let identity = ohlcv_identity();
    let instrument_id = instrument_id.trim().to_ascii_lowercase();
    let yahoo = source
        .map(|s| {
            let n = s.trim().to_ascii_lowercase();
            n == "yahoo" || n == "yahoo_chart"
        })
        .unwrap_or(false);

    if yahoo {
        return HistoryEnvelope {
            identity,
            instrument_id,
            status: HistoryStatus::ResearchSegment,
            data: None,
            rights: Rights::research_fetch_only(),
            ineligible: vec!["rights_forbid_canonical".to_string()],
            research: true,
            canonical: false,
            persist_canonical: false,
            provenance: HistoryProvenance {
                adapter_id: "yahoo_chart".to_string(),
            },
        };
    }

    empty_licensed(instrument_id, "", Vec::new())
}

/// Licensed Binance.com series from the history store. TickBook is not consulted.
pub fn extract_licensed_history(
    book: &HistoryBook,
    instrument_id: &str,
    interval: Option<&str>,
    limit: Option<u32>,
) -> HistoryEnvelope {
    if is_dated_option_contract(instrument_id) {
        return empty_licensed(
            normalize_options_instrument(instrument_id),
            BINANCE_COM_ADAPTER_ID,
            Vec::new(),
        );
    }
    let instrument_id = super::binance_public::normalize_quote_instrument(instrument_id);
    let interval = interval
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_HISTORY_INTERVAL);
    match validate_kline_request(interval, limit) {
        Err(refuse) => {
            return empty_licensed(
                instrument_id,
                BINANCE_COM_ADAPTER_ID,
                vec![refuse.as_ineligible().to_string()],
            );
        }
        Ok(_) => {}
    }
    // Specified id must not inherit another pair's series. Empty id may use
    // first_for_adapter for boot/CI obtain without a query.
    let row = if instrument_id.is_empty() {
        book.first_for_adapter(BINANCE_COM_ADAPTER_ID)
    } else {
        book.get(BINANCE_COM_ADAPTER_ID, &instrument_id, interval)
    };
    let Some(row) = row else {
        return empty_licensed(instrument_id, BINANCE_COM_ADAPTER_ID, Vec::new());
    };
    if is_dated_option_contract(&row.instrument_id) {
        return empty_licensed(instrument_id, BINANCE_COM_ADAPTER_ID, Vec::new());
    }
    licensed_success(row, "binance_klines")
}

/// Named options book series. Mixed-case id. Never `first_for_adapter` (that
/// would serve a leftover spot pair). Empty series stays unavailable.
pub fn extract_options_history(
    book: &HistoryBook,
    instrument_id: &str,
    interval: Option<&str>,
    limit: Option<u32>,
) -> HistoryEnvelope {
    let instrument_id = normalize_options_instrument(instrument_id);
    if instrument_id.is_empty() || !is_dated_option_contract(&instrument_id) {
        return empty_licensed(instrument_id, BINANCE_COM_ADAPTER_ID, Vec::new());
    }
    let interval = interval
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_OPTIONS_HISTORY_INTERVAL);
    match validate_options_kline_request(interval, limit) {
        Err(refuse) => {
            return empty_licensed(
                instrument_id,
                BINANCE_COM_ADAPTER_ID,
                vec![refuse.as_ineligible().to_string()],
            );
        }
        Ok(_) => {}
    }
    let Some(row) = book.get(BINANCE_COM_ADAPTER_ID, &instrument_id, interval) else {
        return empty_licensed(instrument_id, BINANCE_COM_ADAPTER_ID, Vec::new());
    };
    if !is_dated_option_contract(&row.instrument_id) {
        return empty_licensed(instrument_id, BINANCE_COM_ADAPTER_ID, Vec::new());
    }
    licensed_success(row, "eapi_klines")
}

/// Named USDM book series. Uppercase id. Never `first_for_adapter` (spot leftover).
/// Empty series stays unavailable. Store key is `binance-com-usdm`, not COM adapter.
pub fn extract_usdm_history(
    book: &HistoryBook,
    instrument_id: &str,
    interval: Option<&str>,
    limit: Option<u32>,
) -> HistoryEnvelope {
    let instrument_id = normalize_usdm_instrument(instrument_id);
    if instrument_id.is_empty()
        || instrument_id.contains('|')
        || is_dated_option_contract(&instrument_id)
    {
        return empty_licensed(instrument_id, BINANCE_COM_USDM_BOOK_ID, Vec::new());
    }
    let interval = interval
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_USDM_HISTORY_INTERVAL);
    match validate_usdm_kline_request(interval, limit) {
        Err(refuse) => {
            return empty_licensed(
                instrument_id,
                BINANCE_COM_USDM_BOOK_ID,
                vec![refuse.as_ineligible().to_string()],
            );
        }
        Ok(_) => {}
    }
    let Some(row) = book.get(BINANCE_COM_USDM_BOOK_ID, &instrument_id, interval) else {
        return empty_licensed(instrument_id, BINANCE_COM_USDM_BOOK_ID, Vec::new());
    };
    licensed_success(row, "fapi_klines")
}

/// Named Coin-M book series. Venue case (`BTCUSD_PERP`). Never fapi / spot.
pub fn extract_coinm_history(
    book: &HistoryBook,
    instrument_id: &str,
    interval: Option<&str>,
    limit: Option<u32>,
) -> HistoryEnvelope {
    let instrument_id = normalize_coinm_instrument(instrument_id);
    if instrument_id.is_empty()
        || instrument_id.contains('|')
        || is_dated_option_contract(&instrument_id)
    {
        return empty_licensed(instrument_id, BINANCE_COM_COINM_BOOK_ID, Vec::new());
    }
    let interval = interval
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_COINM_HISTORY_INTERVAL);
    match validate_coinm_kline_request(interval, limit) {
        Err(refuse) => {
            return empty_licensed(
                instrument_id,
                BINANCE_COM_COINM_BOOK_ID,
                vec![refuse.as_ineligible().to_string()],
            );
        }
        Ok(_) => {}
    }
    let Some(row) = book.get(BINANCE_COM_COINM_BOOK_ID, &instrument_id, interval) else {
        return empty_licensed(instrument_id, BINANCE_COM_COINM_BOOK_ID, Vec::new());
    };
    licensed_success(row, "dapi_klines")
}

/// Fixture gap vendor series. Separate adapter_id from COM klines (DualNoBlend).
pub fn extract_gap_vendor_history(
    book: &HistoryBook,
    adapter_id: &str,
    instrument_id: &str,
    interval: Option<&str>,
) -> HistoryEnvelope {
    let instrument_id = super::binance_public::normalize_quote_instrument(instrument_id);
    let interval = interval
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(DEFAULT_HISTORY_INTERVAL);
    let row = if instrument_id.is_empty() {
        book.first_for_adapter(adapter_id)
    } else {
        book.get(adapter_id, &instrument_id, interval)
    };
    let Some(row) = row else {
        return empty_licensed(instrument_id, adapter_id, Vec::new());
    };
    licensed_success(row, adapter_id)
}

/// Obtain(history) for a declared gap vendor — never COM, never Kotak.
pub fn gap_history_obtain_data(
    envelope: &HistoryEnvelope,
    adapter_id: &str,
) -> Option<serde_json::Value> {
    if envelope.status != HistoryStatus::Success {
        return None;
    }
    if envelope.provenance.adapter_id != adapter_id {
        return None;
    }
    let data = envelope.data.as_ref()?;
    if data.get("candles").and_then(|c| c.as_array())?.is_empty() {
        return None;
    }
    Some(data.clone())
}

fn licensed_success(row: &HistorySeries, source: &str) -> HistoryEnvelope {
    let rights = Rights::research_fetch_only();
    HistoryEnvelope {
        identity: ohlcv_identity(),
        instrument_id: row.instrument_id.clone(),
        status: HistoryStatus::Success,
        data: Some(series_json(row, source)),
        rights,
        ineligible: Vec::new(),
        research: true,
        canonical: false,
        persist_canonical: persist_canonical(rights),
        provenance: HistoryProvenance {
            adapter_id: row.adapter_id.clone(),
        },
    }
}

fn series_json(row: &HistorySeries, source: &str) -> serde_json::Value {
    let last_close = row.candles.last().map(|c| c.close.clone());
    serde_json::json!({
        "identity": ohlcv_identity(),
        "instrument_id": row.instrument_id,
        "interval": row.interval,
        "candles": row.candles,
        "last_close": last_close,
        "source": source,
        "transport": row.transport,
    })
}

/// Obtain(history) succeeds only with a non-empty licensed series — never Yahoo.
pub fn history_obtain_data(envelope: &HistoryEnvelope) -> Option<serde_json::Value> {
    if envelope.status != HistoryStatus::Success {
        return None;
    }
    if envelope.provenance.adapter_id != BINANCE_COM_ADAPTER_ID {
        return None;
    }
    if envelope
        .ineligible
        .iter()
        .any(|r| r == "rights_forbid_canonical")
    {
        return None;
    }
    let data = envelope.data.as_ref()?;
    if data.get("candles").and_then(|c| c.as_array())?.is_empty() {
        return None;
    }
    Some(data.clone())
}

/// Named USDM/Coin-M series live under the book id, not the COM adapter slug.
pub fn futures_history_obtain_data(envelope: &HistoryEnvelope) -> Option<serde_json::Value> {
    if envelope.status != HistoryStatus::Success {
        return None;
    }
    if envelope.provenance.adapter_id != BINANCE_COM_USDM_BOOK_ID
        && envelope.provenance.adapter_id != BINANCE_COM_COINM_BOOK_ID
    {
        return None;
    }
    if envelope
        .ineligible
        .iter()
        .any(|r| r == "rights_forbid_canonical")
    {
        return None;
    }
    let data = envelope.data.as_ref()?;
    if data.get("candles").and_then(|c| c.as_array())?.is_empty() {
        return None;
    }
    Some(data.clone())
}

pub fn apply_history_series(book: &mut HistoryBook, series: HistorySeries) {
    book.upsert(series);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::binance_klines::{series_from_klines_json, KLINE_LIMIT_DEFAULT};
    use crate::data::identity::{Family, Physics};
    use crate::data::tick::Transport;

    const FIXTURE: &str = include_str!("../../fixtures/binance/klines.json");

    #[test]
    fn yahoo_shaped_forbids_canonical() {
        let env = extract_history("btcusdt", Some("yahoo"));
        assert_eq!(env.status, HistoryStatus::ResearchSegment);
        assert!(env.data.is_none());
        assert!(env
            .ineligible
            .iter()
            .any(|r| r == "rights_forbid_canonical"));
        assert!(!env.canonical);
        assert!(!env.persist_canonical);
        assert_eq!(env.identity.family, Family::Market);
        assert_eq!(env.identity.capability_id.as_str(), "ohlcv");
        assert_eq!(env.identity.physics, Physics::HistoricalSeries);
        assert!(history_obtain_data(&env).is_none());
    }

    #[test]
    fn no_licensed_series_is_unavailable() {
        let env = extract_history("btcusdt", None);
        assert_eq!(env.status, HistoryStatus::Unavailable);
        assert!(env.data.is_none());
        assert!(env.ineligible.is_empty());
        let book = HistoryBook::new();
        let licensed = extract_licensed_history(&book, "btcusdt", Some("1m"), None);
        assert_eq!(licensed.status, HistoryStatus::Unavailable);
        assert!(licensed.data.is_none());
        assert_eq!(licensed.provenance.adapter_id, BINANCE_COM_ADAPTER_ID);
        assert!(!licensed
            .ineligible
            .iter()
            .any(|r| r == "insufficient_retention"));
        assert!(history_obtain_data(&licensed).is_none());
    }

    #[test]
    fn fixture_series_extracts_as_licensed_binance() {
        let mut book = HistoryBook::new();
        let series = series_from_klines_json(FIXTURE, "BTCUSDT", "1m", Transport::Fixture).unwrap();
        apply_history_series(&mut book, series);
        let env = extract_licensed_history(&book, "BTCUSDT", Some("1m"), None);
        assert_eq!(env.status, HistoryStatus::Success);
        assert_eq!(env.instrument_id, "btcusdt");
        assert_eq!(env.provenance.adapter_id, "binance_com");
        assert_eq!(env.identity.physics, Physics::HistoricalSeries);
        let data = history_obtain_data(&env).expect("licensed series");
        assert_eq!(data["last_close"], "0.01590000");
        assert_eq!(data["source"], "binance_klines");
        assert_eq!(data["identity"]["capability_id"], "ohlcv");
        assert_eq!(data["identity"]["physics"], "historical_series");
        assert!(!env.persist_canonical);
        assert!(!env.ineligible.iter().any(|r| r == "insufficient_retention"));
        assert!(HistoryBook::new()
            .get("binance_com", "btcusdt", "1m")
            .is_none());
    }

    #[test]
    fn specified_missing_instrument_does_not_take_another_pair() {
        let mut book = HistoryBook::new();
        let series = series_from_klines_json(FIXTURE, "ETHUSDT", "1m", Transport::Fixture).unwrap();
        apply_history_series(&mut book, series);
        let env = extract_licensed_history(&book, "btcusdt", Some("1m"), None);
        assert_eq!(env.status, HistoryStatus::Unavailable);
        assert_eq!(env.instrument_id, "btcusdt");
        assert!(env.data.is_none());
        assert!(history_obtain_data(&env).is_none());
        let eth = extract_licensed_history(&book, "ethusdt", Some("1m"), None);
        assert_eq!(eth.status, HistoryStatus::Success);
        assert_eq!(eth.data.as_ref().unwrap()["last_close"], "0.01590000");
    }

    #[test]
    fn specified_instrument_extracts_its_own_series() {
        let mut book = HistoryBook::new();
        let series = series_from_klines_json(FIXTURE, "BTCUSDT", "1m", Transport::Fixture).unwrap();
        apply_history_series(&mut book, series);
        let env = extract_licensed_history(&book, "btcusdt", Some("1m"), None);
        assert_eq!(env.status, HistoryStatus::Success);
        assert_eq!(env.instrument_id, "btcusdt");
        assert_eq!(env.data.as_ref().unwrap()["last_close"], "0.01590000");
    }

    #[test]
    fn empty_instrument_may_use_first_for_adapter() {
        let mut book = HistoryBook::new();
        let series = series_from_klines_json(FIXTURE, "ETHUSDT", "1m", Transport::Fixture).unwrap();
        apply_history_series(&mut book, series);
        let env = extract_licensed_history(&book, "", Some("1m"), None);
        assert_eq!(env.status, HistoryStatus::Success);
        assert_eq!(env.instrument_id, "ethusdt");
        assert_eq!(env.data.as_ref().unwrap()["last_close"], "0.01590000");
    }

    #[test]
    fn licensed_history_is_ohlcv_not_force_order() {
        let env = extract_licensed_history(&HistoryBook::new(), "btcusdt", Some("1m"), None);
        assert_eq!(env.identity.capability_id.as_str(), "ohlcv");
        assert_ne!(env.identity.capability_id.as_str(), "force_order");
        assert_eq!(env.identity.physics, Physics::HistoricalSeries);
        assert!(!env.canonical);
    }

    #[test]
    fn unsupported_interval_and_range_are_ineligible_null() {
        let book = HistoryBook::new();
        let interval = extract_licensed_history(&book, "btcusdt", Some("2m"), None);
        assert_eq!(interval.status, HistoryStatus::Unavailable);
        assert!(interval.data.is_none());
        assert_eq!(interval.ineligible, ["unsupported_interval"]);
        assert!(!interval
            .ineligible
            .iter()
            .any(|r| r == "insufficient_retention"));
        let range = extract_licensed_history(&book, "btcusdt", Some("1m"), Some(1001));
        assert!(range.data.is_none());
        assert_eq!(range.ineligible, ["unsupported_range"]);
        assert_eq!(
            extract_licensed_history(&book, "btcusdt", Some("1m"), Some(KLINE_LIMIT_DEFAULT))
                .ineligible
                .len(),
            0
        );
    }

    #[test]
    fn options_history_keeps_mixed_case_and_eapi_source() {
        let json = include_str!("../../fixtures/binance/options_klines.json");
        let series = crate::data::series_from_eapi_klines_json(
            json,
            "BTC-200730-9000-C",
            "1m",
            Transport::Fixture,
        )
        .unwrap();
        let mut book = HistoryBook::new();
        apply_history_series(&mut book, series);
        let env = extract_options_history(&book, "BTC-200730-9000-C", Some("1m"), None);
        assert_eq!(env.status, HistoryStatus::Success);
        assert_eq!(env.instrument_id, "BTC-200730-9000-C");
        assert_eq!(env.data.as_ref().unwrap()["source"], "eapi_klines");
        assert_ne!(env.data.as_ref().unwrap()["source"], "binance_klines");
        assert_eq!(
            env.data.as_ref().unwrap()["candles"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(history_obtain_data(&env).is_some());
        let miss = extract_options_history(&book, "btc-200730-9000-c", Some("1m"), None);
        assert_eq!(miss.status, HistoryStatus::Unavailable);
        assert!(history_obtain_data(&miss).is_none());
    }

    #[test]
    fn options_history_does_not_inherit_spot_series() {
        let json = include_str!("../../fixtures/binance/klines.json");
        let series = series_from_klines_json(json, "BTCUSDT", "1m", Transport::Fixture).unwrap();
        let mut book = HistoryBook::new();
        apply_history_series(&mut book, series);
        let env = extract_options_history(&book, "BTC-200730-9000-C", Some("1m"), None);
        assert_eq!(env.status, HistoryStatus::Unavailable);
        assert!(env.data.is_none());
        let empty = extract_options_history(&HistoryBook::new(), "", Some("1m"), None);
        assert_eq!(empty.status, HistoryStatus::Unavailable);
        assert!(empty.data.is_none());
    }

    #[test]
    fn options_one_s_is_unsupported_not_resample() {
        let env =
            extract_options_history(&HistoryBook::new(), "BTC-200730-9000-C", Some("1s"), None);
        assert_eq!(env.status, HistoryStatus::Unavailable);
        assert_eq!(env.ineligible, ["unsupported_interval"]);
        assert!(history_obtain_data(&env).is_none());
    }

    #[test]
    fn spot_history_does_not_inherit_options_series() {
        let json = include_str!("../../fixtures/binance/options_klines.json");
        let series = crate::data::series_from_eapi_klines_json(
            json,
            "BTC-200730-9000-C",
            "1m",
            Transport::Fixture,
        )
        .unwrap();
        let mut book = HistoryBook::new();
        apply_history_series(&mut book, series);
        let empty = extract_licensed_history(&book, "", Some("1m"), None);
        assert_eq!(empty.status, HistoryStatus::Unavailable);
        assert!(empty.data.is_none());
        let dated = extract_licensed_history(&book, "BTC-200730-9000-C", Some("1m"), None);
        assert_eq!(dated.status, HistoryStatus::Unavailable);
        assert!(dated.data.is_none());
        assert_ne!(
            dated.data.as_ref().and_then(|d| d.get("source")),
            Some(&serde_json::json!("eapi_klines"))
        );
    }

    #[test]
    fn gap_vendor_series_is_not_binance_klines() {
        use crate::data::binance_klines::HistoryCandle;
        use crate::data::LICENSED_HISTORY_ADAPTER_ID;
        let mut book = HistoryBook::new();
        apply_history_series(
            &mut book,
            HistorySeries {
                instrument_id: "nse_cm|2885".into(),
                adapter_id: LICENSED_HISTORY_ADAPTER_ID.into(),
                interval: DEFAULT_HISTORY_INTERVAL.into(),
                candles: vec![HistoryCandle {
                    open_time_ms: 1_700_000_000_000,
                    open: "1400.00".into(),
                    high: "1402.00".into(),
                    low: "1398.00".into(),
                    close: "1401.00".into(),
                    volume: "10".into(),
                    close_time_ms: 1_700_000_060_000,
                }],
                transport: Transport::Fixture,
            },
        );
        let env = extract_gap_vendor_history(
            &book,
            LICENSED_HISTORY_ADAPTER_ID,
            "",
            Some(DEFAULT_HISTORY_INTERVAL),
        );
        assert_eq!(env.status, HistoryStatus::Success);
        assert_eq!(env.provenance.adapter_id, LICENSED_HISTORY_ADAPTER_ID);
        let data = env.data.as_ref().expect("vendor series");
        assert_eq!(data["source"], LICENSED_HISTORY_ADAPTER_ID);
        assert_ne!(data["source"], "binance_klines");
        assert!(gap_history_obtain_data(&env, LICENSED_HISTORY_ADAPTER_ID).is_some());
    }

    #[test]
    fn usdm_history_keeps_fapi_source_and_does_not_inherit_spot() {
        let json = r#"[[1499040000000,"950","1100","900","1000","100",1499040059999,"2",10,"100","10000","0"]]"#;
        let series =
            crate::data::series_from_fapi_klines_json(json, "BTCUSDT", "1m", Transport::Fixture)
                .unwrap();
        let mut book = HistoryBook::new();
        apply_history_series(&mut book, series);
        let env = extract_usdm_history(&book, "BTCUSDT", Some("1m"), None);
        assert_eq!(env.status, HistoryStatus::Success);
        assert_eq!(env.instrument_id, "BTCUSDT");
        assert_eq!(env.data.as_ref().unwrap()["source"], "fapi_klines");
        assert_ne!(env.data.as_ref().unwrap()["source"], "binance_klines");
        assert_eq!(env.provenance.adapter_id, BINANCE_COM_USDM_BOOK_ID);
        assert!(futures_history_obtain_data(&env).is_some());
        assert!(history_obtain_data(&env).is_none());

        let spot = series_from_klines_json(
            include_str!("../../fixtures/binance/klines.json"),
            "BTCUSDT",
            "1m",
            Transport::Fixture,
        )
        .unwrap();
        let mut spot_only = HistoryBook::new();
        apply_history_series(&mut spot_only, spot);
        let miss = extract_usdm_history(&spot_only, "BTCUSDT", Some("1m"), None);
        assert_eq!(miss.status, HistoryStatus::Unavailable);
        assert!(futures_history_obtain_data(&miss).is_none());

        let licensed = extract_licensed_history(&book, "BTCUSDT", Some("1m"), None);
        assert_eq!(licensed.status, HistoryStatus::Unavailable);
        assert!(history_obtain_data(&licensed).is_none());
    }

    #[test]
    fn usdm_one_s_is_unsupported_not_resample() {
        let env = extract_usdm_history(&HistoryBook::new(), "BTCUSDT", Some("1s"), None);
        assert_eq!(env.status, HistoryStatus::Unavailable);
        assert_eq!(env.ineligible, ["unsupported_interval"]);
        assert!(futures_history_obtain_data(&env).is_none());
    }

    #[test]
    fn coinm_history_keeps_dapi_source_and_does_not_inherit_usdm() {
        let json = r#"[[1499040000000,"950","1100","900","1000","100",1499040059999,"2",10,"100","10000","0"]]"#;
        let usdm =
            crate::data::series_from_fapi_klines_json(json, "BTCUSDT", "1m", Transport::Fixture)
                .unwrap();
        let coinm = crate::data::series_from_dapi_klines_json(
            json,
            "BTCUSD_PERP",
            "1m",
            Transport::Fixture,
        )
        .unwrap();
        let mut book = HistoryBook::new();
        apply_history_series(&mut book, usdm);
        apply_history_series(&mut book, coinm);
        let env = extract_coinm_history(&book, "BTCUSD_PERP", Some("1m"), None);
        assert_eq!(env.status, HistoryStatus::Success);
        assert_eq!(env.instrument_id, "BTCUSD_PERP");
        assert_eq!(env.data.as_ref().unwrap()["source"], "dapi_klines");
        assert_ne!(env.data.as_ref().unwrap()["source"], "fapi_klines");
        assert_eq!(env.provenance.adapter_id, BINANCE_COM_COINM_BOOK_ID);
        let usdm_env = extract_usdm_history(&book, "BTCUSDT", Some("1m"), None);
        assert_eq!(usdm_env.data.as_ref().unwrap()["source"], "fapi_klines");
        let miss = extract_coinm_history(&book, "BTCUSDT", Some("1m"), None);
        assert_eq!(miss.status, HistoryStatus::Unavailable);
    }
}
