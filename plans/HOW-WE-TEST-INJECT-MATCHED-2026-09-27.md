# How we test FILL_MATCHED inject — 2026-09-27

**Goal:** Prove declare → inject matched → fidelity/closed snapshot without a real Binance order.

**PRs (all still draft as of write):**
- Console [#378](https://github.com/FExEVIL/tradeautopsy/pull/378) — inject route + env gate
- Station [#85](https://github.com/FExEVIL/tradeautopsy-station/pull/85) — agent forward + `--inject-matched`
- Console [#377](https://github.com/FExEVIL/tradeautopsy/pull/377) — capture hang (separate lane; not required for inject)

---

## Test pyramid for this feature

### T0 — Already automated (no live stack)

| Where | Command / check | PASS |
| --- | --- | --- |
| Console | PR #378 unit/route tests (7) | Gate off → 404; persist FILL_MATCHED; route auth |
| Station | `cargo test --test bar_forward -- --test-threads=1` | Forward POSTs body + Bearer to Console path |
| Station | `cargo test --test station_wire_contract` | Wire tables list `test_fill_matched` |

Run these anytime without Binance or Journal UI.

### T1 — Negative live (flag off) — safe on prod Console

**Setup:** Station Debug with PR #85 agent; Console **without** `BAR_TEST_INJECT_FILL_MATCHED` (prod default).

```bash
./scripts/notch-live-journal.sh \
  --book binance-com-spot --symbol BTCUSDT \
  --inject-matched --wait-closed-sec 5 --keep-declaration
```

**PASS:** Inject HTTP **404**; harness **exit 5**; scoreboard says Console flag off.  
**Meaning:** Opt-in gate works; we did not accidentally match on prod.

### T2 — Positive live (flag on) — local Console or preview only

**Never** set the env on production.

**Setup:**
1. Console local (`:3000`) or Vercel **preview** with `BAR_TEST_INJECT_FILL_MATCHED=1`.
2. Station Debug agent built from PR #85 (or merged main), listening `:9137`.
3. Point agent’s Console base URL at that local/preview host (same as other bar proxies).
4. Gate B: Station signed into Console (JWT ~15 min).
5. Gate C: Binance sync optional for tools; inject itself does not need a fill.

```bash
./scripts/notch-live-journal.sh \
  --book binance-com-spot --symbol BTCUSDT \
  --inject-matched --wait-closed-sec 30 \
  --require-debrief --keep-declaration
```

**PASS checklist:**
- [ ] Gates A/B PASS (C preferred)
- [ ] Declare PASS → `declaration_id` in scoreboard
- [ ] Inject HTTP 2xx; response `status=matched` / `test_only=true`
- [ ] Match fidelity section **not** stuck on `not_ready` (matched / closed snapshot fields populated as harness records)
- [ ] Debrief notes sink still PASS
- [ ] Process exit **0**
- [ ] Journal / declarations UI: that id shows **matched** (eyeball)
- [ ] Cancel or leave `--keep-declaration` then cancel manually after eyeball

**FAIL meanings:**
| Symptom | Likely cause |
| --- | --- |
| Inject 404 | Env unset or wrong Console host |
| Inject 401 | Gate B expired — re-sign Station |
| Agent route missing | Station not on PR #85 build |
| Still `not_ready` after 30s | Inject OK but fidelity poll path wrong — read scoreboard inject fields |
| Exit 5 | Harness treated inject as failed — see scoreboard errors |

### T3 — Optional curl smoke (Console only)

With local Console + flag + Station JWT:

```bash
# After a real declare id exists:
curl -sS -X POST "http://127.0.0.1:3000/api/internal/bar/v1/test/fill-matched" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $STATION_CALLER_JWT" \
  -d '{"declaration_id":"<uuid>","symbol":"BTCUSDT","side":"BUY","quantity":0.01,"price":50000}'
```

Expect JSON with `test_only: true`, `status: "matched"`.

### T4 — Human dogfood (rare)

Plan → tiny **real** fill → Working → close → Debrief. Inject does **not** replace this; it only unlocks automation for match fidelity.

---

## Recommended order to run today

1. Merge or locally check out **#378** + **#85** (draft → ready).
2. Run **T0** unit tests on both repos.
3. Run **T1** against prod Console (expect exit 5 / 404) — proves safety.
4. Run **T2** against **local Console** with env=1 (fastest positive proof).
5. Only then consider preview env=1 for a shared dogfood.

---

## What we are not testing with inject

- Real broker order placement
- Production fill-ingest persisting matched (still a separate gap)
- Toolbar capture → Journal (#377 / REDIS_URL lane)
