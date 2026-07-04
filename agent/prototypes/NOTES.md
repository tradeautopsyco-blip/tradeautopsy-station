# PROTOTYPE — Today v1 engine (agent)

**Question:** Does round-trip reconstruction + WAC realized P&L produce honest Today hero/table state?

**Run:**
```bash
cargo run --manifest-path agent/Cargo.toml --bin prototype-today-engine
```

| Menu | What it exercises |
|------|-----------------|
| **1** | Batch fixtures — hand-calculated expected P&L |
| **2–6** | Step-through ingest — state after each fill |
| **7** | Mixed day — known P&L + unknown basis + open position |

**Related UI prototype:** `station/prototypes/PROTOTYPE-station.html` (layout A/B/C already answered)

---

## Verdict

_(fill after review)_

**When done:** Delete `agent/prototypes/` + `prototype_today_engine` bin, or fold engine into `agent/src/today/` (or similar).
