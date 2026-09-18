# Kotak Neo Trade API — Session Mint and Day Trade Book

---

## Header Block

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| **Topic**        | Kotak Neo TOTP session mint + day trade book (`/quick/user/trades`) |
| **Primary source** | [Kotak-Neo/Kotak-neo-api-v2](https://github.com/Kotak-Neo/Kotak-neo-api-v2) Python SDK (v2) |
| **Snapshot date** | 2026-07-26 |
| **Source version** | Package v2.0.0 / git `main` as of snapshot date |
| **Staleness warning** | Re-verify against the SDK / Kotak Trade API guide before changing paths or headers. |
| **Author**       | TradeAutopsy Station |

---

## Source Inventory

| Source | URL / Citation | Date accessed | Notes |
| ------ | -------------- | ------------- | ----- |
| SDK README | https://raw.githubusercontent.com/Kotak-Neo/Kotak-neo-api-v2/main/README.md | 2026-07-26 | TOTP login + validate + trade_report usage |
| `totp_api.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/api/totp_api.py | 2026-07-26 | Login/validate HTTP |
| `trade_report_api.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/api/trade_report_api.py | 2026-07-26 | Auth/Sid + `sId` query |
| `settings.py` `PROD_URL` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/settings.py | 2026-07-26 | Path constants |
| `urls.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/urls.py | 2026-07-26 | `BASE_URL = https://mis.kotaksecurities.com` |
| `neo_utility.py` | https://github.com/Kotak-Neo/Kotak-neo-api-v2/blob/main/neo_api_client/neo_utility.py | 2026-07-26 | `get_domain(session_init)`, `neotradeapi` fin key |
| Totp_login.md / Totp_validate.md / Trade_report.md | SDK `docs/` | 2026-07-26 | Sample JSON shapes |
| B6 sheet | `docs/research/sheets/kotak_neo.md` | 2026-07-24 · market-data amendment 2026-08-26 | Product refuse list, day-book scope; quotes + scrip master |
| Quotes + scrip master | `docs/reference/india/kotak-neo/REST.md` · `WEBSOCKET.md` | 2026-08-26 | File-paths + REST quotes + subscribe; no history |

---

## Concepts

---

### Login host and neo-fin-key

**Source:** `neo_api_client/urls.py` (`BASE_URL`); `neo_utility.get_domain(session_init=True)`; `get_neo_fin_key()`

**Verbatim definition / formula:**

```
BASE_URL = "https://mis.kotaksecurities.com"
# get_domain(session_init=True) → BASE_URL for prod
# get_neo_fin_key() prod default → "neotradeapi"
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| Login domain | `https://mis.kotaksecurities.com` when `session_init=True` | URL |
| `neo-fin-key` | Default prod `"neotradeapi"` | string header |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Whether consumer key must include a `Bearer ` prefix — SDK passes `configuration.consumer_key` as `Authorization` verbatim.

> **OUR INTERPRETATION**
>
> - Station/agent use `https://mis.kotaksecurities.com` for mint only.
> - Trading calls use the `baseUrl` returned from validate (credential-carried), not a hard-coded CIS host.
> - Pass `Authorization` exactly as the user-supplied consumer key (no Bearer rewrite unless live dogfood proves otherwise).

---

### TOTP login (`tradeApiLogin`)

**Source:** `totp_api.py` `totp_login`; `PROD_URL["totp_login"] = "login/1.0/tradeApiLogin"`; `docs/Totp_login.md`

**Verbatim definition / formula:**

```
POST {mis}/login/1.0/tradeApiLogin
Headers: Authorization={consumer_key}, neo-fin-key={neotradeapi}, Content-Type=application/json
Body: { "mobileNumber": "...", "ucc": "...", "totp": "..." }
Success (2xx): data.token (view), data.sid
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `mobileNumber` | Registered mobile with country code (e.g. `+919999996708`) | string |
| `ucc` | Unique client code | string |
| `totp` | Authenticator TOTP | string |
| `data.token` | View token | string |
| `data.sid` | Session id after login | string |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Exact error body schema for bad TOTP — treat non-2xx / missing `data.token` as `totp_failed`.

> **OUR INTERPRETATION**
>
> - View token is ephemeral intermediate state; never Keychain-persisted.
> - MPIN/TOTP never stored.

---

### TOTP validate / trade token (`tradeApiValidate`)

**Source:** `totp_api.py` `totp_validate`; `PROD_URL["totp_validate"] = "login/1.0/tradeApiValidate"`; `docs/Totp_validate.md`

**Verbatim definition / formula:**

```
POST {mis}/login/1.0/tradeApiValidate
Headers: Authorization={consumer_key}, sid={view_sid}, Auth={view_token}, neo-fin-key={neotradeapi}
Body: { "mpin": "..." }
Success (2xx): data.token (trade), data.sid, data.rid, data.hsServerId, data.dataCenter, data.baseUrl
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `mpin` | Mobile PIN | string |
| `data.token` | Trade token → SDK `edit_token` → HTTP `Auth` | string |
| `data.sid` | Trade sid → SDK `edit_sid` → HTTP `Sid` | string |
| `data.hsServerId` | → SDK `serverId` → query `sId` | string (may be empty in SDK samples) |
| `data.baseUrl` | Trading API base after login | URL string |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Exact `baseUrl` path shape (`/trading` suffix or not) — accept whatever validate returns; host strips scheme and applies path prefix.
- Session TTL — B6 treats as end-of-day IST; JWT may be shorter. Honor `stCode` 1003.

> **OUR INTERPRETATION**
>
> - Keychain blob stores: `consumerKey`, `tradeToken`, `sid`, `baseUrl`, `hsServerId`, optional `expiresAt`.
> - Never store MPIN/TOTP/view token.

---

### Day trade book

**Source:** `trade_report_api.py`; `PROD_URL["trade_report"] = "quick/user/trades"`; `docs/Trade_report.md` sample

**Verbatim definition / formula:**

```
GET {baseUrl}/quick/user/trades?sId={hsServerId}
Headers: Sid={edit_sid}, Auth={edit_token}, accept=application/json
```

Sample row fields from Trade_report.md: `exSeg`, `prod`, `trdSym`, `trnsTp`, `fldQty`, `avgPrc`, `exTm` (`22-Jan-2025 14:28:01`), `flDt` (`22-Jan-2025`), `flTm` (`14:28:16`), `flId`, `exOrdId`, `nOrdNo`, `chrgs` (not always present).

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| `sId` | Query param = `hsServerId` | string |
| `exTm` | Exchange time string in sample | `dd-Mon-yyyy HH:mm:ss` (month name) |
| `stCode` 1003 | Dead session (B6 + host classify) | int in JSON body |
| `stCode` 5203 | Official empty book (`errMsg` "No Data") | int in JSON body |

**Gaps (NOT SPECIFIED IN SOURCE):**

- Whether all gateways accept numeric-month `dd-MM-yyyy` as well as month-name — adapter must accept SDK sample format first.

> **OUR INTERPRETATION**
>
> - v1 ingest: segments `nse_cm`/`bse_cm`, products `CNC`/`MIS` only (B6 refuse NRML/F&O).
> - Empty filtered day book is success. Official v3.0.6 `docs/functions/README.md` "No Data Response" is HTTP 200 + `stat: Not_Ok` + `stCode: 5203` + `errMsg: No Data` — treat as zero fills, never as a dead session. `stCode` 400/403/1003 stay errors.
> - Timestamps treated as IST (+05:30) with no zone marker.

---

## Verification Checklist

- [ ] Live `tradeApiLogin` / `tradeApiValidate` against founder account
- [ ] Live `baseUrl` + `hsServerId` shape matches blob fields
- [ ] `GET …/quick/user/trades?sId=…` returns 200 with Auth/Sid
- [ ] Month-name `exTm` parses to correct UTC ms

---

## Self-Audit

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| Default trading host `cis.kotaksecurities.com` | SDK login host is `mis`; trading host comes from `data.baseUrl` | Mint writes `baseUrl` from validate; no hard-coded CIS for live |
| Omit `sId` | `TradeReportAPI` always sends `query_params = {"sId": serverId}` | Host attaches `sId` |
| Numeric-only date parse | Sample `exTm` is `22-Jan-2025 14:28:01` | Parse month-name format |
| Store MPIN for reconnect | SDK requires fresh TOTP+MPIN each session | Never persist MPIN/TOTP |

No memory fills for paths/headers — taken from SDK source cited above.
