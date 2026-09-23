//! Host-mediated Kotak Neo v3.0.6 historical candles.
//!
//! `GET /market-data/1.0/historical/details` with consumer_key only (SDK
//! `historical_data.py`). Cash `nse_cm` / `bse_cm` only. Empty series is
//! unavailable — never a vendor fill.

use crate::data::kotak_historical::{
    obtain_from_body, plan_cash_history, query_path, unsupported_segment, NativeHistoryPlan,
    HISTORICAL_DETAILS_PATH,
};
use crate::data::{ObtainEnvelope, KOTAK_NSE_BSE_CASH_BOOK_ID};
use crate::kotak_scrip_master::KOTAK_NEO;
use crate::ubi::{
    kotak_base_host, prepare_kotak_market_data_get, BrokerCredentialVault, HostCredentialBlob,
};
use chrono::{FixedOffset, Utc};
use std::sync::Arc;
use std::time::Duration;

use crate::kotak_rest_quotes::SessionLocator;

#[derive(Debug, Clone, PartialEq)]
pub enum NativeHistory {
    Skip,
    Unsupported(ObtainEnvelope),
    /// Kotak answered (success candles, or empty/unusable body). Do not vendor.
    Answered(ObtainEnvelope),
}

fn ist_today() -> chrono::NaiveDate {
    let ist = FixedOffset::east_opt(5 * 3600 + 30 * 60).expect("IST offset");
    Utc::now().with_timezone(&ist).date_naive()
}

pub async fn try_native_cash_history(
    vault: &Arc<dyn BrokerCredentialVault>,
    locator: &SessionLocator,
    instrument: Option<&str>,
    interval: Option<&str>,
    fromdate: Option<&str>,
    todate: Option<&str>,
) -> NativeHistory {
    match plan_cash_history(
        KOTAK_NSE_BSE_CASH_BOOK_ID,
        instrument,
        interval,
        fromdate,
        todate,
        ist_today(),
    ) {
        NativeHistoryPlan::Skip => NativeHistory::Skip,
        NativeHistoryPlan::Unsupported => NativeHistory::Unsupported(unsupported_segment()),
        NativeHistoryPlan::Fetch {
            neosymbol,
            interval,
            fromdate,
            todate,
        } => match fetch_historical_json(
            vault.as_ref(),
            locator,
            &neosymbol,
            &interval,
            &fromdate,
            &todate,
        )
        .await
        {
            Some(body) => NativeHistory::Answered(obtain_from_body(&neosymbol, &interval, &body)),
            None => NativeHistory::Skip,
        },
    }
}

async fn fetch_historical_json(
    vault: &dyn BrokerCredentialVault,
    locator: &SessionLocator,
    neosymbol: &str,
    interval: &str,
    fromdate: &str,
    todate: &str,
) -> Option<serde_json::Value> {
    let loc = locator
        .lock()
        .expect("kotak session locator poisoned")
        .clone()?;
    let (environment, connection_id) = loc;
    let blob = vault
        .load(&environment, KOTAK_NEO, &connection_id)
        .ok()
        .flatten()?;
    let creds = HostCredentialBlob::from(&blob);
    let base_url = match &creds {
        HostCredentialBlob::KotakSession { base_url, .. } => base_url.clone(),
        HostCredentialBlob::Hmac { .. } | HostCredentialBlob::KiteSession { .. } => return None,
    };
    kotak_base_host(&base_url)?;
    let path = query_path(neosymbol, interval, fromdate, todate);
    let prepared = prepare_kotak_market_data_get(&path, &creds).ok()?;
    let resp = crate::egress::shared()
        .send_prepared(
            KOTAK_NSE_BSE_CASH_BOOK_ID,
            crate::egress::Lane::PrivateRead,
            &prepared,
            Duration::from_secs(15),
        )
        .await
        .ok()?;
    if matches!(resp.status, 401 | 403) {
        return None;
    }
    if !resp.is_success() {
        tracing::warn!(
            status = resp.status,
            path = HISTORICAL_DETAILS_PATH,
            "s1 desk: kotak historical HTTP error"
        );
        return None;
    }
    serde_json::from_str(&resp.body).ok()
}
