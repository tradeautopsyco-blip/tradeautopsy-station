# PROTOTYPE — throwaway

**Question:** How should the Station notch appear and disappear — Spotlight glass, an island morph, or the current cut-off close?

Same PLAN card in every variant. Only the motion changes.

**Run**

```bash
open station/prototypes/PROTOTYPE-notch-spotlight-summon.html
```

Or `cd station/prototypes && ./serve-prototype.sh`, then open the summon URL.

**Controls:** Space summons and dismisses. Esc closes. ← → or the bottom bar switches variant. `?variant=A` is shareable.

| Key | Name | Motion |
|-----|------|--------|
| A | Spotlight glass | Card already full size under the menu bar. Opacity and scale `0.97 → 1`, damping `1`, open `0.28s` / close `0.22s`. Desktop shows through. Then it is gone. |
| B | Island morph | Capsule widens into the sheet. One soft settle, damping `0.86`. Folds back, then gone. |
| C | Current Station | Scale `0.96 → 1`, solid fill, open spring `0.22`. Close removes the window on the same frame. |

**Not in this file:** production `NotchPanel` / `NotchTheme`. Fold a winner into those after a verdict.
