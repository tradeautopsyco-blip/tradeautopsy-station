# Direct-broker multi-capability Station runtime

**Status:** accepted  
**Date:** 2026-08-26  
**Decision:** R0 contract approved; implementation starts only on explicit founder command  
**Supersedes:** ADR 0001 only where it limits the adapter export to `fetch_fills`  

## Context

ADR 0001 correctly requires every first- and third-party broker adapter to run as a
sandboxed Wasm component with host-mediated network and Keychain access. Its v0.1 WIT
contract exports only `fetch_fills`, which makes UBI mechanically account/fill-only.
Station Data then grew a separate `binance_public` quote stream, Zerodha-shaped instrument
store, Console LTP fallback, and placeholder history/options routes. That creates two
source architectures and prevents the connected broker from serving the market/reference
capabilities it actually supports.

OpenAlgo demonstrates a coherent read vocabulary across Data and Accounts APIs: quotes,
multiquotes, depth, history, intervals, instruments/search/symbol, derivative contracts,
option chains/Greeks, calendars, funds, positions, holdings, orders, and trades. Running
OpenAlgo itself would add another broker session, credential system, process supervisor,
and health/routing authority.

## Decision

Station connects directly to brokers. OpenAlgo Data + Accounts nouns are the normalized
read compatibility baseline, not a required process, API key, SDK, MCP server, UI, or
execution gateway.

One persistent `BrokerConnectionRuntime` owns one connected broker adapter, credential
handle, session/rate state, account schedulers, and market subscriptions. The adapter
declares a versioned `SourceManifest` containing every account, market, reference, and
provider-derived capability it implements. Each binding retains its own family,
capability ID, physics, rights, auth mode, limits, coverage, health, and provenance.

Selection is capability-specific:

1. fresh local projection produced by the connected broker;
2. connected broker stream;
3. connected broker REST;
4. explicit eligible broker-gap adapter;
5. typed unsupported/unavailable.

Specialized providers serve non-broker domains such as news, economic statistics,
regulatory positioning, legislative records, energy, and macro data. They do not preempt
a capable connected broker merely because a credential exists.

A future optional OpenAlgo-compatible adapter may be installed by a trader who already
runs OpenAlgo. It occupies the same source-adapter seam and cannot create a second control
plane.

## Adapter interface direction

WIT v0.2 is additive during migration. It must provide a deep interface rather than one
method per OpenAlgo endpoint:

- capability discovery (`describe`);
- typed bounded queries (`obtain`);
- host-mediated subscription plans and event normalization for stream physics;
- v0.1 `fetch_fills` compatibility until account reads migrate.

The host remains the sole network authority. Before expanding exports it must enforce:

- broker-bound hosts;
- capability-bound method/path allowlists;
- `public` versus `private_read` auth selected by the host;
- hard refusal of place/modify/cancel, withdrawals, transfers, and key management;
- response-size, timeout, request-count, and concurrency limits;
- shared rate budgets across market and account calls on one connection.

## Consequences

- ADR 0001's uniform Wasm sandbox, provenance-neutral execution privilege, Keychain
  custody, and B6 gate remain accepted.
- `fetch_fills` is no longer the complete definition of a broker adapter.
- Current `binance_public` TickBook is a migration proof. Its public stream may become the
  `binance_com` quote binding implementation; it is not a permanent parallel adapter.
- Broker access is neither an automatic storage/display/redistribution license nor an
  automatic prohibition. Rights are capability- and product-use-specific.
- Option chain is not order-book depth. REST option chain is a bounded market
  cross-section; ordered depth requires snapshot-plus-contiguous-deltas.
- Account and market capabilities may share one broker runtime but never one ambiguous
  identity, cache, health flag, or authorization surface.
- Market/reference/account packets do not enter `ingestSignal`; only separately derived
  behavioral events do.
- Execution remains outside this read architecture and behind `routeOrder()`.

## R0 gate

No implementation begins until the founder explicitly says **start R0**. R0 first lands:

1. the amended family×physics matrix and normalized operation catalog;
2. source-manifest and WIT v0.2 contract tests;
3. host allowlist/auth-mode security tests;
4. broker-primary/explicit-gap router acceptance tests;
5. expanded signed B6 capability evidence for every claimed Binance/Kotak operation.

