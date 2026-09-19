# Plan: NFO options cockpit mosaic

> Source PRD: NFO options cockpit mosaic (conversation 2026-09-19). Prototype: `?asset=options&desk=nfo&kind=opt&board=cockpit&phase=plan`.

## Architectural decisions

Durable decisions that apply across all phases:

- **Surface:** Pre-trade Options routing is the only seam. NFO product kind comes from scrip `instrument_type`, not from the token string and not from a new asset class.
- **Kind map:** OPTIDX / OPTSTK / OPTCUR / OPTCOM → cockpit mosaic + Plan rail. FUTIDX / FUTSTK / other / missing → existing three-zone. Dated Binance contract still wins as crypto Options. Leftover pair on Options stays standard form.
- **Identity:** `nse_fo|token` on `kotak-nse-nfo`. DualNoBlend INR. Lots, not contracts. No COM ticket.
- **Confirm:** LiveBook intent only. TRADE parked. Same submit / kill-switch / warning path as today.
- **Board:** Cockpit seed only. Plan dock is rail. No Edit board, no tile catalog, no hero/focus, no floor dock.
- **Tiles (prototype, not demo data):**

```
planDock: rail
session  x0 y0 w2 h2
oi       x2 y0 w2 h2
payoff   x0 y2 w2 h2
depth    x2 y2 w2 h2
chain    x0 y4 w4 h1
```

- **Honesty:** rearrange existing hosts. Session stays `kotak_history_unsupported`. OI is venue `open_int` (including `"0"`). At-expiry stays the NFO chain-inherited hole. Chain is catalog, `showsStrikeGrid = false`. Greeks stay chips. No eapi copy on this book.
- **Host:** Station-hosted Notch stays PLAN-only. Expanded panel is full-screen; do not re-stack because the old Notch was a strip.
- **Out of this plan:** crypto mosaic, Live/Post, licensed NFO history, NSE settlement identity, Black-76, new book, money-path edits, pasting the HTML file into Notch.

---

## Phase 1: Kind-gated cockpit shell

**User stories:** 3, 4, 5, 6, 7, 8, 9, 22, 23, 25, 26, 27, 28, 29, 35, 36

### What to build

Selecting an NFO **option** scrip on Pre-trade opens a mosaic board with Plan on a rail. Session occupies its seed tile as the existing licensed-series hole. Other tiles may be empty honest placeholders. Confirm on the rail still declares LiveBook and does not POST TRADE.

Selecting an NFO **future**, or an NFO row with no `instrument_type`, keeps today’s three-zone declare. Crypto Options, leftover pairs, spot, equity, USDM, and Coin-M do not change. Station-hosted Notch still cannot leave PLAN.

Demo: bind OPTIDX → mosaic + rail; bind FUTIDX → three-zone; Confirm still works on both.

### Acceptance criteria

- [x] OPT* on Kotak NFO shows mosaic + Plan rail, not stacked three-zone
- [x] FUT* on Kotak NFO shows three-zone
- [x] Missing / unknown `instrument_type` shows three-zone
- [x] Dated Binance contract still shows crypto Options; leftover pair still standard form
- [x] Spot, equity, USDM, Coin-M Pre-trade unchanged
- [x] Confirm on the rail is the same LiveBook submit; TRADE parked; no COM ticket on NFO
- [x] Station-hosted Notch remains PLAN-only
- [x] No new book, no money-path change

---

## Phase 2: Cockpit seed tiles

**User stories:** 1, 10, 11, 12, 13, 14, 15, 16, 17, 18, 30, 31, 32, 34

### What to build

Fill the cockpit seed: open interest, At-expiry, Depth, and Chain sit in the prototype positions beside Session. Each tile is the existing honest host moved onto the grid — DualNoBlend INR, `nse_fo|token`, no eapi language.

Demo: OPT* cockpit shows five tiles at once; dark chain has no strike grid; `"0"` open interest still paints; payoff and session stay holes.

### Acceptance criteria

- [x] Seed layout matches the prototype positions (session, oi, payoff, depth, chain)
- [x] Session copy still names `kotak_history_unsupported` and does not compose 1m bars
- [x] OI paints venue `open_int` digit for digit; `"0"` is a reading; not `sumOpenInterest`
- [x] At-expiry stays the NFO payoff hole (not eapi index / settlement identity)
- [x] Depth uses the existing ladder; display-false is unavailable, not a demo book
- [x] Chain is the NFO master catalog; `showsStrikeGrid` false for nothing-declared, unavailable, empty, and lit
- [x] Greeks remain chips / provenance; no Δ/Γ/Θ
- [x] Tile notes never mention eapi klines or `sumOpenInterest`
- [x] Existing NFO honesty tests stay green

---

## Phase 3: Plan rail complete

**User stories:** 2, 19, 20, 21, 33

### What to build

The Plan rail holds everything the trader needs to Confirm without the old zone C: mood/setup, lots on the NFO id, legs, premortem, max planned loss, plus the undeclared-position and kill-switch banners. Authored-size override still blocks Confirm. Layout change is not a second declare path.

Demo: add a lot, write premortem, Confirm from the rail; banners still appear above the board.

### Acceptance criteria

- [x] Rail includes state check, setup, lots, stop, target, max planned loss, premortem, Confirm
- [x] Legs add/remove on the rail; leftover pair qty cannot Confirm
- [x] Override above authored size blocks Confirm
- [x] Undeclared-position and kill-switch / submit-blocked banners still show
- [x] Same submit readiness and warning copy as today’s three-zone Plan
- [x] Edit board / Rail vs Floor / hero-focus boards remain unshipped
