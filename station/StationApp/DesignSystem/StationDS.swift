import SwiftUI

/// Station's design tokens. Re-homed from the notch package (was `StationDS`) during the
/// Notch dependency severance; values unchanged.
public enum StationDS {
    public enum Fill {
        public static let appPanel = Color(hex: "#0d0d0d")
        public static let sidebar = Color(hex: "#0a0a0a")
        public static let card = Color(hex: "#111111")
        public static let input = Color.white.opacity(0.03)
        public static let inputFocused = Color.white.opacity(0.05)
    }
    public enum Text {
        public static let primary = Color(hex: "#ededed")
        public static let secondary = Color(hex: "#666666")
        public static let muted = Color(hex: "#444444")
        public static let labels = Color(hex: "#333333")
        public static let hint = Color(hex: "#555555")
    }
    public enum Accent {
        public static let teal = Color(hex: "#00e5c0")
        public static let amber = Color(hex: "#eab308")
        public static let red = Color(hex: "#ef4444")
        public static let green = Color(hex: "#22c55e")
        public static let blue = Color(hex: "#60a5fa")
        static let blueFill = Color(hex: "#3b82f6").opacity(0.08)
        static let blueBorder = Color(hex: "#3b82f6").opacity(0.25)
    }
    public enum Border {
        public static let card = Color.white.opacity(0.07)
        public static let section = Color.white.opacity(0.06)
        public static let input = Color.white.opacity(0.10)
        public static let inputFocused = Color.white.opacity(0.25)
        public static let row = Color.white.opacity(0.05)
        public static let divider = Color.white.opacity(0.05)
        public static let subtle = Color.white.opacity(0.08)
        public static let chipUnselected = Color.white.opacity(0.10)
        public static let chipSelected = Color.white.opacity(0.20)
        public static let outlineBtn = Color.white.opacity(0.15)
    }
    enum Semantic {
        static func tealBg() -> Color { Color(hex: "#00e5c0").opacity(0.06) }
        static func tealBorder() -> Color { Color(hex: "#00e5c0").opacity(0.20) }
        static func amberBg() -> Color { Color(hex: "#eab308").opacity(0.06) }
        static func amberBorder() -> Color { Color(hex: "#eab308").opacity(0.20) }
        static func redBg() -> Color { Color(hex: "#ef4444").opacity(0.06) }
        static func redBorder() -> Color { Color(hex: "#ef4444").opacity(0.20) }
        static func greenBg() -> Color { Color(hex: "#22c55e").opacity(0.06) }
        static func greenBorder() -> Color { Color(hex: "#22c55e").opacity(0.20) }
    }
    public enum Radius {
        public static let card: CGFloat = 8
        public static let small: CGFloat = 6
        public static let pill: CGFloat = 20
    }
    public static let borderThin: CGFloat = 0.5
    public enum FontSize {
        public static let body: CGFloat = 13
        public static let bodySmall: CGFloat = 12
        public static let bodyXS: CGFloat = 11
        public static let sectionLabel: CGFloat = 10
        public static let metricValue: CGFloat = 20
        public static let brief: CGFloat = 16
        public static let topbarTitle: CGFloat = 14
        public static let chip: CGFloat = 11
    }
    public static func bodyFont(_ size: CGFloat = FontSize.body, weight: Font.Weight = .regular) -> Font {
        .system(size: size, weight: weight)
    }
    public static func monoFont(_ size: CGFloat, weight: Font.Weight = .medium) -> Font {
        .system(size: size, weight: weight, design: .monospaced)
    }
}
