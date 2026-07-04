# Today v1 — Manual Smoke Checklist

> Binance.US read-only credentials · local agent on port 9137

## Setup

- [ ] Station launches; agent healthy on pulse strip footer
- [ ] Connect Binance.US on Brokers screen (no withdraw permission)
- [ ] Start broker sync; `sync-state` shows `syncing`

## Engine + API

- [ ] Complete one spot round-trip (buy → sell same symbol)
- [ ] `GET /api/daemon/today` returns non-null `hero.pnlTodayUsd` within $0.01 of hand calc
- [ ] Today hero P&L matches Binance Trade Analysis (performance basis) for that trip
- [ ] `performanceBasisNotTax: true` in payload

## Today UI (variant A — active)

- [ ] Today route shows hero tiles, 2 signals, trades table
- [ ] Circuit breaker banner appears when kill switch fires (matches Notch)
- [ ] Review & resume disabled until countdown completes

## Degraded (variant C)

- [ ] Pause broker sync → Today hero/signals show `—`, table empty
- [ ] Pulse strip session P&L shows `—` (not stale INR or last number)

## Empty (variant B)

- [ ] Fresh day with no closed trips → hero `—`, learning baseline on signals

## Release gates (CI)

- [ ] `cargo test --test round_trip_engine_tests --test today_release_gates`
- [ ] `swift test --filter TodayScreenPresentationTests`
- [ ] `swift test --filter TodayReleaseGateTests`
