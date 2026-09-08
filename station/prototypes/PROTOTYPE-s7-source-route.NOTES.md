# PROTOTYPE — S7 source route

**Throwaway.** Question: when Kotak has no history API, who answers, whose book, whose budget, and what does the desk say?

**Run**

```bash
cd /Users/bishnu/tradeautopsy-station/station/prototypes
./serve-prototype.sh
```

Open http://127.0.0.1:8766/PROTOTYPE-s7-source-route.html  
(file:// also works — one HTML file.)

**Already locked (grill / ADR 0003)** — this demo only lets you feel them:

- Vendor fills a **declared gap** only
- Provenance strip: `History: licensed_history (Kotak has none)`
- Vendor `book_id` ≠ Kotak book
- Keys, not URLs
- Per-vendor meter; does not steal broker quote budget
- Placeholder id `licensed_history` — Yahoo/OpenBB are not product until a B6 sheet

**Walkthroughs:** gap / enable / paste URL / separate meters.

**Verdict (2026-09-08):** founder **perfect** — this is the S7 model. Capture: gap stays unsupported until a key-enabled declared vendor; provenance strip + vendor `book_id`; URL paste refused; vendor meter ≠ broker quote meter. Next: spec epic + one tracer (obtain uses `pick_route`; fixture `licensed_history`, no Yahoo).
