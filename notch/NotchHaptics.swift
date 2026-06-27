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
    /// macOS uses `NSHapticFeedbackManager`.
    static func play(_ kind: NotchHapticKind) {
        let performer = NSHapticFeedbackManager.defaultPerformer
        switch kind {
        case .success, .light:
            performer.perform(.levelChange, performanceTime: .default)
        case .warning, .medium:
            performer.perform(.alignment, performanceTime: .default)
        case .error, .heavy:
            performer.perform(.generic, performanceTime: .default)
        }
    }
}
