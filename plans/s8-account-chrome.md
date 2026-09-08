# Plan: S8 Notch account chrome (pulse + ledger)

> Source: prototype C locked 2026-09-08 (`station/prototypes/PROTOTYPE-s8-account-chrome.html?variant=C`)

## Architectural decisions

- **Layout:** prototype **C** — pulse (free + counts + Funds pill) over a full-bleed obtain ledger. Not A (drawer) or B (tabs).
- **Book:** shipping book of Start only (`kotak_neo` → `kotak-nse-bse-cash`, `binance_com` → `binance-com-spot`). Never blend NFO/options/USDM onto this strip.
- **Source:** `GET /api/station/obtain` for `funds`, `holdings`, `positionbook`, `orderbook`. AccountBook already caches after kick.
- **Today:** `GET /api/daemon/positions` fill-inventory is a different surface. Do not feed it from this chrome.
- **DualNoBlend:** drop any obtain envelope whose `book_id` is not the shipping book.
- **Pills:** Quote / Instruments / Account stay on the Notch top bar. Pulse+ledger lives on Settings → Broker.
- **Seam:** Notch compose/presentation of obtain JSON. No new agent route.

---

## Phase 1: Pulse + ledger on Settings Broker

**User stories**: account funds visible after Start; holdings/positions from obtain; orders can be unavailable; Today inventory unchanged.

### What to build

After Start, Settings → Broker shows free funds on the shipping book, counts, and obtain lists. Stop clears the strip.

### Acceptance criteria

- [ ] Kotak Start paints Cash · INR free from obtain(funds), not poller green
- [ ] Holdings/positions rows come from obtain lists
- [ ] Orders may read unavailable/unsupported
- [ ] Today/Open fill-inventory is unchanged
- [ ] A USD obtain envelope does not paint on an INR shipping book
