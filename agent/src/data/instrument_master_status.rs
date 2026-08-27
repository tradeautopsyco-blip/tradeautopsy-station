//! Shared instrument-master fetch status (Binance exchangeInfo / Kotak cash CSV).
//! `last_error` is a class token only — never a header, URL with secrets, or body dump.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InstrumentMasterPhase {
    Idle,
    Loading,
    Loaded,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstrumentMasterErrorClass {
    FilePathsHttp,
    FilePathsJson,
    CsvHttp,
    CsvAllowlist,
    CsvParse,
    ExchangeInfoHttp,
    Empty,
}

impl InstrumentMasterErrorClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FilePathsHttp => "file_paths_http",
            Self::FilePathsJson => "file_paths_json",
            Self::CsvHttp => "csv_http",
            Self::CsvAllowlist => "csv_allowlist",
            Self::CsvParse => "csv_parse",
            Self::ExchangeInfoHttp => "exchange_info_http",
            Self::Empty => "empty",
        }
    }
}

#[derive(Debug)]
pub struct InstrumentMasterFetchError {
    pub class: InstrumentMasterErrorClass,
    pub http_status: Option<u16>,
}

impl InstrumentMasterFetchError {
    pub fn new(class: InstrumentMasterErrorClass, http_status: Option<u16>) -> Self {
        Self { class, http_status }
    }
}

impl std::fmt::Display for InstrumentMasterFetchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.http_status {
            Some(status) => write!(f, "{} ({status})", self.class.as_str()),
            None => write!(f, "{}", self.class.as_str()),
        }
    }
}

impl std::error::Error for InstrumentMasterFetchError {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstrumentMasterStatus {
    pub phase: InstrumentMasterPhase,
    pub adapter_id: Option<String>,
    pub symbol_count: usize,
    /// Class only: `file_paths_http`, `file_paths_json`, `csv_http`, `csv_allowlist`,
    /// `csv_parse`, `exchange_info_http`, `empty`.
    pub last_error: Option<String>,
    pub last_http_status: Option<u16>,
}

impl Default for InstrumentMasterStatus {
    fn default() -> Self {
        Self {
            phase: InstrumentMasterPhase::Idle,
            adapter_id: None,
            symbol_count: 0,
            last_error: None,
            last_http_status: None,
        }
    }
}

impl InstrumentMasterStatus {
    /// Wire mapping shared by sync-state `capabilities.instruments` and search `master_status`.
    pub fn capability_wire(&self) -> &'static str {
        match self.phase {
            InstrumentMasterPhase::Loading => "loading",
            InstrumentMasterPhase::Loaded if self.symbol_count > 0 => "fresh",
            InstrumentMasterPhase::Error if self.symbol_count > 0 => "stale",
            _ => "unavailable",
        }
    }

    pub fn mark_idle(&mut self) {
        self.phase = InstrumentMasterPhase::Idle;
        self.last_error = None;
        self.last_http_status = None;
    }

    pub fn mark_loading(&mut self, adapter_id: &str) {
        self.phase = InstrumentMasterPhase::Loading;
        self.adapter_id = Some(adapter_id.to_string());
        self.last_error = None;
    }

    /// Returns false when a refresh is already in flight (search must not kick a loop).
    pub fn try_begin_loading(&mut self, adapter_id: &str) -> bool {
        if self.phase == InstrumentMasterPhase::Loading {
            return false;
        }
        if self.phase != InstrumentMasterPhase::Idle && self.phase != InstrumentMasterPhase::Error {
            return false;
        }
        self.mark_loading(adapter_id);
        true
    }

    pub fn mark_loaded(&mut self, adapter_id: &str, symbol_count: usize) {
        self.phase = InstrumentMasterPhase::Loaded;
        self.adapter_id = Some(adapter_id.to_string());
        self.symbol_count = symbol_count;
        self.last_error = None;
        self.last_http_status = None;
    }

    /// Cache-only rows: search can use them, but the pill stays idle/unavailable until live fetch.
    pub fn record_cache_rows(&mut self, adapter_id: &str, symbol_count: usize) {
        self.adapter_id = Some(adapter_id.to_string());
        self.symbol_count = symbol_count;
    }

    pub fn mark_error(
        &mut self,
        adapter_id: &str,
        class: InstrumentMasterErrorClass,
        http_status: Option<u16>,
        symbol_count: usize,
    ) {
        self.phase = InstrumentMasterPhase::Error;
        self.adapter_id = Some(adapter_id.to_string());
        self.symbol_count = symbol_count;
        self.last_error = Some(class.as_str().to_string());
        self.last_http_status = http_status;
    }
}

pub fn binance_exchange_info_cache_path(dir: &Path) -> PathBuf {
    dir.join("binance_com").join("exchangeInfo.json")
}

pub fn kotak_csv_cache_path(dir: &Path, segment: &str) -> PathBuf {
    dir.join("kotak_neo").join(format!("{segment}.csv"))
}

pub fn write_raw_cache(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        if let Err(err) = std::fs::create_dir_all(parent) {
            tracing::warn!(error = %err, "instrument master cache mkdir failed");
            return;
        }
    }
    if let Err(err) = std::fs::write(path, bytes) {
        tracing::warn!(error = %err, "instrument master cache write failed");
    }
}

pub fn json_object_keys(body: &str) -> Vec<String> {
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(serde_json::Value::Object(map)) => map.keys().cloned().collect(),
        Ok(serde_json::Value::Array(_)) => vec!["<array>".into()],
        _ => Vec::new(),
    }
}

/// Keys of `field` when it is an object; keys of the first object if it is an array.
/// Never returns values (no LTP, no Sid/Auth).
pub fn json_field_object_keys(body: &str, field: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    match value.get(field) {
        Some(serde_json::Value::Object(map)) => map.keys().cloned().collect(),
        Some(serde_json::Value::Array(arr)) => match arr.first() {
            Some(serde_json::Value::Object(map)) => map.keys().cloned().collect(),
            Some(_) | None => vec!["<array>".into()],
        },
        Some(_) => vec!["<non-object>".into()],
        None => Vec::new(),
    }
}

/// Keys of the first nested object under `field` (map value or array element).
/// Distinguishes a `data` map keyed by `nse_cm|token` from the quote field names
/// inside that row — still keys only.
pub fn json_first_nested_object_keys(body: &str, field: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    let Some(node) = value.get(field) else {
        return Vec::new();
    };
    match node {
        serde_json::Value::Object(map) => map
            .values()
            .find_map(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .find_map(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Keys of the first object in a root JSON array. Founder quotes 200s are arrays.
/// Never returns values.
pub fn json_array_first_object_keys(body: &str) -> Vec<String> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(body) else {
        return Vec::new();
    };
    match value {
        serde_json::Value::Array(arr) => arr
            .iter()
            .find_map(|v| v.as_object().map(|o| o.keys().cloned().collect()))
            .unwrap_or_else(|| vec!["<array>".into()]),
        _ => Vec::new(),
    }
}

pub fn truncate_body(body: &str, max_chars: usize) -> String {
    body.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_wire_maps_loading_fresh_stale_unavailable() {
        let mut st = InstrumentMasterStatus::default();
        assert_eq!(st.capability_wire(), "unavailable");
        st.mark_loading("binance_com");
        assert_eq!(st.capability_wire(), "loading");
        st.mark_loaded("binance_com", 12);
        assert_eq!(st.capability_wire(), "fresh");
        st.mark_loaded("binance_com", 0);
        assert_eq!(st.capability_wire(), "unavailable");
        st.mark_error("kotak_neo", InstrumentMasterErrorClass::Empty, None, 4);
        assert_eq!(st.capability_wire(), "stale");
        assert_eq!(st.last_error.as_deref(), Some("empty"));
        st.mark_error("kotak_neo", InstrumentMasterErrorClass::Empty, None, 0);
        assert_eq!(st.capability_wire(), "unavailable");
        let mut cached = InstrumentMasterStatus::default();
        cached.record_cache_rows("kotak_neo", 4);
        assert_eq!(cached.capability_wire(), "unavailable");
        assert_eq!(cached.symbol_count, 4);
        cached.mark_loading("kotak_neo");
        assert_eq!(cached.capability_wire(), "loading");
        assert_eq!(cached.symbol_count, 4);
        cached.mark_loaded("kotak_neo", 4);
        cached.mark_idle();
        assert_eq!(cached.capability_wire(), "unavailable");
    }

    #[test]
    fn json_field_keys_are_names_not_values() {
        let body = r#"{"stat":"Not_Ok","stCode":1003,"data":{"nse_cm|3721":{"pSymbol":"3721","ltp":"1400.50"}}}"#;
        let mut root = json_object_keys(body);
        root.sort();
        assert_eq!(root, vec!["data", "stCode", "stat"]);
        let mut data = json_field_object_keys(body, "data");
        data.sort();
        assert_eq!(data, vec!["nse_cm|3721"]);
        let mut row = json_first_nested_object_keys(body, "data");
        row.sort();
        assert_eq!(row, vec!["ltp", "pSymbol"]);
        assert!(!row.iter().any(|k| k.contains("1400")));
        let message = json_field_object_keys(
            r#"{"message":[{"instrument_token":"2885","last_traded_price":"1"}]}"#,
            "message",
        );
        assert!(message.contains(&"instrument_token".to_string()));
        assert!(message.contains(&"last_traded_price".to_string()));
        let mut array_row = json_array_first_object_keys(
            r#"[{"ltp":"3224.50","exchange":"nse_cm","exchange_token":"11536"}]"#,
        );
        array_row.sort();
        assert_eq!(array_row, vec!["exchange", "exchange_token", "ltp"]);
        assert!(json_array_first_object_keys(r#"{"message":[]}"#).is_empty());
    }

    #[test]
    fn try_begin_loading_refuses_while_loading() {
        let mut st = InstrumentMasterStatus::default();
        assert!(st.try_begin_loading("kotak_neo"));
        assert!(!st.try_begin_loading("kotak_neo"));
        st.mark_loaded("kotak_neo", 2);
        assert!(!st.try_begin_loading("kotak_neo"));
        st.mark_error(
            "kotak_neo",
            InstrumentMasterErrorClass::CsvHttp,
            Some(500),
            2,
        );
        assert!(st.try_begin_loading("kotak_neo"));
    }
}
