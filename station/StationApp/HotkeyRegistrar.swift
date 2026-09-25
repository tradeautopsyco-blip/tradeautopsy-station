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
@MainActor
public final class HotkeyRegistrar: HotkeyRegistering {
    private static let logger = Logger(subsystem: "in.tradeautopsy.station", category: "hotkeys")
    private static let hotKeySignature: OSType = 0x5441_5348 // 'TASH'

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
    private var eventHandlerRef: EventHandlerRef?
    private var toggleHotKeyRef: EventHotKeyRef?
    private var openHotKeyRef: EventHotKeyRef?
    private var carbonInstalled = false
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

    public func unregisterAll() {
        tearDownCarbonHotKeys()
        toggleNotchHandler = nil
        openStationHandler = nil
    }

    /// Re-install Carbon hotkeys if handlers exist (no Input Monitoring required).
    public func refreshGlobalMonitorIfNeeded() {
        installCarbonHotKeysIfNeeded()
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
        guard !carbonInstalled else { return }

        let status = installEventHandlerIfNeeded()
        guard status == noErr else {
            Self.logger.error("Carbon hotkey event handler failed to install (OSStatus \(status))")
            return
        }

        if toggleNotchHandler != nil {
            let id = EventHotKeyID(signature: Self.hotKeySignature, id: HotKeyID.toggleNotch.rawValue)
            let err = RegisterEventHotKey(
                UInt32(kVK_Space),
                UInt32(optionKey),
                id,
                GetApplicationEventTarget(),
                0,
                &toggleHotKeyRef
            )
            if err != noErr {
                Self.logger.error("RegisterEventHotKey ⌥Space failed (OSStatus \(err))")
            }
        }

        if openStationHandler != nil {
            let id = EventHotKeyID(signature: Self.hotKeySignature, id: HotKeyID.openStation.rawValue)
            let err = RegisterEventHotKey(
                UInt32(kVK_Space),
                UInt32(optionKey | shiftKey),
                id,
                GetApplicationEventTarget(),
                0,
                &openHotKeyRef
            )
            if err != noErr {
                Self.logger.error("RegisterEventHotKey ⌥⇧Space failed (OSStatus \(err))")
            }
        }

        carbonInstalled = toggleHotKeyRef != nil || openHotKeyRef != nil
        if carbonInstalled {
            Self.logger.info("Carbon system hotkeys registered (⌥Space / ⌥⇧Space) — Input Monitoring not required")
        }
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
        carbonInstalled = false
    }

    private func handleCarbonHotKey(id: UInt32) {
        switch HotKeyID(rawValue: id) {
        case .openStation:
            openStationHandler?()
        case .toggleNotch:
            toggleNotchHandler?()
        case nil:
            break
        }
    }

    private func handleKeyDown(_ event: NSEvent) {
        if Self.isOpenStationShortcut(event) {
            openStationHandler?()
            return
        }
        if Self.isToggleNotchShortcut(event) {
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
