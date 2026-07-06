import AppKit
import Foundation
import IOKit.hid
import os

public protocol InputMonitoringChecking: Sendable {
    func isInputMonitoringGranted() -> Bool
    func requestInputMonitoringAccess()
}

public struct DefaultInputMonitoringChecker: InputMonitoringChecking {
    public init() {}

    public func isInputMonitoringGranted() -> Bool {
        if #available(macOS 10.15, *) {
            return CGPreflightListenEventAccess()
        }
        return true
    }

    public func requestInputMonitoringAccess() {
        if #available(macOS 10.15, *) {
            _ = IOHIDRequestAccess(kIOHIDRequestTypeListenEvent)
        }
    }
}

@MainActor
public final class HotkeyRegistrar: HotkeyRegistering {
    private static let logger = Logger(subsystem: "in.tradeautopsy.station", category: "hotkeys")

    private var toggleNotchHandler: (() -> Void)?
    private var openStationHandler: (() -> Void)?
    private var globalMonitor: Any?
    private var localMonitor: Any?
    private var didLogInputMonitoringRequirement = false
    private let inputMonitoringChecker: InputMonitoringChecking

    public init(inputMonitoringChecker: InputMonitoringChecking = DefaultInputMonitoringChecker()) {
        self.inputMonitoringChecker = inputMonitoringChecker
    }

    public func registerToggleNotch(_ handler: @escaping () -> Void) {
        toggleNotchHandler = handler
        installMonitorsIfNeeded()
    }

    public func registerOpenStation(_ handler: @escaping () -> Void) {
        openStationHandler = handler
        installMonitorsIfNeeded()
    }

    public func unregisterAll() {
        if let monitor = globalMonitor {
            NSEvent.removeMonitor(monitor)
            globalMonitor = nil
        }
        if let monitor = localMonitor {
            NSEvent.removeMonitor(monitor)
            localMonitor = nil
        }
        toggleNotchHandler = nil
        openStationHandler = nil
    }

    public func inputMonitoringWarningIfNeeded() -> InputMonitoringWarning? {
        guard !inputMonitoringChecker.isInputMonitoringGranted() else { return nil }
        return InputMonitoringWarning()
    }

    func dispatchKeyDownForTesting(_ event: NSEvent) {
        handleKeyDown(event)
    }

    private func installMonitorsIfNeeded() {
        guard globalMonitor == nil, localMonitor == nil else { return }
        guard toggleNotchHandler != nil || openStationHandler != nil else { return }

        inputMonitoringChecker.requestInputMonitoringAccess()
        logGlobalHotkeyAvailabilityIfNeeded()

        if inputMonitoringChecker.isInputMonitoringGranted() {
            globalMonitor = NSEvent.addGlobalMonitorForEvents(matching: .keyDown) { [weak self] event in
                DispatchQueue.main.async {
                    self?.handleKeyDown(event)
                }
            }
            if globalMonitor == nil {
                Self.logger.error(
                    "Global hotkey monitor failed to install; Option+Space and Option+Shift+Space will not work system-wide."
                )
            }
        }

        localMonitor = NSEvent.addLocalMonitorForEvents(matching: .keyDown) { [weak self] event in
            guard let self, Self.matchesAnyShortcut(event) else { return event }
            self.handleKeyDown(event)
            return nil
        }
    }

    private func logGlobalHotkeyAvailabilityIfNeeded() {
        guard !didLogInputMonitoringRequirement else { return }
        didLogInputMonitoringRequirement = true

        guard !inputMonitoringChecker.isInputMonitoringGranted() else { return }

        Self.logger.warning(
            """
            Input Monitoring permission is not granted. Global hotkeys are disabled until \
            permission is granted in System Settings > Privacy & Security > Input Monitoring \
            and TradeAutopsy Station is quit and reopened. \
            Option+Shift+Space (Open Station) and Option+Space (Toggle Notch) only work \
            while this app is the active (key) application without that permission.
            """
        )
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
        event.keyCode == 49
            && event.modifierFlags.intersection(.deviceIndependentFlagsMask) == .option
    }

    static func isOpenStationShortcut(_ event: NSEvent) -> Bool {
        guard event.keyCode == 49 else { return false }
        let flags = event.modifierFlags.intersection(.deviceIndependentFlagsMask)
        return flags.contains(.option) && flags.contains(.shift)
            && !flags.contains(.command) && !flags.contains(.control)
    }

    private static func matchesAnyShortcut(_ event: NSEvent) -> Bool {
        isToggleNotchShortcut(event) || isOpenStationShortcut(event)
    }
}
