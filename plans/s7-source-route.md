# Plan: S7 source route (declared broker-gap)

> Source: prototype accepted 2026-09-08 (`station/prototypes/PROTOTYPE-s7-source-route.html`)

## Architectural decisions

- **pick_route** is the obtain selector (already unit-tested; not yet on the HTTP seam).
- Connected broker last/quote never yields to a gap vendor.
- Kotak history stays **unsupported** until a declared vendor is enabled with a **key** (not a URL).
- Vendor series uses vendor `book_id` + provenance strip. DualNoBlend.
- Per-vendor meter; does not steal broker quote budget.
- First vendor in tests is fixture `licensed_history`. Yahoo/OpenBB stay off until a B6 sheet.
- Notch account chrome (S8) is a different family — do not paint vendor candles as funds.

---

## Phase 1: obtain uses pick_route (history gap)

**User stories**: Kotak history unsupported; enabling fixture vendor fills history with vendor provenance; quote stays Kotak; URL refused; empty vendor budget does not block quote.

### Acceptance criteria

- [ ] `GET /api/station/obtain` history on Kotak without vendor → `unsupported`
- [ ] With fixture vendor enabled + key → success, `provenance_adapter_id` is the vendor, `book_id` is the vendor book
- [ ] Quote obtain still Kotak when vendor is enabled
- [ ] Vendor budget 0 → history unavailable; quote still picked
- [ ] Credential that looks like a URL is refused
