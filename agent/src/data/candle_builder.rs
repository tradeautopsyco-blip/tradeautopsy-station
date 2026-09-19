//! Interval OHLC from ticks. Not a resample of coarser bars.
//!
//! Seed from the last closed (or still-open) history bar before any tick.
//! An unseeded first tick opens a bucket at the tick price and is `provisional`.
//! DualNoBlend: one builder key is adapter + instrument + interval.

use super::binance_klines::{HistoryCandle, HistorySeries};
use super::binance_options_public::{is_dated_option_contract, normalize_options_instrument};
use super::historybook::HistoryBook;
use super::tick::Transport;
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct PendingTick {
    time_ms: i64,
    price: String,
    qty: Option<String>,
}

pub fn interval_ms(code: &str) -> Option<i64> {
    Some(match code.trim() {
        "1s" => 1_000,
        "1m" | "1min" => 60_000,
        "3m" | "3min" => 180_000,
        "5m" | "5min" => 300_000,
        "10m" | "10min" => 600_000,
        "15m" | "15min" => 900_000,
        "30m" | "30min" => 1_800_000,
        "60min" | "1h" => 3_600_000,
        "2h" => 7_200_000,
        "4h" => 14_400_000,
        "D" | "1d" | "1D" => 86_400_000,
        "W" | "1w" | "1W" => 604_800_000,
        _ => return None,
    })
}

pub fn bucket_open_ms(time_ms: i64, interval_ms: i64, anchor_ms: i64) -> i64 {
    if interval_ms <= 0 {
        return time_ms;
    }
    let delta = time_ms.saturating_sub(anchor_ms);
    let n = delta.div_euclid(interval_ms);
    anchor_ms.saturating_add(n.saturating_mul(interval_ms))
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandleBuilder {
    pub adapter_id: String,
    pub instrument_id: String,
    pub interval: String,
    interval_ms: i64,
    session_anchor_ms: i64,
    current: Option<HistoryCandle>,
    seeded: bool,
    provisional: bool,
}

impl CandleBuilder {
    pub fn new(
        adapter_id: impl Into<String>,
        instrument_id: impl Into<String>,
        interval: impl Into<String>,
        interval_ms: i64,
        session_anchor_ms: i64,
    ) -> Self {
        Self {
            adapter_id: adapter_id.into(),
            instrument_id: instrument_id.into(),
            interval: interval.into(),
            interval_ms,
            session_anchor_ms,
            current: None,
            seeded: false,
            provisional: false,
        }
    }

    pub fn seed(&mut self, bar: HistoryCandle) {
        self.session_anchor_ms = bar.open_time_ms;
        self.current = Some(bar);
        self.seeded = true;
        self.provisional = false;
    }

    pub fn seeded(&self) -> bool {
        self.seeded
    }

    pub fn is_provisional(&self) -> bool {
        self.provisional
    }

    pub fn current(&self) -> Option<&HistoryCandle> {
        self.current.as_ref()
    }

    /// Fold a tick. `qty` is last-traded quantity for ltq-sum. `None` updates OHLC only.
    pub fn on_tick(
        &mut self,
        time_ms: i64,
        price: &str,
        qty: Option<&str>,
    ) -> Option<HistoryCandle> {
        let price_n: f64 = price.parse().ok()?;
        if !price_n.is_finite() || price_n <= 0.0 {
            return None;
        }
        let open_ms = bucket_open_ms(time_ms, self.interval_ms, self.session_anchor_ms);
        let close_ms = open_ms.saturating_add(self.interval_ms.saturating_sub(1));
        match self.current.as_mut() {
            Some(bar) if bar.open_time_ms == open_ms => {
                widen(bar, price, price_n, qty);
                Some(bar.clone())
            }
            Some(bar) if open_ms > bar.open_time_ms => {
                let next = new_bar(open_ms, close_ms, price, qty);
                self.current = Some(next.clone());
                self.provisional = !self.seeded;
                Some(next)
            }
            Some(_) => self.current.clone(),
            None => {
                let next = new_bar(open_ms, close_ms, price, qty);
                self.current = Some(next.clone());
                self.provisional = true;
                Some(next)
            }
        }
    }
}

fn new_bar(open_ms: i64, close_ms: i64, price: &str, qty: Option<&str>) -> HistoryCandle {
    let volume = qty
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("0")
        .to_string();
    HistoryCandle {
        open_time_ms: open_ms,
        open: price.to_string(),
        high: price.to_string(),
        low: price.to_string(),
        close: price.to_string(),
        volume,
        close_time_ms: close_ms,
    }
}

fn widen(bar: &mut HistoryCandle, price: &str, price_n: f64, qty: Option<&str>) {
    bar.close = price.to_string();
    if let Ok(high) = bar.high.parse::<f64>() {
        if price_n > high {
            bar.high = price.to_string();
        }
    }
    if let Ok(low) = bar.low.parse::<f64>() {
        if price_n < low {
            bar.low = price.to_string();
        }
    }
    if let Some(q) = qty {
        bar.volume = add_qty(&bar.volume, q);
    }
}

fn add_qty(vol: &str, qty: &str) -> String {
    let v: f64 = vol.parse().unwrap_or(0.0);
    let q: f64 = qty.parse().unwrap_or(0.0);
    if !q.is_finite() || q <= 0.0 {
        return vol.to_string();
    }
    let sum = v + q;
    if sum.fract() == 0.0 {
        format!("{sum:.0}")
    } else {
        let s = format!("{sum:.8}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

fn builder_key(adapter_id: &str, instrument_id: &str, interval: &str) -> String {
    let instrument = if is_dated_option_contract(instrument_id) {
        normalize_options_instrument(instrument_id)
    } else {
        super::binance_public::normalize_quote_instrument(instrument_id)
    };
    format!("{}\0{}\0{}", adapter_id.trim(), instrument, interval.trim())
}

#[derive(Debug, Default)]
pub struct CandleBuilders {
    rows: HashMap<String, CandleBuilder>,
    pending: HashMap<String, PendingTick>,
}

impl CandleBuilders {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn seed_from_series(&mut self, series: &HistorySeries) {
        let Some(last) = series.candles.last() else {
            return;
        };
        let Some(ms) = interval_ms(&series.interval) else {
            return;
        };
        let key = builder_key(&series.adapter_id, &series.instrument_id, &series.interval);
        if let Some(existing) = self.rows.get_mut(&key) {
            if let Some(cur) = existing.current() {
                if cur.open_time_ms > last.open_time_ms {
                    existing.seeded = true;
                    return;
                }
                if cur.open_time_ms == last.open_time_ms {
                    existing.seeded = true;
                    existing.provisional = false;
                    if let Some(pending) = self.pending.remove(&key) {
                        existing.on_tick(pending.time_ms, &pending.price, pending.qty.as_deref());
                    }
                    return;
                }
            }
        }
        let mut builder = CandleBuilder::new(
            series.adapter_id.clone(),
            series.instrument_id.clone(),
            series.interval.clone(),
            ms,
            last.open_time_ms,
        );
        builder.seed(last.clone());
        if let Some(pending) = self.pending.remove(&key) {
            builder.on_tick(pending.time_ms, &pending.price, pending.qty.as_deref());
        }
        self.rows.insert(key, builder);
    }

    pub fn tick(
        &mut self,
        adapter_id: &str,
        instrument_id: &str,
        interval: &str,
        time_ms: i64,
        price: &str,
        qty: Option<&str>,
    ) -> Option<HistoryCandle> {
        let key = builder_key(adapter_id, instrument_id, interval);
        if let Some(builder) = self.rows.get_mut(&key) {
            if builder.seeded() {
                return builder.on_tick(time_ms, price, qty);
            }
        }
        self.pending.insert(
            key,
            PendingTick {
                time_ms,
                price: price.to_string(),
                qty: qty.map(str::to_string),
            },
        );
        None
    }

    pub fn forming(
        &self,
        adapter_id: &str,
        instrument_id: &str,
        interval: &str,
    ) -> Option<&HistoryCandle> {
        let key = builder_key(adapter_id, instrument_id, interval);
        self.rows.get(&key).and_then(|b| b.current())
    }

    pub fn is_provisional(&self, adapter_id: &str, instrument_id: &str, interval: &str) -> bool {
        let key = builder_key(adapter_id, instrument_id, interval);
        self.rows.get(&key).is_some_and(|b| b.is_provisional())
    }
}

/// Closed REST series plus the in-memory forming bar. Does not persist forming.
pub fn overlay_forming(
    series: &HistorySeries,
    forming: Option<&HistoryCandle>,
) -> Vec<HistoryCandle> {
    let mut candles = series.candles.clone();
    let Some(forming) = forming else {
        return candles;
    };
    match candles.last_mut() {
        Some(last) if last.open_time_ms == forming.open_time_ms => {
            *last = forming.clone();
        }
        Some(last) if forming.open_time_ms > last.open_time_ms => {
            candles.push(forming.clone());
        }
        None => candles.push(forming.clone()),
        Some(_) => {}
    }
    candles
}

pub fn apply_history_series_and_seed(
    book: &mut HistoryBook,
    builders: &mut CandleBuilders,
    series: HistorySeries,
) {
    builders.seed_from_series(&series);
    book.upsert(series);
}

fn json_scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => {
            let t = s.trim();
            (!t.is_empty()).then(|| t.to_string())
        }
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn candle_from_json(c: &Value) -> Option<HistoryCandle> {
    let open_time_ms = c.get("open_time_ms")?.as_i64()?;
    Some(HistoryCandle {
        open_time_ms,
        open: json_scalar(c.get("open")?)?,
        high: json_scalar(c.get("high")?)?,
        low: json_scalar(c.get("low")?)?,
        close: json_scalar(c.get("close")?)?,
        volume: json_scalar(c.get("volume")?).unwrap_or_else(|| "0".into()),
        close_time_ms: c
            .get("close_time_ms")
            .and_then(Value::as_i64)
            .unwrap_or(open_time_ms),
    })
}

/// Replace `data.candles` with closed bars plus the in-memory forming bar.
pub fn overlay_json_candles(
    data: &mut Value,
    builders: &CandleBuilders,
    adapter_id: &str,
    instrument_id: &str,
    interval: &str,
) {
    let Some(raw) = data.get("candles").and_then(Value::as_array) else {
        return;
    };
    let mut candles = Vec::with_capacity(raw.len());
    for row in raw {
        let Some(c) = candle_from_json(row) else {
            return;
        };
        candles.push(c);
    }
    let series = HistorySeries {
        instrument_id: instrument_id.to_string(),
        adapter_id: adapter_id.to_string(),
        interval: interval.to_string(),
        candles,
        transport: Transport::Rest,
    };
    let out = overlay_forming(
        &series,
        builders.forming(adapter_id, instrument_id, interval),
    );
    if let Ok(value) = serde_json::to_value(&out) {
        data["candles"] = value;
    }
    if let Some(last) = out.last() {
        data["last_close"] = json!(last.close);
    }
    if builders.is_provisional(adapter_id, instrument_id, interval) {
        data["provisional"] = json!(true);
    }
}

/// Seed from a just-applied HistoryBook series. No-op on empty.
pub fn seed_builders_from_book(
    builders: &mut CandleBuilders,
    book: &HistoryBook,
    adapter_id: &str,
    instrument_id: &str,
    interval: &str,
) {
    if let Some(series) = book.get(adapter_id, instrument_id, interval) {
        builders.seed_from_series(series);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::descriptor::{BINANCE_COM_ADAPTER_ID, KOTAK_NEO_ADAPTER_ID};
    use crate::data::tick::Transport;

    fn bar(open_ms: i64, o: &str, h: &str, l: &str, c: &str, v: &str) -> HistoryCandle {
        HistoryCandle {
            open_time_ms: open_ms,
            open: o.into(),
            high: h.into(),
            low: l.into(),
            close: c.into(),
            volume: v.into(),
            close_time_ms: open_ms + 59_999,
        }
    }

    fn series(
        adapter: &str,
        instrument: &str,
        interval: &str,
        candles: Vec<HistoryCandle>,
    ) -> HistorySeries {
        HistorySeries {
            instrument_id: instrument.into(),
            adapter_id: adapter.into(),
            interval: interval.into(),
            candles,
            transport: Transport::Rest,
        }
    }

    #[test]
    fn unseeded_first_tick_opens_at_tick_price_and_is_provisional() {
        let mut b = CandleBuilder::new("binance_com", "btcusdt", "1m", 60_000, 0);
        let out = b.on_tick(1_700_000_030_000, "100.5", Some("2")).unwrap();
        assert_eq!(out.open, "100.5");
        assert_eq!(out.volume, "2");
        assert!(b.is_provisional());
        assert!(!b.seeded());
    }

    #[test]
    fn seeded_tick_keeps_history_open_and_sums_ltq() {
        let mut b = CandleBuilder::new("binance_com", "btcusdt", "1m", 60_000, 1_700_000_000_000);
        b.seed(bar(1_700_000_000_000, "100", "101", "99", "100.5", "10"));
        let out = b.on_tick(1_700_000_010_000, "102", Some("3")).unwrap();
        assert_eq!(out.open, "100");
        assert_eq!(out.high, "102");
        assert_eq!(out.close, "102");
        assert_eq!(out.volume, "13");
        assert!(!b.is_provisional());
        let again = b.on_tick(1_700_000_020_000, "98", Some("2")).unwrap();
        assert_eq!(again.low, "98");
        assert_eq!(again.volume, "15");
    }

    #[test]
    fn qty_none_updates_ohlc_only() {
        let mut b = CandleBuilder::new("kotak_neo", "nse_cm|2885", "15min", 900_000, 1);
        b.seed(bar(1, "1400", "1400", "1400", "1400", "10"));
        let out = b.on_tick(2, "1402", None).unwrap();
        assert_eq!(out.high, "1402");
        assert_eq!(out.volume, "10");
    }

    #[test]
    fn dual_no_blend_spot_builder_does_not_write_kotak() {
        let mut builders = CandleBuilders::new();
        builders.seed_from_series(&series(
            BINANCE_COM_ADAPTER_ID,
            "btcusdt",
            "1m",
            vec![bar(1_700_000_000_000, "100", "100", "100", "100", "1")],
        ));
        assert!(builders
            .tick(
                BINANCE_COM_ADAPTER_ID,
                "btcusdt",
                "1m",
                1_700_000_001_000,
                "101",
                Some("1"),
            )
            .is_some());
        assert!(builders
            .tick(
                KOTAK_NEO_ADAPTER_ID,
                "nse_cm|2885",
                "15min",
                1_700_000_001_000,
                "1400",
                Some("1"),
            )
            .is_none());
        assert!(builders
            .forming(KOTAK_NEO_ADAPTER_ID, "nse_cm|2885", "15min")
            .is_none());
    }

    #[test]
    fn overlay_replaces_same_bucket_and_appends_next() {
        let s = series(
            BINANCE_COM_ADAPTER_ID,
            "btcusdt",
            "1m",
            vec![bar(1_000, "1", "1", "1", "1", "1")],
        );
        let same = bar(1_000, "1", "2", "1", "2", "5");
        let out = overlay_forming(&s, Some(&same));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].close, "2");
        let next = bar(61_000, "2", "2", "2", "2", "1");
        let out = overlay_forming(&s, Some(&next));
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn missing_builder_does_not_unseed_open() {
        let mut builders = CandleBuilders::new();
        assert!(builders
            .tick(BINANCE_COM_ADAPTER_ID, "btcusdt", "1m", 1, "100", Some("1"))
            .is_none());
        assert!(builders
            .forming(BINANCE_COM_ADAPTER_ID, "btcusdt", "1m")
            .is_none());
    }

    #[test]
    fn pending_tick_folds_after_seed() {
        let mut builders = CandleBuilders::new();
        assert!(builders
            .tick(
                BINANCE_COM_ADAPTER_ID,
                "btcusdt",
                "1m",
                1_700_000_010_000,
                "102",
                Some("3"),
            )
            .is_none());
        builders.seed_from_series(&series(
            BINANCE_COM_ADAPTER_ID,
            "btcusdt",
            "1m",
            vec![bar(1_700_000_000_000, "100", "101", "99", "100.5", "10")],
        ));
        let forming = builders
            .forming(BINANCE_COM_ADAPTER_ID, "btcusdt", "1m")
            .expect("seeded");
        assert_eq!(forming.open, "100");
        assert_eq!(forming.close, "102");
        assert_eq!(forming.volume, "13");
    }
}
