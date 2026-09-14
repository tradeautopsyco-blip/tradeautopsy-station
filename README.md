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

## Share with a cofounder (unsigned)

Apple Silicon, macOS 14+. **No Developer ID / notarization** — Gatekeeper will warn once.

You (this Mac):

```bash
./scripts/pack-station-dmg.sh
```

Upload `dist/TradeAutopsy-Station-<sha>.dmg` to Drive, or download the GitHub Actions artifact `TradeAutopsy-Station-app` from a `main` run (7 days; needs repo access). `gh release create` works if they are a collaborator.

They:

1. Open the DMG.
2. Drag **TradeAutopsy Station** onto **Applications**.
3. **Right-click → Open → Open** (do not Move to Trash). Or System Settings → Privacy & Security → Open Anyway.
4. **⌥Space** is Notch. Public Binance Options last/chain/OI/session/payoff do not need API keys.

Internet downloads set quarantine; that is why they see Gatekeeper and you may not after a local pack.

## Architecture invariants (never violate)

- Notch never calls tradeautopsy.in directly — all egress via agent on 9137
- Agent never renders UI
- All signals reach brain via ingestSignal() only
- Agent never places entry orders
- No LLM inference inside Station
