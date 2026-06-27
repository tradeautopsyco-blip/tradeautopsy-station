import SwiftUI

/// Aero-Glass design tokens — TradeAutopsy notch.
enum NotchTheme {
    // Backgrounds
    static let bgApp = Color(hex: "#050505")
    static let bgCard = Color(hex: "#030303")
    static let bgCardGlass = Color(hex: "#030303").opacity(0.8)
    static let bgSurface = Color.white.opacity(0.04)
    static let bgSurfaceHover = Color.white.opacity(0.07)

    // Borders
    static let borderDefault = Color.white.opacity(0.06)
    static let borderSubtle = Color.white.opacity(0.03)
    static let borderActive = Color.white.opacity(0.12)

    // Text
    static let textPrimary = Color.white
    static let textSecondary = Color.white.opacity(0.5)
    static let textTertiary = Color.white.opacity(0.3)
    static let textMicro = Color.white.opacity(0.25)

    // Accents
    static let accentTeal = Color(hex: "#00E5C0")
    static let accentDanger = Color(hex: "#FF3B30")
    static let accentWarning = Color(hex: "#FF9500")
    static let accentBlue = Color(hex: "#0A84FF")

    // Glows
    static let glowTeal = Color(hex: "#00E5C0").opacity(0.15)
    static let glowDanger = Color(hex: "#FF3B30").opacity(0.15)
    static let glowWarning = Color(hex: "#FF9500").opacity(0.12)

    // Typography scale
    static let fontMicro: CGFloat = 9
    static let fontSmall: CGFloat = 10
    static let fontBody: CGFloat = 11
    static let fontData: CGFloat = 13
    static let fontLarge: CGFloat = 20
    static let fontHero: CGFloat = 28

    static let trackingMicro: CGFloat = 0.15
    static let trackingSmall: CGFloat = 0.08

    // Radii
    static let radiusSmall: CGFloat = 8
    static let radiusMedium: CGFloat = 12
    static let radiusLarge: CGFloat = 16
    static let radiusXL: CGFloat = 20

    static func scoreColor(_ score: Double) -> Color {
        if score < 0.20 { return accentTeal }
        if score < 0.30 { return accentWarning }
        return accentDanger
    }

    static func scoreGlow(_ score: Double) -> Color {
        if score < 0.20 { return glowTeal }
        if score < 0.30 { return glowWarning }
        return glowDanger
    }

    // MARK: - Legacy names (call sites / haptics)

    static var accent: Color { accentTeal }
    static var danger: Color { accentDanger }
    static var warning: Color { accentWarning }
    static var pureBlack: Color { bgApp }
    static var cardFill: Color { bgCard }
    static var baseDark: Color { bgApp }
    static var glassBorder: Color { borderDefault }
    static var labelCaps: Color { textMicro }
    static var secondaryText: Color { textSecondary }
    static var tertiaryText: Color { textTertiary }

    static func rounded(_ size: CGFloat, weight: Font.Weight = .medium) -> Font {
        .system(size: size, weight: weight, design: .rounded)
    }

    static func mono(_ size: CGFloat, weight: Font.Weight = .regular) -> Font {
        .system(size: size, weight: weight, design: .monospaced)
    }

    static let springExpand = Animation.spring(response: 0.4, dampingFraction: 0.85, blendDuration: 0.2)
    static let springRing = Animation.spring(response: 0.7, dampingFraction: 0.8)
}
