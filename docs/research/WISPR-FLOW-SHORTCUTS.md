# Wispr Flow — global shortcuts & hardware “shortcut button” research

**Status:** `RESEARCH` — primary-source pass  
**Date:** 2026-07-28  
**Local sample:** `/Applications/Wispr Flow.app` · version **1.6.224** (Electron) · Swift helper **1.6.200** · Team ID `C9VQZ78H85` (Wispr AI INC) · notarized Developer ID  
**Repos:** Station `/Users/bishnu/tradeautopsy-station`  
**Related Station code:** `station/StationApp/HotkeyRegistrar.swift` (Carbon `RegisterEventHotKey` for ⌥Space)

### Summary

Wispr Flow’s public docs tell users that **Microphone + Accessibility** are required on Mac, and that **keyboard shortcuts specifically need Accessibility** (separate from mic). They never list **Input Monitoring** as a required permission. Local binary inspection of the shipped **Swift accessibility helper** (`com.electron.wispr-flow.accessibility-mac-app`) shows it implements shortcuts via **`CGEventTapCreate` + run-loop** (`KeyboardService`), detects Apple **Fn** via **IOHIDManager**, monitors **Secure Event Input** (`IsSecureEventInputEnabled` / `SecureInputMonitor`), and supports a **BLE “mic ring”** hardware path — **not** Carbon `RegisterEventHotKey` in the helper. Official docs also document that **hold-to-talk survives Secure Event Input while multi-key shortcuts do not**, implying a dual input path. Closed-source Electron+Swift stack; no first-party open source for the macOS shortcut layer.

---

## Question

How does Wispr Flow implement global keyboard shortcuts and hardware “shortcut buttons” so they work frictionlessly across apps — especially regarding macOS Input Monitoring, Accessibility, Carbon hotkeys, CGEvent taps, etc.?

---

## What Wispr publicly says (primary)

### Required Mac permissions

| Permission | What Wispr says | Source |
|------------|-----------------|--------|
| **Microphone** | Required for dictation | [Setup Guide](https://docs.wisprflow.ai/articles/3152211871-setup-guide), [System requirements](https://docs.wisprflow.ai/articles/1036674442-supported-devices-and-system-requirements), [Re-verify permissions](https://docs.wisprflow.ai/articles/5510622673-re-verify-wispr-flow-permissions-after-updating) |
| **Accessibility** | Required to insert text into other apps; **keyboard shortcuts require Accessibility** (FAQ: “Keyboard shortcuts stopped working even though microphone access is on”) | Same three docs |
| **Screen Recording / System Audio** | Optional for basic dictation; Notetaker / meeting audio | System requirements + Re-verify |
| **Input Monitoring** | **Not mentioned** in required-permissions lists or permission re-verify steps | Exhaustive read of docs above + uninstall guide |

Setup Guide (Mac dictation onboarding): grant mic → grant Accessibility (“Flow uses accessibility access to insert spoken words into other apps”). Shortcut setup follows (default **Fn** on Apple keyboards, else **Ctrl+Opt**; mouse buttons Middle / Mouse 4–10; up to 4 bindings per action; ≤3 keys).

Enterprise note: MDM can pre-grant **Accessibility**; **Microphone cannot** be pre-granted on macOS ([System requirements](https://docs.wisprflow.ai/articles/1036674442-supported-devices-and-system-requirements)).

### Shortcuts product behavior (docs)

- Settings → General → Shortcuts; push-to-talk, hands-free, paste/copy last transcript, Cancel (Esc), Scratchpad, etc. ([Supported & Unsupported Hotkeys](https://docs.wisprflow.ai/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts), [Route with keyboard shortcuts](https://docs.wisprflow.ai/articles/5298382595-route-dictation-directly-to-slack-email-or-calendar-with-keyboard-shortcuts)).
- **Fn** is treated as hardware-level / Apple-keyboard-only; external non-Apple keyboards don’t expose true Apple Fn — fallback **Ctrl+Opt** ([hotkeys doc](https://docs.wisprflow.ai/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts)).
- **Mouse buttons** as shortcuts: Middle, Mouse 4–10; left/right excluded. Logitech MX Master side buttons under Logi Options+: bind keyboard chord in Flow + map that chord in Logi Options+ ([Logitech article](https://docs.wisprflow.ai/articles/1078330210-fix-logitech-mx-master-logi-options-mouse-buttons-not-working-as-a-flow-shortcut)).
- **Secure Event Input / Secure Keyboard Entry (Mac):** shortcuts (e.g. Fn+Space, Esc) stop; **hold-to-talk continues**. Flow notifies naming the blocking app after a few seconds. Docs explicitly: *“Hold-to-talk uses a different input method that isn't affected by Secure Event Input. Keyboard shortcuts require Flow to read your keystrokes…”* ([Secure Event Input fix](https://docs.wisprflow.ai/articles/8841649969-fix-flow-shortcuts-blocked-by-macos-secure-keyboard-entry-secure-event-input)).

### Privacy / marketing pages

[wisprflow.ai/privacy](https://wisprflow.ai/privacy) covers data retention / Privacy Mode / certifications — **no** technical discussion of Input Monitoring, event taps, or hotkey APIs.

### First-party source / changelog

GitHub org [Wispr-AI](https://github.com/Wispr-AI) has ML/infra repos (`whisper_attention_mask`, `vime`, `pr-review-bot`) — **no** macOS client / hotkey source. Shortcut implementation is closed.

---

## Local app inspection (this machine)

Inspected **without** installing anything new — app already at `/Applications/Wispr Flow.app`.

### Bundle shape

| Component | Bundle ID / role | Notes |
|-----------|------------------|-------|
| Main app | `com.electron.wispr-flow` | Electron shell; `NSMicrophoneUsageDescription`, camera/Bluetooth/audio-capture strings; **no** Accessibility usage string (AX is System Settings, not Info.plist) |
| Swift helper | `com.electron.wispr-flow.accessibility-mac-app` | `LSUIElement=true`; path `Contents/Resources/swift-helper-app-dist/Wispr Flow.app`; IPC from Electron (“Waiting for message from Electron to start accessibility monitoring…”) |
| Signing | Developer ID Application: Wispr AI INC (`C9VQZ78H85`), stapled notarization, Hardened Runtime | Entitlements: JIT / unsigned executable memory / disable library validation / dyld env (Electron typical) + `device.audio-input` + `device.camera`. **No** sandbox entitlement. |

### Info.plist usage strings (what users see in TCC prompts)

**Main app:** mic, camera, Bluetooth, system audio capture.  
**Swift helper:** Apple Events (browser window beside meeting recorder), system audio, Bluetooth — *“Allow Wispr Flow to connect to your BLE mic ring for hands-free dictation.”*

Neither plist mentions Input Monitoring (that TCC category has no usage-description key).

### Linked frameworks / imported symbols (Swift helper, arm64) — confidence **Confirmed**

`otool -L` links **Carbon**, **CoreGraphics**, **IOKit**, **AppKit**, **CoreBluetooth**, Accessibility-related ApplicationServices usage via AX APIs, etc.

**Imported symbols present:**

| API family | Symbols (representative) | Likely use |
|------------|--------------------------|------------|
| **CGEvent tap** | `CGEventTapCreate`, `CGEventTapEnable`, `CGEventTapIsEnabled`, `CFMachPort*`, `CFRunLoop*` | Global key/mouse observation |
| **CGEvent inject** | `CGEventCreateKeyboardEvent`, `CGEventPost`, `CGEventSetFlags`, … | Simulate keypress / insert path support |
| **Accessibility** | `AXIsProcessTrusted`, `AXIsProcessTrustedWithOptions`, full `AXUIElement*` / `AXObserver*` set | Trust check + text field / typing |
| **Secure input** | `IsSecureEventInputEnabled` | Detect SEI blockers |
| **IOHID** | `IOHIDManagerCreate` / Open / matching / device callbacks; `IOHIDDevice*` / `IOHIDElement*` | Keyboard/Fn / HID enumeration |
| **AppKit** | `NSEvent` class | Flags / local event helpers |

**Not imported by Swift helper:** `RegisterEventHotKey`, `UnregisterEventHotKey`, `CGPreflightListenEventAccess`, `CGRequestListenEventAccess`.

**Electron Framework** (generic Electron) *does* export/link `RegisterEventHotKey` and `CGEventTapCreate` — expected for Electron’s own `globalShortcut` / Chromium stack. That does **not** prove Wispr’s product shortcuts use Carbon; the product keyboard path lives in the Swift helper (`KeyboardService`, `UpdateShortcuts`, `eventTap`).

### Swift type / string evidence (helper) — confidence **Confirmed** (strings/symbols; not decompiled logic)

- Classes: `KeyboardService`, `SecureInputMonitor`, `BLEAudioManager`, `KeystrokeTraceRecorder`, …
- Strings: `eventTap`, `eventTapRunLoop`, `Failed to create event tap`, `Keyboard service event tap disabled, attempting to restart tap`, `updateShortcuts` / `UpdateShortcuts`, `hasAppleFnKey`, `Found an Apple Internal keyboard…` / `Found an external non-Apple keyboard…`, `Secure input is blocking keyboard shortcuts`, `kCGSSessionSecureInputPID`, `Initiating accessibility API functionality, textbox monitoring, and event taps`, `ble-mic-ring`, BLE pairing/scan logs for a ring peripheral.

---

## Technical mechanisms (with confidence)

### 1. Architecture: Electron UI + native Swift helper

| Claim | Confidence | Evidence |
|-------|------------|----------|
| Shortcut / AX / HID work runs in a separate helper app, started/controlled by Electron over IPC | **Confirmed** | Bundle path + `IPCClient` + “Waiting for message from Electron…” / `StartAccessibilityServices` |
| Helper is agent-style (`LSUIElement`) | **Confirmed** | Info.plist |

### 2. Global shortcut listening: **CGEvent tap**, not Carbon hotkeys / not NSEvent global monitor as primary

| Claim | Confidence | Evidence |
|-------|------------|----------|
| Primary listener is **`CGEventTapCreate`** wired to a run loop | **Confirmed** | Imports + `eventTap` / failure / restart strings in `KeyboardService` |
| Product shortcuts are **not** implemented via Carbon `RegisterEventHotKey` in the helper | **Confirmed** | No HotKey symbols in helper |
| Electron may still have Carbon hotkey capability unused or for other features | **Inferred** | Electron Framework links `RegisterEventHotKey`; no product-doc or helper-string evidence of using it for Flow PTT |
| Not primarily `NSEvent.addGlobalMonitorForEvents` | **Inferred** | `NSEvent` class linked but tap create/enable/run-loop strings dominate; Apple staff note that AX vs Input Monitoring differs for NSEvent monitor vs CGEventTap ([Dev Forums](https://developer.apple.com/forums/thread/707680)) |

### 3. Permissions posture vs Apple’s TCC model

| Claim | Confidence | Evidence |
|-------|------------|----------|
| Wispr **documents** Accessibility (not Input Monitoring) as the permission users must grant for shortcuts | **Confirmed** | Official help articles |
| Wispr **needs Accessibility** for text insertion regardless of hotkey API | **Confirmed** | Docs + extensive AX imports |
| Pure CGEventTap keyboard listening is associated with **Input Monitoring** (`CGPreflightListenEventAccess` / `CGRequestListenEventAccess`) in Apple engineer guidance | **Confirmed** (Apple guidance) | [Dev Forums — Quinn](https://developer.apple.com/forums/thread/735223), [thread/707680](https://developer.apple.com/forums/thread/707680) |
| Whether Flow users also get an **Input Monitoring** TCC row in practice | **Unknown** | TCC.db not readable here; no ListenEvent request APIs in helper; docs silent. Possible that Accessibility-trusted + unsandboxed tap creation covers them without a separate documented step — **not verified** |
| `NSEvent.addGlobalMonitor` historically needs Accessibility | **Confirmed** (Apple guidance) | Same forum threads — relevant to Station’s *previous* approach |

### 4. Dual path: shortcuts vs hold-to-talk under Secure Event Input

| Claim | Confidence | Evidence |
|-------|------------|----------|
| When Secure Event Input is active, **shortcuts fail**, **hold-to-talk works** | **Confirmed** | Wispr help center (multiple articles) |
| Shortcuts path is blocked because it “reads keystrokes” (event intercept) | **Confirmed** | Wispr docs + Apple [TN2150](https://developer.apple.com/library/archive/technotes/tn2150/_index.html) (event taps are keyboard intercept processes; SEI suppresses delivery to interceptors) |
| Hold-to-talk uses a **different** mechanism (docs) | **Confirmed** (product claim) | Secure Event Input article FAQ |
| Hold-to-talk likely uses **IOHID** / device-level Fn (or similar) rather than the CGEvent tap chord path | **Inferred** | IOHID + `hasAppleFnKey` + docs’ “different input method”; TN2150 notes HID seize as a separate intercept class — exact hold-to-talk wiring **not** reverse-engineered |

Helper also implements `SecureInputMonitor` (`IsSecureEventInputEnabled`, `kCGSSessionSecureInputPID`) matching the documented notification UX — **Confirmed** presence; notification policy details **Inferred**.

### 5. Hardware / “shortcut button” style UX

| Mechanism | What it is | Confidence |
|-----------|------------|------------|
| **Apple Fn key** | Default PTT; detected via HID keyboard metadata | **Confirmed** (docs + `hasAppleFnKey` / IOHID strings) |
| **Mouse side / middle buttons** | Bound as Flow shortcuts when HID exposes button count; MX Master via Logi Options+ → keyboard chord workaround | **Confirmed** (docs) |
| **BLE mic ring** | Bluetooth accessory; helper `BLEAudioManager`; usage string for BLE mic ring; feature flag `ble-mic-ring` | **Confirmed** presence in binary + plist; product marketing depth **Unknown** (not covered in the help articles fetched for shortcuts) |
| Dedicated “Wispr hardware shortcut button” API beyond BLE ring / mouse / Fn | **Unknown** | No primary doc naming a branded hardware button product in the fetched set |

---

## Comparison implications for TradeAutopsy Station

Station recently moved Notch ⌥Space from **`NSEvent.addGlobalMonitor`** (Input Monitoring / fragile under ad-hoc Debug) to **Carbon `RegisterEventHotKey`** (`HotkeyRegistrar.swift`).

| Dimension | Wispr Flow (observed) | Station (current Carbon) |
|-----------|----------------------|---------------------------|
| **Primary hotkey API** | CGEvent tap in native helper (**Confirmed**) | `RegisterEventHotKey` (**Confirmed** in Station) |
| **Permission story users see** | Accessibility (+ mic); docs silent on Input Monitoring | Carbon avoids Input Monitoring for ⌥Space (Station’s design intent) |
| **Frictionless across apps** | Yes when AX granted; **fails under Secure Event Input** for chord shortcuts; hold-to-talk remains | Carbon hotkeys are system-registered combinations — different TCC profile; still subject to OS reservation / conflicts; SEI interaction with Carbon specifically **not researched here** |
| **Hold / modifier-only (Fn) UX** | First-class via HID + dual path | Would need HID/flagsChanged path if parity with Fn-style PTT desired |
| **Mouse / hardware button** | First-class mouse buttons + BLE ring | Not in scope of Carbon-only Notch hotkeys |
| **Parity takeaway** | Wispr’s “flawless” feel is **not** “Carbon like Station”; it’s **Accessibility-trusted CGEvent tap + HID Fn + SEI monitoring + separate hold path**. Matching Wispr ≠ copying Carbon. Matching Station’s low-permission Notch toggle ≠ matching Wispr’s dictation stack. |

**Practical recommendation for Station:**

1. Keep **Carbon** for simple global toggles (⌥Space) if the goal is **no Input Monitoring** and Developer ID / Debug resilience — Wispr does **not** validate Carbon as their path.
2. If Station wants **Wispr-like** multi-binding / Fn / mouse / SEI-aware UX, expect **Accessibility** (already needed for any AX insert) + likely **CGEventTap** (and clarify Input Monitoring empirically on target macOS), plus optional **IOHID** for Fn — not more Carbon alone.
3. Do **not** treat Wispr’s silence on Input Monitoring as proof CGEventTap needs none; Apple engineer guidance says otherwise — verify on a clean Mac under System Settings → Privacy & Security.

---

## Sources

### Wispr first-party

| URL | Supports |
|-----|----------|
| https://docs.wisprflow.ai/articles/3152211871-setup-guide | Permissions onboarding; Fn / Ctrl+Opt defaults; mouse buttons; shortcut rules |
| https://docs.wisprflow.ai/articles/1036674442-supported-devices-and-system-requirements | Required Mac permissions list (Mic, Accessibility, optional Screen/System Audio); MDM note |
| https://docs.wisprflow.ai/articles/5510622673-re-verify-wispr-flow-permissions-after-updating | Shortcuts need Accessibility; no Input Monitoring in re-verify steps |
| https://docs.wisprflow.ai/articles/2612050838-supported-unsupported-keyboard-hotkey-shortcuts | Shortcut validation, reserved keys, Fn external-keyboard limits, SEI troubleshooting |
| https://docs.wisprflow.ai/articles/5298382595-route-dictation-directly-to-slack-email-or-calendar-with-keyboard-shortcuts | Shortcut UX; SEI warning; hold-to-talk still works |
| https://docs.wisprflow.ai/articles/8841649969-fix-flow-shortcuts-blocked-by-macos-secure-keyboard-entry-secure-event-input | Dual input path claim; SEI remediation |
| https://docs.wisprflow.ai/articles/1078330210-fix-logitech-mx-master-logi-options-mouse-buttons-not-working-as-a-flow-shortcut | Hardware mouse / Logi Options+ workaround |
| https://docs.wisprflow.ai/articles/3884018196-completely-removing-wispr-flow-from-your-device | Uninstall resets Mic/ScreenCapture; Accessibility manual — still no Input Monitoring |
| https://wisprflow.ai/privacy | Privacy marketing; no hotkey/TCC technical claims |
| Local `/Applications/Wispr Flow.app` (1.6.224) | Info.plist, entitlements, codesign, otool, nm, strings |

### Apple

| URL | Supports |
|-----|----------|
| https://developer.apple.com/library/archive/technotes/tn2150/_index.html | Secure Event Input blocks keyboard intercept processes including CGEvent taps |
| https://developer.apple.com/documentation/coregraphics/cgevent/tapcreate(tap:place:options:eventsofinterest:callback:userinfo:) | `CGEventTapCreate` / Swift `CGEvent.tapCreate` API (redirect target) |
| https://developer.apple.com/forums/thread/735223 | Apple staff: hotkey options; CGEventTap ↔ Input Monitoring; Carbon HotKey legacy note |
| https://developer.apple.com/forums/thread/707680 | Apple staff: NSEvent global monitor ↔ Accessibility; CGEventTap ↔ Input Monitoring |

### Explicitly not used as claim sources

Secondary SEO blogs / aggregator “Input Monitoring” claims that contradicted Wispr’s own required-permission list. Search snippets were only used to **discover** primary `docs.wisprflow.ai` URLs.

---

## Open questions / could not verify from primary sources

1. **Does installing/running Flow create an Input Monitoring TCC entry** for `com.electron.wispr-flow` and/or `…accessibility-mac-app`? (TCC.db access denied on this machine.)
2. Exact **CGEventTap** placement/options (`cgSessionEventTap` vs HID; listen-only vs default) — would need deeper disassembly.
3. Exact **hold-to-talk** implementation (IOHID element callbacks vs flagsChanged vs partial tap) — docs assert a different path; binary consistent with HID Fn, not proven.
4. Whether Electron ever calls **`globalShortcut` / RegisterEventHotKey`** for any Flow binding (helper evidence points to Swift taps instead).
5. Full product docs for **BLE mic ring** hardware (button semantics, pairing UX) beyond plist/binary strings.
6. Interaction of **Carbon `RegisterEventHotKey`** with Secure Event Input (relevant to Station, not Wispr) — out of scope for Wispr binary; needs Apple/Station experiment.
7. Source code / changelog entries naming these APIs — **unavailable** (closed client).

---

## Method notes

- Primary-only claims preferred: Wispr Help Center, local notarized binary metadata/symbols, Apple TN + documentation + Apple staff forums.
- No malware install; inspection limited to already-installed `/Applications/Wispr Flow.app`.
- No deep decompilation / debugger attach; confidence capped at symbols + strings + docs.
