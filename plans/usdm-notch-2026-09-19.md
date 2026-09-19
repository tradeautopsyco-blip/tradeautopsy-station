# Plan: USDM on Notch / Harness (after obtain dogfood)

> Source: lock `issues/compliance/locks/binance-com-usdm.md` · obtain dogfood **signed** 2026-09-19 on `11c848f` · founder asked to add USDM to Notch / UI after connecting COM

**Not this plan:** N1 Confirm/Cancel on **spot**. Coin-M. Margin. Testnet. Cargo-depend `binance-sdk`. Default Start becoming USDM.

## Slice decision 2026-09-19

Founder: **keep** phases **1 → 2 → 3** as separate slices (do **not** merge lock quotes, Harness chrome, and Notch `usdm` class). **Landed 2026-09-19:** Phase 1 public last + Phase 2 named USDM strip + Phase 3 class `usdm` + Phase 4 tickSize persist + Phase 5 income `REALIZED_PNL` owner. Depth/klines stay unnamed. **2026-09-19 ~14:30 IST:** LiveBook **declare** on USDM is in (pre → armed live → cancel; live/post follow USDM positionbook). Stay away from order execution — Phase 6 TRADE / `POST /fapi/v1/order` stays **parked**. Auto-place SL stays off this tab.

## Architectural decisions

Durable decisions that apply across all phases:

- **Book:** `binance-com-usdm` on live slug `binance_com`. Host `https://fapi.binance.com`. Prefix `/fapi/`. Matching is **`book_id`**, never the letters `BTCUSDT` (spot WAC and USDM are two books).
- **Start:** stays `binance-com-spot`. No new slug. No new B6 sheet.
- **Routes:** loopback `GET /api/station/obtain?adapter=binance_com&book=binance-com-usdm&operation=…` (unsigned on loopback). Quotes/last later use the same book query, never slug-only.
- **Obtain now:** `funds` · `positionbook` · `forceorder` (lossy: `idle` / `observing` / `unavailable`, never `synced`) · `quotes` (public last, empty TickBook = `unavailable`).
- **TRADE:** `POST /fapi/v1/order` stays **MutationForbidden**. Notch **Confirm** on USDM is LiveBook declare (intent) only — not a venue place. Auto-place SL is off.
- **PnL owner:** Station `agent/src/usdm_realized_pnl.rs` only. Not `round_trip_engine.rs`. Realized from `GET /fapi/v1/income` `incomeType=REALIZED_PNL`. Venue `unRealizedProfit` may display; do not invent a sum.
- **Tick:** `GET /fapi/v1/exchangeInfo` `filters[].tickSize`. Never `pricePrecision`.
- **DualNoBlend:** USDT-M never blends with spot WAC or INR NFO. Account chrome that still maps Start → `binance-com-spot` must keep dropping USDM envelopes until a named USDM surface exists.
- **Declare class today:** Notch `BarDeclareAssetClass` is spot / equity / options only. No USDM tab until Phase 3 Swift. Harness last stays off this book until Phase 1 **code** (lock already names public ticker; depth/klines still unnamed).

## Skill A — this book (copy from lock)

1. Lock: `issues/compliance/locks/binance-com-usdm.md`
2. Realized PnL owner: `agent/src/usdm_realized_pnl.rs`
3. Charges owner: the lock until a runtime writer copies fill/`COMMISSION` income. Never 0.1%.
4. Allow: `/fapi/` USER_DATA balance + positionRisk + lossy forceOrders plus public `GET /fapi/v1/ticker/price` (no HMAC). Refuse: `/api/v3/` · eapi · dapi · TRADE place · `/fapi/v2/ticker/price` this slice · complete force-order · spot WAC blend · testnet.
5. `instrument_type` never product/exchange. Margin asset from the row. DualNoBlend vs spot / INR.
6. Stop claiming: spot WAC on USDM · complete force-order · `pricePrecision` · default Start=USDM
7. Goldens: `positionAmt` `2` ≠ `1` · tick = `tickSize` · refuse `BTCUSDT` blend · TRADE MutationForbidden · obtain `quotes` **law** = `success`/`unavailable` (not a spot ticker); **running Station** still `unsupported` until Phase 1 code · force-order never `synced`

---

## Phase 0: Founder Notch on **existing** books (now)

**User stories:** After connecting COM, prove the desk you already have — not a USDM tab.

### What to build

Nothing. Dogfood **spot** (and Options if you already use eapi). USDM obtain is loopback-only tonight.

### Acceptance criteria

Current truth **2026-09-19** (no USDM class chip; `BarDeclareAssetClass` remains spot / equity / options until a later Swift slice):

- [x] Notch Start is still COM **spot**
- [x] Account chrome free/holdings are **spot** (`binance-com-spot`) — empty spot wallet is honest even when USDM has USDT
- [x] N1: Confirm applies locally, collapse/expand stays armed, Cancel clears (spot LiveBook)
- [x] Options tab does **not** paint USDM last from a leftover `BTCUSDT`
- [x] No USDM class chip appears (if it does, this rebuild is wrong)

---

## Phase 1: Lock names public USDM quotes

**User stories:** Last on this book is legal to fetch before any Notch paint.

### What to build

**Law this slice (2026-09-19):** amend the USDM lock (and only this book) to allow public `GET /fapi/v1/ticker/price`. Keep HMAC off public calls. Quotes stay **book-scoped**. Depth/klines stay unnamed. Testnet still refused. **Code later:** running obtain `quotes` stays `unsupported` until a Phase 1 code slice.

### Acceptance criteria

- [x] Lock allow-list names the public paths + fetch date *(law this slice)*
- [ ] `obtain quotes` on `binance-com-usdm` is `success` or `unavailable`, not a spot ticker *(Phase 1 code — still `unsupported` in running Station)*
- [ ] Slug-only quotes still do **not** return fapi last
- [x] TRADE still MutationForbidden

---

## Phase 2: Read-only Harness / account chrome for the named USDM book

**User stories:** See USDM funds + positions without placing, without blending spot.

### What to build

A named-book pulse (Harness row or an explicit USDM chrome), keyed `book=binance-com-usdm`. Start slug still maps default chrome to spot. Switching the strip to USDM is an explicit class/book choice, not “COM is connected so show futures USDT.”

### Acceptance criteria

- [ ] USDM holdings (e.g. USDT ~1.04 on the 2026-09-19 obtain) can appear on a **USDM** strip
- [ ] Spot chrome still ignores `book_id=binance-com-usdm`
- [ ] Empty positionbook (`rows=[]`) shows empty, not a fake 1-lot
- [ ] Force-order is not a synced Account chip

---

## Phase 3: Notch class `usdm` + last / glance

**User stories:** Pick USDM on the declare class row; last is fapi on this book.

### What to build

Add a declare asset class for USDM (label distinct from Spot / Options). Glance last (and depth only if Phase 1 named it) on `binance-com-usdm`. `BTCUSDT` on this tab is a **USDM** contract identity, never TickBook spot.

### Acceptance criteria

- [ ] Class row includes USDM only after Phase 1
- [ ] Last on this tab is fapi; spot last does not fill it
- [ ] Leftover spot pair on Options still does not imply USDM
- [ ] Confirm/place still refused (no TRADE)

---

## Phase 4: Tick / lot from `exchangeInfo` (sizer, still no place)

**User stories:** Size and round using venue `tickSize` / `stepSize` before TRADE exists.

### What to build

Persist `GET /fapi/v1/exchangeInfo` filters at runtime for the bound USDM symbol. Sizer/entry round from `tickSize` / `stepSize`. Never `pricePrecision`.

### Acceptance criteria

- [ ] Tick golden: `tickSize` not `"8"`
- [ ] Qty `2` does not collapse to `1`
- [ ] No `POST /fapi/v1/order`

---

## Phase 5: Realized identity (`income` REALIZED_PNL)

**User stories:** Today / journal USDM PnL is venue income, not spot WAC.

### What to build

One writer: `usdm_realized_pnl.rs` copies `GET /fapi/v1/income` `incomeType=REALIZED_PNL` (and fees from fill/`COMMISSION` when named). Display `unRealizedProfit` stays a published string, not a second realized book.

### Acceptance criteria

- [ ] D3: lock cited; single owner file; DualNoBlend vs spot WAC / INR
- [ ] `round_trip_engine.rs` still does not eat this book
- [ ] Force-order envelope still ineligible for Kill / PnL import

---

## Phase 6: TRADE / LiveBook (only if founder names it)

**User stories:** Confirm / cancel / protective / fill on USDM like N1 spot.

### What to build

Lock amendment that allows `POST /fapi/v1/order` (and the matching USER_DATA open-orders/userTrades paths) **on this book only**. Apply-first LiveBook, Console write-behind. Same HMAC key; withdraw still off.

### Acceptance criteria

- [ ] Founder named TRADE in chat **and** the lock allow-list
- [ ] MutationForbidden removed only for the named TRADE paths on `binance-com-usdm`
- [ ] Spot / options / Coin-M unchanged
- [ ] N1-style dogfood: Confirm, collapse stays armed, Cancel clears, fill does not wait on a fake poll
