import Foundation

// MARK: - #126 — collapsed pill archetype variants (unified notch reference §CollapsedView)

/// Leading glyph for the collapsed strip — intraday uses risk coloring from `compositeScore` unless SL is missing.
enum CollapsedNotchScoreIndicatorKind: Equatable {
    case riskColored(compositeScore: Double)
    case slMissingAmber
}

enum CollapsedNotchLayout: Equatable {
    case intervention(keyword: String, chromeBackgroundHex: String, chromeBorderHex: String)
    case intraday(scoreIndicator: CollapsedNotchScoreIndicatorKind, behavioralLabel: String)
    case scalper(tradesProgressLabel: String, sessionLossLabel: String?, timeRemainingLabel: String?)
    case swing(daysLabel: String, statusTitle: String, weeklyPnLLabel: String?)
}

struct CollapsedNotchPresentation: Equatable {
    var layout: CollapsedNotchLayout
    /// Scalper session tilt — entire pill should pulse amber (UI honors Reduce Motion).
    var pillPulseAmber: Bool

    /// Pure presentation from hosted `notch` snapshot + client-resolved archetype. No extra server work.
    static func build(
        notch: BarLiveStateResponse?,
        archetype: TraderArchetype,
        compositeScore: Double,
        behavioralStateLabel: String,
        referenceNow: Date,
    ) -> CollapsedNotchPresentation {
        if let notch, let primary = BarInterventionCardSpec.sortedInterventions(notch.activeInterventions).first {
            let kw = collapsedInterventionKeyword(interventionType: primary.interventionType)
            let chrome = BarInterventionCardSpec.chrome(interventionType: primary.interventionType, isPrimary: true)
            return CollapsedNotchPresentation(
                layout: .intervention(
                    keyword: kw,
                    chromeBackgroundHex: chrome.backgroundHex,
                    chromeBorderHex: chrome.borderColorHex,
                ),
                pillPulseAmber: false,
            )
        }

        switch archetype {
        case .scalper where isScalperSessionActive(notch: notch):
            let tilt = notch?.tiltSignal == true
            return CollapsedNotchPresentation(
                layout: .scalper(
                    tradesProgressLabel: scalperTradesLabel(notch: notch),
                    sessionLossLabel: scalperSessionLossLabel(notch: notch),
                    timeRemainingLabel: scalperTimeRemainingLabel(
                        windowEndsISO: notch?.sessionWindowEndsISO,
                        referenceNow: referenceNow,
                    ),
                ),
                pillPulseAmber: tilt,
            )
        case .swing:
            let overdue = notch?.dailyCheckInRequired == true
            let status = overdue ? "CHECK TODAY" : "INTACT"
            let day = notch?.daysInTrade
            let daysLabel: String
            if let day, day > 0 {
                daysLabel = "Day \(day)"
            } else {
                daysLabel = "Swing"
            }
            return CollapsedNotchPresentation(
                layout: .swing(
                    daysLabel: daysLabel,
                    statusTitle: status,
                    weeklyPnLLabel: weeklyPnLStripeLabel(notch?.weeklyPnL),
                ),
                pillPulseAmber: false,
            )
        case .scalper, .intraday:
            let indicator: CollapsedNotchScoreIndicatorKind =
                isSlMissing(notch?.slStatus) ? .slMissingAmber : .riskColored(compositeScore: compositeScore)
            return CollapsedNotchPresentation(
                layout: .intraday(scoreIndicator: indicator, behavioralLabel: behavioralStateLabel),
                pillPulseAmber: false,
            )
        }
    }

    // MARK: - Intervention keyword strip (COOLING / LIMIT / EXPIRED)

    static func collapsedInterventionKeyword(interventionType: String) -> String {
        let k = interventionType.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        switch k {
        case "token_expired", "bar_broker_session":
            return "EXPIRED"
        case "post_loss_macro", "exit_gate_micro":
            return "COOLING"
        default:
            return "LIMIT"
        }
    }

    // MARK: - Scalper

    private static func isScalperSessionActive(notch: BarLiveStateResponse?) -> Bool {
        guard let n = notch else { return false }
        if n.isSessionLevel == true { return true }
        if n.sessionTradeCount != nil { return true }
        if n.sessionMaxTrades != nil { return true }
        return false
    }

    private static func scalperTradesLabel(notch: BarLiveStateResponse?) -> String {
        let c = notch?.sessionTradeCount
        let m = notch?.sessionMaxTrades
        if let c, let m {
            return "\(c)/\(m)"
        }
        if let c {
            return "\(c)"
        }
        return "—"
    }

    private static func scalperSessionLossLabel(notch: BarLiveStateResponse?) -> String? {
        guard let amt = notch?.sessionLossAmount else { return nil }
        let loss = formatINRWhole(-abs(amt))
        if let lim = notch?.sessionLossLimit, lim > 0 {
            let cap = formatINRWhole(lim)
            return "\(loss) / \(cap)"
        }
        return loss
    }

    private static func scalperTimeRemainingLabel(windowEndsISO: String?, referenceNow: Date) -> String? {
        guard let raw = windowEndsISO?.trimmingCharacters(in: .whitespacesAndNewlines), !raw.isEmpty else {
            return nil
        }
        let end =
            iso8601Full.date(from: raw)
            ?? iso8601Frac.date(from: raw)
            ?? DateFormatter.wireBarSessionEnd.date(from: raw)
        guard let end else { return nil }
        let secs = max(0, end.timeIntervalSince(referenceNow))
        if secs >= 3600 {
            let h = Int(secs / 3600)
            let m = Int((secs.truncatingRemainder(dividingBy: 3600)) / 60)
            return "\(h)h \(m)m"
        }
        if secs >= 60 {
            return "\(Int(secs / 60))m"
        }
        return "\(Int(secs))s"
    }

    // MARK: - Swing

    private static func weeklyPnLStripeLabel(_ v: Double?) -> String? {
        guard let v else { return nil }
        let body = formatINRWhole(abs(v))
        if v > 0 { return "+\(body)" }
        if v < 0 { return "−\(body)" }
        return body
    }

    // MARK: - Intraday SL

    private static func isSlMissing(_ status: String?) -> Bool {
        guard let s = status?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased(), !s.isEmpty else {
            return false
        }
        return s == "missing"
    }

    // MARK: - Formatting

    private static func formatINRWhole(_ v: Double) -> String {
        let f = NumberFormatter()
        f.numberStyle = .currency
        f.currencyCode = "INR"
        f.maximumFractionDigits = 0
        return f.string(from: NSNumber(value: v)) ?? "₹\(Int(v.rounded()))"
    }

    private static let iso8601Full: ISO8601DateFormatter = {
        let f = ISO8601DateFormatter()
        f.formatOptions = [.withInternetDateTime, .withDashSeparatorInDate, .withColonSeparatorInTime]
        return f
    }()

    private static let iso8601Frac: ISO8601DateFormatter = {
        let f = ISO8601DateFormatter()
        f.formatOptions = [
            .withInternetDateTime,
            .withDashSeparatorInDate,
            .withColonSeparatorInTime,
            .withFractionalSeconds,
        ]
        return f
    }()
}

private extension DateFormatter {
    static let wireBarSessionEnd: DateFormatter = {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        formatter.dateFormat = "yyyy-MM-dd'T'HH:mm:ss.SSS'Z'"
        return formatter
    }()
}
