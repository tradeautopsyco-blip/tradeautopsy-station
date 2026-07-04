//! Throwaway Today v1 round-trip + WAC engine — DELETE when real module ships.

use chrono::{DateTime, Local, TimeZone, Utc};
use std::io::{self, Write};

#[derive(Debug, Clone)]
pub struct ProtoFill {
    pub id: &'static str,
    pub symbol: &'static str,
    pub side: Side,
    pub qty: f64,
    pub price: f64,
    pub filled_at: DateTime<Utc>,
    pub fee_amount: f64,
    pub fee_asset: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TripFlag {
    Clean,
    Watch,
    Firing,
    UnknownBasis,
}

#[derive(Debug, Clone)]
pub struct ClosedRoundTrip {
    pub symbol: String,
    pub closed_at: DateTime<Utc>,
    pub avg_entry: f64,
    pub avg_exit: f64,
    pub qty: f64,
    pub net_pnl_usd: Option<f64>,
    pub flag: TripFlag,
}

#[derive(Debug, Clone)]
pub struct OpenPosition {
    pub symbol: String,
    pub qty: f64,
    pub wac: f64,
}

#[derive(Debug, Clone)]
pub struct TodayHero {
    pub pnl_usd: Option<f64>,
    pub pnl_reason: &'static str,
    pub trades_closed: usize,
    pub win_rate: Option<f64>,
    pub win_rate_reason: &'static str,
}

#[derive(Debug, Clone)]
pub struct TodayEngineState {
    pub hero: TodayHero,
    pub round_trips_today: Vec<ClosedRoundTrip>,
    pub open_positions: Vec<OpenPosition>,
    pub unknown_basis_count: usize,
    pub learning_baseline: bool,
    pub total_closed_history: usize,
}

#[derive(Debug, Clone)]
struct SymbolLedger {
    position_qty: f64,
    wac: f64,
    trip_buy_qty: f64,
    trip_buy_notional: f64,
    trip_sell_qty: f64,
    trip_sell_notional: f64,
    trip_fees_usd: f64,
    trip_opened_at: Option<DateTime<Utc>>,
    closed_history: Vec<ClosedRoundTrip>,
}

fn fee_to_usd(fill: &ProtoFill) -> f64 {
    if fill.fee_amount <= 0.0 {
        return 0.0;
    }
    match fill.fee_asset {
        "USDT" | "USD" | "BUSD" | "USDC" => fill.fee_amount,
        asset if asset == fill.symbol.trim_end_matches("USDT") => fill.fee_amount * fill.price,
        _ => fill.fee_amount * fill.price,
    }
}

fn side_str(side: Side) -> &'static str {
    match side {
        Side::Buy => "BUY",
        Side::Sell => "SELL",
    }
}

pub fn run_engine(fills: &[ProtoFill], session_day_local: chrono::NaiveDate) -> TodayEngineState {
    let mut by_symbol: std::collections::BTreeMap<String, SymbolLedger> =
        std::collections::BTreeMap::new();

    let mut sorted: Vec<&ProtoFill> = fills.iter().collect();
    sorted.sort_by_key(|f| f.filled_at);

    for fill in sorted {
        let ledger = by_symbol
            .entry(fill.symbol.to_string())
            .or_insert_with(|| SymbolLedger {
                position_qty: 0.0,
                wac: 0.0,
                trip_buy_qty: 0.0,
                trip_buy_notional: 0.0,
                trip_sell_qty: 0.0,
                trip_sell_notional: 0.0,
                trip_fees_usd: 0.0,
                trip_opened_at: None,
                closed_history: Vec::new(),
            });

        let fee_usd = fee_to_usd(fill);
        ledger.trip_fees_usd += fee_usd;

        match fill.side {
            Side::Buy => {
                if ledger.position_qty <= 1e-12 {
                    ledger.trip_buy_qty = 0.0;
                    ledger.trip_buy_notional = 0.0;
                    ledger.trip_sell_qty = 0.0;
                    ledger.trip_sell_notional = 0.0;
                    ledger.trip_fees_usd = fee_usd;
                    ledger.trip_opened_at = Some(fill.filled_at);
                }
                let new_qty = ledger.position_qty + fill.qty;
                ledger.wac =
                    (ledger.position_qty * ledger.wac + fill.qty * fill.price) / new_qty;
                ledger.position_qty = new_qty;
                ledger.trip_buy_qty += fill.qty;
                ledger.trip_buy_notional += fill.qty * fill.price;
            }
            Side::Sell => {
                if ledger.position_qty <= 1e-12 {
                    ledger.closed_history.push(ClosedRoundTrip {
                        symbol: fill.symbol.to_string(),
                        closed_at: fill.filled_at,
                        avg_entry: 0.0,
                        avg_exit: fill.price,
                        qty: fill.qty,
                        net_pnl_usd: None,
                        flag: TripFlag::UnknownBasis,
                    });
                    ledger.trip_fees_usd = 0.0;
                    continue;
                }

                let sell_qty = fill.qty.min(ledger.position_qty);
                ledger.trip_sell_qty += sell_qty;
                ledger.trip_sell_notional += sell_qty * fill.price;
                ledger.position_qty -= sell_qty;

                if ledger.position_qty <= 1e-12 {
                    ledger.position_qty = 0.0;
                    let avg_entry = ledger.trip_buy_notional / ledger.trip_buy_qty;
                    let avg_exit = ledger.trip_sell_notional / ledger.trip_sell_qty;
                    let gross = ledger.trip_sell_notional - ledger.trip_buy_notional;
                    let net = gross - ledger.trip_fees_usd;
                    ledger.closed_history.push(ClosedRoundTrip {
                        symbol: fill.symbol.to_string(),
                        closed_at: fill.filled_at,
                        avg_entry,
                        avg_exit,
                        qty: ledger.trip_sell_qty,
                        net_pnl_usd: Some(net),
                        flag: TripFlag::Clean,
                    });
                    ledger.trip_buy_qty = 0.0;
                    ledger.trip_buy_notional = 0.0;
                    ledger.trip_sell_qty = 0.0;
                    ledger.trip_sell_notional = 0.0;
                    ledger.trip_fees_usd = 0.0;
                    ledger.trip_opened_at = None;
                    ledger.wac = 0.0;
                }
            }
        }
    }

    let mut all_closed = Vec::new();
    for ledger in by_symbol.values() {
        all_closed.extend(ledger.closed_history.clone());
    }
    all_closed.sort_by(|a, b| b.closed_at.cmp(&a.closed_at));

    let total_closed_history = all_closed.len();
    let learning_baseline = total_closed_history < 10;

    let round_trips_today: Vec<ClosedRoundTrip> = all_closed
        .iter()
        .filter(|rt| local_day(rt.closed_at) == session_day_local)
        .cloned()
        .collect();

    let unknown_basis_count = round_trips_today
        .iter()
        .filter(|rt| rt.flag == TripFlag::UnknownBasis)
        .count();

    let known: Vec<_> = round_trips_today
        .iter()
        .filter(|rt| rt.net_pnl_usd.is_some())
        .collect();

    let pnl_usd = if known.is_empty() {
        None
    } else {
        Some(known.iter().map(|rt| rt.net_pnl_usd.unwrap()).sum())
    };

    let wins = known.iter().filter(|rt| rt.net_pnl_usd.unwrap() > 0.0).count();
    let win_rate = if known.is_empty() {
        None
    } else {
        Some(wins as f64 / known.len() as f64)
    };

    let open_positions: Vec<OpenPosition> = by_symbol
        .iter()
        .filter(|(_, l)| l.position_qty > 1e-12)
        .map(|(sym, l)| OpenPosition {
            symbol: sym.clone(),
            qty: l.position_qty,
            wac: l.wac,
        })
        .collect();

    let (pnl_reason, win_rate_reason) = if round_trips_today.is_empty() {
        ("no trades yet", "no trades yet")
    } else if known.is_empty() && unknown_basis_count > 0 {
        ("unknown basis only", "unknown basis only")
    } else {
        ("performance basis, not tax", "closed round-trips only")
    };

    TodayEngineState {
        hero: TodayHero {
            pnl_usd,
            pnl_reason,
            trades_closed: known.len(),
            win_rate,
            win_rate_reason,
        },
        round_trips_today,
        open_positions,
        unknown_basis_count,
        learning_baseline,
        total_closed_history,
    }
}

fn local_day(ts: DateTime<Utc>) -> chrono::NaiveDate {
    ts.with_timezone(&Local).date_naive()
}

pub fn print_fill(fill: &ProtoFill) {
    println!(
        "  {} {} {} qty={:.6} @ {:.2} fee={:.4} {} @ {}",
        fill.id,
        side_str(fill.side),
        fill.symbol,
        fill.qty,
        fill.price,
        fill.fee_amount,
        fill.fee_asset,
        fill.filled_at.with_timezone(&Local).format("%H:%M:%S")
    );
}

pub fn print_state(state: &TodayEngineState, session_day: chrono::NaiveDate) {
    println!();
    println!("=== SESSION {} (local) ===", session_day);
    println!("learning_baseline: {}", state.learning_baseline);
    println!("total_closed_history: {}", state.total_closed_history);
    println!();

    print!("HERO  P&L today: ");
    match state.hero.pnl_usd {
        Some(v) => println!("${:+.2}  ({})", v, state.hero.pnl_reason),
        None => println!("—  ({})", state.hero.pnl_reason),
    }
    print!("      Trades today: ");
    if state.hero.trades_closed == 0 {
        println!("—");
    } else {
        println!("{}", state.hero.trades_closed);
    }
    print!("      Win rate: ");
    match state.hero.win_rate {
        Some(wr) => println!("{:.0}%  ({})", wr * 100.0, state.hero.win_rate_reason),
        None => println!("—  ({})", state.hero.win_rate_reason),
    }

    if !state.open_positions.is_empty() {
        println!();
        println!("OPEN (omitted from table)");
        for pos in &state.open_positions {
            println!(
                "  {} qty={:.6} wac={:.2}",
                pos.symbol, pos.qty, pos.wac
            );
        }
    }

    println!();
    println!("ROUND-TRIPS TODAY (table rows, max 50)");
    if state.round_trips_today.is_empty() {
        println!("  (empty)");
    } else {
        for rt in &state.round_trips_today {
            let pnl = match rt.net_pnl_usd {
                Some(v) => format!("${:+.2}", v),
                None => "—".to_string(),
            };
            let flag = match rt.flag {
                TripFlag::UnknownBasis => "Unknown basis",
                TripFlag::Clean => "Clean",
                TripFlag::Watch => "Watch",
                TripFlag::Firing => "Firing",
            };
            println!(
                "  {} close={} qty={:.4} entry={:.2} exit={:.2} pnl={} [{}]",
                rt.symbol,
                rt.closed_at.with_timezone(&Local).format("%H:%M:%S"),
                rt.qty,
                rt.avg_entry,
                rt.avg_exit,
                pnl,
                flag
            );
        }
    }

    if state.unknown_basis_count > 0 {
        println!();
        println!(
            "unknown_basis_count: {} (excluded from hero P&L / win rate)",
            state.unknown_basis_count
        );
    }
    println!();
}

pub fn utc_local_day(y: i32, m: u32, d: u32, hh: u32, mm: u32) -> DateTime<Utc> {
    Local
        .with_ymd_and_hms(y, m, d, hh, mm, 0)
        .unwrap()
        .with_timezone(&Utc)
}

pub fn scenario_simple_btc_round_trip() -> Vec<ProtoFill> {
    let t0 = utc_local_day(2026, 7, 2, 10, 0);
    let t1 = utc_local_day(2026, 7, 2, 14, 30);
    vec![
        ProtoFill {
            id: "f1",
            symbol: "BTCUSDT",
            side: Side::Buy,
            qty: 0.01,
            price: 60_000.0,
            filled_at: t0,
            fee_amount: 0.60,
            fee_asset: "USDT",
        },
        ProtoFill {
            id: "f2",
            symbol: "BTCUSDT",
            side: Side::Sell,
            qty: 0.01,
            price: 61_000.0,
            filled_at: t1,
            fee_amount: 0.61,
            fee_asset: "USDT",
        },
    ]
}

pub fn scenario_wac_multi_buy() -> Vec<ProtoFill> {
    let d = utc_local_day(2026, 7, 2, 9, 0);
    vec![
        ProtoFill {
            id: "f1",
            symbol: "ETHUSDT",
            side: Side::Buy,
            qty: 1.0,
            price: 3_000.0,
            filled_at: d + chrono::Duration::minutes(0),
            fee_amount: 3.0,
            fee_asset: "USDT",
        },
        ProtoFill {
            id: "f2",
            symbol: "ETHUSDT",
            side: Side::Buy,
            qty: 1.0,
            price: 3_200.0,
            filled_at: d + chrono::Duration::minutes(30),
            fee_amount: 3.2,
            fee_asset: "USDT",
        },
        ProtoFill {
            id: "f3",
            symbol: "ETHUSDT",
            side: Side::Sell,
            qty: 2.0,
            price: 3_100.0,
            filled_at: d + chrono::Duration::hours(2),
            fee_amount: 6.2,
            fee_asset: "USDT",
        },
    ]
}

pub fn scenario_unknown_basis_sell() -> Vec<ProtoFill> {
    vec![ProtoFill {
        id: "f1",
        symbol: "SOLUSDT",
        side: Side::Sell,
        qty: 10.0,
        price: 150.0,
        filled_at: utc_local_day(2026, 7, 2, 11, 0),
        fee_amount: 1.5,
        fee_asset: "USDT",
    }]
}

pub fn scenario_open_position_partial() -> Vec<ProtoFill> {
    let d = utc_local_day(2026, 7, 2, 10, 0);
    vec![
        ProtoFill {
            id: "f1",
            symbol: "BTCUSDT",
            side: Side::Buy,
            qty: 0.02,
            price: 60_000.0,
            filled_at: d,
            fee_amount: 1.2,
            fee_asset: "USDT",
        },
        ProtoFill {
            id: "f2",
            symbol: "BTCUSDT",
            side: Side::Sell,
            qty: 0.01,
            price: 61_000.0,
            filled_at: d + chrono::Duration::hours(1),
            fee_amount: 0.61,
            fee_asset: "USDT",
        },
    ]
}

pub fn scenario_day_boundary() -> Vec<ProtoFill> {
    vec![
        ProtoFill {
            id: "f1",
            symbol: "BTCUSDT",
            side: Side::Buy,
            qty: 0.01,
            price: 60_000.0,
            filled_at: utc_local_day(2026, 7, 1, 23, 50),
            fee_amount: 0.6,
            fee_asset: "USDT",
        },
        ProtoFill {
            id: "f2",
            symbol: "BTCUSDT",
            side: Side::Sell,
            qty: 0.01,
            price: 61_000.0,
            filled_at: utc_local_day(2026, 7, 2, 0, 10),
            fee_amount: 0.61,
            fee_asset: "USDT",
        },
    ]
}

pub fn scenario_mixed_today() -> Vec<ProtoFill> {
    let mut fills = scenario_simple_btc_round_trip();
    fills.extend(scenario_unknown_basis_sell());
    fills.extend(scenario_open_position_partial());
    fills
}

pub struct Scenario {
    pub name: &'static str,
    pub session_day: chrono::NaiveDate,
    pub fills: Vec<ProtoFill>,
    pub expect_pnl: Option<f64>,
}

pub fn all_scenarios() -> Vec<Scenario> {
    vec![
        Scenario {
            name: "Simple BTC round-trip (hand calc)",
            session_day: chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap(),
            fills: scenario_simple_btc_round_trip(),
            // gross 10, fees 1.21
            expect_pnl: Some(8.79),
        },
        Scenario {
            name: "WAC multi-buy single sell",
            session_day: chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap(),
            fills: scenario_wac_multi_buy(),
            // buys 6200, sells 6200, fees 12.4 → ~-12.4
            expect_pnl: Some(-12.4),
        },
        Scenario {
            name: "Unknown basis sell only",
            session_day: chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap(),
            fills: scenario_unknown_basis_sell(),
            expect_pnl: None,
        },
        Scenario {
            name: "Partial close leaves open position",
            session_day: chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap(),
            fills: scenario_open_position_partial(),
            expect_pnl: None,
        },
        Scenario {
            name: "Day boundary — close rolls to next local day",
            session_day: chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap(),
            fills: scenario_day_boundary(),
            expect_pnl: Some(8.79),
        },
    ]
}

pub fn run_batch_checks() {
    println!("=== BATCH FIXTURE CHECKS ===");
    let mut ok = 0;
    for sc in all_scenarios() {
        let state = run_engine(&sc.fills, sc.session_day);
        let got = state.hero.pnl_usd;
        let pass = match (sc.expect_pnl, got) {
            (None, None) => true,
            (Some(exp), Some(g)) => (g - exp).abs() < 0.02,
            _ => false,
        };
        let mark = if pass { "OK" } else { "FAIL" };
        println!(
            "  [{mark}] {} — expected {:?}, got {:?}",
            sc.name, sc.expect_pnl, got
        );
        if pass {
            ok += 1;
        }
    }
    println!("  {ok}/{} passed\n", all_scenarios().len());
}

pub fn step_through(name: &str, fills: &[ProtoFill], session_day: chrono::NaiveDate) {
    println!("\n=== STEP: {name} ===");
    println!("Press Enter to ingest each fill; full state prints after every step.\n");
    for i in 0..fills.len() {
        print_fill(&fills[i]);
        let _ = io::stdout().flush();
        let mut line = String::new();
        let _ = io::stdin().read_line(&mut line);
        let state = run_engine(&fills[..=i], session_day);
        print_state(&state, session_day);
    }
}

pub fn print_menu() {
    println!("Today v1 engine prototype — pick a scenario:");
    println!("  1  Batch run all fixtures (acceptance-style checks)");
    println!("  2  Step — simple BTC round-trip");
    println!("  3  Step — WAC multi-buy");
    println!("  4  Step — unknown basis sell");
    println!("  5  Step — partial close / open position");
    println!("  6  Step — day boundary rollover");
    println!("  7  Dump — mixed today (hero + table mix)");
    println!("  q  Quit");
    print!("> ");
    let _ = io::stdout().flush();
}
