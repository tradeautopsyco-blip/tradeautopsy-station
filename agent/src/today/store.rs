//! SQLite persistence for closed round-trips + daily snapshots (Today v1 spine).

use crate::round_trip_engine::RoundTrip;
use crate::today::signals::TripBehaviorFlag;
use anyhow::Context;
use chrono::{DateTime, NaiveDate, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct DailySnapshot {
    pub local_date: NaiveDate,
    pub round_trips_closed: u32,
    pub net_pnl_usd: f64,
    pub wins: u32,
    pub losses: u32,
    pub discipline_index: f64,
    pub trades_count: u32,
    pub learning_baseline: bool,
    pub unknown_basis_count: u32,
}

#[derive(Clone)]
pub struct TodayStore {
    conn: Arc<Mutex<Connection>>,
}

impl TodayStore {
    pub fn open(path: &std::path::Path) -> anyhow::Result<Self> {
        let conn =
            Connection::open(path).with_context(|| format!("open today db {}", path.display()))?;
        conn.execute_batch(
            r#"
PRAGMA journal_mode = WAL;
CREATE TABLE IF NOT EXISTS round_trips (
  trip_id TEXT PRIMARY KEY,
  symbol TEXT NOT NULL,
  opened_at_rfc3339 TEXT NOT NULL,
  closed_at_rfc3339 TEXT NOT NULL,
  local_close_date TEXT NOT NULL,
  avg_entry REAL NOT NULL,
  avg_exit REAL NOT NULL,
  qty REAL NOT NULL,
  realized_pnl_usd REAL,
  unknown_basis INTEGER NOT NULL,
  fee_unhandled INTEGER NOT NULL,
  quote_not_usd INTEGER NOT NULL,
  primary_flag TEXT NOT NULL,
  flag_severity TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_round_trips_local_close ON round_trips(local_close_date DESC, closed_at_rfc3339 DESC);

CREATE TABLE IF NOT EXISTS daily_snapshots (
  local_date TEXT PRIMARY KEY,
  round_trips_closed INTEGER NOT NULL,
  net_pnl_usd REAL NOT NULL,
  wins INTEGER NOT NULL,
  losses INTEGER NOT NULL,
  discipline_index REAL NOT NULL,
  trades_count INTEGER NOT NULL,
  learning_baseline INTEGER NOT NULL,
  unknown_basis_count INTEGER NOT NULL,
  updated_at_ms INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS journal_trip_cites (
  declaration_id TEXT PRIMARY KEY,
  net REAL NOT NULL,
  currency TEXT NOT NULL,
  local_close_date TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_journal_trip_cites_close ON journal_trip_cites(local_close_date DESC);
"#,
        )?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn replace_round_trips(
        &self,
        trips: &[(RoundTrip, TripBehaviorFlag)],
    ) -> anyhow::Result<()> {
        let guard = self.conn.lock().expect("today sqlite mutex poisoned");
        guard.execute("DELETE FROM round_trips", [])?;
        let mut stmt = guard.prepare(
            r#"INSERT INTO round_trips (
                trip_id, symbol, opened_at_rfc3339, closed_at_rfc3339, local_close_date,
                avg_entry, avg_exit, qty, realized_pnl_usd,
                unknown_basis, fee_unhandled, quote_not_usd,
                primary_flag, flag_severity
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)"#,
        )?;
        for (rt, flag) in trips {
            let local_close = local_date(rt.closed_at);
            let trip_id = trip_key(&rt.symbol, rt.closed_at);
            stmt.execute(params![
                trip_id,
                rt.symbol,
                rt.opened_at.to_rfc3339(),
                rt.closed_at.to_rfc3339(),
                local_close.format("%Y-%m-%d").to_string(),
                rt.avg_entry_price,
                rt.avg_exit_price,
                rt.qty,
                rt.realized_pnl_usd,
                rt.unknown_basis as i32,
                rt.fee_unhandled as i32,
                rt.quote_not_usd as i32,
                flag.label,
                flag.severity.as_str(),
            ])?;
        }
        Ok(())
    }

    pub fn upsert_daily_snapshot(&self, snap: &DailySnapshot) -> anyhow::Result<()> {
        let guard = self.conn.lock().expect("today sqlite mutex poisoned");
        guard.execute(
            r#"INSERT INTO daily_snapshots (
                local_date, round_trips_closed, net_pnl_usd, wins, losses,
                discipline_index, trades_count, learning_baseline, unknown_basis_count, updated_at_ms
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)
            ON CONFLICT(local_date) DO UPDATE SET
              round_trips_closed=excluded.round_trips_closed,
              net_pnl_usd=excluded.net_pnl_usd,
              wins=excluded.wins,
              losses=excluded.losses,
              discipline_index=excluded.discipline_index,
              trades_count=excluded.trades_count,
              learning_baseline=excluded.learning_baseline,
              unknown_basis_count=excluded.unknown_basis_count,
              updated_at_ms=excluded.updated_at_ms"#,
            params![
                snap.local_date.format("%Y-%m-%d").to_string(),
                snap.round_trips_closed,
                snap.net_pnl_usd,
                snap.wins,
                snap.losses,
                snap.discipline_index,
                snap.trades_count,
                snap.learning_baseline as i32,
                snap.unknown_basis_count,
                Utc::now().timestamp_millis(),
            ],
        )?;
        Ok(())
    }

    pub fn fetch_daily_snapshot(
        &self,
        local_date: NaiveDate,
    ) -> anyhow::Result<Option<DailySnapshot>> {
        let guard = self.conn.lock().expect("today sqlite mutex poisoned");
        let mut stmt = guard.prepare(
            r#"SELECT round_trips_closed, net_pnl_usd, wins, losses, discipline_index,
                      trades_count, learning_baseline, unknown_basis_count
               FROM daily_snapshots WHERE local_date = ?1"#,
        )?;
        let day = local_date.format("%Y-%m-%d").to_string();
        let row = stmt
            .query_row(params![day], |r| {
                Ok(DailySnapshot {
                    local_date,
                    round_trips_closed: r.get(0)?,
                    net_pnl_usd: r.get(1)?,
                    wins: r.get(2)?,
                    losses: r.get(3)?,
                    discipline_index: r.get(4)?,
                    trades_count: r.get(5)?,
                    learning_baseline: r.get::<_, i32>(6)? != 0,
                    unknown_basis_count: r.get(7)?,
                })
            })
            .optional()?;
        Ok(row)
    }

    pub fn fetch_round_trips_for_day(
        &self,
        local_date: NaiveDate,
        limit: usize,
    ) -> anyhow::Result<Vec<PersistedRoundTrip>> {
        let guard = self.conn.lock().expect("today sqlite mutex poisoned");
        let mut stmt = guard.prepare(
            r#"SELECT symbol, closed_at_rfc3339, avg_entry, avg_exit, qty, realized_pnl_usd,
                      unknown_basis, fee_unhandled, quote_not_usd, primary_flag, flag_severity
               FROM round_trips
               WHERE local_close_date = ?1
               ORDER BY closed_at_rfc3339 DESC
               LIMIT ?2"#,
        )?;
        let day = local_date.format("%Y-%m-%d").to_string();
        let rows = stmt.query_map(params![day, limit as i64], |r| {
            Ok(PersistedRoundTrip {
                symbol: r.get(0)?,
                closed_at: r.get(1)?,
                avg_entry: r.get(2)?,
                avg_exit: r.get(3)?,
                qty: r.get(4)?,
                realized_pnl_usd: r.get(5)?,
                unknown_basis: r.get::<_, i32>(6)? != 0,
                fee_unhandled: r.get::<_, i32>(7)? != 0,
                quote_not_usd: r.get::<_, i32>(8)? != 0,
                primary_flag: r.get(9)?,
                flag_severity: r.get(10)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub fn upsert_journal_trip_cite(
        &self,
        declaration_id: &str,
        net: f64,
        currency: &str,
        local_close_date: NaiveDate,
    ) -> anyhow::Result<()> {
        let guard = self.conn.lock().expect("today sqlite mutex poisoned");
        guard.execute(
            r#"INSERT INTO journal_trip_cites (declaration_id, net, currency, local_close_date)
               VALUES (?1, ?2, ?3, ?4)
               ON CONFLICT(declaration_id) DO UPDATE SET
                 net=excluded.net,
                 currency=excluded.currency,
                 local_close_date=excluded.local_close_date"#,
            params![
                declaration_id,
                net,
                currency.to_ascii_uppercase(),
                local_close_date.format("%Y-%m-%d").to_string(),
            ],
        )?;
        Ok(())
    }

    pub fn fetch_journal_trip_cites_between(
        &self,
        start: NaiveDate,
        end: NaiveDate,
    ) -> anyhow::Result<Vec<JournalTripCiteRow>> {
        let guard = self.conn.lock().expect("today sqlite mutex poisoned");
        let mut stmt = guard.prepare(
            r#"SELECT declaration_id, net, currency, local_close_date
               FROM journal_trip_cites
               WHERE local_close_date >= ?1 AND local_close_date <= ?2
               ORDER BY local_close_date DESC, declaration_id ASC"#,
        )?;
        let start_s = start.format("%Y-%m-%d").to_string();
        let end_s = end.format("%Y-%m-%d").to_string();
        let rows = stmt.query_map(params![start_s, end_s], |r| {
            Ok(JournalTripCiteRow {
                declaration_id: r.get(0)?,
                net: r.get(1)?,
                currency: r.get(2)?,
                local_close_date: r.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JournalTripCiteRow {
    pub declaration_id: String,
    pub net: f64,
    pub currency: String,
    pub local_close_date: String,
}

#[derive(Debug, Clone)]
pub struct PersistedRoundTrip {
    pub symbol: String,
    pub closed_at: String,
    pub avg_entry: f64,
    pub avg_exit: f64,
    pub qty: f64,
    pub realized_pnl_usd: Option<f64>,
    pub unknown_basis: bool,
    pub fee_unhandled: bool,
    pub quote_not_usd: bool,
    pub primary_flag: String,
    pub flag_severity: String,
}

fn trip_key(symbol: &str, closed_at: DateTime<Utc>) -> String {
    format!("{}:{}", symbol, closed_at.timestamp_millis())
}

fn local_date(ts: DateTime<Utc>) -> NaiveDate {
    ts.with_timezone(&chrono::Local).date_naive()
}

#[cfg(test)]
mod journal_trip_cite_tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn journal_trip_cites_filter_by_local_close_week() {
        let dir = std::env::temp_dir().join(format!(
            "rta-journal-trip-cites-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("today.db");
        let store = TodayStore::open(&path).unwrap();
        let d11 = NaiveDate::from_ymd_opt(2026, 9, 11).unwrap();
        let d5 = NaiveDate::from_ymd_opt(2026, 9, 5).unwrap();
        store
            .upsert_journal_trip_cite("decl-in-week", 100.0, "inr", d11)
            .unwrap();
        store
            .upsert_journal_trip_cite("decl-out-week", 50.0, "USD", d5)
            .unwrap();
        let rows = store.fetch_journal_trip_cites_between(d11, d11).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].declaration_id, "decl-in-week");
        assert_eq!(rows[0].currency, "INR");
    }
}
