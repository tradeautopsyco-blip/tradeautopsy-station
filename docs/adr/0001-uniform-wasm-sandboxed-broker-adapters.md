# Uniform Wasm/WASI sandboxing for all broker adapters, first-party and community

**Status:** accepted

Station's Universal Broker Interface (UBI) — the renamed broker portion of the
former "Backend Box" — must let traders and third parties (including AI-authored
code) supply their own broker adapters, alongside TradeAutopsy's own. Every
adapter, with no exception for first-party ones, compiles to a Wasm component
that implements a WIT interface the Rust Enforcer defines, exporting nothing but
`fetch_fills(cursor) -> FillEvent[]`. A component never holds a raw credential;
all outbound HTTP and all Keychain-derived auth (HMAC signature, session token,
etc.) are attached by the Enforcer via a host-mediated import
(`broker_http_call(request_shape) -> response_shape`). This includes
TradeAutopsy's own `binance_com` and the not-yet-built `kotak_neo` adapter — no
adapter is exempted by virtue of being first-party. We chose this over a
two-tier model (native first-party, sandboxed community) so that trust is a
property of provenance metadata (`origin` on `BrokerDescriptor`), not of
execution privilege — a community adapter can be promoted to official status by
flipping a field, never by a rewrite. Research basis:
[R11 — UBI extensibility](/Users/bishnu/issues/brokers/research/parts/R11-universal-broker-interface-extensibility.md),
particularly §5 ("Impact").

## Considered options

1. **Two-tier: native first-party adapters, Wasm-sandboxed community adapters.**
   Rejected. Native `binance_com`/`kotak_neo` could ship sooner (no Wasm/WIT/
   bridge infra needed first), but it means our own adapters get implicit trust
   merely by being first-party, and "promote a community adapter to official"
   would require a rewrite into the native tier rather than a metadata flip.
   R11 §3.1's ccxt findings show this is exactly the weaker bar we don't want:
   "whatever credentials the calling application supplies" trust, with no
   runtime capability restriction.
2. **Same-process, no sandbox, code-review-only gate (ccxt/Home Assistant
   model).** Rejected outright. R11 §3.2 documents a real Home Assistant
   security disclosure where a same-process, unsandboxed third-party
   integration exposed "any credential" reachable by the host process — the
   exact failure mode UBI must not reproduce, and this is even less
   acceptable when the adapter author may be an AI with no accountability.
3. **Uniform Wasm/WASI component sandboxing for every adapter (chosen).**
   Every adapter — first-party or community, human- or AI-authored — is a
   Wasm component with zero ambient authority (R11 §3.3: WASI's
   capability-based model, no raw sockets, only a host-granted import). Costs
   more upfront (native `binance_com_spot_adapter.rs` must be rebuilt, not
   reshaped; `kotak_neo` can't ship until the Wasm/WIT/bridge/sandbox
   infrastructure exists) but is the only option that is both macOS-native
   (Wasmtime is Tier-1 on macOS, unlike gVisor/Firecracker/seccomp, all
   Linux-only per R11 §3.3) and gives capability-precise control rather than
   process-level on/off.

## Consequences

- `binance_com_spot_adapter.rs` (agent/src/binance_com_spot_adapter.rs) must be
  **rebuilt** as a Wasm component conforming to the new WIT interface, not
  incrementally reshaped in place. Its existing client/validation logic
  (`binance_com_spot_client.rs`, `binance_com_validation.rs`) is reference
  material, not a direct port target.
- `kotak_neo` cannot ship until the Wasm/WIT/bridge/sandbox infrastructure
  exists — this delays "both first-party brokers live" relative to the
  rejected two-tier alternative. Accepted deliberately for architectural
  consistency.
- `BrokerDescriptor` gains one additive field, `origin: first_party |
  community_reviewed | community_unreviewed`. This is provenance/vetting
  metadata only — it must never be read as an execution-privilege flag, since
  execution is uniform regardless of origin.
- The B6 capability-sheet gate remains mandatory for every adapter regardless
  of origin or sandbox/contract-test pass — passing the Wasm sandbox +
  `FillEvent` contract tests is necessary but not sufficient to go live,
  mirroring the existing D1 rule ("no adapter without B6").
- Console gets zero broker code going forward (see Console `CONTEXT.md`); all
  broker adapter code, Wasm components, WIT definitions, and Keychain-backed
  credential handling live exclusively in Station/Enforcer.

## Founder locks (open questions — 2026-07-25)

1. **Console zero broker code:** Yes — full removal of `lib/brokers/**` including
   live connectors **and** CSV-only parsers / shared scaffolding (not live-secrets
   fence only).
2. **UBI rename boundary:** Broker portion of Backend Box only. Market Data Keys /
   AI / Workflow keys stay outside UBI.
3. **`community_reviewed`:** Deferred. v1 uses `first_party` only (optional
   `community_unreviewed` later with explicit risk UI). Who awards
   `community_reviewed` is undecided until community adapters ship.
4. **Canonical B6 sheets:** `/Users/bishnu/issues/brokers/sheets/` is source of
   truth. Station may mirror or link; do not edit a second original.
