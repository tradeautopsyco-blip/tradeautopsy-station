# Declared broker-gap vendors

**Status:** accepted (grill 2026-09-08)  
**Date:** 2026-09-08  
**Supersedes:** none (amends ADR 0002 selection step 4 wording only)

## Context

ADR 0002 already allows an “explicit eligible broker-gap adapter” after connected broker
REST. `CONTEXT.md` had been read as “vendors must not exist.” Founder wants history when
a shipping broker (Kotak) has no Trade API history, via user-enabled vendors.

Silent stitching (vendor OHLC claimed as `kotak-nse-nfo` last/history) repeals S2 and
DualNoBlend. Arbitrary user-pasted fetch URLs break the host+path fence (SSRF).

## Decision

1. Vendors are **in** as declared broker-gap sources (allowlist + B6 sheet + Keychain key).
2. Desk chrome may sit on the NFO/equity desk. **Provenance strip is mandatory.**
3. Vendor series uses the **vendor book_id**, never the broker book_id.
4. Hosts are allowlisted. Traders paste **keys**, not URLs.
5. Rate limits are **per vendor** (published limits). Shared pooling is later, not this ADR.
6. Kill, P&L, and canonical history must not treat vendor candles as broker fills.

## Consequences

- S7 must ship vendor eligibility + meters before a second history vendor.
- Notch account chrome (S8) is a different family (`account/*`) and does not paint vendor
  history as funds/holdings.
- Yahoo/OpenBB remain not product until named on a B6 sheet.
