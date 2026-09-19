# PROTOTYPE — throwaway

**Question:** After Confirm, what should **Live then Post** look like when chrome is today’s Notch (escrow · capture · Moments A–C), the pending row is the venue ticket, DualNoBlend, TRADE parked — and the board uses TradingView card language?

Three Live/Post layouts via `flow=A|B|C`. Ticket chrome still `ticket=A|B|C`. Plan dock stays. Confirm is LiveBook intent, never a venue POST.

**Verdict (2026-09-18, founder):** Planning stays on the cockpit. Risk after Confirm is the right split. Then: also show **what if take profit hits**. That risk-after-Confirm flow stays on Pre-trade Confirm, not on Live.

## Tickets (plan)

- **A Venue stack** / **B Compact dock** / **C Ticket tile** — unchanged.
- Equity stays Kotak cash.

## Live / Post (`?flow=`)

- **A Notch Live chrome** — PLAN INTACT, Declare-before-trade empty state, Live capture, Unposted (this Mac), Link needs `book_id`, seven escrow slots (declared vs actual), Kill. Post = Moment A / B / C.
- **B TV detect mosaic** — TradingView list rows for Detect + escrow rail.
- **C Book ledger** — named-book positionbook + capture.

Escrow join `{book_id}\\0{symbol}`. Stop actual is plan vs not-placed, never implied STOP_MARKET. Exit cancels intent; flatten MutationForbidden. Empty realized is none. Flat Cross/x/liq is none.

**Refuse:** PCR, max pain, OpenAlgo, Black-76, venue TP/SL POST, auto-place SL on USDM/Coin-M, TRADE POST, mixing spot CATIUSDT with USDM CATIUSDT.

Now on this prototype: Options holds **Crypto** (`binance-com-options`) and **NSE F&O** (`kotak-nse-nfo`, futures + options). NSE **options** uses the same mosaic + Plan dock as crypto options (Session / OI / Payoff / Depth / Chain). NSE **futures** stays three-zone. Cost / Max / Initial Margin as live preview numbers on COM tickets. Cross/50x on crypto Options when positionbook is open (none while flat). Plan dock mood / setup / Confirm stays on Live.

## Run

```bash
cd /Users/bishnu/tradeautopsy-station/station/prototypes && ./serve-prototype.sh
```

- Empty Live: `http://127.0.0.1:8766/PROTOTYPE-notch-options-dashboard.html?asset=usdm&board=cockpit&ticket=A&flow=A&phase=live`
- After Confirm, Close this book’s positionbook → Post.
- swap `asset=` `spot` | `equity` | `options` | `usdm` | `coinm`
- Crypto Options: `?asset=options&desk=crypto&board=cockpit&phase=plan`
- NSE F&O options: `?asset=options&desk=nfo&kind=opt&board=cockpit&phase=plan`
- NSE F&O futures: `?asset=options&desk=nfo&kind=fut&board=cockpit&phase=plan`
- Old `?asset=nfo` remaps to Options → NSE F&O
- swap `flow=` `A` | `B` | `C` (blue pill / Shift ← → on Live/Post)
