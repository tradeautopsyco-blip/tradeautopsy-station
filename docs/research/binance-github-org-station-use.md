# Binance GitHub org → Station crypto books (fetched 2026-09-19 IST)

**Question:** which of the 48 public `github.com/binance` repos Station should cite (or ignore) so USDⓈ-M can be dogfooded, then Coin-M / other crypto classes, without blending books.

**Verdict in one line:** do **not** cargo-merge any Binance connector into Station. Cite official CLI + OpenAPI-generated SDKs for **path / host / field names**. Keep the existing thin REST clients. USDM is already a **named shipping book**; founder test is live `obtain` on that book, not a new slug.

---

## Skill A (already closed for this class)

| Item | Answer |
|------|--------|
| Lock | [`issues/compliance/locks/binance-com-usdm.md`](../../../issues/compliance/locks/binance-com-usdm.md) (SHIPPING, 2026-09-15) |
| Same slug | `binance_com` — **no new B6**, no new broker |
| Host / path | `https://fapi.binance.com` + `/fapi/` |
| PnL owner | `agent/src/usdm_realized_pnl.rs` — **not** `round_trip_engine.rs` |
| Allow this slice | HMAC `GET /fapi/v3/balance`, `GET /fapi/v3/positionRisk`, `GET /fapi/v1/forceOrders` (lossy); public `GET /fapi/v1/ticker/price` (no HMAC) |
| Refuse | spot `/api/v3/` · eapi · dapi · `POST /fapi/v1/order` · `/fapi/v2/ticker/price` this slice · depth/klines until named · testnet · `pricePrecision` as tick · complete force-order · default Start=USDM |
| Sister book | Coin-M [`locks/binance-com-coinm.md`](../../../issues/compliance/locks/binance-com-coinm.md) on `dapi.binance.com` `/dapi/` |

Blended “crypto perps” as one class is **N-A** in [`CLAIM-REGISTRY.md`](../../../issues/compliance/CLAIM-REGISTRY.md). Matching is `book_id`, never the letters `BTCUSDT`.

**B6 drift (closed 2026-09-19):** row 1/12 still describe slug v1 as spot-only. Amendment [sheets/binance_com.md](sheets/binance_com.md) §2026-09-19: named USDM/Coin-M books are locks; Start stays spot; clone is cite-only.

---

## What Station already has (do not re-implement)

| Layer | Path |
|-------|------|
| Client | `agent/src/binance_com_usdm_client.rs` → `fapi.binance.com` |
| Obtain kick | `ensure_usdm_balance` / `ensure_usdm_positions` / `ensure_usdm_force_orders` |
| Manifest | `binance_com.usdm.v1`, book `binance-com-usdm`, implemented `funds` · `positionbook` · `forceorder` |
| Wiremock | `agent/tests/usdm_obtain.rs` |
| Notch | **no** `BarDeclareAssetClass` for USDM (only spot / equity / options). Harness cannot bind this book as a declare desk yet. |

Phase 1 **law** names public `GET /fapi/v1/ticker/price` on this book (HMAC must not attach; last = JSON `price`). Notch class / `BarDeclareAssetClass` for USDM is still a **later** slice. TRADE still out. Running obtain `quotes` stays `unsupported` until Phase 1 **code**. History / tradebook stay unnamed. Do not expect a USDM class chip tonight.

---

## How to include USDM in testing (founder, this weekend)

N1 Confirm/Cancel is **LiveBook**. USDM is **account USER_DATA**. Keep them as two checklists on the same Debug app (`b0918cc`).

### A. N1 (already written)

Spot or Options declare → collapse/expand still armed → Exit trade clears. 24/7. Does not need fapi.

### B. USDM obtain (new, 24/7 — this is the USDM test)

Keys: same COM HMAC already in Keychain. On binance.com API management: **Enable Reading** on USDⓈ-M Futures. **Disable** futures TRADE and withdraw (same withdraw-detect as spot).

1. Xcode Run this tree. Start `binance_com` as today (default book stays **spot**).
2. Loopback obtain — **must** pass `book=binance-com-usdm` (slug-only obtain stays spot):

```text
GET /api/station/obtain?adapter=binance_com&book=binance-com-usdm&operation=funds
GET /api/station/obtain?adapter=binance_com&book=binance-com-usdm&operation=positionbook
GET /api/station/obtain?adapter=binance_com&book=binance-com-usdm&operation=forceorder
```

3. Oracle the same paths with official CLI (cite, not a Station dependency):

```bash
# https://github.com/binance/binance-cli/blob/master/examples/derivatives-trading-usds-futures.md
export BINANCE_API_ENV=prod
binance-cli futures-usds futures-account-balance-v3
binance-cli futures-usds position-information-v3
binance-cli futures-usds users-force-orders --auto-close-type LIQUIDATION
```

CLI paths that must match the lock: `GET /fapi/v3/balance`, `GET /fapi/v3/positionRisk`, `GET /fapi/v1/forceOrders`. Host default `fapi.binance.com`.

**Pass**

| Obtain | Honest result |
|--------|----------------|
| funds | `success`, holdings from `availableBalance` > 0; empty after drop is `holdings: []` not `data: null` |
| positionbook | `success`; `positionAmt` `0` skipped; qty `2` must not collapse to `1`; `exchange_segment=usdm` |
| forceorder | `observing` if rows, `idle` if `[]`, `unavailable` on fail — **never** `synced` / `fresh` |

**Fail if:** spot WAC numbers on this book · INR blend · `pricePrecision` · TRADE place · testnet host without a B6 amendment · Notch paints USDM last from spot `BTCUSDT`.

### Testnet / demo — out until B6 says otherwise

Official CLI allows `BINANCE_API_ENV=testnet` and `BINANCE_FUTURES_USDS_BASE_PATH=https://testnet.binancefuture.com` ([binance-cli README](https://github.com/binance/binance-cli/blob/master/README.md)). Station live client hardcodes `fapi.binance.com`. B6: “testnet separate refuse unless sheet updated.” Wiremock (`binance_usdm_base_url`) is the **unit** seam, not founder dogfood.

---

## The 48 `org:binance` repos (live search 2026-09-19)

Use / ignore / later. **Use** = cite for path/host/HMAC. **Do not cargo-depend.** Station already owns the wire.

### Use for Station crypto (cite)

| Repo | Why |
|------|-----|
| [binance/binance-cli](https://github.com/binance/binance-cli) | **Primary oracle.** Rust CLI, MIT. Product verbs match locks: `futures-usds`, `futures-coin`, `derivatives-options`, `spot`. Already cited on the options lock. |
| [binance/binance-connector-rust](https://github.com/binance/binance-connector-rust) (`binance-sdk` crate) | OpenAPI-generated path names. Feature `derivatives_trading_usds_futures` → production client. **Cite README / generated types. Do not add the crate** — it exposes TRADE (`new-order`) the lock refuses. |
| [binance/binance-spot-api-docs](https://github.com/binance/binance-spot-api-docs) | Spot REST + errors + market-data-only FAQ. Already the B6 / `spot/REST.md` source. |
| [binance/binance-signature-examples](https://github.com/binance/binance-signature-examples) | HMAC / RSA signing examples if a USER_DATA 401 needs a replay. |
| [binance/binance-public-data](https://github.com/binance/binance-public-data) | Official bulk klines/trades dumps — later history backfill, not live TickBook. |
| [binance/binance-api-postman](https://github.com/binance/binance-api-postman) | Human probe of the same REST surface. |

### Cite only while migrating off deprecated names

| Repo | Note |
|------|------|
| [binance/binance-futures-connector-python](https://github.com/binance/binance-futures-connector-python) | **Deprecated.** Retired as lock cite **2026-09-19**. Hosts still `fapi`/`dapi`; oracle is the local Rust clone. |
| [binance/binance-connector-python](https://github.com/binance/binance-connector-python) | Current Python modular connector (spot + derivatives). Same role as rust SDK: **path oracle**, not a Station import. |
| [binance/binance-connector-js](https://github.com/binance/binance-connector-js) · java · go · php · ruby · dotnet · typescript | Language siblings. Ignore unless a field name disagrees across SDKs — then CLI + HTML catalog win. |

### Futures language clones (same as python-deprecated)

`binance-futures-connector-java`, `binance-futures-connector-node`, `binance-futures-java-toolbox` — do not extra-cite. Paths live in CLI `futures-usds` / `futures-coin`.

### Do not use for Station product

| Repo | Why |
|------|-----|
| `binance-pay-*`, `asymmetric-key-generator` | Pay / RSA toy — not a book |
| `binance-web3-connector-*` | Web3 public API — not COM spot/USDM |
| `desktop` | Electron release channel |
| `zkmerkle-proof-of-solvency` | Exchange solvency proof |
| `ai-trading-prototype*` | Strategy bot. Not a venue client. |
| `crypto-trade-analyzer` | Venue-comparison toy |
| `binance-skills-hub` | Agent skills marketplace — not a lock source |
| `binance-fix-connector-python` | FIX. Out of v1. |
| `binance-sbe-*-sample-app` | SBE decode samples. REST JSON is the shipping wire. |
| `binance-mp-demo`, `websocket-demo`, `binance-toolbox-*` | Demos / helper scripts |
| `binance-java-logback` | Logging fork |
| Connector “stocks” feature in rust SDK | USA equity = **REFERENCE**. No lock. |

There is **no** `binance-futures-api-docs` repo on this org. USD-M HTML is `developers.binance.com/docs/derivatives/usds-margined-futures/` (Cloudflare-gated in prior fetches — SDK/CLI win).

---

## Official module map (CLI = rust SDK feature)

Source: [binance-cli README command list](https://github.com/binance/binance-cli/blob/master/README.md) + [binance-connector-rust features](https://github.com/binance/binance-connector-rust/blob/master/README.md).

| CLI product | SDK feature | Station book today | Next |
|-------------|-------------|--------------------|------|
| `spot` | `spot` | `binance-com-spot` SHIPPING | keep |
| `futures-usds` | `derivatives_trading_usds_futures` | `binance-com-usdm` SHIPPING (USER_DATA slice) | **founder obtain first** |
| `futures-coin` | `derivatives_trading_coin_futures` | `binance-com-coinm` SHIPPING (same slice on dapi) | **second**, copy USDM obtain pattern, DualNoBlend |
| `derivatives-options` | `derivatives_trading_options` | `binance-com-options` SHIPPING (last + chain/OI; S8 signed) | no PnL owner yet |
| `margin-trading` | `margin_trading` | citation dump `docs/reference/.../margin/` | **no lock → no book** |
| `derivatives-trading-portfolio-margin` (+ pro) | `derivatives_trading_portfolio_margin` | citation dump `portfolio-margin/` | **no lock → no book** |
| `convert` | `convert` | citation dump `convert/` | no lock |
| `wallet` | `wallet` | citation dump `wallet/` (apiRestrictions already on slug) | keep as slug hygiene, not a book |
| algo / earn / loan / staking / c2c / copy / pay / fiat / mining / gift / rebate / sub-account / vip-loan / dual-investment / alpha | matching features | none | refuse until founder names + lock |
| `stocks` | `stocks` | none | USA REFERENCE |

---

## Order of work (no code in this note)

1. **USDM founder obtain** on live `fapi` with `book=binance-com-usdm` (checklist B). Sign a dogfood file when it matches CLI.
2. **Coin-M obtain** — same three nouns on `dapi`, book `binance-com-coinm`. Never parse with the USDM client.
3. Phase 1 names public `GET /fapi/v1/ticker/price` on **this** book (lock allow-list, 2026-09-19). Notch `BarDeclareAssetClass.usdm` is still a **later** slice. Depth/klines still unnamed. TRADE still out.
4. **Do not** add `binance-sdk` to `Cargo.toml`.
5. **Do not** start margin / portfolio-margin / convert as live books.
6. **Done 2026-09-19:** USDM/Coin-M/spot/options locks + B6 + MECHANICS headers cite local clone `592f16b` / `binance-sdk` 70.1.0. Deprecated python futures connector retired. Pin: [binance-connector-rust-citation.md](binance-connector-rust-citation.md). Wire paths were already aligned — no client behavior change.

---

## Sources (this pass)

- GitHub search `org:binance` → **48** public repos, 2026-09-19 IST
- https://github.com/binance/binance-cli/blob/master/README.md
- https://github.com/binance/binance-cli/blob/master/examples/derivatives-trading-usds-futures.md (`/fapi/v3/balance`, `/fapi/v3/positionRisk`, `/fapi/v1/forceOrders`)
- https://github.com/binance/binance-connector-rust/blob/master/README.md + Context7 `/binance/binance-connector-rust` (feature `derivatives_trading_usds_futures`, production client)
- Local clone `/Users/bishnu/binance-connector-rust` @ `592f16b` (`binance-sdk` 70.1.0) — pin [binance-connector-rust-citation.md](binance-connector-rust-citation.md)
- Locks + Station `usdm_obtain.rs` + `source_manifest.rs` as of local `main` `b0918cc`
