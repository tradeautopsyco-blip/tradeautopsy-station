# Dogfood — `zerodha_kite` cash (`zerodha-nse-bse-cash`)

**Status:** DRAFT — **Tier I Connect beta Enabled** (mechanical prep 2026-09-25); **Tier II / Z11** step 6 open  
**Lock:** `/Users/bishnu/issues/compliance/locks/zerodha-nse-bse-cash.md` (**SHIPPING**)  
**B6:** `/Users/bishnu/issues/brokers/sheets/zerodha_kite.md` (`SIGNED`)  
**ADR:** `docs/adr/0005-zerodha-redirect-callback.md` (`ACCEPTED`)  
**Ladder:** [ADR 0019](/Users/bishnu/tradeautopsy-station/docs/adr/0019-broker-launch-validation-ladder.md)

| Tier | Pipeline | This file |
|------|----------|-----------|
| **I — Integrator** | Steps **1–5** | **OFFLINE** section — not step 6 |
| **II — Live (Z11)** | Step **6** signed | **LIVE** drills + sign-off → desk-live flip |

> Decision 8: **desk-live** = **`catalog_v1()` + registry live-slugs** (ADR 0019 §C).  
> `zerodha_kite` is **Connect beta Enabled** on the cash book row + UI badge (`BrokerDogfoodProgram.connectBetaSlugs`); not desk-live until Z11 sign-off.

---

## OFFLINE — Tier I steps 1–5 (integrator — no live Kite session)

### Pipeline steps

| Step | Deliverable | Status |
|------|-------------|--------|
| **1** Sheet | B6 `zerodha_kite` **SIGNED** | [x] |
| **2** Shape | ADR **0005** **ACCEPTED** | [x] |
| **3** Lock | `zerodha-nse-bse-cash.md` **SHIPPING** | [x] |
| **4** Integrate | Wasm + host session + allowlist + dns + component tests | [x] |
| **5** Fixtures | `agent/fixtures/zerodha_kite/` + `ubi_zerodha_kite_component` | [x] |

### CI tests

| Check | Command / target | Pass when |
|-------|------------------|-----------|
| ADR 0019 connect beta | `cargo test -p tradeautopsy-agent --lib adr_0019_connect_beta` | book row **Enabled**; slug **not** in `catalog_v1()` |
| B6 lint | `cargo test -p tradeautopsy-agent --test b6_sheet_lint` | `zerodha_kite` surface rows 0–25 |
| Component contract | `cargo test -p tradeautopsy-agent --test ubi_zerodha_kite_component` | fills map + secrets-never-seen |
| Kill + path fence | `cargo test -p tradeautopsy-agent --lib zerodha_kite_read_paths_allowed_execution_refused` | read paths OK; order POST refused |

- [x] `adr_0019_connect_beta`
- [x] B6 lint (`signed_zerodha_kite_covers_surface_rows_0_to_25`)
- [x] `ubi_zerodha_kite_component`
- [x] lib allowlist / execution refuse tests

### Fence (`rg`)

```bash
rg -n 'zerodha_kite|api\.kite\.trade|kite\.zerodha\.com' \
  agent/src/ubi/allowlist.rs agent/src/dns_block.rs agent/src/data/host_policy.rs
```

- [x] `/trades`, `/orders`, `/portfolio/holdings`, `/quote/ltp`, `/instruments/*` allowed (GET)
- [x] Order placement paths refused at host policy
- [x] `dns_block` maps slug `zerodha_kite` → `ZERODHA_HOSTS`

### Wasm build

```bash
cargo build -p ubi-zerodha-kite-adapter --target wasm32-wasip2 --release
```

- [x] Release wasm: `target/wasm32-wasip2/release/ubi_zerodha_kite_adapter.wasm`

### Swift Connect beta (no network)

- [x] `BrokerCatalog` cash row: `zerodha_kite`, `availability: .enabled`, `kiteChecksumSession`
- [x] `BrokerDogfoodProgram.showsConnectBetaBadge(slug: "zerodha_kite")` (unit tests)

### Catalog / registry (Tier I — **no desk-live flip**)

- [x] `catalog_books()` cash row `zerodha-nse-bse-cash`: **Enabled** (Connect beta)
- [x] `catalog_v1()` unchanged — still `kotak_neo` + `binance_com` only
- [x] `CLAIM-REGISTRY.md` **Live slugs** unchanged; Zerodha listed under **Connect beta** only

---

## LIVE — Tier II step 6 (Z11 — founder Kite Connect app + account)

**Gate:** Paid Kite Connect app, redirect registered, agent healthy. Founder may defer; do not tick Tier II until drills run on production Kite.

### Preconditions (Z11 — founder)

- [ ] Kite Connect app redirect registered **exactly** (portal requires `https://`):  
  `https://127.0.0.1:9140/api/daemon/broker/zerodha/callback`  
  (developers.kite.trade → your app → Redirect URL; **Postback** left empty)
- [ ] Agent listening on **`127.0.0.1:9137`** (Station.app embedded agent, or release agent on that port)
- [ ] Live Kite account (production; no paper env per B6 row 23)
- [ ] Kite Connect **paid** app with **api_key** + **api_secret** (daily login token model)

Record when done:

| Item | Value |
|------|--------|
| **Agent SHA** | `tradeautopsy-agent/0.1.0 (<git short sha>)` |
| **Station build** | Debug / Release + date IST |
| **Kite app name** | (no secrets) |

### Z11 — Connect runbook (UI)

1. Open **Brokers** → **Zerodha Kite** (badge **Connect beta**).
2. **Connect** → enter API key + secret → browser opens Kite login.
3. Approve login; browser hits loopback callback; Station should auto-start sync.
4. Confirm agent vault: loopback `POST /api/daemon/broker/credentials/present` with body  
   `{"environment":"prod","brokerSlug":"zerodha_kite","brokerConnectionId":"00000000-0000-4000-8000-000000000004"}` → `"present": true` (fixed connection id in `BrokerConnectionIdentity`).

### Z11 — Connect runbook (loopback only, optional)

If UI is unavailable, same flow via HMAC-protected daemon routes (Station signs requests when agent is paired).

1. `POST /api/daemon/broker/zerodha/connect/begin` with `brokerSlug` `zerodha_kite`, connection id above, `apiKey`, `apiSecret`.
2. Open `loginUrl` from response in browser; finish login.
3. Callback: `GET /api/daemon/broker/zerodha/callback?request_token=…&state=…`

### Z11 — Drills (B6 row 25)

| # | Drill | Steps | Pass when | Result |
|---|--------|--------|-----------|--------|
| 1 | Redirect login → checksum → `/trades` poll | Connect → Start if needed → wait for sync | Fills appear in Station sync; INR at insert on `zerodha-nse-bse-cash` | |
| 2 | Daily token / 06:00 expiry or forced logout | Next session after 06:00 IST **or** revoke session in Kite console | `403` / `TokenException` → **reconnect** prompt, **not** Kill | |
| 3 | Rate discipline | Normal sync day; watch agent logs | No sustained 429; obey 10 rps others / 1 rps quote (B6) | |
| 4 | Day-book cursor | Two sync runs same calendar day | Second poll does not duplicate/widen fills | |
| 5 | Kill | Kill switch L3 for slug `zerodha_kite` / broker `zerodha` | Blocks `api.kite.trade`, `kite.zerodha.com`, `ws.kite.trade`; other brokers OK | |
| 6 | DualNoBlend | Zerodha INR + Binance USD (+ Kotak INR if connected) | COM USD hero unchanged; each INR book its own strip | |

**STOP:** If redirect URL is rejected by Kite, or callback never reaches `127.0.0.1:9137`, stop and reopen ADR 0005 — do not invent a public redirect.

---

## Sign-off

**Tier I (steps 1–5 — offline / mechanical):**

- [x] All OFFLINE pipeline + CI rows ticked (2026-09-25 prep)
- [ ] LIVE preconditions + drills (Z11 deferred until founder session)

**Tier II (step 6 — live):** sign only after drills 1–6 green.

- [ ] Drill 1 green (fills + INR insert)
- [ ] Drill 2 green (session expiry → reconnect, not Kill)
- [ ] Drill 3 green (rate discipline)
- [ ] Drill 4 green (cursor / no duplicate widen)
- [ ] Drill 5 green (Kill hosts)
- [ ] Drill 6 green (DualNoBlend)
- [ ] No STOP fired, or STOP recorded with ADR revisit

**Signed:** —  
**Date:** — (IST)

---

## Desk-live flip (Tier II only — **post-sign integrator PR**)

Do **not** open this PR from Tier I ticks alone. Requires founder signature above + green Z11 drills.

### 1. Rust `catalog_v1()` (`agent/src/ubi/catalog.rs`)

Append a third `BrokerDescriptor` (copy fields from the existing `catalog_books()` cash row for `zerodha-nse-bse-cash`):

- `slug`: `zerodha_kite`
- `book_id`: `zerodha-nse-bse-cash`
- `manifest_id`: `tradeautopsy:zerodha-kite-cash@0.1.0`
- `auth_scheme`: `KiteChecksumSession`
- `availability`: `Enabled`

Update tests in the same file:

- `catalog_v1_first_pair_only_enabled` → expect **3** Enabled slugs (or rename to `catalog_v1_desk_live_slugs`)
- `b6_gate_only_signed_first_pair_is_enabled` → extend `signed_pair` / enabled vec to include `zerodha_kite`
- `adr_0019_connect_beta_slugs_enabled_on_book_rows` → remove `zerodha_kite` from `CONNECT_BETA` **or** keep book row Enabled but assert `zerodha_kite` **is** in `catalog_v1()` after flip

Run:

```bash
cd /Users/bishnu/tradeautopsy-station/agent
cargo test --lib catalog_v1
cargo test --lib adr_0019_connect_beta
cargo test --lib b6_gate
```

### 2. Registry (`issues/compliance/CLAIM-REGISTRY.md`)

- **Live slugs (desk-live Tier II):** append `` `zerodha_kite` `` to the sentence (keep `kotak_neo` + `binance_com` until intentionally replaced).
- **Connect beta** bullet: remove `` `zerodha_kite` `` from the list (or footnote “graduated 2026-… IST”).
- Zerodha cash SHIPPING row: change parenthetical to “desk-live Tier II” (drop “Tier II dogfood open”).

### 3. Swift (`station/StationApp/Broker/Models/BrokerDogfoodProgram.swift`)

- Remove `"zerodha_kite"` from `tierIIndiaCashSlugs` / `connectBetaSlugs` so UI drops **Connect beta** badge after desk-live.
- `BrokerCatalog` cash row stays `availability: .enabled` (already set).

Update tests:

- `BrokerBookClassDecisionTests` — `#expect(!BrokerDogfoodProgram.showsConnectBetaBadge(slug: "zerodha_kite"))` after flip
- Any Today / desk-hero test that keys off `catalog_v1()` length in Rust (agent) vs Swift `BrokerCatalog.v1` (11 rows) — follow existing Kotak/Binance pattern for hero eligibility

### 4. PR checklist

- [ ] Dogfood plan: **Signed** + **Date** filled; all LIVE drill Result cells noted
- [ ] `P3-KICKOFF.md` Wave 2 row: **desk-live** for `zerodha_kite`
- [ ] No change to auth, allowlist, or wasm unless a drill exposed a bug

**book_id:** `zerodha-nse-bse-cash`
