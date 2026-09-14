//! Book-scoped quote selection. Each TickBook slot keeps its own bound instrument.

use crate::data::{
    BINANCE_COM_OPTIONS_BOOK_ID, BINANCE_COM_SPOT_BOOK_ID, KOTAK_NSE_BSE_CASH_BOOK_ID,
    KOTAK_NSE_NFO_BOOK_ID,
};
use std::collections::HashMap;

const KOTAK_NEO: &str = "kotak_neo";
const BINANCE_COM: &str = "binance_com";

/// Per-book selected instrument ids (`book_id` → `instrument_id`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuoteSelections {
    by_book: HashMap<String, String>,
}

impl QuoteSelections {
    pub fn bind(&mut self, book_id: &str, instrument: String) {
        let id = book_id.trim();
        if id.is_empty() || instrument.trim().is_empty() {
            return;
        }
        self.by_book.insert(id.to_string(), instrument);
    }

    pub fn selected(&self, book_id: &str) -> Option<&str> {
        self.by_book.get(book_id.trim()).map(String::as_str)
    }

    pub fn clear_book(&mut self, book_id: &str) {
        self.by_book.remove(book_id.trim());
    }

    /// Scoped broker stop: Kotak clears cash + NFO; Binance clears spot only.
    pub fn clear_adapter_books(&mut self, adapter_id: &str) {
        match adapter_id.trim().to_ascii_lowercase().as_str() {
            KOTAK_NEO => {
                self.clear_book(KOTAK_NSE_BSE_CASH_BOOK_ID);
                self.clear_book(KOTAK_NSE_NFO_BOOK_ID);
            }
            BINANCE_COM => {
                self.clear_book(BINANCE_COM_SPOT_BOOK_ID);
            }
            _ => {}
        }
    }

    pub fn clear_all(&mut self) {
        self.by_book.clear();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedQuoteBinding {
    pub book_id: &'static str,
    pub instrument_id: String,
    pub source: QuoteSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteSource {
    BinanceOptionsPublic,
    BinanceSpotPublic,
    KotakPrivate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuoteBindError {
    InstrumentInvalid,
    InstrumentBookMismatch,
    InstrumentNotInMaster,
    BookNotShipping,
    PrivateSessionUnavailable,
    UnknownBook,
}

impl QuoteBindError {
    pub fn refusal_class(self) -> &'static str {
        match self {
            Self::InstrumentInvalid => "instrument_invalid",
            Self::InstrumentBookMismatch => "instrument_book_mismatch",
            Self::InstrumentNotInMaster => "instrument_not_in_master",
            Self::BookNotShipping => "book_not_shipping",
            Self::PrivateSessionUnavailable => "private_session_unavailable",
            Self::UnknownBook => "unknown_book",
        }
    }

    /// Wire `book_id` on a refused bind. Prefer the inferred target book when known.
    pub fn book_id_for_refusal(
        self,
        requested_book: Option<&str>,
        inferred_book: Option<&str>,
    ) -> String {
        if matches!(self, Self::UnknownBook) {
            return requested_book
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .unwrap_or_default()
                .to_string();
        }
        inferred_book
            .or(requested_book.map(str::trim).filter(|s| !s.is_empty()))
            .unwrap_or_default()
            .to_string()
    }
}

pub fn is_known_quote_book(book_id: &str) -> bool {
    matches!(
        book_id.trim(),
        BINANCE_COM_OPTIONS_BOOK_ID
            | BINANCE_COM_SPOT_BOOK_ID
            | KOTAK_NSE_NFO_BOOK_ID
            | KOTAK_NSE_BSE_CASH_BOOK_ID
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kotak_stop_clears_cash_and_nfo_not_options() {
        let mut sel = QuoteSelections::default();
        sel.bind(KOTAK_NSE_BSE_CASH_BOOK_ID, "nse_cm|2885".into());
        sel.bind(KOTAK_NSE_NFO_BOOK_ID, "nse_fo|12345".into());
        sel.bind(BINANCE_COM_OPTIONS_BOOK_ID, "BTC-260925-145000-C".into());
        sel.clear_adapter_books("kotak_neo");
        assert!(sel.selected(KOTAK_NSE_BSE_CASH_BOOK_ID).is_none());
        assert!(sel.selected(KOTAK_NSE_NFO_BOOK_ID).is_none());
        assert_eq!(
            sel.selected(BINANCE_COM_OPTIONS_BOOK_ID),
            Some("BTC-260925-145000-C")
        );
    }

    #[test]
    fn binance_stop_clears_spot_not_options() {
        let mut sel = QuoteSelections::default();
        sel.bind(BINANCE_COM_SPOT_BOOK_ID, "btcusdt".into());
        sel.bind(BINANCE_COM_OPTIONS_BOOK_ID, "BTC-260925-145000-C".into());
        sel.clear_adapter_books("binance_com");
        assert!(sel.selected(BINANCE_COM_SPOT_BOOK_ID).is_none());
        assert_eq!(
            sel.selected(BINANCE_COM_OPTIONS_BOOK_ID),
            Some("BTC-260925-145000-C")
        );
    }
}
