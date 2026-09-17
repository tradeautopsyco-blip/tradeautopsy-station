# PROTOTYPE — throwaway

**Question:** At **true menu-bar scale** (1 CSS px = 1 pt), which closed TradeAutopsy Notch can a trader actually read and grab — a BoringNotch-style island with sound ears, a menu extra, or split ears that leave the camera empty?

**Rule:** Judge the **1:1 strip**. The 3× loupe is inspection only. Do not pinch-zoom the page.

**Law this chrome must keep**

- Height = menu bar (`frame.maxY − visibleFrame.maxY`), default **24pt**. Not the 32px hanging pill. Not the 38pt camera inset.
- Width on a notched Mac = housing + BoringNotch sound ears (`2 × max(0, h − 12) + 20`).
- Content: intervention keyword wins (`EXPIRED` / `COOLING` / `LIMIT`); else logo **T** + desk-signed session P&L. No second money writer.
- Press on pointer-down (`scale 0.97`). 10pt hysteresis: under = click expand, over = move. Hover never expands.
- Spatial: expand grows **from the chip**. Collapse returns to the same slot.

**Not answering:** Expanded PLAN catalog. Harness top bar. Today. Drag-parking on a notched Mac (production parks only on non-notch).

**Run**
```bash
open station/prototypes/PROTOTYPE-notch-closed-pill.html
```

Or `./serve-prototype.sh` then http://127.0.0.1:8766/PROTOTYPE-notch-closed-pill.html?variant=A

← → switches variants. Scene chips change P&L / intervention. Height chips compare 24pt (law) vs 38pt camera hang.

| Key | Name | Structure |
|-----|------|-----------|
| A | Island + ears | One black chin, flush to the top, housing + sound ears. Production SoT (`BarNotchVolumeSlot` + `CollapsedNotchView`). |
| B | Menu extra | Lives in the **right** extras cluster. Same 24pt as File/Edit. Never covers Window. Click opens a menu from the extra. |
| C | Split ears | Camera hole stays empty. Tiny left ear (T) + right ear (P&L). Two pieces, one grab. |

**Why 1:1:** Previous HIG mocks drew a 32×176 capsule *under* the bar. Production closed height is the menu bar. 11pt P&L and a 9pt T either read at 24pt or they don’t — a zoomed mock cannot answer that.

**Verdict:** **A — Island + ears.** Closed Notch on a notched Mac is one black chin, flush to the screen top, system menu-bar height, width = housing + sound ears. T + session P&L + bars, or a centered intervention keyword. Ears may nibble Window. B and C are not shipping.
