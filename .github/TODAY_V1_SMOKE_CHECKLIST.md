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

- [x] `cargo test --test round_trip_engine_tests --test today_release_gates`
- [x] `swift test --filter TodayScreenPresentationTests`
- [x] `swift test --filter TodayReleaseGateTests`
- [x] Agent: trades table `closedAt` descending (most recent first)
- [x] Agent: day-boundary `daily_snapshots` finalization for prior local dates
- [x] Station: session mirror polling refreshes Today/pulse during broker sync

## Manual only (requires Binance.US credentials)

- [ ] Complete one spot round-trip → hero P&L within $0.01 of Binance Trade Analysis
- [ ] Pulse strip updates to match Today hero after round-trip without navigating away
