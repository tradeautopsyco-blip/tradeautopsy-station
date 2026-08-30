import SwiftUI

// MARK: - Tokens (spec ground truth — PLAN notch v2 HTML reference)

public enum BarDS {
    public enum Fill {
        public static let appPanel = Color(hex: "#0d0d0d")
        public static let sidebar = Color(hex: "#0a0a0a")
        public static let card = Color(hex: "#111111")
        public static let elevated = Color(hex: "#1c1c1e")
        public static let input = Color.white.opacity(0.03)
        public static let inputFocused = Color.white.opacity(0.05)
        /// HTML `--glass`
        public static let glass = Color(hex: "#050505").opacity(0.78)
        /// Primary button ink on mint (`#052e2c`)
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
        /// Invalidation chip selected
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
        public static let sheet: CGFloat = 18
    }

    public static let borderThin: CGFloat = 0.5
    public static let sidebarWidth: CGFloat = 176

    public enum Motion {
        /// HTML `--spring: cubic-bezier(0.22, 1, 0.36, 1)`
        public static let spring = Animation.timingCurve(0.22, 1, 0.36, 1, duration: 0.35)
    }

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

// MARK: - Plan / banner dot semantics

/// Row background + state dot for `BarStateBanner` (maps from hosted `plan_state` strings in views).
enum BarPlanBannerKind: Equatable {
    case intact
    case watch
    case broken

    var dotColor: Color {
        switch self {
        case .intact: return BarDS.Accent.teal
        case .watch: return BarDS.Accent.amber
        case .broken: return BarDS.Accent.red
        }
    }

    var pulses: Bool {
        switch self {
        case .intact: return false
        case .watch, .broken: return true
        }
    }

    /// AMBER: 1.8s; RED: 0.9s
    var pulseDuration: Double {
        switch self {
        case .watch: return 1.8
        default: return 0.9
        }
    }

    var titleColor: Color {
        switch self {
        case .intact: return BarDS.Accent.teal
        case .watch: return BarDS.Accent.amber
        case .broken: return BarDS.Accent.red
        }
    }

    var shellBackground: Color {
        switch self {
        case .intact: return BarDS.Semantic.tealBg()
        case .watch: return BarDS.Semantic.amberBg()
        case .broken: return BarDS.Semantic.redBg()
        }
    }

    var shellBorder: Color {
        switch self {
        case .intact: return BarDS.Semantic.tealBorder()
        case .watch: return BarDS.Semantic.amberBorder()
        case .broken: return BarDS.Semantic.redBorder()
        }
    }
}

// MARK: - Flow layout (chips)

struct BarFlowLayout: Layout {
    var spacing: CGFloat = 5
    var rowSpacing: CGFloat = 5

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        layout(proposal: proposal, subviews: subviews).size
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let res = layout(proposal: proposal, subviews: subviews)
        for (i, sub) in subviews.enumerated() {
            let p = res.positions[i]
            sub.place(
                at: CGPoint(x: bounds.minX + p.x, y: bounds.minY + p.y),
                proposal: ProposedViewSize(res.sizes[i])
            )
        }
    }

    private func layout(proposal: ProposedViewSize, subviews: Subviews) -> (
        size: CGSize,
        positions: [CGPoint],
        sizes: [CGSize]
    ) {
        let maxW = proposal.width ?? .infinity
        var x: CGFloat = 0
        var y: CGFloat = 0
        var rowH: CGFloat = 0
        var positions: [CGPoint] = []
        var sizes: [CGSize] = []

        for sub in subviews {
            let s = sub.sizeThatFits(.init(width: maxW, height: nil))
            if x > 0, x + s.width > maxW {
                x = 0
                y += rowH + rowSpacing
                rowH = 0
            }
            positions.append(CGPoint(x: x, y: y))
            sizes.append(s)
            rowH = max(rowH, s.height)
            x += s.width + spacing
        }

        let totalH = y + rowH
        let totalW = maxW.isFinite ? maxW : (positions.isEmpty ? 0 : positions.map(\.x).max()! + (sizes.last?.width ?? 0))
        return (CGSize(width: totalW, height: totalH), positions, sizes)
    }
}

// MARK: - Components

struct BarCard<Content: View>: View {
    @ViewBuilder let content: () -> Content

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            content()
        }
        .padding(.vertical, 13)
        .padding(.horizontal, 15)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin)
        )
        .padding(.bottom, 8)
    }
}

struct BarSectionLabel: View {
    let text: String

    var body: some View {
        Text(text.uppercased())
            .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
            .foregroundColor(BarDS.Text.section)
            .kerning(0.08 * BarDS.FontSize.sectionLabel)
            .padding(.top, 16)
            .padding(.bottom, 8)
            .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct BarMetric: View {
    let label: String
    let value: String
    var valueColor: Color = BarDS.Text.primary
    var sub: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label.uppercased())
                .font(BarDS.bodyFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .kerning(0.06 * 10)
            Text(value)
                .font(BarDS.monoFont(BarDS.FontSize.metricValue, weight: .medium))
                .foregroundColor(valueColor)
            if let sub {
                Text(sub)
                    .font(BarDS.bodyFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

private struct PulsingDot: View {
    let color: Color
    let duration: Double
    @Environment(\.accessibilityReduceMotion) private var reduceMotion

    var body: some View {
        TimelineView(.animation(minimumInterval: 1.0 / 30.0, paused: reduceMotion)) { ctx in
            let t = ctx.date.timeIntervalSinceReferenceDate
            let amp = 0.5 + 0.5 * sin(t * (2 * .pi / duration))
            let op = reduceMotion ? 1.0 : (0.3 + 0.7 * amp)
            Circle()
                .fill(color)
                .frame(width: 8, height: 8)
                .opacity(op)
        }
    }
}

struct BarStateBanner: View {
    let kind: BarPlanBannerKind
    let label: String
    let sentence: String

    var body: some View {
        HStack(alignment: .top, spacing: 10) {
            Group {
                if kind.pulses {
                    PulsingDot(color: kind.dotColor, duration: kind.pulseDuration)
                } else {
                    Circle()
                        .fill(BarDS.Accent.teal)
                        .frame(width: 8, height: 8)
                }
            }
            VStack(alignment: .leading, spacing: 3) {
                Text(label.uppercased())
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(kind.titleColor)
                    .kerning(0.06 * 10)
                Text(sentence)
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                    .foregroundColor(kind.titleColor)
                    .lineSpacing(BarDS.FontSize.body * (1.5 - 1.0))
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 0)
        }
        .padding(12)
        .padding(.horizontal, 2)
        .background(kind.shellBackground)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(kind.shellBorder, lineWidth: BarDS.borderThin)
        )
        .padding(.bottom, 10)
    }
}

struct BarPlanRow: View {
    let key: String
    let value: String
    var valueColor: Color = BarDS.Text.primary
    var showDivider: Bool = true

    var body: some View {
        VStack(spacing: 0) {
            HStack {
                Text(key)
                    .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
                    .foregroundColor(BarDS.Text.hint)
                Spacer(minLength: 8)
                Text(value)
                    .font(BarDS.monoFont(BarDS.FontSize.bodyXS, weight: .medium))
                    .foregroundColor(valueColor)
                    .multilineTextAlignment(.trailing)
            }
            .padding(.vertical, 7)
            .padding(.horizontal, 12)
            if showDivider {
                Rectangle()
                    .fill(BarDS.Border.divider)
                    .frame(height: BarDS.borderThin)
            }
        }
    }
}

struct BarPlanRowsCard: View {
    let rows: [(String, String, Color)]

    var body: some View {
        VStack(spacing: 0) {
            ForEach(Array(rows.enumerated()), id: \.offset) { idx, row in
                BarPlanRow(
                    key: row.0,
                    value: row.1,
                    valueColor: row.2,
                    showDivider: idx < rows.count - 1
                )
            }
        }
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin)
        )
        .padding(.bottom, 10)
    }
}

struct BarCompositeLine: View {
    let leftText: String
    let rightText: String
    var rightColor: Color = BarDS.Text.primary

    var body: some View {
        HStack {
            Text(leftText)
                .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
            Spacer(minLength: 8)
            Text(rightText)
                .font(BarDS.monoFont(BarDS.FontSize.bodySmall, weight: .medium))
                .foregroundColor(rightColor)
        }
        .padding(.vertical, 8)
        .padding(.horizontal, 12)
        .background(Color.white.opacity(0.03))
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.subtle, lineWidth: BarDS.borderThin)
        )
        .padding(.bottom, 10)
    }
}

struct BarBigButton: View {
    enum Style {
        case primary
        case outline
        case danger
    }

    let label: String
    var style: Style = .primary
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(label)
                .font(BarDS.bodyFont(14, weight: .medium))
                .foregroundColor(foreground)
                .frame(maxWidth: .infinity)
                .padding(.vertical, 12)
                .background(background)
                .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
                .overlay(
                    RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                        .stroke(border, lineWidth: style == .primary ? 0 : BarDS.borderThin)
                )
        }
        .buttonStyle(.plain)
    }

    private var foreground: Color {
        switch style {
        case .primary: return BarDS.Fill.accentInk
        case .outline: return BarDS.Text.primary
        case .danger: return BarDS.Accent.red
        }
    }

    private var background: Color {
        switch style {
        case .primary: return BarDS.Accent.teal
        case .outline: return .clear
        case .danger: return BarDS.Accent.red.opacity(0.1)
        }
    }

    private var border: Color {
        switch style {
        case .primary: return .clear
        case .outline: return BarDS.Border.outlineBtn
        case .danger: return BarDS.Accent.red.opacity(0.25)
        }
    }
}

struct BarInputField: View {
    let placeholder: String
    @Binding var text: String
    var marginBottom: CGFloat = 7
    /// Return or focus loss. Fires on both so a pasted id binds without a suggestion row.
    var onCommit: (() -> Void)?

    @FocusState private var focused: Bool

    var body: some View {
        TextField(
            "",
            text: $text,
            prompt:
                Text(placeholder)
                .font(BarDS.bodyFont(12, weight: .regular))
                .foregroundColor(BarDS.Text.hint)
        )
        .textFieldStyle(.plain)
        .font(BarDS.monoFont(BarDS.FontSize.body, weight: .regular))
        .foregroundColor(BarDS.Text.primary)
        .padding(.vertical, 8)
        .padding(.horizontal, 10)
        .background(focused ? BarDS.Fill.inputFocused : BarDS.Fill.input)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(focused ? BarDS.Border.inputFocused : BarDS.Border.input, lineWidth: BarDS.borderThin)
        )
        .focused($focused)
        .onSubmit { onCommit?() }
        .onChange(of: focused) { _, isFocused in
            if !isFocused { onCommit?() }
        }
        .padding(.bottom, marginBottom)
    }
}

struct BarChip: View {
    let label: String
    let selected: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(label)
                .font(BarDS.bodyFont(BarDS.FontSize.chip, weight: .regular))
                .foregroundColor(selected ? Color.white : BarDS.Text.secondary)
                .padding(.vertical, 5)
                .padding(.horizontal, 11)
                .background(selected ? Color.white.opacity(0.16) : Color.white.opacity(0.03))
                .clipShape(Capsule())
                .overlay(
                    Capsule()
                        .stroke(
                            selected ? BarDS.Border.chipSelected : BarDS.Border.chipUnselected,
                            lineWidth: BarDS.borderThin
                        )
                )
        }
        .buttonStyle(.plain)
    }
}

struct BarTab: View {
    let label: String
    let active: Bool
    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(label)
                .font(BarDS.bodyFont(BarDS.FontSize.chip, weight: active ? .medium : .regular))
                .foregroundColor(active ? BarDS.Text.primary : BarDS.Text.muted)
                .padding(.vertical, 5)
                .padding(.horizontal, 12)
                .background(active ? Color.white.opacity(0.08) : Color.white.opacity(0.03))
                .clipShape(Capsule())
                .overlay(
                    Capsule()
                        .stroke(
                            active ? Color.white.opacity(0.15) : Color.white.opacity(0.08),
                            lineWidth: BarDS.borderThin
                        )
                )
        }
        .buttonStyle(.plain)
    }
}

struct BarFeelButton: View {
    enum FeelState {
        case unselected
        case teal
        case amber
        case red
    }

    let number: Int
    let label: String
    let state: FeelState

    var body: some View {
        VStack(spacing: 1) {
            Text("\(number)")
                .font(BarDS.bodyFont(15, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
            Text(label)
                .font(BarDS.bodyFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.hint)
        }
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity)
        .frame(minHeight: 56, maxHeight: 56)
        .background(background)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(borderColor, lineWidth: BarDS.borderThin)
        )
    }

    private var background: Color {
        switch state {
        case .unselected: return Color.white.opacity(0.03)
        case .teal: return BarDS.Accent.teal.opacity(0.08)
        case .amber: return BarDS.Accent.amber.opacity(0.08)
        case .red: return BarDS.Accent.red.opacity(0.08)
        }
    }

    private var borderColor: Color {
        switch state {
        case .unselected: return Color.white.opacity(0.08)
        case .teal: return BarDS.Accent.teal.opacity(0.20)
        case .amber: return BarDS.Accent.amber.opacity(0.20)
        case .red: return BarDS.Accent.red.opacity(0.20)
        }
    }
}

struct BarToggleRow: View {
    let label: String
    let sub: String
    @Binding var isOn: Bool

    var body: some View {
        HStack(alignment: .center) {
            VStack(alignment: .leading, spacing: 1) {
                Text(label)
                    .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                Text(sub)
                    .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
                    .foregroundColor(BarDS.Text.hint)
                    .fixedSize(horizontal: false, vertical: true)
            }
            Spacer(minLength: 12)
            BarCustomToggle(isOn: $isOn)
        }
        .padding(.vertical, 9)
        .padding(.horizontal, 12)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin)
        )
        .padding(.bottom, 8)
    }
}

private struct BarCustomToggle: View {
    @Binding var isOn: Bool

    var body: some View {
        ZStack(alignment: isOn ? .trailing : .leading) {
            Capsule()
                .fill(isOn ? BarDS.Accent.teal : Color.white.opacity(0.1))
                .frame(width: 36, height: 20)
            Circle()
                .fill(Color.white)
                .frame(width: 16, height: 16)
                .padding(2)
        }
        .frame(width: 36, height: 20)
        .contentShape(Rectangle())
        .onTapGesture {
            withAnimation(.linear(duration: 0.15)) {
                isOn.toggle()
            }
        }
        .accessibilityAddTraits(.isButton)
        .accessibilityLabel(isOn ? "On" : "Off")
    }
}

struct BarProgressBlock: View {
    enum FillStyle {
        case risk
        case fidelity
    }

    let label: String
    let valueText: String
    /// 0...1
    let pct: Double
    var fillStyle: FillStyle = .risk

    private var fillColor: Color {
        switch fillStyle {
        case .fidelity:
            return BarDS.Text.primary.opacity(0.55)
        case .risk:
            let p = pct * 100
            if p < 60 { return BarDS.Accent.teal }
            if p < 90 { return BarDS.Accent.amber }
            return BarDS.Accent.red
        }
    }

    private var valueTextColor: Color {
        switch fillStyle {
        case .fidelity: return BarDS.Text.primary
        case .risk: return fillColor
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 5) {
            HStack {
                Text(label)
                    .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
                    .foregroundColor(BarDS.Text.secondary)
                Spacer(minLength: 8)
                Text(valueText)
                    .font(BarDS.monoFont(BarDS.FontSize.bodyXS, weight: .medium))
                    .foregroundColor(valueTextColor)
            }
            GeometryReader { geo in
                let w = geo.size.width
                ZStack(alignment: .leading) {
                    Capsule()
                        .fill(Color.white.opacity(0.06))
                        .frame(height: 4)
                    Capsule()
                        .fill(fillColor)
                        .frame(width: max(0, CGFloat(pct) * w), height: 4)
                }
                .animation(.linear(duration: 0.4), value: pct)
            }
            .frame(height: 4)
        }
        .padding(.bottom, 10)
    }
}

struct BarNonNegotiableCard: View {
    let label: String
    let text: String
    enum Kind { case amber, red }
    let kind: Kind

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(label.uppercased())
                .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                .foregroundColor(kind == .amber ? BarDS.Accent.amber : BarDS.Accent.red)
                .kerning(0.08 * 10)
            Text(text)
                .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                .foregroundColor(BarDS.Text.primary)
                .lineSpacing(BarDS.FontSize.body * 0.55)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(.vertical, 13)
        .padding(.horizontal, 15)
        .background(kind == .amber ? BarDS.Semantic.amberBg() : BarDS.Semantic.redBg())
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(kind == .amber ? BarDS.Semantic.amberBorder() : BarDS.Semantic.redBorder(), lineWidth: BarDS.borderThin)
        )
        .padding(.bottom, 12)
    }
}

struct BarPatternCard: View {
    enum BadgeKind {
        case d
        case w
        case ok
    }

    let title: String
    let badge: String
    let badgeKind: BadgeKind
    let copy: String
    let cost: String
    let costPositive: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(alignment: .center) {
                Text(title)
                    .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                Spacer(minLength: 8)
                Text(badge)
                    .font(BarDS.bodyFont(10, weight: .medium))
                    .foregroundColor(badgeForeground)
                    .padding(.vertical, 2)
                    .padding(.horizontal, 8)
                    .background(badgeBackground)
                    .clipShape(Capsule())
            }
            .padding(.bottom, 6)
            Text(copy)
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .regular))
                .foregroundColor(BarDS.Text.secondary)
                .lineSpacing(BarDS.FontSize.bodyXS * 0.5)
                .padding(.bottom, 5)
            Text(cost)
                .font(BarDS.monoFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundColor(costPositive ? BarDS.Accent.green : BarDS.Accent.red)
        }
        .padding(.vertical, 12)
        .padding(.horizontal, 14)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin)
        )
        .padding(.bottom, 7)
    }

    private var badgeBackground: Color {
        switch badgeKind {
        case .d: return BarDS.Accent.red.opacity(0.15)
        case .w: return BarDS.Accent.amber.opacity(0.15)
        case .ok: return BarDS.Accent.green.opacity(0.15)
        }
    }

    private var badgeForeground: Color {
        switch badgeKind {
        case .d: return BarDS.Accent.red
        case .w: return BarDS.Accent.amber
        case .ok: return BarDS.Accent.green
        }
    }
}

struct BarAdherenceItem: View {
    enum Mark { case unchecked, yes, no }

    let text: String
    let sub: String
    let mark: Mark

    var body: some View {
        HStack(alignment: .center, spacing: 9) {
            ZStack {
                Circle()
                    .strokeBorder(borderCol, lineWidth: BarDS.borderThin)
                    .frame(width: 18, height: 18)
                    .background(
                        Circle()
                            .fill(innerFill)
                    )
                if mark == .yes {
                    Text("✓")
                        .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                        .foregroundColor(BarDS.Accent.teal)
                } else if mark == .no {
                    Text("✗")
                        .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                        .foregroundColor(BarDS.Accent.red)
                }
            }
            .frame(width: 18, height: 18)

            VStack(alignment: .leading, spacing: 1) {
                Text(text)
                    .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .regular))
                    .foregroundColor(Color(hex: "#aaaaaa"))
                Text(sub)
                    .font(BarDS.bodyFont(10, weight: .regular))
                    .foregroundColor(BarDS.Text.muted)
            }
            Spacer(minLength: 0)
        }
        .padding(.vertical, 9)
        .padding(.horizontal, 11)
        .background(rowBg)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(rowBorder, lineWidth: BarDS.borderThin)
        )
        .padding(.bottom, 5)
    }

    private var innerFill: Color {
        switch mark {
        case .unchecked: return .clear
        case .yes: return BarDS.Accent.teal.opacity(0.15)
        case .no: return BarDS.Accent.red.opacity(0.15)
        }
    }

    private var borderCol: Color {
        switch mark {
        case .unchecked: return Color.white.opacity(0.15)
        case .yes: return BarDS.Accent.teal.opacity(0.3)
        case .no: return BarDS.Accent.red.opacity(0.3)
        }
    }

    private var rowBg: Color {
        switch mark {
        case .unchecked: return Color.white.opacity(0.02)
        case .yes: return BarDS.Semantic.tealBg()
        case .no: return BarDS.Semantic.redBg()
        }
    }

    private var rowBorder: Color {
        switch mark {
        case .unchecked: return Color.white.opacity(0.08)
        case .yes: return BarDS.Semantic.tealBorder()
        case .no: return BarDS.Semantic.redBorder()
        }
    }
}

struct BarEscrowMatchNode: View {
    enum NodeState { case ok, brk, wrn }

    let nodeKey: String
    let nodeValue: String
    let state: NodeState

    var body: some View {
        VStack(alignment: .leading, spacing: 1) {
            Text(nodeKey.uppercased())
                .font(BarDS.bodyFont(10, weight: .regular))
                .foregroundColor(headerColor)
                .kerning(0.04 * 10)
            Text(nodeValue)
                .font(BarDS.monoFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundColor(BarDS.Text.primary)
        }
        .padding(.vertical, 6)
        .padding(.horizontal, 10)
        .background(shellBg)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                .stroke(shellBorder, lineWidth: BarDS.borderThin)
        )
        .padding(.bottom, 5)
    }

    private var headerColor: Color {
        switch state {
        case .ok: return BarDS.Accent.teal.opacity(0.7)
        case .brk: return BarDS.Accent.red.opacity(0.7)
        case .wrn: return BarDS.Accent.amber.opacity(0.7)
        }
    }

    private var shellBg: Color {
        switch state {
        case .ok: return BarDS.Accent.teal.opacity(0.05)
        case .brk: return BarDS.Accent.red.opacity(0.05)
        case .wrn: return BarDS.Accent.amber.opacity(0.05)
        }
    }

    private var shellBorder: Color {
        switch state {
        case .ok: return BarDS.Accent.teal.opacity(0.15)
        case .brk: return BarDS.Accent.red.opacity(0.15)
        case .wrn: return BarDS.Accent.amber.opacity(0.15)
        }
    }
}

struct BarDSDivider: View {
    var body: some View {
        Rectangle()
            .fill(BarDS.Border.divider)
            .frame(height: BarDS.borderThin)
    }
}
