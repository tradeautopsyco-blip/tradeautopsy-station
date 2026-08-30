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
