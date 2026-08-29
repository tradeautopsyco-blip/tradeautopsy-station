//! Kotak Neo cash scrip master (Slice B). Separate from Binance `exchangeInfo`.
//!
//! Start: host-mediated SDK-first `GET …/script-details/1.0/masterscrip/file-paths`
//! (`Authorization` = consumer key; Auth+Sid only on 401/403 / 2FA retry; no trade-book `sId`).
//! Cash CSV URLs come from that JSON. Live unsigned lapi GET of `nse_cm.csv` /
//! `nse_cm-v1.csv` and `bse_cm.csv` / `bse_cm-v1.csv` is allowlisted (OpenAlgo
//! `filesPaths`; no Sid/Auth on CSV). Live cash CSV token is `pSymbol`, ticker
//! `pSymbolName` (no `pToken`). F&O CSVs stay refused.

use crate::data::{
    authorize_book_call, authorize_inferred_call, is_kotak_cash_scrip_csv_path, json_object_keys,
    kotak_csv_cache_path, truncate_body, write_raw_cache, InstrumentMasterErrorClass,
    InstrumentMasterFetchError, InstrumentMasterStatus, KOTAK_NSE_BSE_CASH_BOOK_ID,
};
use crate::instruments::normalize_broker_ticker;
use crate::ubi::{
    attach_kotak_file_paths_session, kotak_base_host, prepare_kotak_file_paths_get,
    BrokerCredentialVault, HostCredentialBlob, PreparedHttpRequest,
};
use serde_json::Value;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const FILE_PATHS_PATH: &str = "/script-details/1.0/masterscrip/file-paths";
pub const KOTAK_NEO: &str = "kotak_neo";

const CASH_SEGMENTS: &[&str] = &["nse_cm", "bse_cm"];
const REFUSED_FO_SEGMENTS: &[&str] = &["nse_fo", "bse_fo", "cde_fo", "mcx_fo"];
const REFUSED_FO_INST_TYPES: &[&str] = &[
    "FUTIDX", "FUTSTK", "FUTCUR", "FUTCOM", "FUTIVX", "OPTIDX", "OPTSTK", "OPTCUR", "OPTCOM",
    "SPREAD",
];

/// One cash row keyed by Kotak instrument token + segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KotakInstrument {
    pub instrument_token: i64,
    pub trading_symbol: String,
    pub name: String,
    pub segment: String,
}

/// Search hit against the Kotak cash master. Never a Zerodha `NSE` / Binance row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KotakSearchHit {
    pub trading_symbol: String,
    pub name: String,
    pub exchange: String,
    pub segment: String,
    pub instrument_token: i64,
}

#[derive(Debug, Clone, Default)]
pub struct KotakScripMaster {
    /// `(segment, token)` → row. Dual-listed cash names stay distinct.
    by_token: HashMap<(String, i64), KotakInstrument>,
}

impl KotakScripMaster {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.by_token.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_token.is_empty()
    }

    pub fn contains_id(&self, id: &str) -> bool {
        let Some((segment, token)) = parse_instrument_id(id) else {
            return false;
        };
        self.by_token.contains_key(&(segment, token))
    }

    pub fn iter_tickers(&self) -> impl Iterator<Item = &str> {
        self.by_token
            .values()
            .map(|row| row.trading_symbol.as_str())
    }

    pub fn iter_instrument_ids(&self) -> impl Iterator<Item = String> + '_ {
        self.by_token
            .values()
            .map(|row| instrument_id(&row.segment, row.instrument_token))
    }

    /// Map a typed ticker or `segment|token` to the TickBook key.
    pub fn resolve_id(&self, raw: &str) -> Option<String> {
        if let Some((segment, token)) = parse_instrument_id(raw) {
            return Some(instrument_id(&segment, token));
        }
        self.search(raw, 10)
            .into_iter()
            .next()
            .map(|hit| instrument_id(&hit.segment, hit.instrument_token))
    }

    pub fn merge(&mut self, other: Self) {
        self.by_token.extend(other.by_token);
    }

    /// Prefix search on cash tickers and names. Shortest ticker first; `nse_cm` before `bse_cm`.
    pub fn search(&self, q: &str, limit: usize) -> Vec<KotakSearchHit> {
        let q = q.trim().to_ascii_uppercase();
        if q.len() < 2 {
            return Vec::new();
        }
        let mut hits: Vec<KotakSearchHit> = self
            .by_token
            .values()
            .filter(|row| {
                row.trading_symbol.starts_with(&q) || row.name.to_ascii_uppercase().starts_with(&q)
            })
            .map(|row| KotakSearchHit {
                trading_symbol: row.trading_symbol.clone(),
                name: row.name.clone(),
                exchange: KOTAK_NEO.to_string(),
                segment: row.segment.clone(),
                instrument_token: row.instrument_token,
            })
            .collect();
        hits.sort_by(|a, b| {
            a.trading_symbol
                .len()
                .cmp(&b.trading_symbol.len())
                .then_with(|| a.trading_symbol.cmp(&b.trading_symbol))
                .then_with(|| segment_rank(&a.segment).cmp(&segment_rank(&b.segment)))
        });
        hits.truncate(limit);
        hits
    }

    pub fn from_csv_bytes(bytes: &[u8], file_segment: Option<&str>) -> anyhow::Result<Self> {
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(Cursor::new(bytes));
        let headers = reader.headers()?.clone();
        let col = |name: &str| {
            headers
                .iter()
                .position(|h| h.trim().eq_ignore_ascii_case(name))
        };
        let explicit_token_i = col("pToken").or_else(|| col("token"));
        let psymbol_i = col("pSymbol");
        let symbol_name_i = col("pSymbolName");
        let trd_i = col("pTrdSymbol").or_else(|| col("pTradingSymbol"));
        let seg_i = col("pExchSeg").or_else(|| col("exchange_segment"));
        let inst_i = col("pInstType");
        let name_i = col("pAssetName")
            .or_else(|| col("pAssetCode"))
            .or_else(|| col("pDesc"));
        // Fixture: `pToken` + text `pSymbol`. Live lapi (REST.md): no `pToken`;
        // `pSymbol` is the numeric token, ticker is `pSymbolName` (OpenAlgo NSE_CM).
        let live_schema = explicit_token_i.is_none();
        let token_i = explicit_token_i.or(psymbol_i);
        let ticker_i = if live_schema {
            symbol_name_i.or(trd_i)
        } else {
            psymbol_i.or(trd_i)
        };
        let Some(token_i) = token_i else {
            anyhow::bail!("kotak cash CSV missing pToken/pSymbol");
        };

        let mut master = Self::empty();
        for record in reader.records() {
            let record = record?;
            let token_raw = record.get(token_i).unwrap_or("").trim();
            let Ok(token) = token_raw.parse::<i64>() else {
                continue;
            };
            let segment = seg_i
                .and_then(|i| record.get(i))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .or(file_segment)
                .unwrap_or("")
                .to_ascii_lowercase();
            if !is_cash_segment(&segment) {
                continue;
            }
            let inst_type = inst_i
                .and_then(|i| record.get(i))
                .unwrap_or("")
                .trim()
                .to_ascii_uppercase();
            if is_refused_fo_inst_type(&inst_type) {
                continue;
            }
            let raw_symbol = ticker_i
                .and_then(|i| record.get(i))
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let Some(ticker) = raw_symbol.and_then(normalize_broker_ticker) else {
                continue;
            };
            let name = name_i
                .and_then(|i| record.get(i))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or(ticker.as_str())
                .to_string();
            master.by_token.insert(
                (segment.clone(), token),
                KotakInstrument {
                    instrument_token: token,
                    trading_symbol: ticker,
                    name,
                    segment,
                },
            );
        }
        Ok(master)
    }
}

fn segment_rank(segment: &str) -> u8 {
    match segment {
        "nse_cm" => 0,
        "bse_cm" => 1,
        _ => 2,
    }
}

pub fn is_cash_segment(segment: &str) -> bool {
    CASH_SEGMENTS
        .iter()
        .any(|allowed| segment.eq_ignore_ascii_case(allowed))
}

/// TickBook / REST `neo_symbols` key: `{segment}|{token}` (REST.md).
pub fn instrument_id(segment: &str, token: i64) -> String {
    format!("{}|{token}", segment.trim().to_ascii_lowercase())
}

pub fn parse_instrument_id(raw: &str) -> Option<(String, i64)> {
    let decoded = raw.trim().replace("%7C", "|").replace("%7c", "|");
    let (segment, token) = decoded.split_once('|')?;
    let segment = segment.trim().to_ascii_lowercase();
    if !is_cash_segment(&segment) {
        return None;
    }
    let token = token.trim().parse::<i64>().ok()?;
    Some((segment, token))
}

fn is_refused_fo_inst_type(inst_type: &str) -> bool {
    REFUSED_FO_INST_TYPES
        .iter()
        .any(|refused| inst_type.eq_ignore_ascii_case(refused))
}

/// Cash CSV URLs from a file-paths JSON body that would pass `authorize_csv_get`.
/// F&O and non-allowlisted `nse_cm` paths are dropped so `cash_urls` matches GET attempts.
pub fn cash_csv_urls(file_paths_json: &str) -> anyhow::Result<Vec<String>> {
    let value: Value = serde_json::from_str(file_paths_json)?;
    Ok(files_paths_from_value(&value)
        .into_iter()
        .filter(|url| is_extractable_cash_csv_url(url))
        .collect())
}

fn is_extractable_cash_csv_url(url: &str) -> bool {
    let Some((_, path)) = split_http_url(url) else {
        return false;
    };
    cash_segment_from_url(url).is_some()
        && is_kotak_cash_scrip_csv_path(&path)
        && authorize_csv_get(url).is_ok()
}

fn files_paths_from_value(value: &Value) -> Vec<String> {
    for key in [
        "filesPaths",
        "filePaths",
        "files_paths",
        "file_paths",
        "filesPath",
    ] {
        if let Some(arr) = value.get(key).and_then(Value::as_array) {
            let urls: Vec<String> = arr.iter().filter_map(url_from_path_entry).collect();
            if !urls.is_empty() {
                return urls;
            }
        }
        if let Some(s) = value.get(key).and_then(Value::as_str) {
            if !s.is_empty() {
                return vec![s.to_string()];
            }
        }
    }
    if let Some(data) = value.get("data") {
        return files_paths_from_value(data);
    }
    Vec::new()
}

fn url_from_path_entry(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::to_string)
        .or_else(|| value.get("url").and_then(Value::as_str).map(str::to_string))
        .filter(|s| !s.is_empty())
}

pub fn cash_segment_from_url(url: &str) -> Option<&'static str> {
    let lower = url.to_ascii_lowercase();
    if is_refused_fo_url(&lower) {
        return None;
    }
    CASH_SEGMENTS
        .iter()
        .copied()
        .find(|seg| lower.contains(seg))
}

fn is_refused_fo_url(url: &str) -> bool {
    REFUSED_FO_SEGMENTS.iter().any(|seg| url.contains(seg))
        || url.contains("_fo.csv")
        || url.contains("-fo.csv")
}

fn split_http_url(url: &str) -> Option<(String, String)> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    Some((host, parsed.path().to_string()))
}

/// Host + path for allowlist-refuse logs. Never query, userinfo, headers, or Sid/Auth.
fn csv_allowlist_log_host_path(url: &str) -> (String, String) {
    match split_http_url(url) {
        Some((host, path)) => (host, path),
        None => {
            let path_like = url.split('?').next().unwrap_or("-");
            ("-".to_string(), path_like.to_string())
        }
    }
}

/// Live cash CSV GET is only attempted after the cash book fence allows the URL.
/// FO CSVs stay refused here; the NFO book uses `authorize_nfo_csv_get`.
pub fn authorize_csv_get(url: &str) -> Result<(), String> {
    let (host, path) = split_http_url(url).ok_or_else(|| "path_not_allowlisted".to_string())?;
    authorize_book_call(KOTAK_NSE_BSE_CASH_BOOK_ID, &host, "GET", &path, false)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

const REFRESH_BACKOFFS: [Duration; 3] = [
    Duration::from_secs(5),
    Duration::from_secs(30),
    Duration::from_secs(120),
];

/// Replace `dest` only when `loaded` is non-empty. Empty must not wipe a loaded master.
pub fn install_master_if_nonempty(
    dest: &Mutex<KotakScripMaster>,
    loaded: KotakScripMaster,
) -> bool {
    if loaded.is_empty() {
        return false;
    }
    *dest.lock().expect("kotak scrip master mutex poisoned") = loaded;
    true
}

pub fn try_load_kotak_cache(
    cache_dir: &Path,
    master: &Mutex<KotakScripMaster>,
    status: &Mutex<InstrumentMasterStatus>,
) {
    let mut loaded = KotakScripMaster::empty();
    for seg in CASH_SEGMENTS {
        let path = kotak_csv_cache_path(cache_dir, seg);
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        match KotakScripMaster::from_csv_bytes(&bytes, Some(seg)) {
            Ok(part) => loaded.merge(part),
            Err(err) => {
                tracing::warn!(error = %err, segment = %seg, "s1 desk: kotak cache CSV skipped")
            }
        }
    }
    if loaded.is_empty() {
        return;
    }
    let n = loaded.len();
    *master.lock().expect("kotak scrip master mutex poisoned") = loaded;
    status
        .lock()
        .expect("instrument master status poisoned")
        .record_cache_rows(KOTAK_NEO, n);
    tracing::info!(
        symbols = n,
        "s1 desk: kotak cash master loaded from disk cache"
    );
}

pub fn spawn_refresh(
    master: Arc<Mutex<KotakScripMaster>>,
    vault: Arc<dyn BrokerCredentialVault>,
    environment: String,
    connection_id: String,
    status: Arc<Mutex<InstrumentMasterStatus>>,
    cancel: Arc<AtomicBool>,
    cache_dir: PathBuf,
    connections: Arc<
        Mutex<std::collections::HashMap<String, crate::data::BrokerConnectionRuntime>>,
    >,
    locator: crate::kotak_rest_quotes::SessionLocator,
) {
    {
        let mut st = status.lock().expect("instrument master status poisoned");
        st.mark_loading(KOTAK_NEO);
    }
    tokio::spawn(async move {
        let mut attempt = 0u8;
        loop {
            if !kotak_refresh_still_active(&cancel, &connections, &locator) {
                return;
            }
            match refresh_from_session(vault.as_ref(), &environment, &connection_id, &cache_dir)
                .await
            {
                Ok(loaded) => {
                    let n = loaded.len();
                    if install_master_if_nonempty(&master, loaded) {
                        tracing::info!(symbols = n, "s1 desk: kotak cash scrip master loaded");
                        status
                            .lock()
                            .expect("instrument master status poisoned")
                            .mark_loaded(KOTAK_NEO, n);
                        return;
                    }
                    let kept = master
                        .lock()
                        .expect("kotak scrip master mutex poisoned")
                        .len();
                    tracing::warn!("s1 desk: kotak cash master empty after fetch");
                    status
                        .lock()
                        .expect("instrument master status poisoned")
                        .mark_error(KOTAK_NEO, InstrumentMasterErrorClass::Empty, None, kept);
                }
                Err(err) => {
                    let kept = master
                        .lock()
                        .expect("kotak scrip master mutex poisoned")
                        .len();
                    tracing::warn!(error = %err, "s1 desk: kotak scrip master refresh failed");
                    status
                        .lock()
                        .expect("instrument master status poisoned")
                        .mark_error(KOTAK_NEO, err.class, err.http_status, kept);
                }
            }
            if attempt >= 3 {
                return;
            }
            let delay = REFRESH_BACKOFFS[attempt.min(2) as usize];
            attempt += 1;
            tokio::time::sleep(delay).await;
        }
    });
}

fn kotak_refresh_still_active(
    cancel: &AtomicBool,
    connections: &Mutex<std::collections::HashMap<String, crate::data::BrokerConnectionRuntime>>,
    locator: &crate::kotak_rest_quotes::SessionLocator,
) -> bool {
    if cancel.load(Ordering::Relaxed) {
        return false;
    }
    let has_locator = locator.lock().ok().is_some_and(|g| g.is_some());
    let has_conn = connections
        .lock()
        .ok()
        .is_some_and(|m| m.contains_key(KOTAK_NEO));
    has_locator && has_conn
}

pub async fn refresh_from_session(
    vault: &dyn BrokerCredentialVault,
    environment: &str,
    connection_id: &str,
    cache_dir: &Path,
) -> Result<KotakScripMaster, InstrumentMasterFetchError> {
    let blob = vault
        .load(environment, KOTAK_NEO, connection_id)
        .map_err(|_| {
            InstrumentMasterFetchError::new(InstrumentMasterErrorClass::FilePathsHttp, None)
        })?
        .ok_or_else(|| {
            InstrumentMasterFetchError::new(InstrumentMasterErrorClass::FilePathsHttp, None)
        })?;
    let creds = HostCredentialBlob::from(&blob);
    let base_url = match &creds {
        HostCredentialBlob::KotakSession { base_url, .. } => base_url.clone(),
        HostCredentialBlob::Hmac { .. } => {
            return Err(InstrumentMasterFetchError::new(
                InstrumentMasterErrorClass::FilePathsHttp,
                None,
            ));
        }
    };
    let host = kotak_base_host(&base_url).ok_or_else(|| {
        InstrumentMasterFetchError::new(InstrumentMasterErrorClass::FilePathsHttp, None)
    })?;
    authorize_inferred_call(&host, "GET", FILE_PATHS_PATH, true).map_err(|_| {
        InstrumentMasterFetchError::new(InstrumentMasterErrorClass::FilePathsHttp, None)
    })?;
    let sdk = prepare_kotak_file_paths_get(FILE_PATHS_PATH, &creds).map_err(|_| {
        InstrumentMasterFetchError::new(InstrumentMasterErrorClass::FilePathsHttp, None)
    })?;
    let catalog_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|_| {
            InstrumentMasterFetchError::new(InstrumentMasterErrorClass::FilePathsHttp, None)
        })?;
    let csv_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(90))
        .build()
        .map_err(|_| InstrumentMasterFetchError::new(InstrumentMasterErrorClass::CsvHttp, None))?;
    let (body, catalog_status) = fetch_file_paths_body(&catalog_client, sdk, &creds).await?;
    let urls = cash_csv_urls(&body).map_err(|_| {
        tracing::warn!(
            keys = ?json_object_keys(&body),
            "s1 desk: kotak file-paths JSON unusable"
        );
        InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::FilePathsJson,
            Some(catalog_status),
        )
    })?;
    tracing::info!(
        cash_urls = urls.len(),
        "s1 desk: kotak file-paths cash CSV URLs"
    );
    if urls.is_empty() {
        tracing::warn!(
            keys = ?json_object_keys(&body),
            "s1 desk: no cash filesPaths in file-paths body"
        );
        return Err(InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::FilePathsJson,
            Some(catalog_status),
        ));
    }
    let mut master = KotakScripMaster::empty();
    let mut saw_http = false;
    let mut saw_allowlist = false;
    let mut saw_parse = false;
    let mut last_csv_status: Option<u16> = None;
    for url in urls {
        let segment = cash_segment_from_url(&url);
        match authorize_csv_get(&url) {
            Ok(()) => match csv_client.get(&url).send().await {
                Ok(resp) => {
                    let csv_status = resp.status();
                    last_csv_status = Some(csv_status.as_u16());
                    if !csv_status.is_success() {
                        saw_http = true;
                        tracing::warn!(
                            status = %csv_status,
                            "s1 desk: kotak cash CSV HTTP error"
                        );
                        continue;
                    }
                    match resp.bytes().await {
                        Ok(bytes) => match KotakScripMaster::from_csv_bytes(&bytes, segment) {
                            Ok(part) => {
                                if !part.is_empty() {
                                    if let Some(seg) = segment {
                                        write_raw_cache(
                                            &kotak_csv_cache_path(cache_dir, seg),
                                            &bytes,
                                        );
                                    }
                                    master.merge(part);
                                } else {
                                    saw_parse = true;
                                }
                            }
                            Err(err) => {
                                saw_parse = true;
                                tracing::warn!(
                                    error = %err,
                                    "s1 desk: kotak cash CSV parse failed"
                                );
                            }
                        },
                        Err(err) => {
                            saw_http = true;
                            tracing::warn!(
                                error = %err,
                                "s1 desk: kotak cash CSV read failed"
                            );
                        }
                    }
                }
                Err(err) => {
                    saw_http = true;
                    tracing::warn!(error = %err, "s1 desk: kotak cash CSV GET failed");
                }
            },
            Err(err) => {
                saw_allowlist = true;
                let (host, path) = csv_allowlist_log_host_path(&url);
                tracing::warn!(
                    host = %host,
                    path = %path,
                    error = %err,
                    "kotak cash CSV GET fail-closed (host/path not allowlisted)"
                );
            }
        }
    }
    if master.is_empty() {
        let class = if saw_http {
            InstrumentMasterErrorClass::CsvHttp
        } else if saw_allowlist {
            InstrumentMasterErrorClass::CsvAllowlist
        } else if saw_parse {
            InstrumentMasterErrorClass::CsvParse
        } else {
            InstrumentMasterErrorClass::Empty
        };
        return Err(InstrumentMasterFetchError::new(class, last_csv_status));
    }
    tracing::info!(
        symbols = master.len(),
        "s1 desk: kotak cash scrip master loaded"
    );
    Ok(master)
}

fn file_paths_needs_session_retry(status: reqwest::StatusCode, body: &str) -> bool {
    matches!(status.as_u16(), 401 | 403) || body.contains("Complete the 2fa process")
}

fn log_file_paths_http_error(status: reqwest::StatusCode, body: &str) {
    tracing::warn!(
        status = %status,
        keys = ?json_object_keys(body),
        body_preview = %truncate_body(body, 200),
        "s1 desk: kotak file-paths HTTP error"
    );
}

async fn send_prepared(
    client: &reqwest::Client,
    prepared: &PreparedHttpRequest,
) -> Result<(reqwest::StatusCode, String), InstrumentMasterFetchError> {
    let mut req = client.get(&prepared.url);
    for (name, value) in &prepared.headers {
        req = req.header(name.as_str(), value.as_str());
    }
    let resp = req.send().await.map_err(|_| {
        InstrumentMasterFetchError::new(InstrumentMasterErrorClass::FilePathsHttp, None)
    })?;
    let status = resp.status();
    let body = resp.text().await.map_err(|_| {
        InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::FilePathsHttp,
            Some(status.as_u16()),
        )
    })?;
    Ok((status, body))
}

async fn fetch_file_paths_body(
    client: &reqwest::Client,
    sdk: PreparedHttpRequest,
    creds: &HostCredentialBlob,
) -> Result<(String, u16), InstrumentMasterFetchError> {
    let (status, body) = send_prepared(client, &sdk).await?;
    if status.is_success() && !body.contains("Complete the 2fa process") {
        tracing::info!(
            variant = "sdk_consumer_key",
            status = status.as_u16(),
            keys = ?json_object_keys(&body),
            "s1 desk: kotak file-paths GET succeeded"
        );
        return Ok((body, status.as_u16()));
    }
    if file_paths_needs_session_retry(status, &body) {
        let session = attach_kotak_file_paths_session(sdk, creds).map_err(|_| {
            InstrumentMasterFetchError::new(
                InstrumentMasterErrorClass::FilePathsHttp,
                Some(status.as_u16()),
            )
        })?;
        let (status2, body2) = send_prepared(client, &session).await?;
        if status2.is_success() {
            tracing::info!(
                variant = "session_auth_sid",
                status = status2.as_u16(),
                keys = ?json_object_keys(&body2),
                "s1 desk: kotak file-paths GET succeeded"
            );
            return Ok((body2, status2.as_u16()));
        }
        log_file_paths_http_error(status2, &body2);
        return Err(InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::FilePathsHttp,
            Some(status2.as_u16()),
        ));
    }
    if !status.is_success() {
        log_file_paths_http_error(status, &body);
        return Err(InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::FilePathsHttp,
            Some(status.as_u16()),
        ));
    }
    Ok((body, status.as_u16()))
}

/// Parse file-paths JSON, keep cash URLs, fetch CSV via `fetch_csv` (fixture in tests).
#[cfg(test)]
pub fn load_master_from_file_paths_json(
    file_paths_json: &str,
    mut fetch_csv: impl FnMut(&str) -> anyhow::Result<Vec<u8>>,
) -> anyhow::Result<KotakScripMaster> {
    let mut master = KotakScripMaster::empty();
    for url in cash_csv_urls(file_paths_json)? {
        let segment = cash_segment_from_url(&url);
        match fetch_csv(&url) {
            Ok(bytes) => match KotakScripMaster::from_csv_bytes(&bytes, segment) {
                Ok(part) => master.merge(part),
                Err(err) => tracing::warn!(url = %url, error = %err, "kotak cash CSV skipped"),
            },
            Err(err) => {
                tracing::warn!(url = %url, error = %err, "kotak cash CSV skipped");
            }
        }
    }
    Ok(master)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_CSV: &str = include_str!("../fixtures/kotak/nse_cm_cash.csv");
    const FIXTURE_PATHS: &str = include_str!("../fixtures/kotak/scrip_file_paths.json");
    const FIXTURE_PATHS_LIVE: &str = include_str!("../fixtures/kotak/scrip_file_paths_live.json");

    #[test]
    fn file_paths_keep_cash_and_drop_fo() {
        let urls = cash_csv_urls(FIXTURE_PATHS).unwrap();
        assert_eq!(urls.len(), 2);
        assert!(urls.iter().any(|u| u.ends_with("/nse_cm.csv")));
        assert!(urls.iter().any(|u| u.ends_with("/bse_cm.csv")));
        assert!(urls.iter().all(|u| cash_segment_from_url(u).is_some()));
        assert!(FIXTURE_PATHS.contains("nse_fo.csv"));
        assert!(urls
            .iter()
            .all(|u| !u.contains("nse_fo") && !u.contains("bse_fo")));
    }

    #[test]
    fn wrapped_data_files_paths_are_accepted() {
        let wrapped = serde_json::json!({
            "data": serde_json::from_str::<Value>(FIXTURE_PATHS).unwrap()
        });
        let urls = cash_csv_urls(&wrapped.to_string()).unwrap();
        assert_eq!(urls.len(), 2);
    }

    #[test]
    fn file_paths_alias_keys_are_accepted() {
        let aliased = serde_json::json!({
            "filePaths": [
                "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_cm.csv"
            ]
        });
        let urls = cash_csv_urls(&aliased.to_string()).unwrap();
        assert_eq!(urls.len(), 1);
        assert!(urls[0].ends_with("/nse_cm.csv"));
    }

    #[test]
    fn object_url_array_and_singular_files_path_extract_cash() {
        let objects = serde_json::json!({
            "filesPaths": [
                { "url": "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_cm.csv" },
                { "url": "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv" }
            ]
        });
        let urls = cash_csv_urls(&objects.to_string()).unwrap();
        assert_eq!(urls.len(), 1);
        assert!(urls[0].ends_with("/nse_cm.csv"));

        let singular = serde_json::json!({
            "filesPath": "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/bse_cm.csv"
        });
        let urls = cash_csv_urls(&singular.to_string()).unwrap();
        assert_eq!(urls.len(), 1);
        assert!(urls[0].ends_with("/bse_cm.csv"));
    }

    #[test]
    fn file_paths_authorize_uses_port_stripped_v2_host() {
        let host =
            crate::ubi::kotak_base_host("https://e21.kotaksecurities.com:443/trading").unwrap();
        assert_eq!(host, "e21.kotaksecurities.com");
        crate::data::authorize_inferred_call(&host, "GET", FILE_PATHS_PATH, true)
            .expect("v2 data-center file-paths remains allowlisted");
        let blocked = crate::data::authorize_inferred_call(
            "e21.kotaksecurities.com:443",
            "GET",
            FILE_PATHS_PATH,
            true,
        );
        assert!(blocked.is_err(), "host with :port must not match allowlist");
    }

    #[test]
    fn fixture_csv_indexes_reliance_and_refuses_fo() {
        let master = KotakScripMaster::from_csv_bytes(FIXTURE_CSV.as_bytes(), None).unwrap();
        assert_eq!(master.len(), 2, "nse_cm + bse_cm RELIANCE; F&O dropped");
        let hits = master.search("RELIANCE", 10);
        assert!(!hits.is_empty());
        assert!(hits.iter().all(|h| h.trading_symbol == "RELIANCE"));
        assert!(hits
            .iter()
            .any(|h| h.segment == "nse_cm" && h.instrument_token == 2885));
        assert!(hits.iter().all(|h| h.exchange == KOTAK_NEO));
        assert!(hits
            .iter()
            .all(|h| h.exchange != "BINANCE" && h.exchange != "NSE"));
        assert!(master.search("NIFTY", 10).is_empty());
        assert!(master.contains_id("nse_cm|2885"));
        assert!(master.contains_id("NSE_CM|2885"));
        assert!(!master.contains_id("RELIANCE"));
        assert!(!master.contains_id("nse_cm|999999"));
        assert!(!master.contains_id("NSE|2885"));
    }

    #[test]
    fn search_hits_are_not_zerodha_or_binance_waterfall() {
        let master = KotakScripMaster::from_csv_bytes(FIXTURE_CSV.as_bytes(), None).unwrap();
        let hit = &master.search("REL", 10)[0];
        assert_eq!(hit.trading_symbol, "RELIANCE");
        assert_eq!(hit.exchange, "kotak_neo");
        assert_eq!(hit.segment, "nse_cm");
        assert_eq!(
            master.resolve_id("RELIANCE").as_deref(),
            Some("nse_cm|2885")
        );
        assert_eq!(
            master.resolve_id("nse_cm|2885").as_deref(),
            Some("nse_cm|2885")
        );
        assert_ne!(hit.exchange, "NSE");
        assert_ne!(hit.exchange, "BINANCE");
        assert_ne!(hit.exchange, "binance_com");
    }

    #[test]
    fn live_csv_uses_psymbol_token_and_psymbolname_ticker() {
        let csv = "pSymbol,pGroup,pExchSeg,pInstType,pSymbolName,pTrdSymbol,pDesc\n\
2885,EQ,nse_cm,EQ,RELIANCE,RELIANCE-EQ,Reliance Industries Ltd\n\
26000,,nse_cm,,NIFTY,NIFTY,NIFTY\n\
999,EQ,nse_fo,OPTIDX,NIFTY,NIFTY25APR24000CE,NIFTY\n";
        let master = KotakScripMaster::from_csv_bytes(csv.as_bytes(), Some("nse_cm")).unwrap();
        assert!(master.contains_id("nse_cm|2885"));
        assert_eq!(
            master.resolve_id("RELIANCE").as_deref(),
            Some("nse_cm|2885")
        );
        let hit = &master.search("REL", 1)[0];
        assert_eq!(hit.trading_symbol, "RELIANCE");
        assert_eq!(hit.instrument_token, 2885);
        assert_eq!(hit.name, "Reliance Industries Ltd");
        assert!(master.contains_id("nse_cm|26000"));
        assert!(!master.contains_id("nse_cm|999"));
        assert!(
            master.search("2885", 1).is_empty(),
            "numeric pSymbol must not become the ticker"
        );
    }

    #[test]
    fn live_sample_cash_csv_urls_are_allowlisted() {
        let urls = cash_csv_urls(FIXTURE_PATHS).unwrap();
        assert_eq!(urls.len(), 2);
        for url in urls {
            authorize_csv_get(&url).expect("cash CSV GET allowlisted");
        }
        let fo = "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv";
        let err = authorize_csv_get(fo).expect_err("F&O CSV stays refused");
        assert!(
            err.contains("path_not_allowlisted") || err.contains("host_blocked"),
            "unexpected refuse: {err}"
        );
        let queried = "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_cm.csv?x=1";
        authorize_csv_get(queried).expect("cash CSV GET with query is allowlisted");
        let (host, path) = super::csv_allowlist_log_host_path(queried);
        assert_eq!(host, "lapi.kotaksecurities.com");
        assert_eq!(
            path,
            "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_cm.csv"
        );
        assert!(!path.contains('?'));
        assert!(!host.contains('@'));
    }

    #[test]
    fn live_v1_cash_csv_urls_are_kept_and_allowlisted() {
        let live = serde_json::json!({
            "filesPaths": [
                "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed-v1/nse_cm-v1.csv",
                "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed-v1/bse_cm-v1.csv",
                "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed-v1/nse_fo-v1.csv",
                "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv"
            ]
        });
        let urls = cash_csv_urls(&live.to_string()).unwrap();
        assert_eq!(urls.len(), 2);
        assert!(urls.iter().any(|u| u.ends_with("/nse_cm-v1.csv")));
        assert!(urls.iter().any(|u| u.ends_with("/bse_cm-v1.csv")));
        assert!(urls
            .iter()
            .all(|u| !u.contains("nse_fo") && !u.contains("bse_fo")));
        for url in &urls {
            authorize_csv_get(url).expect("live nse_cm-v1 / bse_cm-v1 GET allowlisted");
        }
        for fo in [
            "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv",
            "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed-v1/nse_fo-v1.csv",
        ] {
            assert!(cash_segment_from_url(fo).is_none());
            let err = authorize_csv_get(fo).expect_err("F&O CSV stays refused");
            assert!(
                err.contains("path_not_allowlisted") || err.contains("host_blocked"),
                "unexpected refuse: {err}"
            );
        }

        let fixture_urls = cash_csv_urls(FIXTURE_PATHS_LIVE).unwrap();
        assert_eq!(fixture_urls.len(), 2);
        assert!(fixture_urls.iter().any(|u| u.ends_with("/nse_cm-v1.csv")));
        assert!(fixture_urls.iter().any(|u| u.ends_with("/bse_cm-v1.csv")));
        assert!(FIXTURE_PATHS_LIVE.contains("nse_fo.csv"));
        for url in fixture_urls {
            authorize_csv_get(&url).expect("live fixture cash CSV GET allowlisted");
        }
    }

    #[test]
    fn cash_csv_urls_drops_nse_cm_outside_allowlisted_path() {
        let off_path = serde_json::json!({
            "filesPaths": [
                "https://lapi.kotaksecurities.com/not-scripmaster/nse_cm.csv",
                "https://lapi.kotaksecurities.com/nse_cm.csv",
                "https://lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_fo.csv"
            ]
        });
        let urls = cash_csv_urls(&off_path.to_string()).unwrap();
        assert!(
            urls.is_empty(),
            "nse_cm substring without allowlisted path must not count: {urls:?}"
        );
        let err = authorize_csv_get("https://lapi.kotaksecurities.com/not-scripmaster/nse_cm.csv")
            .expect_err("non-scripmaster nse_cm stays refused");
        assert!(
            err.contains("path_not_allowlisted") || err.contains("host_blocked"),
            "unexpected refuse: {err}"
        );
    }

    #[test]
    fn csv_allowlist_log_host_path_omits_query_and_userinfo() {
        let with_secret = "https://sid:auth@lapi.kotaksecurities.com/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_cm.csv?token=1";
        let (host, path) = super::csv_allowlist_log_host_path(with_secret);
        assert_eq!(host, "lapi.kotaksecurities.com");
        assert_eq!(
            path,
            "/wso2-scripmaster/v1/prod/2025-01-22/transformed/nse_cm.csv"
        );
        assert!(!path.contains('?'));
        assert!(!host.contains('@'));
        assert!(!host.contains("sid"));
        assert!(!path.contains("token"));
        assert!(!path.contains("auth"));

        let (host, path) = super::csv_allowlist_log_host_path("not a url?secret=1");
        assert_eq!(host, "-");
        assert!(!path.contains('?'));
        assert!(!path.contains("secret"));
    }

    #[test]
    fn fixture_fetch_loads_master_from_file_paths() {
        let master = load_master_from_file_paths_json(FIXTURE_PATHS, |url| {
            if url.ends_with("nse_cm.csv") {
                Ok(FIXTURE_CSV.as_bytes().to_vec())
            } else {
                anyhow::bail!("skip {url}")
            }
        })
        .unwrap();
        assert!(!master.is_empty());
        assert_eq!(master.search("RELIANCE", 1)[0].segment, "nse_cm");
    }

    #[test]
    fn live_style_fetch_leaves_master_empty_when_csv_fetch_fails() {
        let master =
            load_master_from_file_paths_json(FIXTURE_PATHS, |url| anyhow::bail!("skip {url}"))
                .unwrap();
        assert!(master.is_empty());
    }

    #[test]
    fn empty_fetch_does_not_overwrite_a_loaded_master() {
        let planted = KotakScripMaster::from_csv_bytes(FIXTURE_CSV.as_bytes(), None).unwrap();
        assert!(!planted.is_empty());
        let dest = Mutex::new(planted);
        let empty =
            load_master_from_file_paths_json(FIXTURE_PATHS, |url| anyhow::bail!("skip {url}"))
                .unwrap();
        assert!(empty.is_empty());
        assert!(!install_master_if_nonempty(&dest, empty));
        assert!(!dest.lock().unwrap().is_empty());
        assert!(dest.lock().unwrap().search("RELIANCE", 1).len() >= 1);
    }

    #[test]
    fn cache_rows_do_not_mark_master_fresh() {
        let mut st = crate::data::InstrumentMasterStatus::default();
        st.record_cache_rows(KOTAK_NEO, 2);
        assert_eq!(st.phase, crate::data::InstrumentMasterPhase::Idle);
        assert_eq!(st.symbol_count, 2);
        assert_eq!(st.capability_wire(), "unavailable");
        st.mark_loading(KOTAK_NEO);
        assert_eq!(st.capability_wire(), "loading");
        assert_eq!(st.symbol_count, 2);
    }
}
