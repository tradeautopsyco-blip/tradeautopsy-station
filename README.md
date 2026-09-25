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

## Updates (signed releases)

Production builds are **Developer ID signed and notarized**. In-app updates use [Sparkle](https://sparkle-project.org/) against `https://updates.tradeautopsy.in/appcast.xml` (see `updater/` and `.github/workflows/release.yml`).

The first signed install (**0.2.0+**) replaces unsigned builds: TCC (mic, speech, screen recording) and Keychain items do **not** carry over from ad-hoc installs.

Kill-switch DNS still requires a **one-time** admin install of `sudoers.d/99-tradeautopsy-dns` — that is separate from Gatekeeper.

## Share with a cofounder (unsigned dev drop)

Apple Silicon, macOS 14+. **No Developer ID / notarization** — Gatekeeper will warn once. Use this only until a signed release is available.

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
