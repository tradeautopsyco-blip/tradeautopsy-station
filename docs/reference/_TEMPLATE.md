# [TOPIC] Reference — [Short Descriptor]

<!--
  HOW TO USE THIS TEMPLATE
  ========================
  Replace every [PLACEHOLDER] with real content.
  Do not delete any section — if a section is empty, write why it is empty.
  The self-audit and verification checklist are mandatory; they are the point.
  See agent/reference/binance-us/API-REFERENCE-v1.md for the canonical example.
-->

---

## Header Block

| Field            | Value                                                                 |
| ---------------- | --------------------------------------------------------------------- |
| **Topic**        | [e.g. Binance.US REST API — Order Fills and Fee Computation]         |
| **Primary source** | [Full URL or citation]                                              |
| **Snapshot date** | [YYYY-MM-DD — the date you read and captured this source]           |
| **Source version** | [API version, document revision, or "undated" if not versioned]   |
| **Staleness warning** | Re-verify against live source before building against this doc. |
| **Author**       | [Your name or handle]                                                 |

---

> ⚠️ **BLOCKER** *(delete this block if no blockers)*
>
> The following facts are needed for implementation but are **NOT SPECIFIED IN SOURCE**:
>
> - [Fact 1 — e.g. "Exact rounding rule for fee computation"]
> - [Fact 2]
>
> Do not implement the affected code paths until these are resolved from a primary source.

---

## Source Inventory

List every file, page, or paper you read to produce this document.

| Source | URL / Citation | Date accessed | Notes |
| ------ | -------------- | ------------- | ----- |
| [Name] | [URL or citation] | [YYYY-MM-DD] | [e.g. "section 4.2 only"] |
| [Name] | [URL or citation] | [YYYY-MM-DD] | |

---

## Concepts

*Add one sub-section per concept, formula, or definition. Copy the block below as many times
as needed. Never merge two concepts into one block.*

---

### [Concept Name — e.g. "Fill Price"]

**Source:** [Exact citation — document title, section, URL]

**Verbatim definition / formula:**

```
[Paste the exact text or formula from the source. Use quotation marks for prose.
 Never paraphrase in this block — paraphrase belongs in OUR INTERPRETATION below.]
```

**Field / term reference:**

| Term / Field | Source definition | Units / type |
| ------------ | ----------------- | ------------ |
| [term]       | [exact source wording] | [e.g. string, decimal, epoch ms] |

**Gaps (NOT SPECIFIED IN SOURCE):**

- [Any fact about this concept that the source does not address]
- [Leave blank if source is complete for this concept]

---

> **OUR INTERPRETATION**
>
> *This block is separated from the source. What follows is the TradeAutopsy team's
> reading, mapping, or adaptation — not a quote.*
>
> - [How this concept maps to our domain model]
> - [Edge cases we decided how to handle, and why]
> - [Any difference from what a naive reading suggests]

---

### [Next Concept — copy block above]

---

## Verification Checklist

Facts that still need confirming against the live source or a test environment before
code is written against them.

- [ ] [Claim to verify — e.g. "Fee rate of 0.1% confirmed against live /api/v3/account response"]
- [ ] [Claim to verify]
- [ ] [Add one line per unconfirmed fact; check off as confirmed]

---

## Self-Audit

*This section is mandatory. Its purpose: surface every place the author was tempted to fill
a gap from memory or assumption rather than from the cited source.*

| Temptation | What the source actually says | Resolution |
| ---------- | ----------------------------- | ---------- |
| [e.g. "I assumed fee rate was 0.1% from memory"] | [e.g. "Source says 0.1% for maker, 0.1% for taker at base tier — confirmed"] | [e.g. "Kept verbatim from source; added NOT SPECIFIED for tier upgrade rules"] |
| [Temptation 2] | [Source evidence] | [Resolution] |

*If you filled nothing from memory and the source covered every needed fact — write that
explicitly: "No memory fills. All facts sourced from [document]." Do not leave this section
blank.*
