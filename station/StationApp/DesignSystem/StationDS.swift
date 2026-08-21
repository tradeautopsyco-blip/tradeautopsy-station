import SwiftUI

/// Station's design tokens — same Apple dark semantic set as NOTCH-ui.html / BarDS.
public enum StationDS {
    public enum Fill {
        public static let appPanel = Color(hex: "#0d0d0d")
        public static let sidebar = Color(hex: "#0a0a0a")
        public static let card = Color(hex: "#111111")
        public static let elevated = Color(hex: "#1c1c1e")
        public static let input = Color.white.opacity(0.03)
        public static let inputFocused = Color.white.opacity(0.05)
        public static let glass = Color(hex: "#050505").opacity(0.78)
        public static let accentInk = Color(hex: "#052e2c")
    }
    public enum Text {
        public static let primary = Color.white
        public static let secondary = Color.white.opacity(0.60)
        public static let muted = Color.white.opacity(0.30)
        public static let labels = Color.white.opacity(0.18)
        public static let hint = Color.white.opacity(0.30)
        public static let section = Color.white.opacity(0.50)
    }
    public enum Accent {
        public static let teal = Color(hex: "#63E6E2")
        public static let amber = Color(hex: "#FF9F0A")
        public static let red = Color(hex: "#FF453A")
        public static let green = Color(hex: "#30D158")
        public static let blue = Color(hex: "#0A84FF")
        static let blueFill = Color(hex: "#0A84FF").opacity(0.08)
        static let blueBorder = Color(hex: "#0A84FF").opacity(0.25)
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
        static func tealBg() -> Color { Accent.teal.opacity(0.06) }
        static func tealBorder() -> Color { Accent.teal.opacity(0.20) }
        static func amberBg() -> Color { Accent.amber.opacity(0.06) }
        static func amberBorder() -> Color { Accent.amber.opacity(0.20) }
        static func redBg() -> Color { Accent.red.opacity(0.06) }
        static func redBorder() -> Color { Accent.red.opacity(0.20) }
        static func greenBg() -> Color { Accent.green.opacity(0.06) }
        static func greenBorder() -> Color { Accent.green.opacity(0.20) }
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
        public static let sectionLabel: CGFloat = 11
        public static let chrome: CGFloat = 10
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
