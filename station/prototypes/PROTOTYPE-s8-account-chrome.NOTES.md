# PROTOTYPE — S8 Notch account chrome

**Throwaway.** Question: what should funds / holdings / positions / orders look like on Notch after Start, from **obtain + AccountBook**, not Today fill-inventory?

**Run**

```bash
cd /Users/bishnu/tradeautopsy-station/station/prototypes
python3 -m http.server 8766
```

**Verdict (2026-09-08):** founder picked **C — pulse + ledger**. A and B are discarded.

Open http://127.0.0.1:8766/PROTOTYPE-s8-account-chrome.html?variant=C

| Variant | Structure |
|---------|-----------|
| **A** | Pills stay; **drawer** of lists under Settings-style chrome |
| **B** | Same pills; **tabbed sheet** Funds / Holdings / Positions / Orders |
| **C** | **Pulse** free INR + counts; full-bleed obtain ledger |

**Locked for this prototype (grill):** shipping book of Start only · DualNoBlend (INR strip, no USD on it) · Today stays fill-inventory · orders can show unavailable · no Notch production code.

**Not this file:** vendor history chart, AlertBus, fake-broker deletion, Console `:3000`.

When a variant wins, fold into Swift (`BarSettingsView` / `BarNotchShell`); delete the HTML from `main` or leave it in `prototypes/` as the throwaway record.
