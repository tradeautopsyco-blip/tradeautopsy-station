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
2. **Loopback Station↔agent auth** uses ephemeral `AGENT_DAEMON_SECRET` + wire-v1 HMAC —
   **machine integrity only**, not Console user identity (A8).
3. **Console / brain identity** uses Signed Caller JWT (`aud=station`) from Keychain
   (`Authorization: Bearer`). Agent must not send `x-daemon-secret` + `x-user-id` as
   who-am-I to `TRADEAUTOPSY_SERVER_BASE_URL`.
4. **No secrets in logs, SSE, or UI models** — API keys, HMAC signatures, and auth
   headers are structurally forbidden from reaching any log or observable surface.
5. **Kill switch is always on** — Stop pauses broker sync only; it does not stop the
   kill switch, audit log, or SSE.
6. **Keychain is the only credential store** — broker keys and Station Caller tokens;
   no plaintext secrets in app preferences or files.
7. **Withdraw permission = hard block** — never save or use a Binance.US key that has
   withdrawal permission enabled.
8. **Station product code lives in this repo** (`agent/`, `notch/`, `station/`) — not in
   `FExEVIL/tradeautopsy` `src-tauri`.

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
