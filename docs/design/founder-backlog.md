# Founder backlog (after multi-trade LiveBook)

**Status:** queued — not in scope for the LiveBook PR.  
**LiveBook shipped:** multi-slot `pending_declarations`, declaration-scoped cancel/protective, Working list + Plan another — see [harness-trade-arc.md](./harness-trade-arc.md) §7 step 1–2 and PR #21.

Do not treat items below as specified formulas or broker capabilities until reference docs and code paths exist.

---

1. **SL suggestor** — Trader picks risk % of margin (e.g. 2%); Station suggests size / stop for **all asset classes** (DualNoBlend per book; cite `docs/reference/` before any formula).

2. **Indian options UI** — Same interaction pattern as crypto USDM declare/working UI (not a copy of USDM math).

3. **Enable Station unavailable surfaces** — Option chain, OI, history, etc. When the connected broker adapter does not expose data, show an honest prompt to add a **data vendor** (no invented chain/OI).

4. **Chart crosshair** — Read-only crosshair on session/working charts where quote/history is bound.

5. **Tags + playbooks + live conditions** — Authoring and wiring TBD (Console vs Station setup); conditions remain per `declaration_id`.

6. **User-custom hotkeys** — User-defined shortcuts in Station/Notch; no default map without product spec.
