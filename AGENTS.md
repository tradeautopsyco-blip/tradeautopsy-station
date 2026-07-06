# TradeAutopsy Station — Agent Context

TradeAutopsy Station is the macOS native app + Rust agent backend that forms the local
execution layer of TradeAutopsy. It pairs with the web product at
`FExEVIL/tradeautopsy` (see that repo's `AGENTS.md` for full product context).

**Stack:**
- `station/` — Swift/SwiftUI macOS app (Xcode project)
- `agent/` — Rust HTTP API (Axum, port 9137)
- `notch/` — Notch UI components

**Pairing repo:** `FExEVIL/tradeautopsy` (web + behavioral engine)
**Product AGENTS.md:** `Tradeautopsy1/AGENTS.md` (authoritative product invariants,
signal weights, ship-it pipeline)

---

## Architectural Invariants — Station

1. **Agent port is 9137** — never change; web daemon relies on this.
2. **All agent auth uses `x-daemon-secret` header** — no other auth mechanism.
   **Shipped model (Phase 0):** Station generates an ephemeral `AGENT_DAEMON_SECRET`
   per launch (`SecRandomCopyBytes`), passes it to the spawned agent via env, and sends
   the same value on loopback requests as `x-daemon-secret` with wire-v1 HMAC signing.
   **Not implemented:** UDS / 0600 token-file auth described in `station-wire/v1.json` and
   early TRDs — do not assume that pattern exists when debugging auth failures.
3. **No secrets in logs, SSE, or UI models** — API keys, HMAC signatures, and auth
   headers are structurally forbidden from reaching any log or observable surface.
4. **Kill switch is always on** — Stop pauses broker sync only; it does not stop the
   kill switch, audit log, or SSE.
5. **Keychain is the only credential store** — no plaintext secrets in app preferences,
   files, or memory beyond the lifetime of a single request.
6. **Withdraw permission = hard block** — never save or use a Binance.US key that has
   withdrawal permission enabled.

---

## Reference Library Discipline

**Any feature touching financial calculation, market mechanics, or behavioral science
must cite a `docs/reference/` document before implementation.**

Specifically:

- If the feature involves a formula, threshold, fee rate, settlement rule, cost-basis
  method, or behavioral-science claim — a reference doc must exist in `docs/reference/`
  before a single line of feature code is written.
- If no reference doc exists for the required topic, **create one first** using
  [`docs/reference/_TEMPLATE.md`](docs/reference/_TEMPLATE.md), sourced from an
  authoritative primary source (broker docs, exchange rulebook, peer-reviewed paper).
- **No formula or mechanic may be implemented from AI memory or assumption.** If a fact
  cannot be found in a primary source, write `NOT SPECIFIED IN SOURCE` in the reference
  doc and leave the code path unimplemented until the source is found.
- Gaps are never guessed. AI recall is not a source. A blog post is not a source.
- Cite the reference doc in the feature's PRD and TRD.

See [`docs/reference/README.md`](docs/reference/README.md) for category definitions,
authority hierarchy, and the discipline in full.

---

## Commit Convention

```
feat:     new user-visible behavior
fix:      bug correction
chore:    config, deps, tooling
refactor: internal restructure, no behavior change
test:     adding or fixing tests only
docs:     documentation and reference library changes only
```

Subject ≤ 50 chars. Body only if "why" is not obvious from subject.

---

## Test User

All broker sync and behavioral features must be tested against:

```
broker:   Binance.US
context:  validated credentials (read-only, no withdraw permission)
```

Every Binance.US validation flow must confirm the withdraw-permission hard block before
shipping.
