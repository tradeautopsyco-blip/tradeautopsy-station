# Hotkey actions

**Status:** proposal for a founder pick. Not wired. No default key map.
**Date:** 2026-10-01
**Audience:** the next coding agent and the founder.

Prefs already store bindings (`notch/DeskHotkeyPreferences.swift`, UserDefaults key `tradeautopsy.station.hotkey_bindings`) for two ids: `toggle_notch`, `open_station`. `HotkeyRegistrar` does not read that store. It still registers Carbon ⌥Space (toggle Notch) and ⌥⇧Space (open Station).

This note is the action list that store can grow. It does not change Swift.

Stable ids are lowercase snake_case. Once a build saves an id, do not rename it. A rename orphans the saved row.

One key, one id. A hotkey calls the same function as the button it names. It does not skip a confirm, a countdown, or `submitReadiness`.

## Action ids

| Id | Binding opens | Existing entry | Bound behavior |
| --- | --- | --- | --- |
| `kill` | Existing Kill warning chrome | `NotchViewModel.presentPlanKillWarning`, `BarPlanKillChrome` | Opens the warning. Does not call `activateKillSwitch`. The card’s own Confirm still fires. |
| `confirm_declare` | Plan Confirm | `BarDeclarationFlowView` `submit()` after `BarIntradayDeclareValidator.submitReadiness` | Same fail-closed gates as the button. Not ready → existing hint, no declare. |
| `plan_another` | Plan with an empty form | Working “Plan another →” → `presentBarDeclarationForm()` | Does not replace other pending rows. |
| `focus_open` | Open | `BarNotchScreen.morning` | Sidebar focus only. |
| `focus_plan` | Plan | `BarNotchScreen.pretrade` | Sidebar focus only. |
| `focus_working` | Working | `BarNotchScreen.live` | Sidebar focus only. |
| `focus_debrief` | Debrief | `BarNotchScreen.posttrade` | Sidebar focus only. |
| `cancel_selected_declaration` | Cancel chrome for the **selected** declaration | `BarCancelDeclarationChrome` (“Cancel declaration”) | Opens the reason step for `selected_declaration_id`. Does not pick a reason and does not submit. No selection → no-op. Copy already says this cancels intent and is not a flatten. |
| `capture_working_condition` | Working-condition capture | `captureWorkingConditionAtClose` → `POST /api/daemon/bar/capture-working-condition` | Same call as the session-close path. No selected pending declaration → no-op. |
| `protective_sl_chrome` | Protective SL status row, when that chrome exists | `BarProtectiveSlPlanChrome` | Focuses the status. `showsSetSlButton` is false. Does not send `place_sl`. Does not press Cancel SL. No status row → no-op. |
| `dismiss_kill_overlay` | “I’m Calm” | `KillSwitchOverlayView` → `dismissKillSwitchFromOverlay` | Runs only when `calmButtonEnabled` is already true. Does not skip the countdown. |
| `open_manual_fill` | Debrief manual-fill lane | `BarPostTradeView` “Broker missed a fill…” → `BarManualFillPanel` | Shows the panel. Does not queue a fill. |
| `toggle_notch` | Notch expand/collapse | Already in prefs. Carbon ⌥Space today | Unchanged until a saved binding is applied. |
| `open_station` | Station window forward | Already in prefs. Carbon ⌥⇧Space today | Unchanged until a saved binding is applied. |

## Kill copy vs the id

`kill` is the id. It is not `flatten`, not `max_loss`, and not a regulatory sentence.

Today’s warning body already says this is not a max-loss flatten (`BarPlanKillChrome.warningBody`). Full flatten-to-conditions is **rolling soon**. That phrase, and any regulatory line, belongs in the Kill chrome. It does not belong in the hotkey id. The binding may still open the Kill chrome that ships now.

## Default key map

**Option A — no default map.** Recommended until the founder picks. A new install keeps `DeskHotkeyPreferences` empty (`load()` returns `[]`). None of the ids above are bound by this note. The Carbon pair that already ships (⌥Space, ⌥⇧Space) stays shell code for `toggle_notch` and `open_station` only. It is not a proposed default for Kill, Confirm, Cancel, or I’m Calm.

**Option B — minimal safe defaults.** Only if the founder wants keys before anyone opens Settings. Limit any such set to non-committing focus: `focus_open`, `focus_plan`, `focus_working`, `focus_debrief`, and `plan_another`. Leave unbound: `kill`, `confirm_declare`, `cancel_selected_declaration`, `capture_working_condition`, `protective_sl_chrome`, `dismiss_kill_overlay`, `open_manual_fill`.

This note does not choose Option B. Do not ship a default map in the same change as the id list. Founder pick first.

## Wiring later

When a later change teaches `HotkeyRegistrar` to read prefs:

1. Register only saved rows.
2. Empty prefs → keep today’s two Carbon hotkeys and nothing else.
3. A saved `toggle_notch` or `open_station` replaces the matching Carbon default for that install.
4. `configurableActions` in `DeskHotkeyPreferences` grows to this table. Labels can change. Ids cannot.
