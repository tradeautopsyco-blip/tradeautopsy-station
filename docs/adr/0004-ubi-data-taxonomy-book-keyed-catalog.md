# ADR 0004: UBI taxonomy (`ubi-data@0.3.0`) + book-keyed catalog

**Status:** accepted (P1 law; founder bundle 2026-09-23 closes program decisions
1–4: two-axis taxonomy adopted, bump target `ubi-data@0.3.0`, this number
pre-assigned to the taxonomy ADR, book-keyed catalog explicit in P1).

**Program:** `issues/brokers/FULL-COVERAGE-PROGRAM.md` Phase 1 (unblocks second
books: categories 2, 3, 4, 6, 7). Registry: `issues/compliance/CLAIM-REGISTRY.md`
(6 SHIPPING rows — no flips in P1).

## Context

Three defects block every second book on a live slug:

1. **String class.** `ubi-data@0.2.0` declares `asset-class: string`. Seven legacy
   nouns float through the tree (`crypto_spot`, `equities`, `crypto_options`,
   `crypto_usdm`, `crypto_coinm`, `nfo`, `crypto`) with no closed vocabulary and
   no rejection rule — an unknown value silently becomes whatever the writer
   typed.
2. **Slug-keyed stamp (F3).** The host stamps every fill with the *slug's* first
   pair: an NFO fill inherits `equities` from Kotak cash, a Coin-M fill would
   inherit `crypto_spot`. Until the catalog is book-keyed, second books STOP at
   ticket §0 (program §4.3). The old behavior is locked by
   `host_stamps_equities_on_nfo_row_f3` so the flip is visible, not silent.
3. **No instrument axis.** Asset alone cannot separate spot from future from
   option, and Coin-M margining (`is_inverse`) has no home except a class
   string — which would then be mistaken for an instrument.

A stashed two-axis design existed; 2026-09-23 oracle mining confirmed its enums
match Nautilus exactly (zero gaps), so the program adopts it as law rather than
inventing a dialect.

## Decision

### 1. Closed two-axis enums, host-stamped (Nautilus-aligned)

`asset_class` ∈ {`fx`, `equity`, `commodity`, `debt`, `index`, `cryptocurrency`,
`alternative`} (7); `instrument_class` ∈ {`spot`, `swap`, `future`,
`futures_spread`, `forward`, `cfd`, `bond`, `option`, `option_spread`, `warrant`,
`sports_betting`, `binary_option`} (12). `is_inverse` is a boolean (Coin-M),
never a class string.

- **Unknown value = reject, never default.** `AssetClass::parse` /
  `InstrumentClass::parse` return `None` on anything outside the closed set —
  including every legacy noun. Per-fill unknown =
  `NOT SPECIFIED IN SOURCE` → reject/park.
- **Host stamps class; adapter never classifies.** `run_fetch_fills` overwrites
  connection, slug, and all three axes on every fill from the connection's
  shipping **book** config (`stamp_fill_identity`). Adapter-supplied values are
  ignored the same way spoofed connection ids are.
- **No silent legacy-string mapping.** There is deliberately no
  `crypto_spot → (cryptocurrency, spot)` table anywhere. Pre-bump (0.2.0)
  components are rejected after migration (see §4).

### 2. Contract bump: `tradeautopsy:ubi-data@0.3.0`

`docs/contracts/ubi-data.wit` is now `package tradeautopsy:ubi-data@0.3.0`:
`asset-class` is a closed enum (was `string`), `instrument-class` enum and
`is-inverse: bool` are added to `fill-event`. The stale "not wired yet" header
is fixed: the host loads this world via `wasmtime::component::bindgen!` and the
adapters implement it via `wit_bindgen::generate!` against the same dir.

- **Wire form is snake_case** (`futures_spread`, `binary_option`) at the
  Rust/JSON layer (`#[serde(rename_all = "snake_case")]`). The component model
  carries enum discriminants, so the WIT source uses canonical kebab-case
  (`futures-spread`). The two spellings denote one vocabulary; the
  catalog → WIT `From` maps are exhaustive matches, so a new member fails
  compilation until both sides agree.
- **WIT keyword escapes.** `future` and `option` are WIT keywords and cannot be
  bare enum cases; the WIT spells them `%future` / `%option`. Bindgen strips
  the escape, so Rust/Wire see `Future`/`future` and `Option`/`option`. This is
  standard WIT escaping, not a dialect fork — recorded here so nobody
  "un-escapes" it and breaks the parse.
- **New first book reusing a class = no bump.** The enum sets are the stable
  surface; filling in unused members (e.g. first `commodity` book) needs no
  contract change.

### 3. Book-keyed catalog: one row per SHIPPING book

`catalog_v1()` (slug-keyed first pair) is kept for slug-keyed callers
(`desk.rs`, Swift session payloads — see §6). New `catalog_books()` /
`descriptor_for_book_id()` carry one row per live book; unknown book (including
empty string and slug-looking input) returns `None` — fail closed, never a slug
fallback, or F3 returns through the back door.

| book_id | slug | asset | instrument | is_inverse | manifest_id |
|---|---|---|---|---|---|
| `binance-com-spot` | `binance_com` | cryptocurrency | spot | false | `binance_com.s1.v1` |
| `binance-com-options` | `binance_com` | cryptocurrency | option | false | `binance_com.options.v1` |
| `binance-com-usdm` | `binance_com` | cryptocurrency | future | false | `binance_com.usdm.v1` |
| `binance-com-coinm` | `binance_com` | cryptocurrency | future | true | `binance_com.coinm.v1` |
| `kotak-nse-bse-cash` | `kotak_neo` | equity | spot | false | `kotak_neo.s1k.v1` |
| `kotak-nse-nfo` | `kotak_neo` | equity | option | false | `kotak_neo.nfo.v1` |

Manifest ids are asserted equal to `manifest_for_book_id` in catalog tests, so
catalog/manifest drift fails CI. The Start path (`build_wasm_runtime_adapter_for_book`)
now resolves axes via `descriptor_for_book_id(book_id)` — the `book_id` was
already validated + adapter-match-checked there, so the migration is a lookup
swap, not new trust.

Calc profiles added for the second books (`crypto_options_usd`,
`crypto_usdm_usd`, `crypto_coinm_usd`, `equities_inr_nfo`) are **descriptive**
(quote currency + asset axis) — they are not money owners. NFO/options stay
fills-display-only; USDM/Coin-M keep their income-file owners. Compliance
profiles are reused per slug (withdraw-block is a credential-shape property, not
a book property). Coin-M `quote_currency` is USD (contracts are quoted
`BTCUSD_PERP`); the base-asset margining leg is `is_inverse`, not the quote.

### 4. Pre-bump rejection is loud

A 0.2.0-built component's exports no longer match the 0.3.0 world, so
instantiation fails. The host maps that failure (and any present-but-unloadable
bytes) to `UbiHostError::ContractMismatch`, whose message names the expected
contract (`tradeautopsy:ubi-data@0.3.0`), names the rejected generation
(0.2.0), and points at the rebuild. A *missing* component file stays a plain
`Io(NotFound)` so "adapter not built" never misreports as "pre-bump".

### 5. NFO book stamp: `(equity, option, false)` — with an explicit FUT limitation

The NFO lock (`issues/compliance/locks/kotak-nse-nfo.md`) is unambiguous that
the book is mixed: Skill A item 5 lists `instrument_type` **CE/PE/FUT** from
`pOptionType`/`pInstType`, the fee schedule prices both options (STT 0.15% of
premium) and futures (STT 0.05%), and live dogfood shows both legs
(`NIFTY2692221000PE` and `NIFTY26SEPFUT`). The closed `instrument-class` enum
has no `mixed` member by design (taxonomy describes instruments, Nautilus-style;
a mixed *book* has no single instrument). A per-fill stamp derived from venue
type fields would exceed the P1 "stamp from the connection BOOK" instruction,
and silently picking one leg is forbidden — so this ADR picks explicitly and
records the cost:

- **Stamp:** `(equity, option, false)`. The asset leg is exact (NSE *equity*
  derivatives per the lock). The instrument leg names the book's representative
  instrument: every lock golden/probe centers an index option (qty>1 OPTIONS
  lot fixture on the `PE` row, `ltp`/`open_int` probes on option contracts),
  and the book's Notch surface is the NFO options cockpit.
- **Limitation (explicit, not silent):** FUT fills in this book are stamped
  `option` at book level. That stamp is coarse metadata only — per-fill
  precision lives in `BrokerFill.instrument_type` (CE/PE/FUT from the
  `nse_fo` symbol suffix), which `fill_event_to_broker_fill` derives from
  venue fields untouched by this ADR. No money routes on the stamp today: NFO
  has no realized-PnL owner (fills display only), and the P5 Today partition
  keys on non-spot vs spot (both legs are non-spot), so the limitation cannot
  misroute a rupee.
- **Follow-up NOT decided here:** per-fill host derivation of
  `instrument-class` from venue type fields vs an NFO-OPT / NFO-FUT book split
  is a P5/P6 decision with its own lock evidence. This ADR only forbids the
  third option: silently pretending the book is single-leg.

### 6. Swift/Notch: routing stays the source of truth in P1 (decided, not drifted)

The program asked for Swift `BrokerCatalog` per-book class *or* an explicit
decision that Notch routing stays source of truth. P1 decides the latter:
`station/` and `notch/` are untouched in this slice, slug-keyed desk payloads
(`calc_profile_id` for the first pair) are unchanged, and per-book class
reaches the UI only through existing book-routed surfaces (fences, obtain
nouns, book-keyed Start). A Swift per-book-class projection is future work with
its own slice — not a silent omission.

## Considered options

1. **Per-fill instrument derivation in P1** (host reads CE/PE/FUT from venue
   fields and stamps each fill). Rejected for this slice: it exceeds the
   "stamp from the connection BOOK" instruction and needs its own lock-grade
   mapping table (suffix rule vs `pInstType` enums). Parked as the named P5/P6
   follow-up in §5.
2. **A `mixed` instrument member for NFO.** Rejected: it breaks the
   Nautilus-aligned closed set (zero-gap law) for one book and teaches every
   downstream consumer a member that means "ask someone else".
3. **NFO stamped `(equity, future)`.** Rejected for the representative-instrument
   reason in §5 (lock goldens/probes/cockpit all center options). Either leg
   would need the same explicit limitation; options minimizes the mis-stamped
   share of the book's actual flow.
4. **Silent legacy-string map (`nfo → (equity, option)`, …).** Rejected by P1
   law: silent maps turn contract drift into silent drift. `parse` rejects all
   seven legacy nouns, tested.

## Consequences

- **Wave-2 owns:** rebuilding the three adapter crates against 0.3.0
  (`ubi-binance-com-adapter`, `ubi-kotak-neo-adapter`, `ubi-fixture-adapter`
  all `generate!` against `docs/contracts`, so they fail until migrated);
  migrating `source_manifest.rs` coverage `asset_classes` (still legacy
  strings — the only prod-code exempt list, see below); `ubi_fill_contract.rs`
  + contract-component tests.
- **Tail lane owns:** flipping `host_stamps_equities_on_nfo_row_f3` (expects
  the old `equities` stamp — goes red by design) and the other
  `agent/tests/ubi_*` string asserts; needs `mod.rs` + `lib.rs` re-exports for
  `AssetClass`, `InstrumentClass`, `catalog_books`, `stamp_fill_identity`,
  `WitAssetClass`, `WitInstrumentClass`, `UBI_DATA_CONTRACT_VERSION`
  (P1-A exports only `descriptor_for_book_id` from `mod.rs` — "re-exports only
  if needed" — the rest stay crate-visible via full paths until the
  integrator wires them).
- **Exempt legacy strings** (P1 acceptance "zero legacy strings outside exempt
  list"): `agent/src/data/source_manifest.rs` coverage vocabulary (wave-2);
  `agent/ubi-*-adapter/**` + `agent/tests/**` + `agent/fixtures/**` (wave-2 /
  tail); `agent/wit/ubi.wit` v0.1.0 (legacy world, intentionally untouched —
  the 0.1.0 `fetch_fills` world is not the data contract and is out of P1
  scope); the API `"crypto"` alias in Start-request test fixtures (decided:
  exempt alias, not a class — behavioral payload, not a taxonomy); and
  `calc_profile_id` values (`crypto_spot_usd`, … — decided: exempt profile
  identifiers, not class nouns; renaming them would break the Swift session
  payload contract, which is frozen in P1).
- **F3 STOP lifted for stamps, not for books.** Second-book tickets may now
  pass program ticket §0 *on the stamp question*; every other §0 input (B6,
  lock, registry, ADR-lite) still gates per book.
- **Integrator verifies:** `git status` shows P1-A owner files only; full lib
  suite green (896 on this lane); `-D unused` deny-gate delta is zero new
  errors in `ubi/` + `broker_sync_control.rs` (the gate is red on untouched
  files — pre-existing, not this lane); no `if book ==` branches added; no
  fixture/component/adapter file touched.

## Appendix A: P1 legacy-string exempt list (tail lane, 2026-09-23)

P1 acceptance is "zero legacy strings outside this list". Probe:

`rg -n "crypto_spot|\"equities\"|crypto_options|crypto_usdm|crypto_coinm|\"nfo\"|\"crypto\"" agent/src agent/tests station/StationApp`

After the tail sweep the probe returns exactly the 52 lines below — every one
exempt with a reason. Everything else (the 5 `ubi_*` test targets' configs +
asserts, the F3 flip, the `desk.rs` test name) was migrated to the closed
enums. Line numbers are as of the tail-lane tree; any future move must carry
its reason with it.

### A. Coverage vocabulary — `agent/src/data/source_manifest.rs` (wave-2 owns, 15)

`Coverage.asset_classes` is still a free-string vec; wave-2 migrates it to the
closed enums. Tail lane does not touch prod coverage semantics.

- `:433`, `:439` — `binance_com.s1.v1` coverage + history coverage `crypto_spot`
- `:506`, `:512` — `kotak_neo.s1k.v1` coverage + history coverage `equities`
- `:592` — `kotak_neo.nfo.v1` coverage `nfo`
- `:667`, `:673` — `binance_com.options.v1` coverage + history coverage `crypto_options`
- `:745`, `:751` — `binance_com.usdm.v1` coverage + history coverage `crypto_usdm`
- `:807`, `:813` — `binance_com.coinm.v1` coverage + history coverage `crypto_coinm`
- `:1402`, `:1680`, `:1752`, `:1753` — tests asserting the above coverage values
  (migrate with the vocabulary in wave-2, not before)

Decision recorded: `agent/src/data/amfi.rs:126` `mutual_fund_nav` is NOT one of
the 7 legacy nouns — it is AMFI-NAV coverage vocabulary (same wave-2 family,
mutual funds are outside the 6-book catalog and outside P1 scope). Exempt.

### B. API `"crypto"` alias — Start-request / behavioral payloads (6)

`BrokerSyncStartRequest.asset_class` (`assetClass` on the wire) and
`BrokerConnectionIdentityFields.asset_class` are free-form behavioral strings,
not the taxonomy. `"crypto"` here is the decided-exempt Start alias (P1 law),
not a class noun. No enum layer exists on these payloads in P1.

- `agent/src/broker_sync_control.rs:457` — `identity_request()` test fixture
- `agent/tests/broker_sync_control.rs:283` — Start JSON body (B2 wire-secret test)
- `agent/tests/today_api.rs:83` — Start JSON body (Today harness)
- `agent/tests/common/mod.rs:388` — shared `identity_start_body()` helper
- `agent/tests/backend_box_release_gates.rs:44` — behavioral identity fixture
- `agent/tests/broker_redaction_behavioral.rs:12` — behavioral identity fixture

### C. Segment vocabulary, not class taxonomy (2)

`exchange_segment` / `segment` is the venue-segment domain (`options`, `usdm`,
`spot`, `nse_fo`, …) — never the asset/instrument taxonomy. Both hits are
negative guards proving an options row is not an NFO row.

- `agent/src/binance_com_options_client.rs:399`
- `agent/tests/options_funds_positions_obtain.rs:244`

### D. `calc_profile_id` values — frozen session-payload contract (13)

Profile identifiers (`crypto_spot_usd`, …), not class nouns. Renaming any of
them would break the Swift session payload contract, which is frozen in P1
(§6). Exempt on both sides of the wire.

- `agent/src/ubi/catalog.rs:194`, `:207`, `:212`, `:217` — profile table keys
- `agent/src/ubi/catalog.rs:265`, `:317`, `:332`, `:347`, `:362` — book-row ids
- `agent/src/ubi/catalog.rs:502`, `agent/src/ubi/desk.rs:53` — desk asserts
- `station/StationApp/Models/Broker/BrokerCatalog.swift:73` — Swift mirror id
- `station/StationApp/Models/DeskMoneyFormatting.swift:65` — Swift fallback id

### E. Rejection proof + law text — must keep (9)

These ARE the P1 proof: the no-mapping law, the test that every legacy noun
rejects, and the fail-closed lookup test. Deleting them would delete the gate.

- `agent/src/ubi/catalog.rs:66` — `parse` doc comment (no-mapping law text)
- `agent/src/ubi/catalog.rs:539`–`:545` — `legacy_strings_are_rejected_never_mapped`
- `agent/src/ubi/catalog.rs:646` — `descriptor_for_book_id("crypto_spot")` is `None`

### F. Swift per-book-class table — P1-SWIFT-FOLLOWUP owns (7)

Per tail-lane law the Swift side keeps its string table in P1; no Swift enum
layer is invented here, and changing these values without the agent-side
contract change would silently fork the Start/session payload. Follow-up owns
the Swift projection with its own slice.

- `station/StationApp/Models/Broker/BrokerCatalog.swift:70`, `:82` — v1 table
- `station/StationApp/Broker/Services/BrokerConnectServices.swift:15` — unknown-slug fallback
- `station/StationApp/Broker/Services/BrokerEnvironmentController.swift:57` — identity default
- `station/StationApp/Broker/Services/BrokerEnvironmentController.swift:87` — binanceUS identity alias
- `station/StationApp/Broker/Services/BrokerEnvironmentController.swift:100`, `:115` — binanceCom/kotakNeo identities

(`station/StationApp/Session/SessionModel.swift` and
`station/StationApp/Models/DeskMoneyFormatting.swift:67` carry no probe match —
the former only passes `calcProfileId` through, the latter's `equities_inr_cash`
is a profile id that does not match the 7-noun probe.)
