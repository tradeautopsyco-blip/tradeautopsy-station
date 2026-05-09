import AppKit
import SwiftUI

// MARK: - Visual effect (AppKit)

struct VisualEffectView: NSViewRepresentable {
    var material: NSVisualEffectView.Material
    var blendingMode: NSVisualEffectView.BlendingMode

    func makeNSView(context: Context) -> NSVisualEffectView {
        let view = NSVisualEffectView()
        view.material = material
        view.blendingMode = blendingMode
        view.state = .active
        return view
    }

    func updateNSView(_ nsView: NSVisualEffectView, context: Context) {}
}

// MARK: - Glass modifiers

struct GlassCard: ViewModifier {
    var radius: CGFloat = 12

    func body(content: Content) -> some View {
        content
            .background(
                ZStack {
                    RoundedRectangle(cornerRadius: radius, style: .continuous)
                        .fill(Color(hex: "#030303").opacity(0.8))
                    RoundedRectangle(cornerRadius: radius, style: .continuous)
                        .fill(Color.white.opacity(0.03))
                }
            )
            .overlay(
                RoundedRectangle(cornerRadius: radius, style: .continuous)
                    .stroke(Color.white.opacity(0.06), lineWidth: 0.5)
            )
    }
}

struct GlassPanel: ViewModifier {
    var radius: CGFloat = 16

    func body(content: Content) -> some View {
        content
            .background(
                ZStack {
                    VisualEffectView(
                        material: .hudWindow,
                        blendingMode: .behindWindow
                    )
                    RoundedRectangle(cornerRadius: radius, style: .continuous)
                        .fill(Color.black.opacity(0.75))
                    RoundedRectangle(cornerRadius: radius, style: .continuous)
                        .fill(Color.white.opacity(0.02))
                }
                .clipShape(
                    RoundedRectangle(cornerRadius: radius, style: .continuous)
                )
            )
            .overlay(
                RoundedRectangle(cornerRadius: radius, style: .continuous)
                    .stroke(Color.white.opacity(0.06), lineWidth: 0.5)
            )
    }
}

struct MicroLabel: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(.system(size: 9, weight: .medium, design: .monospaced))
            .foregroundColor(Color.white.opacity(0.3))
            .tracking(1.5)
            .textCase(.uppercase)
    }
}

extension View {
    func glassCard(radius: CGFloat = 12) -> some View {
        modifier(GlassCard(radius: radius))
    }

    func glassPanel(radius: CGFloat = 16) -> some View {
        modifier(GlassPanel(radius: radius))
    }

    func glowEffect(_ color: Color, radius: CGFloat = 20) -> some View {
        shadow(color: color, radius: radius, x: 0, y: 0)
    }

    func microLabel() -> some View {
        modifier(MicroLabel())
    }
}

// MARK: - Shared chrome (module-wide)

@ViewBuilder
func sectionHeader(_ text: String) -> some View {
    Text(text)
        .font(.system(size: 9, weight: .semibold, design: .monospaced))
        .foregroundColor(Color.white.opacity(0.25))
        .tracking(1.5)
        .textCase(.uppercase)
        .padding(.bottom, 4)
}

@ViewBuilder
func divider() -> some View {
    Rectangle()
        .fill(Color.white.opacity(0.05))
        .frame(height: 0.5)
        .padding(.vertical, 2)
}

@ViewBuilder
func emptyState(_ message: String) -> some View {
    HStack {
        Spacer()
        Text(message)
            .font(.system(size: 10, weight: .regular, design: .rounded))
            .foregroundColor(Color.white.opacity(0.2))
        Spacer()
    }
    .padding(.vertical, 8)
}

@ViewBuilder
func primaryButton(_ label: String, accessibilityLabel: String? = nil, action: @escaping () -> Void) -> some View {
    Button(action: action) {
        Text(label)
            .font(.system(size: 11, weight: .semibold, design: .rounded))
            .foregroundColor(Color(hex: "#050505"))
            .frame(maxWidth: .infinity)
            .padding(.vertical, 9)
    }
    .buttonStyle(.plain)
    .accessibilityLabel(accessibilityLabel ?? label)
    .background(Color(hex: "#00E5C0"))
    .cornerRadius(10)
    .shadow(
        color: Color(hex: "#00E5C0").opacity(0.3),
        radius: 10, x: 0, y: 0
    )
}

@ViewBuilder
func dangerButton(_ label: String, accessibilityLabel: String? = nil, action: @escaping () -> Void) -> some View {
    Button(action: action) {
        Text(label)
            .font(.system(size: 11, weight: .semibold, design: .rounded))
            .foregroundColor(Color(hex: "#FF3B30"))
            .frame(maxWidth: .infinity)
            .padding(.vertical, 9)
    }
    .buttonStyle(.plain)
    .accessibilityLabel(accessibilityLabel ?? label)
    .background(Color(hex: "#FF3B30").opacity(0.08))
    .cornerRadius(10)
    .overlay(
        RoundedRectangle(cornerRadius: 10)
            .stroke(Color(hex: "#FF3B30").opacity(0.25), lineWidth: 0.5)
    )
}

@ViewBuilder
func ghostButton(_ label: String, accessibilityLabel: String? = nil, action: @escaping () -> Void) -> some View {
    Button(action: action) {
        Text(label)
            .font(.system(size: 10, weight: .medium, design: .rounded))
            .foregroundColor(Color.white.opacity(0.4))
            .frame(maxWidth: .infinity)
            .padding(.vertical, 8)
    }
    .buttonStyle(.plain)
    .accessibilityLabel(accessibilityLabel ?? label)
    .background(Color.white.opacity(0.03))
    .cornerRadius(10)
    .overlay(
        RoundedRectangle(cornerRadius: 10)
            .stroke(Color.white.opacity(0.06), lineWidth: 0.5)
    )
}
