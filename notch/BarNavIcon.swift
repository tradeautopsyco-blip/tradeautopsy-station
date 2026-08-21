import SwiftUI

/// HTML `NOTCH-ui.html` 24×24 symbols, drawn at 18pt. Line vs fill matches `.plan-nav.on`.
struct BarNavIcon: View {
    enum Glyph: Equatable {
        case sun, clipboard, bolt, list, shield, brain, chart, down, gear, eyeSlash, chevron
    }

    let glyph: Glyph
    var filled: Bool = false
    var size: CGFloat = 18

    var body: some View {
        let shape = BarNavIconShape(glyph: glyph, filled: filled)
        Group {
            if filled {
                shape.fill(style: FillStyle(eoFill: false))
            } else {
                shape.stroke(
                    style: StrokeStyle(
                        lineWidth: 1.7 * (size / 24),
                        lineCap: .round,
                        lineJoin: .round
                    )
                )
            }
        }
        .frame(width: size, height: size)
        .accessibilityHidden(true)
    }
}

private struct BarNavIconShape: Shape {
    var glyph: BarNavIcon.Glyph
    var filled: Bool

    func path(in rect: CGRect) -> Path {
        let s = rect.width / 24
        func p(_ x: CGFloat, _ y: CGFloat) -> CGPoint {
            CGPoint(x: rect.minX + x * s, y: rect.minY + y * s)
        }
        var path = Path()
        switch glyph {
        case .sun:
            if filled {
                path.addEllipse(in: CGRect(x: 7.2 * s, y: 7.2 * s, width: 9.6 * s, height: 9.6 * s))
                let rays: [(CGFloat, CGFloat, CGFloat, CGFloat)] = [
                    (11.1, 2.8, 1.8, 2.1), (11.1, 19.1, 1.8, 2.1),
                    (2.8, 11.1, 2.1, 1.8), (19.1, 11.1, 2.1, 1.8),
                ]
                for r in rays {
                    path.addRoundedRect(
                        in: CGRect(x: r.0 * s, y: r.1 * s, width: r.2 * s, height: r.3 * s),
                        cornerSize: CGSize(width: 0.9 * s, height: 0.9 * s)
                    )
                }
            } else {
                path.addEllipse(in: CGRect(x: 8 * s, y: 8 * s, width: 8 * s, height: 8 * s))
                path.move(to: p(12, 3)); path.addLine(to: p(12, 4.5))
                path.move(to: p(12, 19.5)); path.addLine(to: p(12, 21))
                path.move(to: p(4.2, 4.2)); path.addLine(to: p(5.3, 5.3))
                path.move(to: p(18.7, 18.7)); path.addLine(to: p(19.8, 19.8))
                path.move(to: p(3, 12)); path.addLine(to: p(4.5, 12))
                path.move(to: p(19.5, 12)); path.addLine(to: p(21, 12))
                path.move(to: p(4.2, 19.8)); path.addLine(to: p(5.3, 18.7))
                path.move(to: p(18.7, 5.3)); path.addLine(to: p(19.8, 4.2))
            }
        case .clipboard:
            if filled {
                path.addRoundedRect(
                    in: CGRect(x: 4.6 * s, y: 4.5 * s, width: 14.8 * s, height: 16.4 * s),
                    cornerSize: CGSize(width: 2.4 * s, height: 2.4 * s)
                )
            } else {
                path.addRoundedRect(
                    in: CGRect(x: 7 * s, y: 4.5 * s, width: 10 * s, height: 15 * s),
                    cornerSize: CGSize(width: 2 * s, height: 2 * s)
                )
                path.move(to: p(9, 4.5)); path.addLine(to: p(15, 4.5))
                path.addLine(to: p(15, 6.7)); path.addLine(to: p(9, 6.7)); path.closeSubpath()
                path.move(to: p(9.5, 11.2)); path.addLine(to: p(14.5, 11.2))
                path.move(to: p(9.5, 14.2)); path.addLine(to: p(13, 14.2))
            }
        case .bolt:
            path.move(to: p(13, 3))
            path.addLine(to: p(5.8, 13.2))
            path.addLine(to: p(11.2, 13.2))
            path.addLine(to: p(10, 21))
            path.addLine(to: p(18.4, 9.6))
            path.addLine(to: p(12.8, 9.6))
            path.closeSubpath()
        case .list:
            if filled {
                for y: CGFloat in [7.5, 12, 16.5] {
                    path.addEllipse(in: CGRect(x: 4.1 * s, y: (y - 1.1) * s, width: 2.2 * s, height: 2.2 * s))
                    path.addRoundedRect(
                        in: CGRect(x: 8.5 * s, y: (y - 0.9) * s, width: 10.5 * s, height: 1.8 * s),
                        cornerSize: CGSize(width: 0.9 * s, height: 0.9 * s)
                    )
                }
            } else {
                path.move(to: p(8.5, 7.5)); path.addLine(to: p(18.5, 7.5))
                path.move(to: p(8.5, 12)); path.addLine(to: p(18.5, 12))
                path.move(to: p(8.5, 16.5)); path.addLine(to: p(18.5, 16.5))
                path.move(to: p(5.2, 7.5)); path.addLine(to: p(5.21, 7.5))
                path.move(to: p(5.2, 12)); path.addLine(to: p(5.21, 12))
                path.move(to: p(5.2, 16.5)); path.addLine(to: p(5.21, 16.5))
            }
        case .shield:
            path.move(to: p(12, 3.5))
            path.addLine(to: p(19, 6.2))
            path.addLine(to: p(19, 11.5))
            path.addQuadCurve(to: p(12, 20.3), control: p(19, 17.5))
            path.addQuadCurve(to: p(5, 11.5), control: p(5, 17.5))
            path.addLine(to: p(5, 6.2))
            path.closeSubpath()
        case .brain:
            path.addEllipse(in: CGRect(x: 4.2 * s, y: 5.2 * s, width: 7.2 * s, height: 12.6 * s))
            path.addEllipse(in: CGRect(x: 12.6 * s, y: 5.2 * s, width: 7.2 * s, height: 12.6 * s))
            if !filled {
                path.move(to: p(12, 5.2)); path.addLine(to: p(12, 18))
                path.move(to: p(9.2, 9.2)); path.addLine(to: p(11.8, 9.2))
                path.move(to: p(12.2, 12.4)); path.addLine(to: p(15, 12.4))
            }
        case .chart:
            if filled {
                path.addRoundedRect(
                    in: CGRect(x: 5.2 * s, y: 10.2 * s, width: 3.6 * s, height: 8.8 * s),
                    cornerSize: CGSize(width: 1 * s, height: 1 * s)
                )
                path.addRoundedRect(
                    in: CGRect(x: 10.2 * s, y: 5.6 * s, width: 3.6 * s, height: 13.4 * s),
                    cornerSize: CGSize(width: 1 * s, height: 1 * s)
                )
                path.addRoundedRect(
                    in: CGRect(x: 15.2 * s, y: 9.8 * s, width: 3.6 * s, height: 9.2 * s),
                    cornerSize: CGSize(width: 1 * s, height: 1 * s)
                )
            } else {
                path.move(to: p(6, 17.5)); path.addLine(to: p(6, 11))
                path.move(to: p(12, 17.5)); path.addLine(to: p(12, 7))
                path.move(to: p(18, 17.5)); path.addLine(to: p(18, 13.3))
            }
        case .down:
            path.addEllipse(in: CGRect(x: 4 * s, y: 4 * s, width: 16 * s, height: 16 * s))
            if filled {
                // even-odd cutout is awkward; overlay arrow as extra stroke in the view instead
            } else {
                path.move(to: p(12, 8.2)); path.addLine(to: p(12, 15.4))
                path.move(to: p(8.8, 12.8)); path.addLine(to: p(12, 16)); path.addLine(to: p(15.2, 12.8))
            }
        case .gear:
            path.addEllipse(in: CGRect(x: 9 * s, y: 9 * s, width: 6 * s, height: 6 * s))
            let teeth = 8
            for i in 0..<teeth {
                let a = (Double(i) / Double(teeth)) * .pi * 2 - .pi / 2
                let inner: CGFloat = 6.2
                let outer: CGFloat = 8.6
                let cx: CGFloat = 12
                let cy: CGFloat = 12
                let x1 = cx + inner * CGFloat(cos(a))
                let y1 = cy + inner * CGFloat(sin(a))
                let x2 = cx + outer * CGFloat(cos(a))
                let y2 = cy + outer * CGFloat(sin(a))
                path.move(to: p(x1, y1))
                path.addLine(to: p(x2, y2))
            }
        case .eyeSlash:
            path.move(to: p(4, 5.5)); path.addLine(to: p(19.5, 21))
            path.move(to: p(9.6, 9.8)); path.addQuadCurve(to: p(13.4, 15.1), control: p(15, 11))
            path.move(to: p(6.4, 7.2)); path.addQuadCurve(to: p(20.5, 12), control: p(12, 5.5))
            path.move(to: p(8.1, 16.2)); path.addQuadCurve(to: p(20.2, 9), control: p(16, 17))
        case .chevron:
            path.move(to: p(7, 10)); path.addLine(to: p(12, 15)); path.addLine(to: p(17, 10))
        }
        return path
    }
}

extension BarNotchScreen {
    var navGlyph: BarNavIcon.Glyph {
        switch self {
        case .morning: return .sun
        case .pretrade: return .clipboard
        case .live: return .bolt
        case .posttrade: return .list
        case .escrow: return .shield
        case .patterns: return .brain
        case .fidelity: return .chart
        case .triage: return .down
        case .settings: return .gear
        }
    }

    var navTitle: String {
        switch self {
        case .fidelity: return "Fidelity"
        default: return rawValue
        }
    }
}
