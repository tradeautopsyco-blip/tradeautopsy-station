# TradeAutopsy Station

Native macOS companion to [TradeAutopsy](https://tradeautopsy.in) — the behavioral
circuit breaker between trader psychology and broker execution.

## Components

| Folder | What it is |
|--------|-----------|
| `agent/` | Rust/Axum loopback daemon (127.0.0.1:9137) — kill switch enforcement, broker sync, Ed25519 audit |
| `notch/` | Swift/SwiftUI Notch HUD — floating pill, warning cards, pre-trade declare, verdict UI |

## Wire protocol

`station-wire/v1.json` is the versioned contract between this repo and the brain
(`FExEVIL/tradeautopsy`). Pin `station_version` and `brain_min_version` on every release.

## Requirements

- macOS 14+
- Rust toolchain (stable)
- Xcode 15+
- `sudoers.d/99-tradeautopsy-dns` for L3 DNS enforcement

## Dev

```bash
# Run the agent
cd agent && cargo run

# Tests
cd agent && cargo test
```

## Architecture invariants (never violate)

- Notch never calls tradeautopsy.in directly — all egress via agent on 9137
- Agent never renders UI
- All signals reach brain via ingestSignal() only
- Agent never places entry orders
- No LLM inference inside Station
