# Kotak Neo RMS Limits — `account/funds`

---

## Header Block

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| **Topic**        | Kotak Neo `POST /quick/user/limits` as the cash-book `funds` snapshot |
| **Primary source** | [Kotak-Neo/Kotak-neo-api-v2](https://github.com/Kotak-Neo/Kotak-neo-api-v2) `docs/Limits.md`, `limits_api.py`, `settings.py` `PROD_URL["limits"]` · B6 sheet [`docs/research/sheets/kotak_neo.md`](../../research/sheets/kotak_neo.md) · adjacent [`MARGIN-CALCULATOR.md`](./MARGIN-CALCULATOR.md) |
| **Snapshot date** | 2026-09-07 |
| **Source version** | SDK git tree cited by REST.md / MARGIN-CALCULATOR.md (`8cee5bda63bd9334f8501bb23b7f1d2945f93397`) · Limits.md sample JSON as fetched 2026-09-07 |
| **Staleness warning** | Re-verify `Limits.md` / `limits_api.py` before changing path, method, body keys, or which sample keys are copied. |
| **Author**       | TradeAutopsy Station |

---

## Source Inventory

| Source | URL / Citation | Date accessed | Notes |
| ------ | -------------- | ------------- | ----- |
| SDK `Limits.md` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/docs/Limits.md | 2026-09-07 | Title, wrapper, parameter table, sample JSON keys |
| SDK `limits_api.py` | cited from [`MARGIN-CALCULATOR.md`](./MARGIN-CALCULATOR.md) | 2026-08-27 | **POST**, `Sid`/`Auth`, query `sId`, form body `seg, exch, prod` |
| SDK `PROD_URL` | `settings.py` `"limits": "quick/user/limits"` | 2026-08-27 | Path constant |
| B6 sheet | [`docs/research/sheets/kotak_neo.md`](../../research/sheets/kotak_neo.md) | 2026-08-26 | Currency **INR**; fetch path names `limits` as a word |
| Margin calculator | [`MARGIN-CALCULATOR.md`](./MARGIN-CALCULATOR.md) | 2026-08-27 | Limits is **not** `account/margin_estimate`; do not parse SPAN |

---

## Concepts

---

### Limits HTTP host / path / method / auth

**Source:** `Limits.md`; `limits_api.py`; `PROD_URL["limits"]`; MARGIN-CALCULATOR.md “Adjacent — Limits snapshot”

**Verbatim definition / formula:**

`Limits.md`:

```
Get RMS Limits details of your demat account

client.limits()
```

HTTP (`limits_api.py`, quoted in MARGIN-CALCULATOR.md):

```
POST {get_url_details("limits")}
Query: sId={serverId}
Headers: Sid={edit_sid}, Auth={edit_token}, Content-Type=application/x-www-form-urlencoded
Body: seg, exch, prod
```

`Limits.md` parameter table (defaults):

```
segment  CASH, CUR, FO, ALL (Default value - ALL)
exchange NSE, BSE, ALL (Default value - ALL)
product  CNC, MIS, NRML, ALL (Default value - ALL)
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Path | `quick/user/limits` | path |
| Method | `POST` | HTTP |
| Host | Session `baseUrl` via `get_url_details` | URL |
| `seg` / `exch` / `prod` | Form body keys (`limits_api.py`) | strings |
| Default request | `ALL` / `ALL` / `ALL` (`Limits.md` parameter table) | enum strings |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Whether a cash-book fetch must send `CASH`/`NSE`/`CNC` instead of `ALL`.
- Rate-limit window for this POST.

> **OUR INTERPRETATION**
>
> - Station copies the SDK **default** keys `seg=ALL`, `exch=ALL`, `prod=ALL` for the cash-book funds kick, wrapped as form field `jData` JSON per `neo_api_client/rest.py` (not raw `seg=…&exch=…` urlencoding). That is the documented demat-account snapshot, not a per-scrip calculator.
> - Named NFO book `kotak-nse-nfo` sends `seg=FO&exch=ALL&prod=ALL` (Limits.md `segment` enum **FO**, re-fetched **2026-09-15 IST**). Same copy of `Net`/`MarginUsed`. Do not reuse the cash ALL snapshot as NFO funds.
> - Host fence stays the session Kotak R0 host. This POST is a **private read**, not place/modify/cancel (see `is_mutation` RMS-read exception already in host policy).

---

### Sample keys `Net` and `MarginUsed` — copied, not computed

**Source:** `Limits.md` sample response

**Verbatim definition / formula:**

Sample JSON keys (names and string values only; `Limits.md`):

```
"Net": "19.409999999999997",
"MarginUsed": "18.78",
"CollateralValue": "38.19",
"NotionalCash": "0",
"SpanMarginPrsnt": "0",
"ExposureMarginPrsnt": "0",
"stCode": 200,
"stat": "Ok"
```

No prose in `Limits.md` defines `Net`, `MarginUsed`, `CollateralValue`, or `NotionalCash`.

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `Net` | Sample key only — **definition NOT SPECIFIED IN SOURCE** | numeric string in sample |
| `MarginUsed` | Sample key only — **definition NOT SPECIFIED IN SOURCE** | numeric string in sample |
| Currency of those numbers | **NOT SPECIFIED IN SOURCE** on Limits.md | — |
| Desk currency | B6 row 7: **INR** | INR |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Formula relating `Net`, `CollateralValue`, `NotionalCash`, and `MarginUsed`.
- Whether `Net` can be negative.
- Units (rupees vs paise). B6 says the desk is INR; Limits.md does not name a unit on the sample values.

> **OUR INTERPRETATION**
>
> - `obtain(funds)` on `kotak-nse-bse-cash` **and** `kotak-nse-nfo` is this RMS snapshot, **not** `account/margin_estimate`. SPAN/exposure keys stay unparsed (MARGIN-CALCULATOR.md). NFO is not a PnL engine.
> - One holding row: `asset = "INR"` (B6 currency). `free` copies sample key `Net`. `locked` copies sample key `MarginUsed`. Station does **not** compute `CollateralValue - MarginUsed`.
> - Zero `Net` and zero `MarginUsed` → success with `holdings: []` (same drop rule as Binance `free+locked <= 0`).
> - `unrealized_pnl` stays `null` — `CashUnRlsMtomPrsnt` is a sample key with no definition.

---

## Verification Checklist

- [x] Path/method/body keys match `limits_api.py` as already recorded in MARGIN-CALCULATOR.md
- [x] Sample keys `Net` / `MarginUsed` present on Limits.md sample fetched 2026-09-07
- [ ] Live dogfood: confirm `stat`/`stCode` envelope and that `Net` is the number the RMS UI labels as available funds (definition still unspecified)

---

## Self-Audit

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| Treat `POST /limits` as margin calculator / SPAN | MARGIN-CALCULATOR.md: field *names* are not SPAN mechanics; do not parse into `margin_estimate` | Host policy maps this POST to **funds**, not `margin_estimate`. `check-margin` stays the dark calculator-adjacent POST. |
| Compute `free = CollateralValue - MarginUsed` because the sample almost adds up | Limits.md never states that formula | Copy `Net` and `MarginUsed` only. |
| Invent INR units or paise scale | Limits.md sample is bare numeric strings; B6 says desk currency INR | Label the holding `INR`; do not rescale. |
| Send `seg=nse_cm` because this is the cash book | Parameter table default is ALL | Use documented defaults `ALL`/`ALL`/`ALL`. |
| Send cash `ALL` on the NFO book | Limits.md names `FO` as a `segment` enum (re-fetched **2026-09-15 IST**) | NFO funds body is `seg=FO&exch=ALL&prod=ALL`. Cash stays `ALL`/`ALL`/`ALL`. Do not guess `exch=NSE` / `prod=NRML`. |
| Parse `SpanMarginPrsnt` because NFO uses SPAN | Limits.md sample keys are names, not SPAN mechanics | Copy `Net`/`MarginUsed` only. SPAN stays unparsed on both books. |
