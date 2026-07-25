# TradeAutopsy Station — Ubiquitous language

Native macOS app: Kill Switch, Notch, and the Universal Broker Interface.
Decisions live in [docs/adr/](./docs/adr/).

## Broker adapters (Universal Broker Interface)

**Universal Broker Interface (UBI)**:
Station-native system letting TradeAutopsy and third parties (including AI
authors) supply broker adapters, each a sandboxed Wasm component behind one
WIT contract. Replaces the broker portion of "Backend Box." See [ADR
0001](./docs/adr/0001-uniform-wasm-sandboxed-broker-adapters.md).
_Avoid:_ Backend Box (broker section only — non-broker Backend Box sections,
e.g. Market Data Keys, are unaffected by this rename), web UBI (Console's old
registry — removed)

**Broker Adapter (Adapter Component)**:
A Wasm component — first-party or community/AI-authored, no execution
distinction — that exports `fetch_fills(cursor) -> FillEvent[]` and imports
`broker_http_call`. Never holds a raw credential.
_Avoid:_ connector (Console's legacy term for its removed TS classes),
"native adapter" (no such tier exists post-ADR-0001)

**WIT Interface (Adapter Contract)**:
The Rust Enforcer-defined component-model interface every adapter must
type-check against to be loadable at all — the mechanical, non-human gate
(distinct from the B6 human gate below).
_Avoid:_ "the adapter spec" alone (ambiguous with B6, which is a different kind of spec)

**FillEvent**:
The single normalized output shape every adapter component produces via
`fetch_fills`: `fill_id, broker_slug, connection_id, asset_class, symbol,
side, qty, price, currency, fee_amount, fee_currency, filled_at`, optional
`exchange_segment`, `product`, `trade_id`. The literal WIT export type, not
just a Rust struct.
_Avoid:_ `BrokerFill` (pre-rebuild crypto-only shape, being replaced)

**Host-mediated broker call**:
The only way a component reaches a real broker: it calls the Enforcer's
`broker_http_call(request_shape) -> response_shape` import; the Enforcer
looks up the Keychain credential, attaches auth (HMAC for Binance, session
token for Kotak), makes the real HTTP call, and returns the JSON body. The
component never sees the raw credential.
_Avoid:_ direct HTTP from adapter code (structurally impossible by design,
not just discouraged)

**BrokerDescriptor**:
Catalog entry for a broker: `slug, displayName, assetClass, quoteCurrency,
authScheme, calcProfileId, complianceProfileId, availability, origin`.
_Avoid:_ conflating with the WIT interface — this is catalog/UI metadata, the
WIT interface is the execution contract

**Origin (adapter provenance)**:
`first_party | community_reviewed | community_unreviewed` on
`BrokerDescriptor` — records who authored/vetted an adapter. Purely
informational; never gates or grants execution privilege. **v1 ships
`first_party` only;** `community_reviewed` awarding is deferred until
community adapters are a product goal.
_Avoid:_ "trust tier" (implies execution difference; there is none);
inventing a review board before community adapters ship

**Adapter Sandbox**:
The Wasmtime-hosted execution environment every component runs in — zero
ambient authority, network reachable only via the host-mediated broker call
import.
_Avoid:_ "the sandbox" without qualifying which stage (contract-test sandbox
against fixtures vs. live sandbox against a real Keychain credential are
different runtime instances)

**B6 Capability Sheet**:
Mandatory, human-authored, signed research doc (rate limits, history depth,
refuse lists, real broker-API facts) required before any adapter — any
origin — goes live. Passing sandbox + FillEvent contract tests is necessary
but never sufficient on its own; B6 sign-off is the separate, additional
gate (mirrors the existing D1 rule: no adapter without B6).
**Canonical path:** `/Users/bishnu/issues/brokers/sheets/<slug>.md` (Station
mirrors are non-authoritative).
_Avoid:_ treating a passing sandbox/contract-test run as substituting for B6;
editing a Station copy as if it were SoT

## Existing terms (for reference, unchanged by this ADR)

**Enforcer**:
The Rust agent that hosts the Wasm adapter sandbox, mediates all broker HTTP
calls and Keychain access, and is the sole execution authority for adapter
components.

**Kill Switch**:
Escalating protections culminating in an always-on DNS-level block of broker
hosts — orthogonal to adapter origin or execution model; applies uniformly
whether the connected adapter is first-party or community.
