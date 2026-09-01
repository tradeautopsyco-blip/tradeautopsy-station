//! Kotak Neo NSE F&O scrip master (named book `kotak-nse-nfo`).
//!
//! Parser + URL picker only this pass. Not cash (`kotak_scrip_master`). Not Binance
//! `eapi` / `exchangeInfo`. Do not blend FO rows into cash.
//!
//! `spawn_nfo_master_refresh` / `refresh_from_session`: parent wires HTTP fetch after
//! this module is `mod`-ed. Same `FILE_PATHS_PATH` as cash, then
//! `authorize_book_call("kotak-nse-nfo", host, GET, FILE_PATHS_PATH, true)` and unsigned
//! GET of the FO CSV (`authorize_book_call("kotak-nse-nfo", "lapi.kotaksecurities.com",
//! GET, fo_path, false)`). Cache via `crate::data::kotak_csv_cache_path(dir, "nse_fo")`.
//! Do not write cash master. Do not call `cash_csv_urls`.

use crate::data::{
    authorize_book_call, is_kotak_nse_fo_scrip_csv_path, json_object_keys, kotak_csv_cache_path,
    parse_nfo_instrument_id, truncate_body, write_raw_cache, InstrumentMasterErrorClass,
    InstrumentMasterFetchError, KOTAK_NEO_ADAPTER_ID, KOTAK_NSE_NFO_BOOK_ID,
};
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

/// SDK file-paths catalog (parent fetch). Same path as cash; different book fence.
pub const FILE_PATHS_PATH: &str = "/script-details/1.0/masterscrip/file-paths";

/// Live FO header 2026-08-28 IST (79 columns; `pToken` absent).
/// Trailing spaces in names are significant; do not trim when comparing to lock.
pub const LOCK_HEADER: &str = "pSymbol,pGroup,pExchSeg,pInstType,pSymbolName,pTrdSymbol,pOptionType,pScripRefKey,pISIN,pAssetCode,pSubGroup,pCombinedSymbol,pDesc,pAmcCode,pContractId,dTickSize ,lLotSize,lExpiryDate ,lMultiplier ,lPrecision,dStrikePrice;,pExchange,pInstName,pExpiryDate,pIssueDate,pMaturityDate,pListingDate,pNoDelStartDate,pNoDelEndDate,pBookClsStartDate,pBookClsEndDate,pRecordDate,pCreditRating,pReAdminDate,pExpulsionDate,pLocalUpdateTime,pDeliveryUnits,pPriceUnits,pLastTradingDate,pTenderPeridEndDate,pTenderPeridStartDate,pSellVarMargin,pBuyVarMargin,pInstrumentInfo,pRemarksText,pSegment,pNav,pNavDate,pMfAmt,pSipSecurity,pFaceValue,pTrdUnits,pExerciseStartDate,pExerciseEndDate,pElmMargin,pVarMargin,pTotProposedLimitValue,pScripBasePrice,pSettlementType,pCurrectionTime,iPermittedToTrade,iBoardLotQty ,iMaxOrderSize ,iLotSize,dOpenInterest ,dHighPriceRange ,dLowPriceRange ,dPriceNum   ,dGenDen,dGenNum,dPriceQuatation ,dIssuerate ,dPriceDen,dWarningQty ,dIssueCapital ,dExposureMargin ,dMinRedemptionQty ,lFreezeQty,CASEligible";

const NFO_SEGMENT: &str = "nse_fo";

/// Option `pInstType` values on the FO master. Mirrors the option half of the cash
/// parser's `REFUSED_FO_INST_TYPES` (`kotak_scrip_master.rs`), which is the only
/// place the venue's instrument-type vocabulary is already written down.
///
/// A futures row shares `pSymbolName` **and** `lExpiryDate` with the options on the
/// same underlying, so name+expiry alone does not separate them. An option chain
/// that contains a FUTIDX row is claiming a future is an option.
const OPTION_INST_TYPES: &[&str] = &["OPTIDX", "OPTSTK", "OPTCUR", "OPTCOM"];

/// True for the option instrument types this book serves. Case-insensitive; the
/// master is uppercase but the lock does not promise that.
pub fn is_option_inst_type(inst_type: &str) -> bool {
    let inst_type = inst_type.trim();
    OPTION_INST_TYPES
        .iter()
        .any(|known| known.eq_ignore_ascii_case(inst_type))
}

/// Sort key for a raw `dStrikePrice;` cell. Ordering only — this parse never reaches
/// the wire and makes no claim about the strike's scale (still NOT SPECIFIED; see
/// `OPTIONS-PRICING.md`). Unparseable cells sort last, never interleaved.
fn strike_sort_key(strike_raw: &str) -> (u8, f64) {
    match strike_raw.trim().parse::<f64>() {
        Ok(value) if value.is_finite() => (0, value),
        _ => (1, 0.0),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KotakNfoContract {
    pub instrument_token: i64,   // pSymbol
    pub segment: String,         // nse_fo
    pub trading_symbol: String,  // pTrdSymbol
    pub name: String,            // pSymbolName
    pub instrument_type: String, // pInstType
    pub option_type: String,     // pOptionType
    pub lot: i64,                // number
    pub tick_raw: String,        // raw dTickSize cell
    pub strike_raw: String,      // raw dStrikePrice; cell — NOT scaled
    pub expiry_raw: String,      // raw lExpiryDate/pExpiryDate
}

impl KotakNfoContract {
    /// Option row (not FUTIDX/FUTSTK/SPREAD). Chain callers must filter on this.
    pub fn is_option(&self) -> bool {
        is_option_inst_type(&self.instrument_type)
    }

    pub fn instrument_id(&self) -> String {
        format!("nse_fo|{}", self.instrument_token)
    }

    pub fn to_contract_row(&self) -> crate::data::ContractRow {
        crate::data::ContractRow {
            instrument_id: self.instrument_id(),
            lot: self.lot,
            trading_symbol: self.trading_symbol.clone(),
            segment: self.segment.clone(),
            instrument_type: self.instrument_type.clone(),
            option_type: self.option_type.clone(),
            strike_raw: self.strike_raw.clone(),
            expiry_raw: self.expiry_raw.clone(),
        }
    }

    pub fn to_chain_row(&self) -> crate::data::ChainRow {
        crate::data::ChainRow {
            instrument_id: self.instrument_id(),
            lot: self.lot,
            trading_symbol: self.trading_symbol.clone(),
            segment: self.segment.clone(),
            instrument_type: self.instrument_type.clone(),
            option_type: self.option_type.clone(),
            strike_raw: self.strike_raw.clone(),
            expiry_raw: self.expiry_raw.clone(),
            last: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct KotakNfoScripMaster {
    by_token: HashMap<i64, KotakNfoContract>,
}

impl KotakNfoScripMaster {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.by_token.len()
    }

    pub fn is_empty(&self) -> bool {
        self.by_token.is_empty()
    }

    /// `nse_fo|{token}` only. Cash / other FO segments → false.
    pub fn contains_id(&self, id: &str) -> bool {
        let Some(canonical) = parse_nfo_instrument_id(id) else {
            return false;
        };
        canonical
            .split_once('|')
            .and_then(|(_, token)| token.parse::<i64>().ok())
            .is_some_and(|token| self.by_token.contains_key(&token))
    }

    pub fn get(&self, token: i64) -> Option<&KotakNfoContract> {
        self.by_token.get(&token)
    }

    /// Exact trading-symbol lookup (`pTrdSymbol`). Trim + ASCII uppercase on both sides.
    /// Duplicate symbols in the master → `None` (miss). Does not prefix-search.
    pub fn get_by_trading_symbol(&self, sym: &str) -> Option<&KotakNfoContract> {
        let needle = sym.trim().to_ascii_uppercase();
        if needle.is_empty() {
            return None;
        }
        let mut hits: Vec<&KotakNfoContract> = self
            .by_token
            .values()
            .filter(|row| row.trading_symbol.trim().to_ascii_uppercase() == needle)
            .collect();
        if hits.len() != 1 {
            return None;
        }
        hits.pop()
    }

    pub fn iter(&self) -> impl Iterator<Item = &KotakNfoContract> + '_ {
        self.by_token.values()
    }

    pub fn merge(&mut self, other: Self) {
        self.by_token.extend(other.by_token);
    }

    /// Underlying key is lock `pSymbolName` (sample `NIFTY`), not the token.
    /// `nse_fo|{token}` resolves that row's name + raw expiry, then the same subset.
    /// Missing master row → absent (no ghost strike).
    pub fn rows_for_underlying(&self, underlying_or_id: &str) -> Vec<&KotakNfoContract> {
        let raw = underlying_or_id.trim();
        if raw.is_empty() {
            return Vec::new();
        }
        if let Some(canonical) = parse_nfo_instrument_id(raw) {
            let token = canonical
                .split_once('|')
                .and_then(|(_, t)| t.parse::<i64>().ok());
            let Some(anchor) = token.and_then(|t| self.by_token.get(&t)) else {
                return Vec::new();
            };
            let name = anchor.name.to_ascii_uppercase();
            let expiry = anchor.expiry_raw.clone();
            return self
                .by_token
                .values()
                .filter(|row| row.name.to_ascii_uppercase() == name && row.expiry_raw == expiry)
                .collect();
        }
        let name = raw.to_ascii_uppercase();
        self.by_token
            .values()
            .filter(|row| row.name.to_ascii_uppercase() == name)
            .collect()
    }

    /// Option rows only, in a stable order — the shape an option chain may claim.
    ///
    /// Two reasons this is not `rows_for_underlying`:
    /// futures share name + expiry with options and must not enter a chain, and
    /// `by_token` is a `HashMap`, so the unsorted order differs between calls.
    /// A ladder that reshuffles per request is not a snapshot.
    ///
    /// `rows_for_underlying` stays unfiltered: the forward leg needs the FUTIDX row.
    pub fn option_rows_for_underlying(&self, underlying_or_id: &str) -> Vec<&KotakNfoContract> {
        let mut rows: Vec<&KotakNfoContract> = self
            .rows_for_underlying(underlying_or_id)
            .into_iter()
            .filter(|row| row.is_option())
            .collect();
        rows.sort_by(|a, b| {
            a.expiry_raw
                .cmp(&b.expiry_raw)
                .then_with(|| {
                    let (a_rank, a_strike) = strike_sort_key(&a.strike_raw);
                    let (b_rank, b_strike) = strike_sort_key(&b.strike_raw);
                    a_rank
                        .cmp(&b_rank)
                        .then_with(|| a_strike.total_cmp(&b_strike))
                })
                .then_with(|| a.option_type.cmp(&b.option_type))
                .then_with(|| a.instrument_token.cmp(&b.instrument_token))
        });
        rows
    }

    /// Prefix search on trading_symbol / name. NFO rows only (cash is never stored).
    pub fn search(&self, q: &str, limit: usize) -> Vec<&KotakNfoContract> {
        let q = q.trim().to_ascii_uppercase();
        if q.is_empty() {
            return Vec::new();
        }
        let mut hits: Vec<&KotakNfoContract> = self
            .by_token
            .values()
            .filter(|row| {
                row.trading_symbol.to_ascii_uppercase().starts_with(&q)
                    || row.name.to_ascii_uppercase().starts_with(&q)
            })
            .collect();
        hits.sort_by(|a, b| {
            a.trading_symbol
                .len()
                .cmp(&b.trading_symbol.len())
                .then_with(|| a.trading_symbol.cmp(&b.trading_symbol))
        });
        hits.truncate(limit);
        hits
    }

    pub fn from_csv_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(Cursor::new(bytes));
        let headers = reader.headers()?.clone();
        let col = |name: &str| headers.iter().position(|h| h.trim() == name);

        let token_i = col("pSymbol");
        let seg_i = col("pExchSeg");
        let inst_i = col("pInstType");
        let name_i = col("pSymbolName");
        let trd_i = col("pTrdSymbol");
        let opt_i = col("pOptionType");
        let tick_i = col("dTickSize");
        let l_lot_i = col("lLotSize");
        let i_lot_i = col("iLotSize");
        let strike_i = col("dStrikePrice;");
        let l_exp_i = col("lExpiryDate");
        let p_exp_i = col("pExpiryDate");

        let Some(token_i) = token_i else {
            return Ok(Self::empty());
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
                .unwrap_or("");
            if !segment.eq_ignore_ascii_case(NFO_SEGMENT) {
                continue;
            }
            let Some(lot) = lot_from_cells(
                l_lot_i.and_then(|i| record.get(i)),
                i_lot_i.and_then(|i| record.get(i)),
            ) else {
                continue;
            };
            let expiry_raw = first_nonempty_raw(
                l_exp_i.and_then(|i| record.get(i)),
                p_exp_i.and_then(|i| record.get(i)),
            );
            master.by_token.insert(
                token,
                KotakNfoContract {
                    instrument_token: token,
                    segment: NFO_SEGMENT.to_string(),
                    trading_symbol: cell_raw(trd_i.and_then(|i| record.get(i))),
                    name: cell_raw(name_i.and_then(|i| record.get(i))),
                    instrument_type: cell_raw(inst_i.and_then(|i| record.get(i))),
                    option_type: cell_raw(opt_i.and_then(|i| record.get(i))),
                    lot,
                    tick_raw: cell_raw(tick_i.and_then(|i| record.get(i))),
                    strike_raw: cell_raw(strike_i.and_then(|i| record.get(i))),
                    expiry_raw,
                },
            );
        }
        Ok(master)
    }
}

fn cell_raw(value: Option<&str>) -> String {
    value.map(str::trim).unwrap_or("").to_string()
}

fn first_nonempty_raw(primary: Option<&str>, fallback: Option<&str>) -> String {
    let primary = cell_raw(primary);
    if !primary.is_empty() {
        return primary;
    }
    cell_raw(fallback)
}

fn parse_i64_cell(raw: Option<&str>) -> Option<i64> {
    let t = raw?.trim();
    if t.is_empty() {
        return None;
    }
    t.parse().ok()
}

/// Both lot columns present and unequal → skip (which-wins NOT SPECIFIED). One present → use it.
fn lot_from_cells(l_lot: Option<&str>, i_lot: Option<&str>) -> Option<i64> {
    match (parse_i64_cell(l_lot), parse_i64_cell(i_lot)) {
        (Some(a), Some(b)) if a == b => Some(a),
        (Some(_), Some(_)) => None,
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// Replace `dest` only when `loaded` is non-empty. Empty must not wipe a loaded master.
pub fn install_master_if_nonempty(
    dest: &Mutex<KotakNfoScripMaster>,
    loaded: KotakNfoScripMaster,
) -> bool {
    if loaded.is_empty() {
        return false;
    }
    *dest.lock().expect("kotak nfo scrip master mutex poisoned") = loaded;
    true
}

/// Keep `nse_fo.csv` / `nse_fo-v1.csv` under `/wso2-scripmaster/`. Drop other FO and cash.
pub fn fo_csv_urls(file_paths_json: &str) -> anyhow::Result<Vec<String>> {
    let value: Value = serde_json::from_str(file_paths_json)?;
    Ok(files_paths_from_value(&value)
        .into_iter()
        .filter(|url| is_extractable_nfo_csv_url(url))
        .collect())
}

fn is_extractable_nfo_csv_url(url: &str) -> bool {
    let Some((host, path)) = split_http_url(url) else {
        return false;
    };
    is_kotak_nse_fo_scrip_csv_path(&path)
        && authorize_book_call(KOTAK_NSE_NFO_BOOK_ID, &host, "GET", &path, false).is_ok()
}

pub fn authorize_nfo_csv_get(url: &str) -> Result<(), String> {
    let (host, path) = split_http_url(url).ok_or_else(|| "path_not_allowlisted".to_string())?;
    authorize_book_call(KOTAK_NSE_NFO_BOOK_ID, &host, "GET", &path, false)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

const REFRESH_BACKOFFS: [Duration; 3] = [
    Duration::from_secs(5),
    Duration::from_secs(30),
    Duration::from_secs(120),
];

/// Load `nse_fo.csv` cache. Empty must not wipe. Does not touch the cash master or cash status.
pub fn try_load_nfo_cache(cache_dir: &Path, master: &Mutex<KotakNfoScripMaster>) {
    let path = kotak_csv_cache_path(cache_dir, "nse_fo");
    let Ok(bytes) = std::fs::read(&path) else {
        return;
    };
    match KotakNfoScripMaster::from_csv_bytes(&bytes) {
        Ok(loaded) => {
            if install_master_if_nonempty(master, loaded) {
                tracing::info!("s1 desk: kotak nfo master loaded from disk cache");
            }
        }
        Err(err) => tracing::warn!(error = %err, "s1 desk: kotak nfo cache CSV skipped"),
    }
}

pub fn spawn_refresh(
    master: Arc<Mutex<KotakNfoScripMaster>>,
    vault: Arc<dyn BrokerCredentialVault>,
    environment: String,
    connection_id: String,
    cancel: Arc<AtomicBool>,
    cache_dir: PathBuf,
    connections: Arc<
        Mutex<std::collections::HashMap<String, crate::data::BrokerConnectionRuntime>>,
    >,
    locator: crate::kotak_rest_quotes::SessionLocator,
) {
    tokio::spawn(async move {
        let mut attempt = 0u8;
        loop {
            if !nfo_refresh_still_active(&cancel, &connections, &locator) {
                return;
            }
            match refresh_from_session(vault.as_ref(), &environment, &connection_id, &cache_dir)
                .await
            {
                Ok(loaded) => {
                    let n = loaded.len();
                    if install_master_if_nonempty(&master, loaded) {
                        tracing::info!(symbols = n, "s1 desk: kotak nfo scrip master loaded");
                        return;
                    }
                    tracing::warn!("s1 desk: kotak nfo master empty after fetch");
                }
                Err(err) => {
                    tracing::warn!(error = %err, "s1 desk: kotak nfo scrip master refresh failed");
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

fn nfo_refresh_still_active(
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
        .is_some_and(|m| m.contains_key(KOTAK_NSE_NFO_BOOK_ID));
    has_locator && has_conn
}

pub async fn refresh_from_session(
    vault: &dyn BrokerCredentialVault,
    environment: &str,
    connection_id: &str,
    cache_dir: &Path,
) -> Result<KotakNfoScripMaster, InstrumentMasterFetchError> {
    let blob = vault
        .load(environment, KOTAK_NEO_ADAPTER_ID, connection_id)
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
    authorize_book_call(KOTAK_NSE_NFO_BOOK_ID, &host, "GET", FILE_PATHS_PATH, true).map_err(
        |_| InstrumentMasterFetchError::new(InstrumentMasterErrorClass::FilePathsHttp, None),
    )?;
    let sdk = prepare_kotak_file_paths_get(FILE_PATHS_PATH, &creds).map_err(|_| {
        InstrumentMasterFetchError::new(InstrumentMasterErrorClass::FilePathsHttp, None)
    })?;
    let (body, catalog_status) = fetch_file_paths_body(sdk, &creds).await?;
    let urls = fo_csv_urls(&body).map_err(|_| {
        tracing::warn!(
            keys = ?json_object_keys(&body),
            "s1 desk: kotak nfo file-paths JSON unusable"
        );
        InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::FilePathsJson,
            Some(catalog_status),
        )
    })?;
    tracing::info!(
        nfo_urls = urls.len(),
        "s1 desk: kotak file-paths nse_fo CSV URLs"
    );
    if urls.is_empty() {
        return Err(InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::FilePathsJson,
            Some(catalog_status),
        ));
    }
    let mut master = KotakNfoScripMaster::empty();
    let mut saw_http = false;
    let mut saw_allowlist = false;
    let mut saw_parse = false;
    let mut last_csv_status: Option<u16> = None;
    for url in urls {
        match authorize_nfo_csv_get(&url) {
            Ok(()) => match fetch_nfo_csv(&url).await {
                Ok(resp) => {
                    last_csv_status = Some(resp.status);
                    if !resp.is_success() {
                        saw_http = true;
                        tracing::warn!(status = resp.status, "s1 desk: kotak nfo CSV HTTP error");
                        continue;
                    }
                    let bytes = resp.body.into_bytes();
                    match KotakNfoScripMaster::from_csv_bytes(&bytes) {
                        Ok(part) => {
                            if !part.is_empty() {
                                write_raw_cache(&kotak_csv_cache_path(cache_dir, "nse_fo"), &bytes);
                                master.merge(part);
                            } else {
                                saw_parse = true;
                            }
                        }
                        Err(err) => {
                            saw_parse = true;
                            tracing::warn!(error = %err, "s1 desk: kotak nfo CSV parse failed");
                        }
                    }
                }
                Err(err) => {
                    saw_http = true;
                    tracing::warn!(error = %err, "s1 desk: kotak nfo CSV GET failed");
                }
            },
            Err(err) => {
                saw_allowlist = true;
                tracing::warn!(error = %err, "kotak nfo CSV GET fail-closed (host/path not allowlisted)");
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
    Ok(master)
}

/// Signed by `prepare_kotak_file_paths_get`, still admitted by the engine. The
/// NFO book shares Kotak's one meter with cash, because the lock does not say
/// they have separate budgets.
async fn send_prepared(
    prepared: &PreparedHttpRequest,
) -> Result<(reqwest::StatusCode, String), InstrumentMasterFetchError> {
    let resp = crate::egress::shared()
        .send_prepared(
            KOTAK_NSE_NFO_BOOK_ID,
            crate::egress::Lane::PrivateRead,
            prepared,
            Duration::from_secs(15),
        )
        .await
        .map_err(|_| {
            InstrumentMasterFetchError::new(InstrumentMasterErrorClass::FilePathsHttp, None)
        })?;
    let status = reqwest::StatusCode::from_u16(resp.status)
        .unwrap_or(reqwest::StatusCode::INTERNAL_SERVER_ERROR);
    Ok((status, resp.body))
}

/// Public F&O scrip CSV on `lapi`. Large, slow, never coalesced.
async fn fetch_nfo_csv(
    url: &str,
) -> Result<crate::egress::EgressResponse, InstrumentMasterFetchError> {
    let Some((host, path, query)) = crate::egress::split_url(url) else {
        return Err(InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::CsvHttp,
            None,
        ));
    };
    let call = crate::egress::EgressCall::get(
        KOTAK_NSE_NFO_BOOK_ID,
        &host,
        &path,
        crate::egress::Lane::MarketData,
    )
    .with_query(query)
    .with_timeout(Duration::from_secs(90));
    crate::egress::shared()
        .send(&call)
        .await
        .map_err(|_| InstrumentMasterFetchError::new(InstrumentMasterErrorClass::CsvHttp, None))
}

async fn fetch_file_paths_body(
    sdk: PreparedHttpRequest,
    creds: &HostCredentialBlob,
) -> Result<(String, u16), InstrumentMasterFetchError> {
    let (status, body) = send_prepared(&sdk).await?;
    if status.is_success() && !body.contains("Complete the 2fa process") {
        return Ok((body, status.as_u16()));
    }
    if matches!(status.as_u16(), 401 | 403) || body.contains("Complete the 2fa process") {
        let session = attach_kotak_file_paths_session(sdk, creds).map_err(|_| {
            InstrumentMasterFetchError::new(
                InstrumentMasterErrorClass::FilePathsHttp,
                Some(status.as_u16()),
            )
        })?;
        let (status2, body2) = send_prepared(&session).await?;
        if status2.is_success() {
            return Ok((body2, status2.as_u16()));
        }
        tracing::warn!(
            status = %status2,
            keys = ?json_object_keys(&body2),
            body_preview = %truncate_body(&body2, 200),
            "s1 desk: kotak nfo file-paths HTTP error"
        );
        return Err(InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::FilePathsHttp,
            Some(status2.as_u16()),
        ));
    }
    if !status.is_success() {
        return Err(InstrumentMasterFetchError::new(
            InstrumentMasterErrorClass::FilePathsHttp,
            Some(status.as_u16()),
        ));
    }
    Ok((body, status.as_u16()))
}

fn split_http_url(url: &str) -> Option<(String, String)> {
    let parsed = reqwest::Url::parse(url).ok()?;
    let host = parsed.host_str()?.to_ascii_lowercase();
    Some((host, parsed.path().to_string()))
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

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_CSV: &str = include_str!("../fixtures/kotak/nse_fo_header.csv");
    const CASH_CSV: &str = include_str!("../fixtures/kotak/nse_cm_cash.csv");
    const FIXTURE_PATHS: &str = include_str!("../fixtures/kotak/scrip_file_paths.json");

    fn fixture_header_line() -> &'static str {
        FIXTURE_CSV.lines().next().expect("fixture header")
    }

    #[test]
    fn get_by_trading_symbol_exact_match_case_insensitive() {
        let master = KotakNfoScripMaster::from_csv_bytes(FIXTURE_CSV.as_bytes()).unwrap();
        assert_eq!(
            master
                .get_by_trading_symbol("nifty2692221000pe")
                .map(|r| r.lot),
            Some(65)
        );
        assert!(master.get_by_trading_symbol("NIFTY25JUL24000CE").is_none());
    }

    #[test]
    fn get_by_trading_symbol_duplicate_returns_miss() {
        let csv = csv_with_rows(&[
            &[],
            &[(COL_SYMBOL, "56528"), (COL_TRD_SYMBOL, "NIFTY2692221000PE")],
        ]);
        let master = KotakNfoScripMaster::from_csv_bytes(csv.as_bytes()).unwrap();
        assert!(master.get_by_trading_symbol("NIFTY2692221000PE").is_none());
    }

    #[test]
    fn fixture_header_equals_lock_verbatim() {
        assert_eq!(fixture_header_line(), LOCK_HEADER);
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .from_reader(Cursor::new(FIXTURE_CSV.as_bytes()));
        let joined = reader
            .headers()
            .unwrap()
            .iter()
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(joined, LOCK_HEADER);
    }

    #[test]
    fn parse_fixture_one_nfo_row_lot_65() {
        let master = KotakNfoScripMaster::from_csv_bytes(FIXTURE_CSV.as_bytes()).unwrap();
        assert_eq!(master.len(), 2, "fixture includes OPTIDX + OPTSTK rows");
        let row = master.get(56526).expect("token 56526");
        assert_eq!(row.instrument_token, 56526);
        assert_eq!(row.segment, "nse_fo");
        assert_eq!(row.lot, 65i64);
        assert_ne!(row.lot, 25);
        assert_ne!(row.lot, 50);
        assert_ne!(row.lot, 75);
        assert_eq!(row.trading_symbol, "NIFTY2692221000PE");
        assert_eq!(row.option_type, "PE");
        assert_eq!(row.instrument_type, "OPTIDX");
        assert_eq!(row.name, "NIFTY");
        assert!(
            row.strike_raw.contains("2.1e+06") || row.strike_raw.contains("2100000"),
            "strike_raw must be the cell, not a scaled guess: {:?}",
            row.strike_raw
        );
        assert_ne!(row.strike_raw, "57500");
        assert_eq!(row.tick_raw, "5");
        assert_eq!(row.expiry_raw, "1474554600");
        assert_eq!(row.instrument_id(), "nse_fo|56526");
        let extracted = row.to_contract_row();
        assert_eq!(extracted.lot, 65);
        assert_eq!(extracted.instrument_id, "nse_fo|56526");
        assert_eq!(extracted.strike_raw, row.strike_raw);
        let by_name = master.rows_for_underlying("NIFTY");
        assert_eq!(by_name.len(), 1);
        assert_eq!(by_name[0].instrument_id(), "nse_fo|56526");
        let by_id = master.rows_for_underlying("nse_fo|56526");
        assert_eq!(by_id.len(), 1);
        assert!(master.rows_for_underlying("BANKNIFTY").is_empty());
        assert!(master.rows_for_underlying("nse_cm|2885").is_empty());
    }

    /// Build a CSV body from the locked header plus caller-supplied rows derived
    /// from the fixture row. Column indices follow `LOCK_HEADER`.
    fn csv_with_rows(rows: &[&[(usize, &str)]]) -> String {
        let header = fixture_header_line();
        let base: Vec<&str> = FIXTURE_CSV
            .lines()
            .nth(1)
            .expect("fixture data row")
            .split(',')
            .collect();
        let mut body = String::from(header);
        for overrides in rows {
            let mut cells = base.clone();
            for (index, value) in *overrides {
                cells[*index] = value;
            }
            body.push('\n');
            body.push_str(&cells.join(","));
        }
        body
    }

    // LOCK_HEADER column indices used below.
    const COL_SYMBOL: usize = 0;
    const COL_INST_TYPE: usize = 3;
    const COL_TRD_SYMBOL: usize = 5;
    const COL_OPTION_TYPE: usize = 6;
    const COL_STRIKE: usize = 20;

    #[test]
    fn futures_row_sharing_name_and_expiry_is_not_a_chain_row() {
        // Same pSymbolName (NIFTY) and same lExpiryDate as the option — the pair
        // name+expiry cannot separate them, only pInstType can.
        let csv = csv_with_rows(&[
            &[],
            &[
                (COL_SYMBOL, "56527"),
                (COL_INST_TYPE, "FUTIDX"),
                (COL_TRD_SYMBOL, "NIFTY26SEPFUT"),
                (COL_OPTION_TYPE, "XX"),
                (COL_STRIKE, "0"),
            ],
        ]);
        let master = KotakNfoScripMaster::from_csv_bytes(csv.as_bytes()).unwrap();
        assert_eq!(master.len(), 2, "both rows parse into the master");

        // The unfiltered accessor still sees the future — the forward leg needs it.
        let all = master.rows_for_underlying("NIFTY");
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|row| row.instrument_type == "FUTIDX"));

        // The chain accessor must not.
        let options = master.option_rows_for_underlying("NIFTY");
        assert_eq!(options.len(), 1, "FUTIDX must not reach an option chain");
        assert_eq!(options[0].instrument_id(), "nse_fo|56526");
        assert!(options.iter().all(|row| row.is_option()));

        // Anchoring by the option's own id resolves name+expiry, and still refuses
        // the future that shares both.
        let by_id = master.option_rows_for_underlying("nse_fo|56526");
        assert_eq!(by_id.len(), 1);
        assert_eq!(by_id[0].instrument_type, "OPTIDX");

        // And it is absent from the rendered chain, not merely untagged.
        let rows: Vec<_> = options.into_iter().map(|row| row.to_chain_row()).collect();
        let envelope = crate::data::extract_chain_from(
            Some(KOTAK_NSE_NFO_BOOK_ID),
            "NIFTY",
            Some(&rows),
            None,
        );
        let wire = serde_json::to_string(&envelope).unwrap();
        assert!(
            !wire.contains("FUTIDX"),
            "chain wire named a future: {wire}"
        );
        assert!(!wire.contains("NIFTY26SEPFUT"));
        assert_eq!(envelope.data.as_ref().unwrap()["row_count"], 1);
    }

    #[test]
    fn option_rows_are_ordered_not_hash_ordered() {
        let csv = csv_with_rows(&[
            &[],
            &[
                (COL_SYMBOL, "56528"),
                (COL_TRD_SYMBOL, "NIFTY2692220000CE"),
                (COL_OPTION_TYPE, "CE"),
                (COL_STRIKE, "2e+06"),
            ],
            &[
                (COL_SYMBOL, "56529"),
                (COL_TRD_SYMBOL, "NIFTY2692222000CE"),
                (COL_OPTION_TYPE, "CE"),
                (COL_STRIKE, "2.2e+06"),
            ],
        ]);
        let master = KotakNfoScripMaster::from_csv_bytes(csv.as_bytes()).unwrap();
        let ids: Vec<String> = master
            .option_rows_for_underlying("NIFTY")
            .into_iter()
            .map(|row| row.instrument_id())
            .collect();
        // Strike ascending: 2e+06 < 2.1e+06 < 2.2e+06. Scientific notation means a
        // lexical sort of the raw cell would order these wrong.
        assert_eq!(ids, vec!["nse_fo|56528", "nse_fo|56526", "nse_fo|56529"]);

        // Stable across calls — `by_token` is a HashMap.
        for _ in 0..8 {
            let again: Vec<String> = master
                .option_rows_for_underlying("NIFTY")
                .into_iter()
                .map(|row| row.instrument_id())
                .collect();
            assert_eq!(again, ids);
        }
    }

    #[test]
    fn only_option_inst_types_are_options() {
        for inst in ["OPTIDX", "OPTSTK", "OPTCUR", "OPTCOM", "optidx"] {
            assert!(is_option_inst_type(inst), "{inst} is an option type");
        }
        for inst in [
            "FUTIDX", "FUTSTK", "FUTCUR", "FUTCOM", "FUTIVX", "SPREAD", "",
        ] {
            assert!(!is_option_inst_type(inst), "{inst} must not be an option");
        }
    }

    #[test]
    fn contains_id_nse_fo_only() {
        let master = KotakNfoScripMaster::from_csv_bytes(FIXTURE_CSV.as_bytes()).unwrap();
        assert!(master.contains_id("nse_fo|56526"));
        assert!(!master.contains_id("nse_cm|2885"));
    }

    #[test]
    fn cash_csv_yields_empty() {
        let master = KotakNfoScripMaster::from_csv_bytes(CASH_CSV.as_bytes()).unwrap();
        assert!(
            master.is_empty(),
            "nse_cm skipped; pToken is not the FO token"
        );
        assert!(!master.contains_id("nse_cm|2885"));
        assert!(!master.contains_id("nse_fo|12345"));
    }

    #[test]
    fn fo_csv_urls_keeps_nse_fo_drops_others() {
        let urls = fo_csv_urls(FIXTURE_PATHS).unwrap();
        assert_eq!(urls.len(), 1);
        assert!(urls.iter().any(|u| u.ends_with("/nse_fo.csv")));
        assert!(urls.iter().all(|u| {
            let name = u.rsplit('/').next().unwrap_or(u);
            name.to_ascii_lowercase().starts_with("nse_fo")
        }));
        assert!(urls.iter().all(|u| !u.contains("cde_fo")
            && !u.contains("mcx_fo")
            && !u.contains("bse_fo")
            && !u.contains("nse_cm")));
        assert!(FIXTURE_PATHS.contains("cde_fo.csv"));
        assert!(FIXTURE_PATHS.contains("mcx_fo.csv"));
        assert!(FIXTURE_PATHS.contains("bse_fo.csv"));
        assert!(FIXTURE_PATHS.contains("nse_cm.csv"));
        for url in &urls {
            authorize_nfo_csv_get(url).expect("nse_fo CSV GET allowlisted on kotak-nse-nfo");
        }
    }

    #[test]
    fn bad_non_numeric_token_skipped() {
        let csv = format!(
            "{LOCK_HEADER}\nabc,XX,nse_fo,OPTIDX,NIFTY,NIFTYBAD,PE,NIFTYBAD,,26000,,,,,,5,65,1474554600,-1,2,2.1e+06,NSE,OPTIDX,1474554600\n"
        );
        let master = KotakNfoScripMaster::from_csv_bytes(csv.as_bytes()).unwrap();
        assert!(master.is_empty());

        let mixed = format!(
            "{}\nnot-a-token,XX,nse_fo,OPTIDX,NIFTY,BAD,PE\n",
            FIXTURE_CSV.trim_end()
        );
        let master = KotakNfoScripMaster::from_csv_bytes(mixed.as_bytes()).unwrap();
        assert_eq!(master.len(), 2);
        assert!(master.contains_id("nse_fo|56526"));
    }

    #[test]
    fn search_nifty_returns_nfo_not_cash() {
        let master = KotakNfoScripMaster::from_csv_bytes(FIXTURE_CSV.as_bytes()).unwrap();
        let hits = master.search("NIFTY", 10);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].trading_symbol, "NIFTY2692221000PE");
        assert_eq!(hits[0].instrument_token, 56526);
        assert!(hits.iter().all(|row| row.segment == "nse_fo"));
        assert!(hits.iter().all(|row| row.segment != "nse_cm"));

        let cash_as_nfo = KotakNfoScripMaster::from_csv_bytes(CASH_CSV.as_bytes()).unwrap();
        assert!(cash_as_nfo.search("NIFTY", 10).is_empty());
        assert!(cash_as_nfo.search("RELIANCE", 10).is_empty());
    }

    #[test]
    fn lot_columns_must_agree_or_row_is_skipped() {
        let disagree =
            "pSymbol,pExchSeg,lLotSize,iLotSize,pTrdSymbol,pSymbolName,pInstType,pOptionType\n\
56526,nse_fo,65,50,NIFTY2692221000PE,NIFTY,OPTIDX,PE\n";
        let master = KotakNfoScripMaster::from_csv_bytes(disagree.as_bytes()).unwrap();
        assert!(master.is_empty());

        let one_col = "pSymbol,pExchSeg,lLotSize,pTrdSymbol\n56526,nse_fo,65,NIFTY2692221000PE\n";
        let master = KotakNfoScripMaster::from_csv_bytes(one_col.as_bytes()).unwrap();
        assert_eq!(master.get(56526).map(|r| r.lot), Some(65));
    }

    #[test]
    fn empty_install_does_not_wipe_loaded_master() {
        let planted = KotakNfoScripMaster::from_csv_bytes(FIXTURE_CSV.as_bytes()).unwrap();
        let dest = Mutex::new(planted);
        assert!(!install_master_if_nonempty(
            &dest,
            KotakNfoScripMaster::empty()
        ));
        assert!(dest.lock().unwrap().contains_id("nse_fo|56526"));
    }

    #[test]
    fn merge_iter_and_file_paths_constant() {
        assert_eq!(
            FILE_PATHS_PATH,
            "/script-details/1.0/masterscrip/file-paths"
        );
        let part = KotakNfoScripMaster::from_csv_bytes(FIXTURE_CSV.as_bytes()).unwrap();
        let mut master = KotakNfoScripMaster::empty();
        master.merge(part);
        assert_eq!(master.iter().count(), 2);
        assert_eq!(master.len(), 2);
    }
}
