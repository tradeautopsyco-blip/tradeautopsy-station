//! PROTOTYPE — interactive Today v1 round-trip engine. DELETE when real module ships.
//!
//! Run: cargo run --manifest-path agent/Cargo.toml --bin prototype-today-engine

#[path = "../../prototypes/today_engine.rs"]
mod engine;

use engine::*;
use std::io::{self, Write};

fn main() {
    println!("PROTOTYPE — Today v1 round-trip + WAC engine");
    println!("See agent/prototypes/LOGIC.md\n");

    loop {
        print_menu();
        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            break;
        }
        match line.trim() {
            "1" => run_batch_checks(),
            "2" => step_through(
                "Simple BTC round-trip",
                &scenario_simple_btc_round_trip(),
                chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap(),
            ),
            "3" => step_through(
                "WAC multi-buy",
                &scenario_wac_multi_buy(),
                chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap(),
            ),
            "4" => step_through(
                "Unknown basis sell",
                &scenario_unknown_basis_sell(),
                chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap(),
            ),
            "5" => step_through(
                "Partial close / open position",
                &scenario_open_position_partial(),
                chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap(),
            ),
            "6" => step_through(
                "Day boundary rollover",
                &scenario_day_boundary(),
                chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap(),
            ),
            "7" => {
                let day = chrono::NaiveDate::from_ymd_opt(2026, 7, 2).unwrap();
                let fills = scenario_mixed_today();
                println!("\n=== MIXED TODAY DUMP ===");
                for f in &fills {
                    print_fill(f);
                }
                let state = run_engine(&fills, day);
                print_state(&state, day);
            }
            "q" | "Q" => break,
            other => println!("Unknown option: {other:?}\n"),
        }
    }
}
