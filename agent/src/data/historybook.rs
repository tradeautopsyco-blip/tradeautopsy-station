//! Runtime historical_series store. Not TickBook. Not session OHLC on the quote envelope.

use super::binance_klines::HistorySeries;
use super::history_store::HistoryStore;
use std::collections::HashMap;
use std::path::Path;

fn key(adapter_id: &str, instrument_id: &str, interval: &str) -> String {
    format!(
        "{}\0{}\0{}",
        adapter_id.trim(),
        super::binance_public::normalize_quote_instrument(instrument_id),
        interval.trim()
    )
}

#[derive(Clone, Default)]
pub struct HistoryBook {
    rows: HashMap<String, HistorySeries>,
    store: Option<HistoryStore>,
}

impl std::fmt::Debug for HistoryBook {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HistoryBook")
            .field("rows", &self.rows)
            .field("persisted", &self.store.is_some())
            .finish()
    }
}

impl HistoryBook {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open sqlite coverage, then load every stored series into memory.
    pub fn open(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let store = HistoryStore::open(path)?;
        let mut rows = HashMap::new();
        for series in store.load_all_series()? {
            if series.candles.is_empty() {
                continue;
            }
            let k = key(&series.adapter_id, &series.instrument_id, &series.interval);
            rows.insert(k, series);
        }
        Ok(Self {
            rows,
            store: Some(store),
        })
    }

    pub fn get(
        &self,
        adapter_id: &str,
        instrument_id: &str,
        interval: &str,
    ) -> Option<&HistorySeries> {
        self.rows
            .get(&key(adapter_id, instrument_id, interval))
            .filter(|row| !row.candles.is_empty())
    }

    pub fn upsert(&mut self, series: HistorySeries) {
        if series.candles.is_empty() {
            return;
        }
        if let Some(store) = &self.store {
            if let Err(err) = store.upsert_series(&series) {
                tracing::warn!(error = %err, "history store persist failed");
            }
        }
        let k = key(&series.adapter_id, &series.instrument_id, &series.interval);
        self.rows.insert(k, series);
    }

    pub fn first_for_adapter(&self, adapter_id: &str) -> Option<&HistorySeries> {
        self.rows
            .values()
            .find(|row| row.adapter_id == adapter_id && !row.candles.is_empty())
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::binance_klines::series_from_klines_json;
    use crate::data::tick::Transport;

    const FIXTURE: &str = include_str!("../../fixtures/binance/klines.json");

    fn temp_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("rta-historybook-unit-{}.db", uuid::Uuid::new_v4()))
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(path.with_extension("db-wal"));
        let _ = std::fs::remove_file(path.with_extension("db-shm"));
    }

    #[test]
    fn historybook_upsert_reopen_returns_same_last_close() {
        let path = temp_path();
        cleanup(&path);
        let series =
            series_from_klines_json(FIXTURE, "BTCUSDT", "1m", Transport::Fixture).expect("fixture");
        let want_close = series.candles.last().unwrap().close.clone();

        {
            let mut book = HistoryBook::open(&path).expect("open");
            book.upsert(series);
            assert_eq!(
                book.get("binance_com", "btcusdt", "1m")
                    .and_then(|row| row.candles.last())
                    .map(|c| c.close.as_str()),
                Some(want_close.as_str())
            );
        }

        let book = HistoryBook::open(&path).expect("reopen");
        let loaded = book.get("binance_com", "BTCUSDT", "1m").expect("loaded");
        assert_eq!(
            loaded.candles.last().map(|c| c.close.as_str()),
            Some(want_close.as_str())
        );
        assert_eq!(loaded.candles.len(), 2);
        assert_eq!(loaded.candles[0].open, "0.01634790");
        cleanup(&path);
    }

    #[test]
    fn historybook_new_skips_empty_and_does_not_need_store() {
        let mut book = HistoryBook::new();
        book.upsert(HistorySeries {
            instrument_id: "btcusdt".into(),
            adapter_id: "binance_com".into(),
            interval: "1m".into(),
            candles: Vec::new(),
            transport: Transport::Fixture,
        });
        assert!(book.get("binance_com", "btcusdt", "1m").is_none());
        assert!(book.is_empty());
    }
}
