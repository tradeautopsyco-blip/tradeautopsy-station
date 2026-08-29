import AppKit
import SwiftUI

/// Aero-Glass design tokens — TradeAutopsy notch.
///
/// Motion tokens follow Apple *Designing Fluid Interfaces*: critically damped springs
/// for chrome (no bounce); underdamped only when momentum/gesture justifies it.
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
    static let accentTeal = Color(hex: "#63E6E2")
    static let accentDanger = Color(hex: "#FF453A")
    static let accentWarning = Color(hex: "#FF9F0A")
    static let accentBlue = Color(hex: "#0A84FF")

    // Glows
    static let glowTeal = Color(hex: "#63E6E2").opacity(0.15)
    static let glowDanger = Color(hex: "#FF453A").opacity(0.15)
    static let glowWarning = Color(hex: "#FF9F0A").opacity(0.12)

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

    /// Expand / collapse chrome — critically damped (Apple default UI spring).
    static let springExpand = Animation.spring(response: 0.35, dampingFraction: 1.0, blendDuration: 0)

    /// ⌥Space summon — Spotlight pop: opacity + slight scale only, no travel, no bounce
    /// (a keypress has no flick momentum). Interruptible: retargets from presentation value.
    static let springSummon = Animation.spring(response: 0.22, dampingFraction: 1.0, blendDuration: 0)

    /// Scale the expanded surface enters from (Spotlight ≈ 0.96 → 1.0). Reduce Motion: no scale.
    static var summonScaleFrom: CGFloat {
        NSWorkspace.shared.accessibilityDisplayShouldReduceMotion ? 1.0 : 0.96
    }

    /// How long the `NSPanel` holds the expanded frame after collapse so the exit fade can
    /// play before the window snaps back to the pill. Never animate the frame itself.
    static var summonFrameHoldDuration: TimeInterval {
        NSWorkspace.shared.accessibilityDisplayShouldReduceMotion ? 0.2 : 0.18
    }

    /// Sidebar / content remaps — snappier critical settle.
    static let springContent = Animation.spring(response: 0.28, dampingFraction: 1.0, blendDuration: 0)

    /// Score / ring pulses that ride continuous state — slight underdamping OK.
    static let springRing = Animation.spring(response: 0.55, dampingFraction: 0.9, blendDuration: 0)

    /// Honors Reduce Motion: short cross-fade instead of spring travel.
    static var expandCollapseAnimation: Animation {
        if NSWorkspace.shared.accessibilityDisplayShouldReduceMotion {
            return .easeInOut(duration: 0.2)
        }
        return springSummon
    }

    static var contentAnimation: Animation {
        if NSWorkspace.shared.accessibilityDisplayShouldReduceMotion {
            return .easeInOut(duration: 0.15)
        }
        return springContent
    }
}

/// Instant press-down scale — Apple: feedback on pointer-down, not release.
struct NotchPressButtonStyle: ButtonStyle {
    var pressedScale: CGFloat = 0.97

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .scaleEffect(configuration.isPressed ? pressedScale : 1)
            .animation(
                configuration.isPressed
                    ? .easeOut(duration: 0.08)
                    : .spring(response: 0.28, dampingFraction: 1.0),
                value: configuration.isPressed
            )
    }
}
