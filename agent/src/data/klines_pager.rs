//! Page walk for COM `GET /api/v3/klines` under the documented per-call `limit` max 1000.
//!
//! REST.md: klines are uniquely identified by open time and returned chronologically.
//! A wall-clock span longer than 1000 bars is another page (`startTime` = last open + 1),
//! not a raised `limit` and not a stitch. Live desk fetch is not wired here.

#![allow(dead_code)]

use super::binance_klines::{
    candles_from_klines_json, klines_url, validate_kline_request, HistoryCandle, KlineRequestRefuse,
};
use std::collections::BTreeMap;

/// Merge `new_page` into `existing`. Sort by `open_time_ms`; same open prefers the later page.
pub fn page_klines(existing: &[HistoryCandle], new_page: &[HistoryCandle]) -> Vec<HistoryCandle> {
    let mut by_open = BTreeMap::new();
    for candle in existing {
        by_open.insert(candle.open_time_ms, candle.clone());
    }
    for candle in new_page {
        by_open.insert(candle.open_time_ms, candle.clone());
    }
    by_open.into_values().collect()
}

/// Next page `startTime`: last candle open time + 1 ms. Do not overlap-guess.
pub fn next_start_time_ms(last_open_time_ms: i64) -> i64 {
    last_open_time_ms.saturating_add(1)
}

/// Stop when the page is empty, shorter than `limit`, or last open ≥ `endTime`.
pub fn walk_is_complete(
    page_len: usize,
    limit: u32,
    last_open_ms: Option<i64>,
    end_time_ms: Option<i64>,
) -> bool {
    if page_len == 0 {
        return true;
    }
    if page_len < limit as usize {
        return true;
    }
    matches!(
        (last_open_ms, end_time_ms),
        (Some(last), Some(end)) if last >= end
    )
}

/// One COM klines URL for a single page. `limit` 0 or > 1000 → `unsupported_range`.
/// Does not fetch; desk live GET stays with the selected-symbol owner.
pub fn fetch_klines_page(
    symbol: &str,
    interval: &str,
    limit: u32,
    start_time_ms: Option<i64>,
    end_time_ms: Option<i64>,
) -> Result<String, KlineRequestRefuse> {
    let limit = validate_kline_request(interval, Some(limit))?;
    Ok(klines_url(
        symbol,
        interval,
        limit,
        start_time_ms,
        end_time_ms,
    ))
}

/// Result of ingesting one REST page into the series so far.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WalkedPage {
    pub candles: Vec<HistoryCandle>,
    pub complete: bool,
    /// `startTime` for the next GET: this page's last open + 1. `None` when complete.
    pub next_start_time_ms: Option<i64>,
}

/// Apply one fetched page body. Empty / unparseable JSON completes the walk
/// and is not an empty success series.
pub fn walk_klines_page(
    existing: &[HistoryCandle],
    page_json: &str,
    limit: u32,
    end_time_ms: Option<i64>,
) -> WalkedPage {
    let Some(new_page) = candles_from_klines_json(page_json) else {
        return WalkedPage {
            candles: existing.to_vec(),
            complete: true,
            next_start_time_ms: None,
        };
    };
    let last_open_ms = new_page.last().map(|c| c.open_time_ms);
    let complete = walk_is_complete(new_page.len(), limit, last_open_ms, end_time_ms);
    let next_start_time_ms = if complete {
        None
    } else {
        last_open_ms.map(next_start_time_ms)
    };
    WalkedPage {
        candles: page_klines(existing, &new_page),
        complete,
        next_start_time_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::binance_klines::{
        series_from_klines_json, validate_kline_request, KlineRequestRefuse, KLINE_LIMIT_MAX,
    };
    use crate::data::tick::Transport;

    const FIXTURE: &str = include_str!("../../fixtures/binance/klines.json");

    fn candle(open_time_ms: i64, close: &str) -> HistoryCandle {
        HistoryCandle {
            open_time_ms,
            open: "1".into(),
            high: "2".into(),
            low: "0.5".into(),
            close: close.into(),
            volume: "10".into(),
            close_time_ms: open_time_ms + 59_999,
        }
    }

    #[test]
    fn two_pages_merge_to_chronological_unique_opens() {
        let page1 = candles_from_klines_json(FIXTURE).expect("fixture page");
        assert_eq!(page1.len(), 2);
        assert_eq!(page1[0].open_time_ms, 1_499_040_000_000);
        assert_eq!(page1[1].open_time_ms, 1_499_644_800_000);

        // Later page: overlapping first open (new close wins) + a newer bar.
        let page2 = vec![
            candle(1_499_040_000_000, "9.00000000"),
            candle(1_500_000_000_000, "0.02000000"),
        ];
        let merged = page_klines(&page1, &page2);
        assert_eq!(merged.len(), 3);
        assert_eq!(
            merged.iter().map(|c| c.open_time_ms).collect::<Vec<_>>(),
            vec![1_499_040_000_000, 1_499_644_800_000, 1_500_000_000_000]
        );
        assert_eq!(merged[0].close, "9.00000000");
        assert_eq!(merged[1].close, "0.01590000");
        assert_eq!(merged[2].close, "0.02000000");

        let reversed = page_klines(&page2, &page1);
        assert_eq!(reversed.len(), 3);
        assert_eq!(reversed[0].open_time_ms, 1_499_040_000_000);
        assert_eq!(reversed[0].close, page1[0].close);
    }

    #[test]
    fn limit_1001_is_still_unsupported_range() {
        assert_eq!(
            validate_kline_request("1m", Some(1001)).unwrap_err(),
            KlineRequestRefuse::UnsupportedRange
        );
        assert_eq!(
            fetch_klines_page("BTCUSDT", "1m", 1001, None, None).unwrap_err(),
            KlineRequestRefuse::UnsupportedRange
        );
        assert_eq!(
            fetch_klines_page("BTCUSDT", "1m", 0, None, None).unwrap_err(),
            KlineRequestRefuse::UnsupportedRange
        );
        let url = fetch_klines_page(
            "btcusdt",
            "1m",
            KLINE_LIMIT_MAX,
            Some(1_499_040_000_000),
            Some(1_500_000_000_000),
        )
        .expect("max page is allowed");
        assert!(url.contains("limit=1000"));
        assert!(!url.contains("limit=1001"));
        assert!(url.contains("startTime=1499040000000"));
        assert!(url.contains("endTime=1500000000000"));
        assert!(url.starts_with("https://api.binance.com/api/v3/klines?"));
        assert!(!url.contains("uiKlines"));
        assert_ne!(
            KlineRequestRefuse::UnsupportedRange.as_ineligible(),
            "insufficient_retention"
        );
    }

    #[test]
    fn next_page_start_is_last_open_plus_one() {
        assert_eq!(next_start_time_ms(1_499_644_800_000), 1_499_644_800_001);
        let page1 = candles_from_klines_json(FIXTURE).unwrap();
        let last = page1.last().unwrap().open_time_ms;
        assert_eq!(next_start_time_ms(last), last + 1);
    }

    #[test]
    fn empty_page_completes_and_is_not_empty_success_series() {
        assert!(series_from_klines_json("[]", "BTCUSDT", "1m", Transport::Fixture).is_none());
        assert!(candles_from_klines_json("[]").is_none());
        assert!(walk_is_complete(0, 500, None, None));
        assert!(walk_is_complete(0, 1000, Some(1), Some(2)));

        let existing = candles_from_klines_json(FIXTURE).unwrap();
        let walked = walk_klines_page(&existing, "[]", 500, None);
        assert!(walked.complete);
        assert_eq!(walked.candles, existing);
        assert!(walked.next_start_time_ms.is_none());

        let from_scratch = walk_klines_page(&[], "not-json", 500, None);
        assert!(from_scratch.complete);
        assert!(from_scratch.candles.is_empty());
    }

    #[test]
    fn short_page_or_last_open_at_end_time_completes() {
        assert!(walk_is_complete(3, 500, Some(10), None));
        assert!(walk_is_complete(1000, 1000, Some(50), Some(50)));
        assert!(walk_is_complete(1000, 1000, Some(51), Some(50)));
        assert!(!walk_is_complete(1000, 1000, Some(49), Some(50)));
        assert!(!walk_is_complete(500, 500, Some(10), None));

        let fixture = candles_from_klines_json(FIXTURE).unwrap();
        let short = walk_klines_page(&[], FIXTURE, 500, None);
        assert_eq!(short.candles, fixture);
        assert!(short.complete);

        let end = fixture.last().unwrap().open_time_ms;
        let at_end = walk_klines_page(&[], FIXTURE, 2, Some(end));
        assert!(at_end.complete);
        assert_eq!(at_end.candles.len(), 2);

        let before_end = walk_klines_page(&[], FIXTURE, 2, Some(end + 1));
        assert!(!before_end.complete);
        assert_eq!(before_end.next_start_time_ms, Some(next_start_time_ms(end)));
    }
}
