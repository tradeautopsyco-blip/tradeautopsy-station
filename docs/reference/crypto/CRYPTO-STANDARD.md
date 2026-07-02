# Crypto Reference Documentation Standard

**Applies to:** `docs/reference/crypto/` and all subfolders  
**Created:** 2026-07-02  
**Enforced by:** AGENTS.md — no feature ships without a cited reference doc

---

## Rule 0 — Exchange Isolation

Binance.US (`api.binance.us`) and Binance Global (`api.binance.com` / `fapi` / `dapi` / `eapi` / `papi`) are **separate entities**. Different base URLs, different permission models, different endpoint availability. They share similar API design but are not interchangeable.

**Never merge facts between them.**

- Every file lives under its exchange folder: `binance-us/` or `binance-global/`
- Every file header must name its exchange explicitly — never just "Binance"
- If a fact's exchange is unclear from source: write `NOT SPECIFIED IN SOURCE`, do not guess

---

## Rule 1 — Source Hierarchy

When sources conflict, higher tier wins. Always cite the tier used.

| Tier | Source | Trust |
|---|---|---|
| 1 | Official OpenAPI schema (`schema.yaml`) | Highest — machine-generated, versioned, exact endpoint/param/response shapes |
| 2 | Official reference docs (developers.binance.com, docs.binance.us markdown) | Narrative detail the spec doesn't carry: rate limit philosophy, connection lifecycle, order book procedure |
| 3 | Official changelog | Dated, but narrative-heavy. Cross-check against Tier 1/2 before treating as current state |
| 4 | Official FAQ / support articles | Lowest. Fill genuine gaps only |
| ✗ | Forums, third-party blogs, AI memory | Never |

---

## Rule 2 — Citation Mandatory

Every non-obvious fact must trace to a named source file + retrieval date. Quote exact field names, endpoint paths, or error codes verbatim from the source. No paraphrase-as-fact.

---

## Rule 3 — No Invented Facts

If a source doesn't specify something, write `NOT SPECIFIED IN SOURCE`. Do not fill from training memory, from the other exchange's docs, or from what "seems right."

**Self-audit before committing:** re-read every claim. "Did I get this from the cited source, or did I just know it?" If the second — delete it or mark `NOT SPECIFIED IN SOURCE`.

---

## Rule 4 — Staleness Discipline

Every file header carries:
- `Source:` — exact file or URL
- `Snapshot date:` — when retrieved
- `Reviewed:` — yes/no

Files older than 90 days must be re-verified before justifying new feature code. Exchanges change endpoints without much notice (e.g. COIN-M/USDⓈ-M architecture merger, effective 2026-06-30).

---

## Rule 5 — Uniform File Structure

Each `exchange/asset-class/` folder contains:

| File | Content |
|---|---|
| `REST.md` | Endpoints, params, response shapes, rate limits |
| `WEBSOCKET.md` | Streams, connection lifecycle, message formats, ping/pong |
| `ENUMS-FILTERS.md` | Order types, statuses, symbol filters, rate limit definitions |
| `ERRORS.md` | Error codes relevant to this asset class |
| `CHANGELOG-NOTES.md` | Curated extract: only changes that affect TradeAutopsy assumptions (breaking changes, deprecations, new mandatory params). Not a full dump. |

Missing files are fine. Files with invented content are not.

---

## Rule 6 — Deduplication

Combined source dumps repeat entire sections 2–5x (confirmed in this project's uploads). Before writing from a combined dump: identify the canonical occurrence, diff duplicates for drift, write once.

---

## Rule 7 — Cross-Product Change Flagging

When a changelog entry spans multiple asset classes (e.g. COIN-M merging into USDⓈ-M architecture), note the cross-reference in every affected `CHANGELOG-NOTES.md`, not just one.

---

## Rule 8 — Scope Discipline

Documenting an asset class ≠ building it. Reference material can get ahead of the build. The build never gets ahead of the reference material. Locked product scope (Binance.US spot, read-only, circuit-breaker, no order placement) is unaffected by how much reference doc exists.
