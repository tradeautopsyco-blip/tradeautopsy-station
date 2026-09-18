# Kotak founder dogfood — 18 Sep 2026 (IST)

Loopback agent from **TradeAutopsy Station.app** (Xcode Debug). Wire v1 signed probes. NSE session **open** (~15:08 IST).

**Agent:** `tradeautopsy-agent/0.1.0 (6909f9377c16)` · boot_id `01M2SXY2N2X6JF9WXVM9SADMD7`

---

## TOTP remint (S6-FO)

**Not required this run.** Kotak session is live:

| Signal | Value |
|--------|--------|
| `syncState` | `synced` |
| `runtimeStatus` | `syncing` |
| `lastError` | null |
| `requiresManualRetry` | false |
| `dataClasses.balances_holdings` | current |
| `dataClasses.open_orders` | current |
| `dataClasses.fills_trade_history` | current |

Holdings and orders succeed over obtain. Failure is isolated to **limits (funds)** and **positions**, not a dead TOTP session.

If limits stay dark after a code fix, then try **Backend Box → Kotak → Refresh session** (fresh TOTP + MPIN) and re-run the curl block below.

---

## S6 obtain — cash book `kotak-nse-bse-cash`

| Operation | Result | Notes |
|-----------|--------|--------|
| **holdings** | **PASS** `success` | Live rows (e.g. IDBI, IDFCFIRSTB, PNB, …), `segment=nse_cm` |
| **orderbook** | **PASS** `success` | `order_count=0`, empty rows OK |
| **funds** | **FAIL** `unavailable` | `data=null` after kick + retry; capability `funds: unavailable` |
| **positionbook** | **FAIL** `unavailable` | Use `positionbook`, not `positions` (`positions` → `unsupported`) |

**NFO book**

| Operation | Result |
|-----------|--------|
| **orderbook** | **PASS** `success`, 0 rows, no cash segments |
| **funds** | **FAIL** `unavailable` (expected until NFO limits path is wired; not empty success) |

**S6-FO verdict:** **partial** — 2/4 cash obtains signed. Blocker is **limits + positions API path**, not remint.

---

## S3 — Kotak depth (weekday ladder)

| Probe | Result |
|-------|--------|
| Cash `nse_cm\|2885` (RELIANCE) `/api/station/depth` | **PASS** `success`, 5 bid / 5 ask levels |
| NFO `nse_fo\|61466` `/api/station/depth` | **PASS** `success`, 5 bid / 5 ask levels |

Not run here (Binance-only S3 items): COM gap → Unusable, 5000-level glance rebuild.

**S3-FO Kotak slice:** **signed** for cash + NFO depth in session.

---

## S5-FO (optional)

Skipped — eapi greeks glance is Binance Options, not Kotak.

---

## Health / vendors (sanity)

- `licensed_history`: unsupported (no key / disabled)
- `amfi`: up

---

## Re-run probes (same machine, while Station is running)

Export secret from the running agent (changes each launch):

```bash
export AGENT_DAEMON_SECRET="$(ps eww -p "$(pgrep -f 'TradeAutopsy Station.app.*tradeautopsy-agent' | head -1)" | tr ' ' '\n' | sed -n 's/^AGENT_DAEMON_SECRET=//p')"
```

Then use the wire-signed Python block in session notes or Station’s own Brokers UI after remint.

---

## Engineering fix (2026-09-18, post-dogfood)

Aligned with [Kotak-neo-api-v2](https://github.com/Kotak-Neo/Kotak-neo-api-v2) `rest.py` + `limits_api.py`:

1. **`funds`** — POST `/quick/user/limits` must send `application/x-www-form-urlencoded` field **`jData`** containing JSON `{"seg","exch","prod"}`, not raw `seg=ALL&exch=…`. Raw bodies returned **HTTP 500** on `e21.kotaksecurities.com`; jData returns **200** + `Net` / `MarginUsed`.
2. **`positionbook`** — HTTP 200 + `stCode` **5203** / `errMsg` **No Data** is an empty positions snapshot (same rule as trade book), not a venue error.

**Re-verify:** rebuild Station (Xcode Debug) so the agent picks up the fix, then re-run the obtain probes below. Expect `funds` **success** and `positionbook` **success** (empty rows OK on a CNC-only book).

## Recommended next step

Re-run this checklist after rebuild; flip S6-FO to **signed** when live obtain matches the live probe (`limits` stat Ok, `positionbook` empty success).
