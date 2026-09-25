//! SHIPPING book → realized-PnL owner path (CLAIM-REGISTRY rows).

use crate::data::book_accepts_symbol;
use crate::data::is_dated_option_contract;
use crate::broker::BrokerFill;
use crate::inr_cash_wac;
use crate::fx_cds_realized_pnl;
use crate::mcx_realized_pnl;
use crate::nfo_realized_pnl;

pub const SHIPPING_BOOK_COUNT: usize = 22;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoneyOwner {
    /// Path under repo root, e.g. `agent/src/inr_cash_wac.rs`.
    Engine(&'static str),
    /// Documented refusal / lock not amended.
    ExplicitNone(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BookMoneyRow {
    pub book_id: &'static str,
    pub owner: MoneyOwner,
}

/// All twenty-two SHIPPING books from `issues/compliance/CLAIM-REGISTRY.md`.
pub fn shipping_money_matrix() -> [BookMoneyRow; SHIPPING_BOOK_COUNT] {
    [
        BookMoneyRow {
            book_id: "kotak-nse-bse-cash",
            owner: MoneyOwner::Engine(inr_cash_wac::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "zerodha-nse-bse-cash",
            owner: MoneyOwner::Engine(inr_cash_wac::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "upstox-nse-bse-cash",
            owner: MoneyOwner::Engine(inr_cash_wac::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "fyers-nse-bse-cash",
            owner: MoneyOwner::Engine(inr_cash_wac::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "dhan-nse-bse-cash",
            owner: MoneyOwner::Engine(inr_cash_wac::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "groww-nse-bse-cash",
            owner: MoneyOwner::Engine(inr_cash_wac::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "binance-com-spot",
            owner: MoneyOwner::Engine("agent/src/round_trip_engine.rs"),
        },
        BookMoneyRow {
            book_id: "bybit-com-spot",
            owner: MoneyOwner::Engine("agent/src/round_trip_engine.rs"),
        },
        BookMoneyRow {
            book_id: "okx-com-spot",
            owner: MoneyOwner::Engine("agent/src/round_trip_engine.rs"),
        },
        BookMoneyRow {
            book_id: "kraken-com-spot",
            owner: MoneyOwner::Engine("agent/src/round_trip_engine.rs"),
        },
        BookMoneyRow {
            book_id: "coinbase-advanced-spot",
            owner: MoneyOwner::Engine("agent/src/round_trip_engine.rs"),
        },
        BookMoneyRow {
            book_id: "kotak-nse-nfo",
            owner: MoneyOwner::Engine(nfo_realized_pnl::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "kotak-nse-cds",
            owner: MoneyOwner::Engine(fx_cds_realized_pnl::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "kotak-mcx-future",
            owner: MoneyOwner::Engine(mcx_realized_pnl::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "zerodha-nse-nfo",
            owner: MoneyOwner::Engine(nfo_realized_pnl::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "upstox-nse-nfo",
            owner: MoneyOwner::Engine(nfo_realized_pnl::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "fyers-nse-nfo",
            owner: MoneyOwner::Engine(nfo_realized_pnl::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "dhan-nse-nfo",
            owner: MoneyOwner::Engine(nfo_realized_pnl::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "groww-nse-nfo",
            owner: MoneyOwner::Engine(nfo_realized_pnl::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "binance-com-options",
            owner: MoneyOwner::ExplicitNone(crate::options_realized_pnl::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "binance-com-usdm",
            owner: MoneyOwner::Engine(crate::usdm_realized_pnl::OWNER_PATH),
        },
        BookMoneyRow {
            book_id: "binance-com-coinm",
            owner: MoneyOwner::Engine(crate::coinm_realized_pnl::OWNER_PATH),
        },
    ]
}

pub fn owner_for_book(book_id: &str) -> Option<MoneyOwner> {
    shipping_money_matrix()
        .into_iter()
        .find(|row| row.book_id == book_id)
        .map(|row| row.owner)
}

/// True when Today may feed this fill into COM spot `RoundTripEngine`.
pub fn is_binance_com_spot_fill(fill: &BrokerFill) -> bool {
    if inr_cash_wac::is_inr_cash_fill(fill) || nfo_realized_pnl::is_nfo_fill(fill) {
        return false;
    }
    if is_dated_option_contract(&fill.symbol) {
        return false;
    }
    if fill.symbol.contains('|') {
        return false;
    }
    let seg = fill
        .exchange_segment
        .as_deref()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if seg == "usdm" || seg == "coinm" || seg == "nse_fo" {
        return false;
    }
    let upper = fill.symbol.to_ascii_uppercase();
    if upper.ends_with("_PERP") {
        return false;
    }
    [
        "binance-com-spot",
        "bybit-com-spot",
        "okx-com-spot",
        "kraken-com-spot",
        "coinbase-advanced-spot",
    ]
    .into_iter()
    .any(|book_id| book_accepts_symbol(book_id, &fill.symbol))
}

/// Book ids for every SHIPPING row in `issues/compliance/CLAIM-REGISTRY.md` (2026-09-25).
pub const CLAIM_REGISTRY_SHIPPING_BOOK_IDS: [&str; SHIPPING_BOOK_COUNT] = [
    "binance-com-coinm",
    "binance-com-options",
    "binance-com-spot",
    "binance-com-usdm",
    "bybit-com-spot",
    "coinbase-advanced-spot",
    "dhan-nse-bse-cash",
    "dhan-nse-nfo",
    "fyers-nse-bse-cash",
    "fyers-nse-nfo",
    "groww-nse-bse-cash",
    "groww-nse-nfo",
    "kotak-mcx-future",
    "kotak-nse-bse-cash",
    "kotak-nse-cds",
    "kotak-nse-nfo",
    "kraken-com-spot",
    "okx-com-spot",
    "upstox-nse-bse-cash",
    "upstox-nse-nfo",
    "zerodha-nse-bse-cash",
    "zerodha-nse-nfo",
];

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn matrix_book_ids_match_claim_registry() {
        let mut ids: Vec<_> = shipping_money_matrix()
            .iter()
            .map(|r| r.book_id)
            .collect();
        ids.sort_unstable();
        let mut expected: Vec<_> = CLAIM_REGISTRY_SHIPPING_BOOK_IDS.to_vec();
        expected.sort_unstable();
        assert_eq!(ids, expected);
    }

    #[test]
    fn all_shipping_books_covered() {
        let matrix = shipping_money_matrix();
        assert_eq!(matrix.len(), SHIPPING_BOOK_COUNT);
        let ids: Vec<_> = matrix.iter().map(|r| r.book_id).collect();
        assert_eq!(ids.len(), ids.iter().collect::<std::collections::BTreeSet<_>>().len());
        assert!(ids.contains(&"kotak-nse-nfo"));
        assert!(ids.contains(&"zerodha-nse-nfo"));
        assert!(ids.contains(&"upstox-nse-nfo"));
        assert!(ids.contains(&"fyers-nse-nfo"));
        assert!(ids.contains(&"dhan-nse-nfo"));
        assert!(ids.contains(&"groww-nse-nfo"));
        assert!(ids.contains(&"binance-com-options"));
        assert!(ids.contains(&"binance-com-coinm"));
    }

    #[test]
    fn kotak_nfo_owner_is_nfo_realized_not_wac() {
        let owner = owner_for_book("kotak-nse-nfo").expect("nfo row");
        assert_eq!(
            owner,
            MoneyOwner::Engine("agent/src/nfo_realized_pnl.rs")
        );
        assert_ne!(
            owner,
            MoneyOwner::Engine("agent/src/round_trip_engine.rs")
        );
    }

    #[test]
    fn spot_fill_gate_excludes_nfo_and_options() {
        let nfo = BrokerFill {
            fill_id: "n1".into(),
            trade_id: "t1".into(),
            symbol: "NIFTY".into(),
            side: "BUY".into(),
            qty: 2.0,
            price: 10.0,
            filled_at: Utc::now(),
            broker: "kotak_neo".into(),
            currency: Some("INR".into()),
            product: Some("NRML".into()),
            exchange_segment: Some("nse_fo".into()),
            lot: Some(65),
            ..Default::default()
        };
        assert!(!is_binance_com_spot_fill(&nfo));

        let opt = BrokerFill {
            fill_id: "o1".into(),
            trade_id: "t2".into(),
            symbol: "BTC-200730-9000-C".into(),
            side: "BUY".into(),
            qty: 1.0,
            price: 100.0,
            filled_at: Utc::now(),
            broker: "binance_com".into(),
            ..Default::default()
        };
        assert!(!is_binance_com_spot_fill(&opt));

        let spot = BrokerFill {
            fill_id: "s1".into(),
            trade_id: "t3".into(),
            symbol: "BTCUSDT".into(),
            side: "BUY".into(),
            qty: 0.01,
            price: 1.0,
            filled_at: Utc::now(),
            broker: "binance_com".into(),
            ..Default::default()
        };
        assert!(is_binance_com_spot_fill(&spot));

        let bybit_spot = BrokerFill {
            fill_id: "b1".into(),
            trade_id: "t3b".into(),
            symbol: "ETHUSDT".into(),
            side: "BUY".into(),
            qty: 0.1,
            price: 1.0,
            filled_at: Utc::now(),
            broker: "bybit_com".into(),
            ..Default::default()
        };
        assert!(is_binance_com_spot_fill(&bybit_spot));

        let usdm = BrokerFill {
            fill_id: "u1".into(),
            trade_id: "t4".into(),
            symbol: "BTCUSDT".into(),
            side: "BUY".into(),
            qty: 1.0,
            price: 1.0,
            filled_at: Utc::now(),
            broker: "binance_com".into(),
            exchange_segment: Some("usdm".into()),
            ..Default::default()
        };
        assert!(!is_binance_com_spot_fill(&usdm));

        let perp = BrokerFill {
            fill_id: "p1".into(),
            trade_id: "t5".into(),
            symbol: "ETHUSDT_PERP".into(),
            side: "BUY".into(),
            qty: 1.0,
            price: 1.0,
            filled_at: Utc::now(),
            broker: "binance_com".into(),
            ..Default::default()
        };
        assert!(!is_binance_com_spot_fill(&perp));
    }

    #[test]
    fn p4_cex_spot_books_use_round_trip_owner_after_registry_shipping() {
        let ids: Vec<_> = shipping_money_matrix()
            .iter()
            .map(|r| r.book_id)
            .collect();
        for book in [
            "bybit-com-spot",
            "okx-com-spot",
            "kraken-com-spot",
            "coinbase-advanced-spot",
        ] {
            assert!(ids.contains(&book), "{book} must be SHIPPING in CLAIM-REGISTRY");
            let owner = owner_for_book(book).expect("row");
            assert_eq!(
                owner,
                MoneyOwner::Engine("agent/src/round_trip_engine.rs")
            );
        }
        assert_eq!(ids.len(), SHIPPING_BOOK_COUNT);
        assert!(ids.contains(&"kotak-nse-cds"));
        assert!(ids.contains(&"kotak-mcx-future"));
    }
}
