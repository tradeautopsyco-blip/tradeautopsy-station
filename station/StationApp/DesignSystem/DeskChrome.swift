import SwiftUI

/// Local Mac desk shell — HIG type scale, 8pt grid, card recipe, three button roles.
/// Station desk only; Notch uses BarDS.
enum DeskChrome {
    enum Space {
        static let unit: CGFloat = 8
        static let x1: CGFloat = 8
        static let x2: CGFloat = 16
        static let x3: CGFloat = 24
        static let pageInset: CGFloat = 24
        static let cardInset: CGFloat = 16
    }

    enum TypeScale {
        static let largeTitle: CGFloat = 26
        static let title2: CGFloat = 22
        static let title3: CGFloat = 20
        static let headline: CGFloat = 17
        static let body: CGFloat = 13
        static let callout: CGFloat = 12
        static let footnote: CGFloat = 11
        static let caption: CGFloat = 10
    }

    static func sans(_ size: CGFloat, weight: Font.Weight = .regular) -> Font {
        .system(size: size, weight: weight, design: .default)
    }

    static func mono(_ size: CGFloat, weight: Font.Weight = .medium) -> Font {
        .system(size: size, weight: weight, design: .monospaced)
    }

    static let cardRadius: CGFloat = StationDS.Radius.card
    static let cardFill = StationDS.Fill.card
    static let cardStroke = StationDS.Border.card
}

// MARK: - Page shell

struct DeskPageShell<Trailing: View, Content: View>: View {
    let title: String
    let subtitle: String?
    @ViewBuilder var trailing: () -> Trailing
    @ViewBuilder var content: () -> Content

    init(
        title: String,
        subtitle: String? = nil,
        @ViewBuilder trailing: @escaping () -> Trailing = { EmptyView() },
        @ViewBuilder content: @escaping () -> Content
    ) {
        self.title = title
        self.subtitle = subtitle
        self.trailing = trailing
        self.content = content
    }

    var body: some View {
        VStack(spacing: 0) {
            header
            ScrollView {
                content()
                    .padding(DeskChrome.Space.pageInset)
                    .frame(maxWidth: 840, alignment: .leading)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
        .background(WorkspaceChrome.ground)
    }

    private var header: some View {
        HStack(alignment: .center, spacing: DeskChrome.Space.x2) {
            VStack(alignment: .leading, spacing: 4) {
                Text(title)
                    .font(DeskChrome.sans(DeskChrome.TypeScale.largeTitle, weight: .bold))
                    .foregroundStyle(StationDS.Text.primary)
                if let subtitle, !subtitle.isEmpty {
                    Text(subtitle)
                        .font(DeskChrome.sans(DeskChrome.TypeScale.callout))
                        .foregroundStyle(StationDS.Text.secondary)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }
            Spacer(minLength: DeskChrome.Space.x2)
            trailing()
        }
        .padding(.horizontal, DeskChrome.Space.pageInset)
        .padding(.vertical, DeskChrome.Space.x2)
        .overlay(alignment: .bottom) {
            Rectangle()
                .fill(StationDS.Border.divider)
                .frame(height: StationDS.borderThin)
        }
        .accessibilityElement(children: .contain)
    }
}

// MARK: - Card

struct DeskStatusPill: View {
    let text: String
    var tone: Tone = .neutral

    enum Tone {
        case neutral, live, success, warning, danger
    }

    var body: some View {
        Text(text)
            .font(DeskChrome.sans(DeskChrome.TypeScale.footnote, weight: .medium))
            .foregroundStyle(foreground)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(background, in: Capsule())
    }

    private var foreground: Color {
        switch tone {
        case .neutral: StationDS.Text.secondary
        case .live, .success: StationDS.Accent.green
        case .warning: StationDS.Accent.amber
        case .danger: StationDS.Accent.red
        }
    }

    private var background: Color {
        foreground.opacity(0.14)
    }
}

struct DeskCard<Body: View, Footer: View>: View {
    let title: String
    var status: DeskStatusPill?
    @ViewBuilder var bodyContent: () -> Body
    @ViewBuilder var footer: () -> Footer

    init(
        title: String,
        status: DeskStatusPill? = nil,
        @ViewBuilder body: @escaping () -> Body,
        @ViewBuilder footer: @escaping () -> Footer = { EmptyView() }
    ) {
        self.title = title
        self.status = status
        self.bodyContent = body
        self.footer = footer
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(alignment: .firstTextBaseline, spacing: DeskChrome.Space.x1) {
                Text(title)
                    .font(DeskChrome.sans(DeskChrome.TypeScale.headline, weight: .semibold))
                    .foregroundStyle(StationDS.Text.primary)
                Spacer(minLength: 0)
                if let status {
                    status
                }
            }
            .padding(.bottom, DeskChrome.Space.x1)

            bodyContent()
                .font(DeskChrome.sans(DeskChrome.TypeScale.body))
                .foregroundStyle(StationDS.Text.secondary)

            footer()
                .padding(.top, DeskChrome.Space.x2)
        }
        .padding(DeskChrome.Space.cardInset)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(DeskChrome.cardFill, in: RoundedRectangle(cornerRadius: DeskChrome.cardRadius, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: DeskChrome.cardRadius, style: .continuous)
                .stroke(DeskChrome.cardStroke, lineWidth: StationDS.borderThin)
        )
    }
}

// MARK: - Buttons (Primary · Secondary · Destructive)

struct DeskPrimaryButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(DeskChrome.sans(DeskChrome.TypeScale.callout, weight: .semibold))
            .foregroundStyle(StationDS.Fill.accentInk)
            .padding(.horizontal, 14)
            .padding(.vertical, 7)
            .background(
                WorkspaceChrome.accent.opacity(configuration.isPressed ? 0.82 : 1),
                in: RoundedRectangle(cornerRadius: StationDS.Radius.small, style: .continuous)
            )
    }
}

struct DeskSecondaryButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(DeskChrome.sans(DeskChrome.TypeScale.callout, weight: .medium))
            .foregroundStyle(StationDS.Text.primary)
            .padding(.horizontal, 14)
            .padding(.vertical, 7)
            .background(
                StationDS.Fill.input.opacity(configuration.isPressed ? 0.9 : 1),
                in: RoundedRectangle(cornerRadius: StationDS.Radius.small, style: .continuous)
            )
            .overlay(
                RoundedRectangle(cornerRadius: StationDS.Radius.small, style: .continuous)
                    .stroke(StationDS.Border.outlineBtn, lineWidth: StationDS.borderThin)
            )
    }
}

struct DeskDestructiveButtonStyle: ButtonStyle {
    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(DeskChrome.sans(DeskChrome.TypeScale.callout, weight: .medium))
            .foregroundStyle(StationDS.Accent.red)
            .padding(.horizontal, 14)
            .padding(.vertical, 7)
            .background(
                StationDS.Semantic.redBg(),
                in: RoundedRectangle(cornerRadius: StationDS.Radius.small, style: .continuous)
            )
            .overlay(
                RoundedRectangle(cornerRadius: StationDS.Radius.small, style: .continuous)
                    .stroke(StationDS.Semantic.redBorder(), lineWidth: StationDS.borderThin)
            )
    }
}

extension ButtonStyle where Self == DeskPrimaryButtonStyle {
    static var deskPrimary: DeskPrimaryButtonStyle { DeskPrimaryButtonStyle() }
}

extension ButtonStyle where Self == DeskSecondaryButtonStyle {
    static var deskSecondary: DeskSecondaryButtonStyle { DeskSecondaryButtonStyle() }
}

extension ButtonStyle where Self == DeskDestructiveButtonStyle {
    static var deskDestructive: DeskDestructiveButtonStyle { DeskDestructiveButtonStyle() }
}
