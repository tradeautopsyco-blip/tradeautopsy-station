# Backend Box v1 — Manual Binance.US Smoke Checklist

**Never required in CI.** Run against the founder Binance.US account before release.

## Preconditions

- [ ] Release Station `.app` with bundled `tradeautopsy-agent`
- [ ] Real Binance.US API key (read-only or trade-enabled; **no withdraw permission**)
- [ ] Network access to Binance.US and TradeAutopsy prod backend

## Checklist

1. [ ] **Add credentials** — Open Brokers → Connect → enter API key and secret
2. [ ] **Live validation succeeds** — Card leaves Validating without Failed
3. [ ] **Auto-start sync** — Card reaches Syncing without manual Start
4. [ ] **90-day fills backfill** — Agent logs or sync summary show fill import activity
5. [ ] **Balances/holdings snapshot** — Exposure data appears (or honest empty)
6. [ ] **Open orders snapshot** — Open orders appear, including explicit empty state
7. [ ] **Stop pauses without agent restart** — Stop → Paused; agent health stays green; Notch/SSE unaffected
8. [ ] **Start resumes** — Start → Syncing again with fresh Keychain read
9. [ ] **Delete removes Keychain** — Delete → confirm → card returns to Not configured; Keychain item gone
10. [ ] **Behavioral opt-out** — With opt-out enabled, sync continues but behavioral uploads stay off

## Release sign-off

| Check | Owner | Date |
|-------|-------|------|
| Manual smoke complete | | |
| CI release gates green | | |
| No raw broker/auth payload in logs | | |
