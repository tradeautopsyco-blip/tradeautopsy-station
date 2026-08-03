import Foundation

/// Pure presentation rules for Station Slice B L2/L3 fullscreen overlay (#189).
enum KillSwitchOverlayPresentation {
    static func shouldShowFullscreenOverlay(
        active: Bool,
        level: String?,
        countdownSecs: Int?
    ) -> Bool {
        guard active else { return false }
        let normalized = level?.uppercased()
        if normalized == "L2" || normalized == "L3" || normalized == "2" || normalized == "3" {
            return true
        }
        return countdownSecs != nil
    }

    static func calmButtonEnabled(countdownSecs: Int?) -> Bool {
        guard let secs = countdownSecs else { return false }
        return secs <= 0
    }

    static func formattedCountdown(secs: Int) -> String {
        let clamped = max(0, secs)
        let minutes = clamped / 60
        let seconds = clamped % 60
        return "\(minutes):\(String(format: "%02d", seconds))"
    }

    static func calmButtonTitle(countdownSecs: Int?) -> String {
        guard let secs = countdownSecs else { return "I'm Calm" }
        if secs <= 0 { return "I'm Calm" }
        if secs <= 30 { return "I'm Calm (\(secs)s)" }
        return "I'm Calm (wait \(secs)s)"
    }
}
