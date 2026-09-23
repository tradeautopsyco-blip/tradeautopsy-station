# Citation pin — OpenAlgo (Indian brokers oracle)

**Verified:** 2026-09-23 IST
**Local tree:** `/Users/bishnu/openalgo-mac/openalgo` (has `.git`)
**Remote:** https://github.com/marketcalls/openalgo
**SHA:** `ad3cd54df476b330c4e4b01a31a3ad53deb9012b` (verified via `git rev-parse HEAD`; commit date 2026-09-19, "chore: auto-build frontend dist [skip ci]")
**Licence:** AGPL (`License.md`: GNU Affero General Public License v3) — cite, do **not** vendor into Station, do **not** depend on, do **not** copy code blocks. AGPL is refused project-wide (`CORRECTNESS.md`, every lock); not legal advice — the rule is architectural anyway (ADR 0001: adapters are Wasm components on the host `broker-http-call` import; OpenAlgo is a Python server that places orders).

Station owns the wire. This tree is the **login-shape / field-map / vocab oracle** for
Indian-broker books. Every signal below is a *lead* ("OpenAlgo `broker/<x>/api/auth_api.py`
shows a checksum login — verify at <official URL>"), never evidence. A B6 row with only
an oracle citation stays `TBD`.

## Worktree-dirty warning

The local worktree has ~20 locally-modified files **not** at the pin, including
`docs/prompt/crypto-symbol-format.md` and `docs/prompt/order-constants.md`
(`git status --porcelain`, 2026-09-23). Citations from dirty files below were taken at
HEAD via `git show HEAD:<path>` and are marked `@HEAD`. Re-verify against the pin before
any wave cites them.

## Login shapes (leads only — each B6 proves the real shape at official docs)

`authenticate_broker` signatures at pin (file: `broker/<name>/api/auth_api.py`):

| Family | Brokers (signal) | Signature shape | Session host seen in tree |
|--------|------------------|-----------------|---------------------------|
| Redirect → request token → SHA-256 checksum → daily access token | `zerodha` → `zerodha_kite` (Wave 1) | `authenticate_broker(request_token)`; `checksum = sha256(api_key + request_token + api_secret)`; `POST https://api.kite.trade/session/token` → `data.access_token` (`broker/zerodha/api/auth_api.py:8-42`) | `api.kite.trade` |
| Redirect (OAuth-shaped) → code → token | `upstox` (`authenticate_broker(code)`), `fyers` (`authenticate_broker(request_token)`), `dhan` (`authenticate_broker(code)`), `groww` (`authenticate_broker(code)`), `shoonya` (`authenticate_broker(code)`), `rmoney` (`authenticate_broker(request_token)`), `fivepaisaxts` (`authenticate_broker(request_token)`) | code/request-token exchange per broker | `api.upstox.com`, `api-t1.fyers.in`, `auth.dhan.co`, `api.groww.in` |
| TOTP + PIN/MPIN session | `kotak` (2-step: `{"mobileNumber","ucc","totp"}` → view token+sid, then `{"mpin"}` → trading token+sid; `authenticate_broker(mobile_number, totp, mpin)`), `angel` (`authenticate_broker(clientcode, broker_pin, totp_code)`), `motilal` (`authenticate_broker(userid, broker_pin, totp_code, date_of_birth)`) | session mint, dynamic `base_url` in Kotak (`…:::base_url:::…` token quad) | Kotak: `mis.kotaksecurities.com` (login/validate), `cis.kotaksecurities.com` (API base, per docstring); Angel: `apiconnect.angelone.in` |
| OTP + secret (unclassified — W0.4) | `definedge` (`authenticate_broker(otp_token, otp, api_secret=None)`) | place in Wave 2/3/Park per appendix signals | (read at W0.4) |

Kotak detail (W0.6 lead): `POST https://mis.kotaksecurities.com/login/1.0/tradeApiLogin`
then `…/tradeApiValidate`; `base_url` travels inside the session token and prefixes all
later calls (e.g. `{base_url}/quick/user/check-margin`, `broker/kotak/api/margin_api.py:21-51`).

## Hosts seen in tree (leads for B6 rows 10/22 — verify at official docs)

| Broker | Hosts |
|--------|-------|
| `zerodha` | `api.kite.trade` (`/session/token`, `/orders/regular…`, quote endpoints in `api/data.py:90`) |
| `kotak` | `mis.kotaksecurities.com`, `cis.kotaksecurities.com` |
| `angel` | `apiconnect.angelone.in` |
| `upstox` | `api.upstox.com` |
| `fyers` | `api-t1.fyers.in` |
| `dhan` | `auth.dhan.co` |
| `groww` | `api.groww.in` |

(One row per broker at wave time; the table above covers only brokers whose auth hosts
were read for this pin.)

## Field maps (leads — clean-room rewrite from cited behavior)

Per-broker maps live in `broker/<name>/mapping/`: `transform_data.py` (order-shape
inbound), `order_data.py` (book/position/holdings outbound), `margin_data.py`
(funds), plus `gtt_data.py` where the broker has GTT (refused — see below).

Sample — Zerodha `mapping/transform_data.py:7-80` (field vocabulary lead):

- inbound: `symbol` → `tradingsymbol` (via `get_br_symbol`), `exchange`, `action` → `transaction_type` (upper), `pricetype` → `order_type`, `quantity`, `product`, `price`/`trigger_price`/`disclosed_quantity` (default `"0"`), `validity: "DAY"`, `tag: "openalgo"`.
- `map_order_type`: `MARKET/LIMIT/SL/SL-M`.
- `map_product_type` / `reverse_map_product_type`: `CNC / NRML / MIS` (default `MIS` inbound — Station must **not** inherit silent defaults; B6 names the product explicitly).
- `order_data.py:307-315`: holdings forced to `CNC` ("Holdings sit in the demat account") — a Station B6 must decide this itself per book; the oracle's choice is not a fact.

Data-shape leads (read paths, citable as questions): `broker/<name>/api/data.py`
(`get_api_response`, quote/depth favours), `broker/<name>/api/funds.py`
(`get_margin_data`), Kotak `get_order_book` / `get_trade_book` / `get_positions` /
`get_holdings` (`broker/kotak/api/order_api.py:55-163` — the *read* functions in an
otherwise refused file).

## Vocab as BAR (check the B6 against it — never a source)

`supported_exchanges` from `broker/<name>/plugin.json` at pin (sample):

| Broker | `supported_exchanges` |
|--------|-----------------------|
| `zerodha` | NSE, BSE, NFO, BFO, CDS, MCX, NCO, NSE_INDEX, BSE_INDEX, MCX_INDEX, GLOBAL_INDEX |
| `kotak` | NSE, BSE, NFO, BFO, CDS, MCX, NSE_INDEX, BSE_INDEX |
| `angel` | NSE, BSE, NFO, BFO, CDS, MCX, NSE_INDEX, BSE_INDEX, MCX_INDEX |
| `upstox` | NSE, BSE, NFO, BFO, CDS, BCD, MCX, NSE_INDEX, BSE_INDEX, GLOBAL_INDEX |
| `dhan` | NSE, BSE, NFO, BFO, CDS, BCD, MCX, NSE_INDEX, BSE_INDEX |
| `groww` | NSE, BSE, NFO, BFO, NSE_INDEX, BSE_INDEX |
| `shoonya` | NSE, BSE, NFO, BFO, CDS, MCX, NSE_INDEX, BSE_INDEX |
| `fivepaisaxts` | NSE, BSE, NFO, BFO, NSE_INDEX, BSE_INDEX |
| `binance` / `deltaexchange` | `n` (single-letter crypto code — Station N-A / Park; never a Station exchange) |

Station posture (unchanged): first India book = NSE/BSE cash (CNC+MIS),
`equities_inr_cash`; NFO/BFO/MCX later books behind their own locks; CDS N-A.
`NCO`/`BCD`/`GLOBAL_INDEX`/index pseudo-exchanges in the oracle are bars to ask about,
not segments to adopt. Products: `CNC / MIS / NRML` (+ `CO/BO` intraday variants where
the broker shows them — refused on a cash first book).

## Hard refuse in this tree (never call, never port, never adopt)

| Refused capability | Oracle location (at pin) |
|--------------------|--------------------------|
| Place / modify / cancel orders | `broker/<name>/api/order_api.py`: `place_order_api`, `modify_order`, `cancel_order`, `cancel_all_orders_api` (e.g. Zerodha `:162/:402/:358/:459`; Kotak `:237/:478/:441/:538`); `POST https://api.kite.trade/orders/regular` and siblings |
| Position exit / close-all | `close_all_positions`, `get_open_position` order-closing paths (Kotak `order_api.py:213-383` region) |
| GTT | `broker/<name>/api/gtt_api.py`, `broker/<name>/mapping/gtt_data.py` (`transform_place_gtt`, `transform_modify_gtt`, `map_gtt_book`) |
| Basket orders | `services/basket_order_service.py` (+ flow/action-center basket executors) — a second control plane Station refuses |
| Margin-trade / leverage flows | `broker/<name>/api/margin_api.py` order-taking paths (read-only `check-margin` shape is a lead; any margin order placement is refused) |
| `exchange: CRYPTO` canonical forms | `@HEAD` `docs/prompt/crypto-symbol-format.md`: single `CRYPTO` exchange code, `brexchange` venue tag, `BTCUSDFUT` / `BTC28FEB25FUT` dated renderers reusing Indian F&O syntax. Station: never OpenAlgo `exchange: CRYPTO`; crypto books cite venue-native symbology only |
| `opengreeks` | `services/option_greeks_service.py`, `services/gamma_density_service.py`, `services/iv_chart_service.py`, `requirements-nginx.txt`, greeks tests — Station has no options-analytics service; refused |
| Credential storage | `database/auth_db.py` (`Auth`, `ApiKeys`, `ActiveSession`, `LoginAttempt`, Fernet session cache), `.sample.env` (`BROKER_API_KEY/SECRET`, `…_MARKET` pairs), `os.getenv("BROKER_API_*")` reads in every `auth_api.py`. Station credentials live in Keychain behind tagged blobs; never env files, never a session DB |
| Servers / second control planes | `services/order_update_service.py`, schedulers, websockets, MCP/UI, blueprints (`brlogin.py` etc.) — Station has one agent loop and the Swift shell |
| Copied code blocks | any — clean-room rewrite from cited behavior only |

## AGPL refuse (restated)

OpenAlgo is AGPL (`License.md`). Nothing from this tree is linked, vendored,
path-depended, or copied into Station or Console. It is already runtime `N-A` in
`CLAIM-REGISTRY.md`. This doc cites file + pin only.

## Realign pointer

W0.6 audits live `kotak_neo` (session mint, `baseUrl`, trade-book fields,
product/segment vocab) against `broker/kotak` at this pin. Hosts/paths matching there
does not imply trade-book shape matches — same rule as the Nautilus cage.
