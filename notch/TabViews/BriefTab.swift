import SwiftUI
import os
import Foundation

struct BriefLeftView: View {
    @ObservedObject var viewModel: NotchViewModel

    private static let preM10StubLog = Logger(subsystem: "in.tradeautopsy.notch", category: "pre_m10_patterns")

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            briefCapsHeader("Session")

            BarCard {
                Text("Pre-market")
                    .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                    .foregroundColor(BarDS.Text.section)
                    .textCase(.uppercase)
                    .kerning(0.006 * 11)
                    .padding(.bottom, 8)

                HStack(spacing: 6) {
                    marketIndex(
                        "Nifty",
                        value: viewModel.niftyValue,
                        change: viewModel.niftyChange
                    )
                    marketIndex(
                        "BNF",
                        value: viewModel.bnfValue,
                        change: viewModel.bnfChange
                    )
                    marketIndex(
                        "VIX",
                        value: viewModel.vixValue,
                        change: nil,
                        accent: viewModel.vixValue < 15
                            ? BarDS.Accent.teal
                            : viewModel.vixValue < 20
                                ? BarDS.Accent.amber
                                : BarDS.Accent.red
                    )
                }
            }

            BarDSDivider()
                .padding(.vertical, 2)

            briefCapsHeader("What matters")

            BarCard {
                if let b = viewModel.morningBrief {
                    behavioralBriefContent(b)
                } else {
                    Text("No session series yet.")
                        .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                        .foregroundColor(BarDS.Text.primary.opacity(0.72))
                        .fixedSize(horizontal: false, vertical: true)
                }
            }

            if let rec = viewModel.morningBrief?.recommendation, !rec.isEmpty {
                BarCard {
                    Text(rec)
                        .font(BarDS.bodyFont(BarDS.FontSize.body, weight: .regular))
                        .foregroundColor(BarDS.Text.primary.opacity(0.72))
                        .lineSpacing(3)
                        .fixedSize(horizontal: false, vertical: true)
                }
            }

            BarBigButton(label: "Start trading →", style: .primary) {
                viewModel.startTradingFromMorningBrief()
            }
            .accessibilityLabel("Start trading, open plan declaration")
            .padding(.top, 4)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }

    @ViewBuilder
    private func briefCapsHeader(_ text: String) -> some View {
        Text(text.uppercased())
            .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
            .foregroundColor(BarDS.Text.section)
            .kerning(0.006 * BarDS.FontSize.sectionLabel)
            .padding(.bottom, 6)
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func marketIndex(
        _ name: String,
        value: Double,
        change: Double?,
        accent: Color = .white
    ) -> some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(name)
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundColor(BarDS.Text.secondary)
                .textCase(.uppercase)
            Text(formatNumber(value))
                .font(BarDS.monoFont(BarDS.FontSize.body, weight: .medium))
                .monospacedDigit()
                .foregroundColor(accent)
            if let change {
                Text(formatChange(change))
                    .font(BarDS.monoFont(BarDS.FontSize.bodyXS, weight: .medium))
                    .monospacedDigit()
                    .foregroundColor(change >= 0 ? BarDS.Accent.green : BarDS.Accent.red)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }

    private func formatNumber(_ v: Double) -> String {
        String(format: "%.2f", v)
    }

    private func formatChange(_ c: Double) -> String {
        if abs(c) < 0.0001 { return "—" }
        return String(format: "%+.2f%%", c)
    }

    @ViewBuilder
    private func behavioralBriefContent(_ b: MorningBrief) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            analysisHeadlineBlock(b)
                .accessibilitySortPriority(8)

            briefScoreChip(b)

            analysisMetricGrid(b)

            if BriefMorningBriefPresentation.showsEstablishedBranch(patterns: b.patterns) {
                establishedBrief(b)
            } else if BriefMorningBriefPresentation.shouldShowPreM10PatternsStub(
                isNewUser: b.isNewUser,
                tradeCount: b.tradeCount,
                patterns: b.patterns,
            ) {
                preM10PatternsStub(b)
            } else {
                newUserBrief(b)
            }
        }
    }

    /// Headline with emphasized first clause when a separator is present (#8 mockup 2).
    @ViewBuilder
    private func analysisHeadlineBlock(_ b: MorningBrief) -> some View {
        let raw = (b.behavioralHeadline ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        if !raw.isEmpty, let parts = analysisHeadlineParts(raw) {
            (
                Text(parts.leading)
                    .font(BarDS.bodyFont(BarDS.FontSize.brief, weight: .regular))
                    .foregroundColor(BarDS.Text.primary)
                + Text(parts.trailing)
                    .font(BarDS.bodyFont(14, weight: .medium))
                    .foregroundColor(BarDS.Text.primary.opacity(0.78))
            )
            .lineSpacing(6)
            .fixedSize(horizontal: false, vertical: true)
        } else if !raw.isEmpty {
            Text(raw)
                .font(BarDS.bodyFont(BarDS.FontSize.brief, weight: .regular))
                .foregroundColor(BarDS.Text.primary)
                .lineSpacing(6)
                .fixedSize(horizontal: false, vertical: true)
        } else if let line = b.behavioralDateLine, !line.isEmpty {
            Text(line)
                .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .medium))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)
        }
    }

    private struct HeadlineParts {
        let leading: String
        let trailing: String
    }

    private func analysisHeadlineParts(_ raw: String) -> HeadlineParts? {
        for sep in [" — ", " – ", ". "] {
            if let r = raw.range(of: sep) {
                let lead = String(raw[..<r.lowerBound]).trimmingCharacters(in: .whitespacesAndNewlines)
                let trail = String(raw[r.lowerBound...]).trimmingCharacters(in: .whitespacesAndNewlines)
                if !lead.isEmpty { return HeadlineParts(leading: lead, trailing: trail) }
            }
        }
        return nil
    }

    /// Compact score chip — plan adherence when available, else trade depth.
    private func briefScoreChip(_ b: MorningBrief) -> some View {
        let title: String
        let value: String
        let valueColor: Color
        if let p = b.planAdherencePct {
            title = "Adherence"
            value = formatPercentMetric(p)
            valueColor = p >= 0.6 ? BarDS.Accent.teal.opacity(0.92) : BarDS.Accent.amber.opacity(0.92)
        } else {
            title = "Depth"
            value = "\(b.tradeCount) tr"
            valueColor = BarDS.Text.primary.opacity(0.88)
        }
        return HStack {
            VStack(alignment: .leading, spacing: 2) {
                Text(title).microLabel()
                Text(value)
                    .font(BarDS.monoFont(14, weight: .semibold))
                    .foregroundColor(valueColor)
            }
            Spacer(minLength: 4)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
        .background(Color.white.opacity(0.06))
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Accent.teal.opacity(0.2), lineWidth: BarDS.borderThin),
        )
        .accessibilityElement(children: .combine)
        .accessibilityLabel("\(title) \(value)")
    }

    private func analysisMetricGrid(_ b: MorningBrief) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            Text("KEY METRICS")
                .font(BarDS.bodyFont(10, weight: .bold))
                .foregroundColor(BarDS.Text.muted)
                .tracking(0.9)
            LazyVGrid(
                columns: [GridItem(.flexible(), spacing: 6), GridItem(.flexible(), spacing: 6)],
                spacing: 6,
            ) {
                metricCard(
                    title: "SESSION P&L",
                    value: b.sessionPnLKpi.map { formatBriefInr($0) },
                    subtitle: "Last rolling window",
                )
                metricCard(
                    title: "ADHERENCE",
                    value: b.planAdherencePct.map { formatPercentMetric($0) },
                    subtitle: "Plan vs impulse",
                )
                metricCard(
                    title: "WIN RATE",
                    value: b.winRateKpi.map { formatPercentMetric($0) },
                    subtitle: "Recent sample",
                )
                metricCard(
                    title: "LEFT ON TABLE",
                    value: b.leftOnTableInr.map { formatBriefInr($0) },
                    subtitle: "Declared targets",
                )
            }
            .accessibilitySortPriority(6)
        }
    }

    private func metricCard(title: String, value: String?, subtitle: String) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title)
                .font(BarDS.bodyFont(10, weight: .medium))
                .foregroundColor(BarDS.Text.muted)
                .tracking(0.5)
            Text(value ?? "—")
                .font(BarDS.monoFont(13, weight: .medium))
                .foregroundColor(BarDS.Text.primary.opacity(0.9))
            Text(subtitle)
                .font(BarDS.bodyFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(8)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private func preM10PatternsStub(_ b: MorningBrief) -> some View {
        let _ = Self.preM10StubLog.debug(
            "pre_m10_patterns_stub_rendered tradeCount=\(b.tradeCount) isNewUser=\(b.isNewUser) patterns=0",
        )
        return VStack(alignment: .leading, spacing: 8) {
            Text("BETA · NON-SCORING")
                .font(BarDS.bodyFont(8, weight: .bold))
                .foregroundColor(BarDS.Fill.sidebar)
                .padding(.horizontal, 8)
                .padding(.vertical, 4)
                .background(BarDS.Accent.amber.opacity(0.85))
                .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))

            Text(
                "Model-10 pattern science is not enabled in this build. Nothing here is a validated classifier output — use it as narrative context only until M10 ships.",
            )
            .font(BarDS.bodyFont(10, weight: .medium))
            .foregroundColor(BarDS.Text.secondary)
            .fixedSize(horizontal: false, vertical: true)

            if let rule = b.nonNegotiableRule, !rule.isEmpty {
                BarNonNegotiableCard(label: "Watch today", text: rule, kind: .amber)
            }
        }
    }

    private func newUserBrief(_ b: MorningBrief) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text(BriefMorningBriefPresentation.newUserProgressTitle(tradeCount: b.tradeCount))
                .font(BarDS.bodyFont(10, weight: .medium))
                .foregroundColor(BarDS.Text.secondary)
                .fixedSize(horizontal: false, vertical: true)

            Text(BriefMorningBriefPresentation.newUserProgressCounter(tradeCount: b.tradeCount))
                .font(BarDS.monoFont(10, weight: .semibold))
                .foregroundColor(BarDS.Text.primary.opacity(0.85))

            if let m = b.ownMetrics {
                HStack(alignment: .top, spacing: 8) {
                    Text(BriefMorningBriefPresentation.ownMetricsStopLine(m))
                        .briefMetricPillBar()
                    Text(BriefMorningBriefPresentation.ownMetricsExitLine(m))
                        .briefMetricPillBar()
                }
            }
        }
    }

    private func establishedBrief(_ b: MorningBrief) -> some View {
        let buckets = bucketedPatterns(b.patterns)
        return VStack(alignment: .leading, spacing: 10) {
            if !buckets.established.isEmpty || !buckets.improving.isEmpty {
                briefSubsectionHeader("YOUR PATTERNS")
                ForEach(buckets.established) { pattern in
                    patternRow(pattern, showImprovingBadge: false)
                }
                ForEach(buckets.improving) { pattern in
                    patternRow(pattern, showImprovingBadge: true)
                }
            }

            if !buckets.preliminary.isEmpty {
                briefSubsectionHeader("WHAT WE'RE WATCHING")
                ForEach(buckets.preliminary) { pattern in
                    patternRow(pattern, showImprovingBadge: false)
                }
            }

            if let rule = b.nonNegotiableRule, !rule.isEmpty {
                BarNonNegotiableCard(label: "Non-negotiable today", text: rule, kind: .amber)
            }
        }
    }

    private struct PatternBuckets {
        var established: [BriefBehavioralPattern]
        var preliminary: [BriefBehavioralPattern]
        var improving: [BriefBehavioralPattern]
    }

    private func bucketedPatterns(_ patterns: [BriefBehavioralPattern]) -> PatternBuckets {
        var e: [BriefBehavioralPattern] = []
        var p: [BriefBehavioralPattern] = []
        var i: [BriefBehavioralPattern] = []
        for pat in patterns {
            switch BriefMorningBriefPresentation.patternConfidenceBucket(pat.confidence) {
            case .established: e.append(pat)
            case .preliminary: p.append(pat)
            case .improving: i.append(pat)
            }
        }
        return PatternBuckets(established: e, preliminary: p, improving: i)
    }

    private func briefSubsectionHeader(_ title: String) -> some View {
        Text(title)
            .font(BarDS.bodyFont(10, weight: .bold))
            .foregroundColor(BarDS.Text.muted)
            .tracking(0.9)
    }

    private func patternRow(_ pattern: BriefBehavioralPattern, showImprovingBadge: Bool) -> some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack(alignment: .top, spacing: 8) {
                Text(pattern.condition.isEmpty ? "—" : pattern.condition)
                    .font(BarDS.bodyFont(BarDS.FontSize.bodySmall, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                    .fixedSize(horizontal: false, vertical: true)
                Spacer(minLength: 4)
                patternTrailingBadge(pattern, showImprovingBadge: showImprovingBadge)
            }
            if let cost = pattern.costInr {
                Text(formatBriefInr(cost))
                    .font(BarDS.monoFont(BarDS.FontSize.bodyXS, weight: .medium))
                    .foregroundColor(BarDS.Accent.red.opacity(0.88))
            }
            if let rec = pattern.recoveredInr, rec > 0, showImprovingBadge {
                Text("Recovered \(formatBriefInr(rec))")
                    .font(BarDS.monoFont(10, weight: .medium))
                    .foregroundColor(BarDS.Accent.teal.opacity(0.88))
            }
        }
        .padding(10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: BarDS.Radius.card, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    @ViewBuilder
    private func patternTrailingBadge(_ pattern: BriefBehavioralPattern, showImprovingBadge: Bool) -> some View {
        if showImprovingBadge {
            Text("Improving")
                .font(BarDS.bodyFont(9, weight: .bold))
                .foregroundColor(BarDS.Fill.sidebar)
                .padding(.horizontal, 6)
                .padding(.vertical, 3)
                .background(BarDS.Accent.teal.opacity(0.85))
                .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        } else {
            Text(BriefMorningBriefPresentation.confidenceBadgeLabel(pattern.confidence))
                .font(BarDS.bodyFont(9, weight: .bold))
                .foregroundColor(BarDS.Fill.sidebar)
                .padding(.horizontal, 6)
                .padding(.vertical, 3)
                .background(patternBadgeBackground(pattern.confidence))
                .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
        }
    }

    private func patternBadgeBackground(_ confidence: String) -> Color {
        switch BriefMorningBriefPresentation.patternConfidenceBucket(confidence) {
        case .preliminary: BarDS.Accent.amber.opacity(0.9)
        case .established: Color.white.opacity(0.82)
        case .improving: BarDS.Accent.teal.opacity(0.85)
        }
    }

    private func formatPercentMetric(_ v: Double) -> String {
        let pct = (v >= 0 && v <= 1) ? v * 100 : v
        return String(format: "%.0f%%", pct)
    }

    private func formatBriefInr(_ v: Double) -> String {
        let f = NumberFormatter()
        f.numberStyle = .currency
        f.currencyCode = "INR"
        f.maximumFractionDigits = 0
        return f.string(from: NSNumber(value: v)) ?? "₹\(Int(v.rounded()))"
    }
}

private extension Text {
    func briefMetricPillBar() -> some View {
        font(BarDS.bodyFont(8, weight: .medium))
            .foregroundColor(BarDS.Text.secondary)
            .padding(.horizontal, 8)
            .padding(.vertical, 4)
            .background(Color.white.opacity(0.06))
            .clipShape(RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous))
            .overlay(
                RoundedRectangle(cornerRadius: BarDS.Radius.small, style: .continuous)
                    .stroke(Color.white.opacity(0.08), lineWidth: BarDS.borderThin)
            )
    }
}

struct BriefRightView: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            VStack(alignment: .leading, spacing: 8) {
                if !viewModel.edgeSymbols.isEmpty {
                    Text("EDGE TODAY")
                        .font(BarDS.bodyFont(BarDS.FontSize.sectionLabel, weight: .medium))
                        .foregroundColor(BarDS.Text.labels)
                        .kerning(0.08 * 10)
                        .padding(.bottom, 4)
                    VStack(spacing: 4) {
                        ForEach(viewModel.edgeSymbols, id: \.self) { sym in
                            HStack {
                                Text("▸")
                                    .font(BarDS.bodyFont(8, weight: .bold))
                                    .foregroundColor(BarDS.Accent.teal)
                                Text(sym)
                                    .font(BarDS.bodyFont(BarDS.FontSize.bodyXS, weight: .semibold))
                                    .foregroundColor(BarDS.Text.primary)
                                Spacer()
                            }
                        }
                    }
                }
            }

            Spacer()
        }
        .padding(12)
        .glassCard(radius: 10)
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
    }
}
