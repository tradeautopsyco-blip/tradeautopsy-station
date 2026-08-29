import AppKit

enum NotchHapticKind {
    case success
    case warning
    case error
    /// Smart-trigger intensity mapping (BoringNotch-style).
    case light
    case medium
    case heavy
}

enum NotchHaptics {
    /// macOS uses `NSHapticFeedbackManager`. Pass `.now` for events that must land on the
    /// same frame as the first pixels (⌥Space summon); `.default` waits for the next draw.
    static func play(
        _ kind: NotchHapticKind,
        at time: NSHapticFeedbackManager.PerformanceTime = .default
    ) {
        let performer = NSHapticFeedbackManager.defaultPerformer
        switch kind {
        case .success, .light:
            performer.perform(.levelChange, performanceTime: time)
        case .warning, .medium:
            performer.perform(.alignment, performanceTime: time)
        case .error, .heavy:
            performer.perform(.generic, performanceTime: time)
        }
    }
}
