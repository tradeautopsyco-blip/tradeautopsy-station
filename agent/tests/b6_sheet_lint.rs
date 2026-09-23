//! W0.8 (F6) — B6 sheet completeness lint.
//!
//! Mechanical, not reviewer memory: every broker sheet must cover its surface
//! §1 rows with no row MISSING and no blank-by-accident answer.
//!
//! - SIGNED sheets (`binance_com`, `kotak_neo`): Status SIGNED + core rows 0–13.
//! - RESEARCH sheet (`zerodha_kite`): full surface rows 0–25 (W0.9 widening).
//! - Present-but-TBD is OK (green-with-TBDs); value / refuse / NOT SPECIFIED
//!   is the ideal; a MISSING row number or a blank answer fails.
//!
//! Sheets are read at test time from the canonical SoT dir via a relative path.
//! Row numbers are parsed from `| <n> | Field | Answer |` table rows; non-numeric
//! first cells (headers, separators, `F1`-style amendment rows) are ignored.

use std::collections::BTreeMap;
use std::path::PathBuf;

fn sheets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../issues/brokers/sheets")
}

/// Parse `| <n> | Field | Answer |` rows out of a sheet's markdown tables.
/// Returns row-number → (field, answer). First occurrence wins.
fn sheet_rows(text: &str) -> BTreeMap<u32, (String, String)> {
    let mut rows = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        // split('|') on `| a | b | c |` yields ["", " a ", " b ", " c ", ""].
        let cells: Vec<&str> = line.split('|').map(str::trim).collect();
        if cells.len() < 5 {
            continue;
        }
        let num: u32 = match cells[1].parse() {
            Ok(n) => n,
            Err(_) => continue, // header (`#`), separator (`---`), amendment (`F1`)
        };
        rows.entry(num)
            .or_insert((cells[2].to_string(), cells[3].to_string()));
    }
    rows
}

fn read_sheet(slug: &str) -> String {
    let path = sheets_dir().join(format!("{slug}.md"));
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("cannot read B6 sheet {}: {e}", path.display()))
}

fn status_line(text: &str, slug: &str) -> String {
    text.lines()
        .find(|l| l.contains("**Status:**"))
        .unwrap_or_else(|| panic!("{slug}.md has no Status line"))
        .to_string()
}

fn check_rows(slug: &str, text: &str, expected: std::ops::RangeInclusive<u32>) {
    let rows = sheet_rows(text);
    let mut missing = Vec::new();
    let mut blank = Vec::new();
    let mut tbd = Vec::new();
    for n in expected {
        match rows.get(&n) {
            None => missing.push(n),
            Some((field, answer)) => {
                if field.trim().is_empty() || answer.trim().is_empty() {
                    blank.push(n);
                } else if answer.contains("TBD") {
                    tbd.push(n);
                }
            }
        }
    }
    println!("{slug}: {} numbered rows parsed; TBD rows: {tbd:?}", rows.len());
    assert!(
        missing.is_empty(),
        "B6 sheet {slug}.md MISSING surface rows: {missing:?} \
         (present-but-TBD is OK; absent is not)"
    );
    assert!(
        blank.is_empty(),
        "B6 sheet {slug}.md has blank-by-accident rows \
         (fill with value, refuse, or TBD — never empty): {blank:?}"
    );
}

#[test]
fn signed_sheets_stay_signed_with_core_rows() {
    for slug in ["binance_com", "kotak_neo"] {
        let text = read_sheet(slug);
        let status = status_line(&text, slug);
        assert!(
            status.contains("`SIGNED`") && !status.contains("`RESEARCH`"),
            "{slug}.md lost its SIGNED Status: {status}"
        );
        check_rows(slug, &text, 0..=13);
    }
}

#[test]
fn research_sheet_covers_surface_rows_0_to_25() {
    let slug = "zerodha_kite";
    let text = read_sheet(slug);
    let status = status_line(&text, slug);
    assert!(
        status.contains("`RESEARCH`"),
        "{slug}.md is no longer the RESEARCH stub — update this lint's expectations: {status}"
    );
    check_rows(slug, &text, 0..=25);
}
