import AppKit
import Carbon
import Foundation
import os

public protocol InputMonitoringChecking: Sendable {
    func isInputMonitoringGranted() -> Bool
    func requestInputMonitoringAccess()
}

public struct DefaultInputMonitoringChecker: InputMonitoringChecking {
    public init() {}

    public func isInputMonitoringGranted() -> Bool {
        // ⌥Space / ⌥⇧Space use Carbon RegisterEventHotKey — Input Monitoring is not used.
        true
    }

    public func requestInputMonitoringAccess() {
        // No-op: requesting IOHID listen access would show a TCC prompt we do not need.
    }
}

/// Registers ⌥Space / ⌥⇧Space via Carbon `RegisterEventHotKey`.
/// System hotkeys work over other apps without Input Monitoring (unlike `NSEvent` global monitors).
/// Saved Notch rows replace a default only when that action id is in the list. Empty prefs add nothing.
@MainActor
public final class HotkeyRegistrar: HotkeyRegistering {
    private static let logger = Logger(subsystem: "in.tradeautopsy.station", category: "hotkeys")
    private static let hotKeySignature: OSType = 0x5441_5348 // 'TASH'
    /// Saved rows start here so they never collide with the two shell defaults.
    private static let firstSavedHotKeyID: UInt32 = 10

    private enum HotKeyID: UInt32 {
        case toggleNotch = 1
        case openStation = 2
    }

    /// Ignore Caps Lock / Fn / numeric pad so matching isn't brittle.
    private static let significantModifiers: NSEvent.ModifierFlags = [
        .shift, .control, .option, .command,
    ]

    private var toggleNotchHandler: (() -> Void)?
    private var openStationHandler: (() -> Void)?
    private var deskActionHandler: ((String) -> Void)?
    private var eventHandlerRef: EventHandlerRef?
    private var toggleHotKeyRef: EventHotKeyRef?
    private var openHotKeyRef: EventHotKeyRef?
    private var savedHotKeyRefs: [UInt32: EventHotKeyRef] = [:]
    private var idToAction: [UInt32: String] = [:]
    private var lastBindings: [DeskHotkeyRegistration] = []
    private var suppressedDefaultToggle = false
    private var suppressedDefaultOpen = false
    private let inputMonitoringChecker: InputMonitoringChecking

    public init(inputMonitoringChecker: InputMonitoringChecking = DefaultInputMonitoringChecker()) {
        self.inputMonitoringChecker = inputMonitoringChecker
    }

    public func registerToggleNotch(_ handler: @escaping () -> Void) {
        toggleNotchHandler = handler
        installCarbonHotKeysIfNeeded()
    }

    public func registerOpenStation(_ handler: @escaping () -> Void) {
        openStationHandler = handler
        installCarbonHotKeysIfNeeded()
    }

    public func registerDeskActions(_ handler: @escaping (String) -> Void) {
        deskActionHandler = handler
    }

    public func reloadSavedBindings(_ bindings: [DeskHotkeyRegistration]) {
        lastBindings = bindings
        applySavedBindings(bindings)
    }

    public func unregisterAll() {
        unregisterSavedHotKeys()
        tearDownCarbonHotKeys()
        toggleNotchHandler = nil
        openStationHandler = nil
        deskActionHandler = nil
        lastBindings = []
        suppressedDefaultToggle = false
        suppressedDefaultOpen = false
    }

    /// Re-install Carbon hotkeys if handlers exist (no Input Monitoring required).
    public func refreshGlobalMonitorIfNeeded() {
        installCarbonHotKeysIfNeeded()
        applySavedBindings(lastBindings)
    }

    /// Carbon hotkeys do not require Input Monitoring — never surface that banner for ⌥Space.
    public func inputMonitoringWarningIfNeeded() -> InputMonitoringWarning? {
        nil
    }

    func dispatchKeyDownForTesting(_ event: NSEvent) {
        handleKeyDown(event)
    }

    func dispatchCarbonHotKeyForTesting(id: UInt32) {
        handleCarbonHotKey(id: id)
    }

    private func installCarbonHotKeysIfNeeded() {
        guard toggleNotchHandler != nil || openStationHandler != nil else { return }
        let status = installEventHandlerIfNeeded()
        guard status == noErr else {
            Self.logger.error("Carbon hotkey event handler failed to install (OSStatus \(status))")
            return
        }
        if !suppressedDefaultToggle {
            registerDefaultToggleHotKey()
        }
        if !suppressedDefaultOpen {
            registerDefaultOpenHotKey()
        }
    }

    private func registerDefaultToggleHotKey() {
        guard toggleNotchHandler != nil, toggleHotKeyRef == nil, !suppressedDefaultToggle else { return }
        let id = EventHotKeyID(signature: Self.hotKeySignature, id: HotKeyID.toggleNotch.rawValue)
        var ref: EventHotKeyRef?
        let err = RegisterEventHotKey(
            UInt32(kVK_Space),
            UInt32(optionKey),
            id,
            GetApplicationEventTarget(),
            0,
            &ref
        )
        if err != noErr {
            Self.logger.error("RegisterEventHotKey ⌥Space failed (OSStatus \(err))")
            return
        }
        toggleHotKeyRef = ref
    }

    private func registerDefaultOpenHotKey() {
        guard openStationHandler != nil, openHotKeyRef == nil, !suppressedDefaultOpen else { return }
        let id = EventHotKeyID(signature: Self.hotKeySignature, id: HotKeyID.openStation.rawValue)
        var ref: EventHotKeyRef?
        let err = RegisterEventHotKey(
            UInt32(kVK_Space),
            UInt32(optionKey | shiftKey),
            id,
            GetApplicationEventTarget(),
            0,
            &ref
        )
        if err != noErr {
            Self.logger.error("RegisterEventHotKey ⌥⇧Space failed (OSStatus \(err))")
            return
        }
        openHotKeyRef = ref
    }

    private func applySavedBindings(_ bindings: [DeskHotkeyRegistration]) {
        unregisterSavedHotKeys()
        let hasToggle = bindings.contains { $0.actionId == "toggle_notch" }
        let hasOpen = bindings.contains { $0.actionId == "open_station" }
        setDefaultSuppressed(toggle: hasToggle, open: hasOpen)

        var nextId = Self.firstSavedHotKeyID
        var rows: [(id: UInt32, binding: DeskHotkeyRegistration)] = []
        for binding in bindings {
            let action = binding.actionId.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !action.isEmpty, binding.keyCode > 0 else { continue }
            idToAction[nextId] = action
            rows.append((nextId, binding))
            nextId += 1
        }

        let status = installEventHandlerIfNeeded()
        guard status == noErr else {
            Self.logger.error("Carbon hotkey event handler failed to install (OSStatus \(status))")
            return
        }

        for row in rows {
            var ref: EventHotKeyRef?
            let hotKeyID = EventHotKeyID(signature: Self.hotKeySignature, id: row.id)
            let err = RegisterEventHotKey(
                row.binding.keyCode,
                row.binding.carbonModifiers,
                hotKeyID,
                GetApplicationEventTarget(),
                0,
                &ref
            )
            if err != noErr {
                Self.logger.error("RegisterEventHotKey saved binding failed (OSStatus \(err))")
            } else if let ref {
                savedHotKeyRefs[row.id] = ref
            }
        }
    }

    private func setDefaultSuppressed(toggle: Bool, open: Bool) {
        if toggle {
            if let ref = toggleHotKeyRef {
                UnregisterEventHotKey(ref)
                toggleHotKeyRef = nil
            }
            suppressedDefaultToggle = true
        } else if suppressedDefaultToggle {
            suppressedDefaultToggle = false
            registerDefaultToggleHotKey()
        }

        if open {
            if let ref = openHotKeyRef {
                UnregisterEventHotKey(ref)
                openHotKeyRef = nil
            }
            suppressedDefaultOpen = true
        } else if suppressedDefaultOpen {
            suppressedDefaultOpen = false
            registerDefaultOpenHotKey()
        }
    }

    private func unregisterSavedHotKeys() {
        for (_, ref) in savedHotKeyRefs {
            UnregisterEventHotKey(ref)
        }
        savedHotKeyRefs.removeAll()
        idToAction.removeAll()
    }

    private func installEventHandlerIfNeeded() -> OSStatus {
        if eventHandlerRef != nil { return noErr }

        var eventType = EventTypeSpec(
            eventClass: OSType(kEventClassKeyboard),
            eventKind: UInt32(kEventHotKeyPressed)
        )
        let userData = Unmanaged.passUnretained(self).toOpaque()
        return InstallEventHandler(
            GetApplicationEventTarget(),
            { _, event, userData -> OSStatus in
                guard let userData, let event else { return noErr }
                var hotKeyID = EventHotKeyID()
                let size = MemoryLayout<EventHotKeyID>.size
                let paramStatus = GetEventParameter(
                    event,
                    EventParamName(kEventParamDirectObject),
                    EventParamType(typeEventHotKeyID),
                    nil,
                    size,
                    nil,
                    &hotKeyID
                )
                guard paramStatus == noErr else { return paramStatus }
                let registrar = Unmanaged<HotkeyRegistrar>.fromOpaque(userData).takeUnretainedValue()
                // Carbon delivers on the app event target (main thread) — handle synchronously
                // so the Notch summon starts on the same run-loop turn as the keypress.
                // An async hop here costs 1–2 frames of visible latency on ⌥Space.
                if Thread.isMainThread {
                    MainActor.assumeIsolated {
                        registrar.handleCarbonHotKey(id: hotKeyID.id)
                    }
                } else {
                    DispatchQueue.main.async {
                        registrar.handleCarbonHotKey(id: hotKeyID.id)
                    }
                }
                return noErr
            },
            1,
            &eventType,
            userData,
            &eventHandlerRef
        )
    }

    private func tearDownCarbonHotKeys() {
        if let ref = toggleHotKeyRef {
            UnregisterEventHotKey(ref)
            toggleHotKeyRef = nil
        }
        if let ref = openHotKeyRef {
            UnregisterEventHotKey(ref)
            openHotKeyRef = nil
        }
        if let handler = eventHandlerRef {
            RemoveEventHandler(handler)
            eventHandlerRef = nil
        }
    }

    private func handleCarbonHotKey(id: UInt32) {
        if let action = idToAction[id] {
            dispatchAction(action)
            return
        }
        switch HotKeyID(rawValue: id) {
        case .openStation:
            guard !suppressedDefaultOpen else { return }
            openStationHandler?()
        case .toggleNotch:
            guard !suppressedDefaultToggle else { return }
            toggleNotchHandler?()
        case nil:
            break
        }
    }

    private func dispatchAction(_ action: String) {
        switch action {
        case "toggle_notch":
            toggleNotchHandler?()
        case "open_station":
            openStationHandler?()
        default:
            deskActionHandler?(action)
        }
    }

    private func handleKeyDown(_ event: NSEvent) {
        if Self.isOpenStationShortcut(event) {
            guard !suppressedDefaultOpen else { return }
            openStationHandler?()
            return
        }
        if Self.isToggleNotchShortcut(event) {
            guard !suppressedDefaultToggle else { return }
            toggleNotchHandler?()
        }
    }

    static func isToggleNotchShortcut(_ event: NSEvent) -> Bool {
        guard event.keyCode == 49 else { return false }
        let flags = event.modifierFlags.intersection(significantModifiers)
        return flags == .option
    }

    static func isOpenStationShortcut(_ event: NSEvent) -> Bool {
        guard event.keyCode == 49 else { return false }
        let flags = event.modifierFlags.intersection(significantModifiers)
        return flags == [.option, .shift]
    }
}
