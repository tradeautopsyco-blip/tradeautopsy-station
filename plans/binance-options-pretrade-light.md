# Plan: Honest Binance Options Pre-trade lights

> Source: founder dogfood 2026-09-10 (XRP dated contract on Notch crypto Options). Prototype: `station/prototypes/PROTOTYPE-binance-options-pretrade-light.html` (logic, not UI variants). HTML `notch-options-declare.html` “fixture live” is **out** — that is BANKNIFTY Sensibull copy, not this book.

## Architectural decisions

- **Book:** `binance-com-options` only. Same live slug `binance_com`. No new slug, no new B6, no `/eapi/v1/optionChain`, no spot `@trade` / klines on a dated contract.
- **Bind:** mixed-case eapi id (`XRP-260911-1.36-C`) is the instrument. Three typed boxes (underlying / YYMMDD / strike) do not bind Last.
- **Glance already succeeds.** Chain and OI extracts return `status: success`. Notch currently keeps only that status string and paints a provenance hole. The work is **paint venue strings already on the envelope**, not a new upstream path.
- **Chain paint = catalog list, not a strike grid.** Rows are `optionSymbols` for the same (`underlying`, `expiryDate`): mixed-case `symbol`, `strike_raw`, `side`, `expiry_raw`. Optional TickBook `last` overlay when present. Clicking a row binds that contract (same as paste). Do not invent CE/PE/IV/OI columns. Do not guess missing strikes. `showsStrikeGrid` stays false in the Sensibull sense.
- **OI paint = LatestState strings.** Selected symbol → `sumOpenInterest` / `sumOpenInterestUsd` / `timestamp` as published. No exact symbol match → show the expiry row list keyed by mixed-case `symbol`, never pick a random row, never `{oi: 0}`.
- **Last = REST ticker into TickBook.** `GET /eapi/v1/ticker?symbol=` on dated bind (public, no HMAC). Empty TickBook stays **unavailable**, never last=0. REST last has no exchange timestamp → chip **unknown**, not fresh. Do not map WS last (field still NOT SPECIFIED). Twin of spot `prime` ticker: dial REST **before** any stream record; options has no last WS to subscribe.
- **How every panel lights — venue copy, not the HTML fixture.** BANKNIFTY “fixture live” stays illegal. Each hole has one honest source on `eapi.binance.com` or it stays dark.

  | Panel | Honest light | Still dark if |
  |---|---|---|
  | Last / premium / Quote | `GET /eapi/v1/ticker` `lastPrice` into TickBook | empty book (never `0`) |
  | Chain | paint `optionSymbols` catalog rows already on the glance | invent IV/OI call-put grid |
  | Open interest | paint `sumOpenInterest` strings already on the glance | `{oi:0}` |
  | Greeks | already `VenuePublished` mark | Station-computed Δ |
  | Session / OHLCV | **new lock slice** then `GET /eapi/v1/klines?symbol=&interval=` as `market/ohlcv` HistoricalSeries | spot `XRPUSDT` klines, derived resample, Yahoo |
  | Payoff | **new lock slice**: European cash settlement shape from strike + right + buy/sell + typed premium; x-axis = `GET /eapi/v1/index?underlying=` (same host, not spot TickBook). Short-call wing stays open (“unlimited” is not a number). Per contract; do not invent `unit=1` | Black-76, NFO SPAN, fixture Δ |
  | DTE | calendar from contract `expiryDate` ms vs now | σ-scaled DTE |
  | σ rungs 2–5 / margin | stay dark this plan | markIV-as-σ, USER_DATA margin |

- **Klines exist on eapi; they are not in this book’s lock yet.** Official CLI/OpenAPI: `GET /eapi/v1/klines` public, `symbol` + `interval` required, intervals `1m…1M`. OpenAPI dump types the body as `array of arrays` only — **do not copy spot kline column order from memory**. Phase 0 fetches the official field table onto the lock + Station REST.md, then product may dial it. `OHLCV-RESAMPLE.md` still blocks Station-built coarser bars; venue `interval` wins.
- **Payoff is settlement identity, not `OPTIONS-PRICING.md` ModelComputed.** That blocker fences Station-computed trader greeks / Black-76. A long option’s expiry P&L shape is the European cash-settled identity in options MECHANICS. Index for `S` is eapi `GET /eapi/v1/index`, not `binance-com-spot`.
- **Greeks:** already `VenuePublished` from mark. Do not recompute. Do not fold mark into TickBook last.
- **Seam:** one extract envelope per panel. Notch compose. No second writer of last. Dual-cwd: Station only. Lock edits live in `issues/compliance/locks/binance-com-options.md`.
- **Dispatch:** Phase 0 (lock fetch) is human/`needs-research` — never `ready-for-agent` until the page is cited. Then ticket 1 (Rust last) is the first cloud-capable slice. Swift chrome is `needs-macos`. One `ready-for-agent` at a time.

---

## Phase 0: Lock klines + index (blocks session and payoff)

**User stories:** the book’s lock names `GET /eapi/v1/klines` column identities and `GET /eapi/v1/index` so later slices copy venue bars and an underlying `S` without stitching spot.

### What to build

Fetch official Options Market Data pages (CLI + developers catalog; HTML may be Cloudflare-gated — SDK/CLI win as on slice 0). Write field names, weight, public auth, interval ENUM, and a refuse-list (no `/api/v3/klines`, no resample, no Black-76) onto the options lock. Station REST.md is citation only.

### Acceptance criteria

- [ ] Lock cites URL + fetch date for klines and index
- [ ] Kline columns named from that page, not from spot memory
- [ ] `interval` ENUM copied; unknown interval stays unsupported
- [ ] Index query is `underlying` (e.g. `XRPUSDT`), host `eapi.binance.com`
- [ ] Settlement-payoff identity sentence: long max-loss = premium paid; short call wing unbounded; not ModelComputed
- [ ] No product dial until this phase merges

---

## Phase 1: REST last on dated bind

**User stories:** paste a live eapi id → Last / entry premium / Quote chip leave `unavailable` when ticker `lastPrice` is a positive string.

### What to build

Dated bind dials public eapi ticker, plants `lastPrice` on `{binance-com-options}\0{mixed-case}`. Notch already binds Last from the quote extract when the envelope has a positive last — the hole today is TickBook empty because ticker is recorded, not dialed.

### Acceptance criteria

- [ ] Paste `BTC-200730-9000-C` (or a live listed id) → quote extract last is the ticker string, not `0`, not spot `c`
- [ ] Empty TickBook stays `unavailable`
- [ ] Chip is `unknown` (age unknown), not `fresh`
- [ ] Dated id never dials `/api/v3/ticker/price` or spot `@trade`
- [ ] Mixed-case identity does not collapse via lowercase

---

## Phase 2: Paint OI LatestState

**User stories:** when OI glance is `success`, the Open interest panel shows Binance’s `sumOpenInterest` strings for this contract / expiry, not a dashed provenance box.

### What to build

Keep OI glance `data` on the crypto Options surface. Render venue strings. Provenance line stays (`GET /eapi/v1/openInterest`). Not depth, not a strike grid.

### Acceptance criteria

- [ ] Exact symbol match paints `sumOpenInterest` (and Usd/timestamp when present) verbatim
- [ ] No match + non-empty expiry rows: list by mixed-case symbol, no invented total
- [ ] Empty/missing rows stay `unavailable`, never `0`
- [ ] NFO three-zone and spot Options tab unchanged

---

## Phase 3: Paint chain catalog list

**User stories:** when chain glance is `success`, the Chain panel lists catalog contracts around the bound expiry; clicking a row binds that id (Last/OI/greeks follow). Missing contract is empty, not a guessed strike.

### What to build

Keep chain glance `rows`. Render a catalog table: symbol, strike_raw, side, expiry. Optional last overlay from TickBook only. Row click = existing paste-bind. Still refuse an IV/OI call-put grid and `/eapi/v1/optionChain`.

### Acceptance criteria

- [ ] `success` with rows shows catalog rows from `optionSymbols`, not the “no strike grid” empty block
- [ ] `empty` stays “no contracts” (not a filled grid)
- [ ] Clicking a row binds that mixed-case id and refreshes extracts
- [ ] Last overlay absent when TickBook empty — no `0`
- [ ] NFO three-zone unchanged (still no invented 57500 grid)

---

## Phase 4: Venue klines session series

**User stories:** Session panel shows this contract’s eapi klines, not “no Binance obtain in this slice”.

### What to build

`market/ohlcv` HistoricalSeries from `GET /eapi/v1/klines` on the dated symbol. Notch session chart consumes that extract only. Default one venue interval from the lock ENUM (do not resample).

### Acceptance criteria

- [ ] Dated bind obtains klines on `binance-com-options`; empty series is `unavailable`, not zeros
- [ ] Spot `/api/v3/klines` never runs for a dated contract
- [ ] Interval not on the ENUM stays unsupported
- [ ] Derived/ohlcv resample stays a hole

---

## Phase 5: Settlement payoff + DTE

**User stories:** At-expiry panel draws this leg’s cash-settled shape against eapi index; DTE is calendar from `expiryDate`.

### What to build

X-axis from `GET /eapi/v1/index`. Curve from strike, call/put, buy/sell, typed entry premium, contract count. Per contract; no invented `unit`. Short call: open wing, no fake cap. DTE chip from expiry ms. σ rungs 2–5 and margin stay dark.

### Acceptance criteria

- [ ] Long call/put shape flips with right and side; max loss on a long is the premium paid
- [ ] Short call does not print a numeric “unlimited”
- [ ] Index miss keeps payoff `unavailable` (no spot last substitute)
- [ ] DTE lights from `expiryDate` without a greeks model
- [ ] No Black-76, no NFO formula, no fixture Δ

---

## Out of this plan

- σ ladder rungs 2–5 (needs a named IV→σ model)
- Venue margin (`marginAccount` is USER_DATA)
- HTML `fixture live` BANKNIFTY numbers
- `POST /eapi/v1/order`, fills/PnL, rho, IV ≤ 0, carrying mark onto NFO
- Spot klines / Yahoo / derived resample as options history
