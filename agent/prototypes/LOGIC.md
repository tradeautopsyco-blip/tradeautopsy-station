# PROTOTYPE — Today v1 round-trip + WAC engine

**Question:** Does the round-trip reconstruction + WAC realized P&L model feel right — including unknown basis, day boundaries, and hero aggregate honesty?

**Branch:** LOGIC (UI already answered in `station/prototypes/PROTOTYPE-station.html`)

**Run:**
```bash
cargo run --manifest-path agent/Cargo.toml --bin prototype-today-engine
```

**Assumption:** Fees in USDT/BUSD/USD are 1:1 USD; fees in base asset convert at fill price. Matches v1 Binance.US spot where quote is typically USD-stable.

**Fixtures:** Hand-calculated cases aligned with PRD acceptance criteria (WAC net of fees, unknown basis excluded from hero, open positions omitted, local calendar day).

**Verdict:** Engine + API + Station UI shipped in production modules (`round_trip_engine`, `today/`, `TodayView`). Delete prototype when manual smoke passes.

**Production run:** `GET /api/daemon/today` · Station Today route
