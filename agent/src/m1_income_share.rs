//! M1 share-up from Binance income poll (`ensure_*_realized_income`).

use crate::coinm_realized_pnl::{realized_pnl_rows_local_today, CoinmRealizedSlot};
use crate::data::{BINANCE_COM_COINM_BOOK_ID, BINANCE_COM_USDM_BOOK_ID};
use crate::fact_outbox::FactOutbox;
use crate::usdm_realized_pnl::{realized_pnl_rows_local_today as usdm_today_rows, UsdmRealizedSlot};
use std::sync::Arc;

pub fn share_usdm_realized_income_today(outbox: &FactOutbox, slot: &UsdmRealizedSlot) {
    let rows = usdm_today_rows(&slot.income_rows);
    let _ = outbox.enqueue_cited_usdm_income(BINANCE_COM_USDM_BOOK_ID, &rows);
}

pub fn share_coinm_realized_income_today(outbox: &FactOutbox, slot: &CoinmRealizedSlot) {
    let rows = realized_pnl_rows_local_today(&slot.income_rows);
    let _ = outbox.enqueue_cited_coinm_income(BINANCE_COM_COINM_BOOK_ID, &rows);
}

pub fn share_usdm_from_state(
    outbox: &Arc<FactOutbox>,
    slot: &Arc<std::sync::Mutex<Option<UsdmRealizedSlot>>>,
) {
    let Ok(guard) = slot.lock() else {
        return;
    };
    let Some(ref s) = *guard else {
        return;
    };
    share_usdm_realized_income_today(outbox.as_ref(), s);
}

pub fn share_coinm_from_state(
    outbox: &Arc<FactOutbox>,
    slot: &Arc<std::sync::Mutex<Option<CoinmRealizedSlot>>>,
) {
    let Ok(guard) = slot.lock() else {
        return;
    };
    let Some(ref s) = *guard else {
        return;
    };
    share_coinm_realized_income_today(outbox.as_ref(), s);
}
