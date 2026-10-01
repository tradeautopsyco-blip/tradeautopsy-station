# Founder backlog (after multi-trade LiveBook)

**Status:** partial (2026-10-01) — locked FixedRisk rows and saved hotkey application ship in the agent and Notch. Unlocked sizing rows stay dashed. No default key map.  
**LiveBook shipped:** multi-slot `pending_declarations`, declaration-scoped cancel/protective, Working list + Plan another — PR #21.

Do not treat items below as specified formulas or broker capabilities until reference docs and code paths exist.

---

1. **SL suggestor** — Plan risk strip calls `POST /api/daemon/risk/preview`. Locked rows (C1, C2 with a lot, C7, C8) can Apply a size. Suggested stop stays **—**. Unlocked books stay dashed. DualNoBlend per book. Fees stay 0.

2. **Indian options UI** — Kotak NFO options cockpit on declare; Working adds `BarOptionsGlanceStrip` when cockpit surface applies (not USDM math).

3. **Enable Station unavailable surfaces** — `BarDataVendorHonesty` copy on chain/OI holes; still no invented chain/OI.

4. **Chart crosshair** — Read-only crosshair on `BarOptionsSessionChart` when history is bound (hover).

5. **Tags + playbooks + live conditions** — Local tags/playbooks on Plan; condition fires per `declaration_id` + list chips; Console authoring still TBD.

6. **User-custom hotkeys** — Settings stores every id in [`hotkey-actions.md`](./hotkey-actions.md). Empty prefs keep Carbon ⌥Space and ⌥⇧Space only. A saved row replaces the matching default. `HotkeyRegistrar` applies the saved list. No default map for Kill, Confirm, or Cancel.
