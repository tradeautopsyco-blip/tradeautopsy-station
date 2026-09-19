import Foundation
import SwiftUI

/// Prototype 6-cell options glance (`strip()`, not the cash 3-cell).
enum BarOptionsGlanceStripSeed {
    enum Kind: Equatable, Hashable {
        case last, index, delta, gammaTheta, oi, margin
    }

    static let kinds: [Kind] = [.last, .index, .delta, .gammaTheta, .oi, .margin]
}

enum BarOptionsGlanceCopy {
    static let nfoOiNote = "open_int · this row"
    static let cryptoOiNote = "sumOpenInterest · this row"
    static let nfoIndexNote = "DualNoBlend INR · index unspecified"
    static let cryptoIndexNote = "eapi index · European"
    static let nfoGreeksNote = "not Black-76"
    static let cryptoGreeksNote = "VenuePublished mark"
    static let marginNote = "venue engine, not ours"
    static let lastNote = "market/quote"
}

enum BarOptionsGlanceHonesty {
    enum CellValue: Equatable {
        case text(String)
        case chip(HonestyStatus)
    }

    struct Inputs: Equatable {
        var isCrypto: Bool
        var lastEntry: String?
        var lastStatus: String
        var indexStatus: String
        var indexPrice: String?
        var greeksDisplay: Bool
        var greeksStatus: String
        var delta: String?
        var gamma: String?
        var theta: String?
        var oi: String?

        static func crypto(
            lastEntry: String? = nil,
            lastStatus: String = "success",
            indexStatus: String = "unavailable",
            indexPrice: String? = nil,
            greeksDisplay: Bool = false,
            greeksStatus: String = "unavailable",
            delta: String? = nil,
            gamma: String? = nil,
            theta: String? = nil,
            oi: String? = nil
        ) -> Inputs {
            Inputs(
                isCrypto: true,
                lastEntry: lastEntry,
                lastStatus: lastStatus,
                indexStatus: indexStatus,
                indexPrice: indexPrice,
                greeksDisplay: greeksDisplay,
                greeksStatus: greeksStatus,
                delta: delta,
                gamma: gamma,
                theta: theta,
                oi: oi
            )
        }

        static func nfo(oi: String? = nil) -> Inputs {
            Inputs(
                isCrypto: false,
                lastEntry: nil,
                lastStatus: "unavailable",
                indexStatus: "unavailable",
                indexPrice: nil,
                greeksDisplay: false,
                greeksStatus: "unavailable",
                delta: nil,
                gamma: nil,
                theta: nil,
                oi: oi
            )
        }
    }

    static func value(_ kind: BarOptionsGlanceStripSeed.Kind, inputs: Inputs) -> CellValue {
        switch kind {
        case .last:
            let entry = (inputs.lastEntry ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
            if !entry.isEmpty { return .text(entry) }
            return .chip(HonestyStatus.fromWire(inputs.lastStatus) ?? .unavailable)
        case .index:
            guard inputs.isCrypto else { return .chip(.unavailable) }
            let wire = inputs.indexStatus.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
            if wire == "success", let price = inputs.indexPrice, !price.isEmpty {
                return .text(price)
            }
            return .chip(HonestyStatus.fromWire(inputs.indexStatus) ?? .unavailable)
        case .delta:
            guard inputs.isCrypto, inputs.greeksDisplay else { return .chip(.unavailable) }
            let wire = inputs.greeksStatus.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
            if wire == "success", let delta = inputs.delta, !delta.isEmpty {
                return .text(delta)
            }
            return .chip(HonestyStatus.fromWire(inputs.greeksStatus) ?? .unavailable)
        case .gammaTheta:
            guard inputs.isCrypto, inputs.greeksDisplay else { return .chip(.unavailable) }
            let wire = inputs.greeksStatus.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
            if wire == "success", let gamma = inputs.gamma, let theta = inputs.theta,
               !gamma.isEmpty, !theta.isEmpty
            {
                return .text("\(gamma) · \(theta)")
            }
            return .chip(HonestyStatus.fromWire(inputs.greeksStatus) ?? .unavailable)
        case .oi:
            if let oi = inputs.oi { return .text(oi) }
            return .chip(.unavailable)
        case .margin:
            return .chip(.unavailable)
        }
    }
}

/// Six-cell options glance. Crypto lights eapi hosts; NFO keeps DualNoBlend holes.
struct BarOptionsGlanceStrip: View {
    @ObservedObject var viewModel: NotchViewModel

    var body: some View {
        HStack(spacing: 6) {
            ForEach(BarOptionsGlanceStripSeed.kinds, id: \.self) { kind in
                cell(kind)
            }
        }
    }

    private var isCrypto: Bool {
        BarOptionsDeclareSurface.usesCryptoOptions(
            for: viewModel.declareAssetClass,
            slug: viewModel.resolvedDeskSlug,
            instrumentId: viewModel.deskSelectedInstrumentId,
            instrumentType: viewModel.deskSelectedInstrumentType,
        )
    }

    private var inputs: BarOptionsGlanceHonesty.Inputs {
        let entry = viewModel.declEntryPrice.trimmingCharacters(in: .whitespacesAndNewlines)
        return BarOptionsGlanceHonesty.Inputs(
            isCrypto: isCrypto,
            lastEntry: entry.isEmpty ? nil : entry,
            lastStatus: viewModel.deskLastStatus,
            indexStatus: viewModel.deskIndexStatus,
            indexPrice: viewModel.deskIndexPrice,
            greeksDisplay: viewModel.deskGreeksDisplay,
            greeksStatus: viewModel.deskGreeksStatus,
            delta: viewModel.deskGreeksDelta,
            gamma: viewModel.deskGreeksGamma,
            theta: viewModel.deskGreeksTheta,
            oi: isCrypto ? viewModel.deskOiSumOpenInterest : viewModel.deskOiOpenInt,
        )
    }

    @ViewBuilder
    private func cell(_ kind: BarOptionsGlanceStripSeed.Kind) -> some View {
        VStack(alignment: .leading, spacing: 3) {
            Text(title(kind))
                .font(BarDS.monoFont(9, weight: .medium))
                .foregroundColor(BarDS.Text.muted)
                .kerning(0.6)
            valueRow(kind)
            Text(subtitle(kind))
                .font(BarDS.monoFont(10, weight: .regular))
                .foregroundColor(BarDS.Text.muted)
                .lineLimit(2)
                .fixedSize(horizontal: false, vertical: true)
        }
        .padding(.vertical, 7)
        .padding(.horizontal, 9)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(BarDS.Fill.card)
        .clipShape(RoundedRectangle(cornerRadius: 10, style: .continuous))
        .overlay(
            RoundedRectangle(cornerRadius: 10, style: .continuous)
                .stroke(BarDS.Border.card, lineWidth: BarDS.borderThin),
        )
    }

    private func title(_ kind: BarOptionsGlanceStripSeed.Kind) -> String {
        switch kind {
        case .last: return "LAST · YOUR ENTRY"
        case .index: return "INDEX S"
        case .delta: return "DELTA"
        case .gammaTheta: return "GAMMA · THETA"
        case .oi: return "OI"
        case .margin: return "MARGIN"
        }
    }

    @ViewBuilder
    private func valueRow(_ kind: BarOptionsGlanceStripSeed.Kind) -> some View {
        switch BarOptionsGlanceHonesty.value(kind, inputs: inputs) {
        case .text(let text):
            HStack(spacing: 6) {
                Text(text)
                    .font(BarDS.monoFont(13, weight: .medium))
                    .foregroundColor(BarDS.Text.primary)
                    .lineLimit(1)
                    .minimumScaleFactor(0.7)
                if kind == .last,
                   let freshness = BarDeskLastFormatting.freshnessBesideLast(
                       status: viewModel.deskLastStatus
                   )
                {
                    Text(freshness)
                        .font(BarDS.monoFont(10, weight: .regular))
                        .foregroundColor(BarDS.Accent.amber)
                }
            }
        case .chip(let status):
            HonestyChip(status: status)
        }
    }

    private func subtitle(_ kind: BarOptionsGlanceStripSeed.Kind) -> String {
        switch kind {
        case .last: return BarOptionsGlanceCopy.lastNote
        case .index: return isCrypto ? BarOptionsGlanceCopy.cryptoIndexNote : BarOptionsGlanceCopy.nfoIndexNote
        case .delta, .gammaTheta:
            return isCrypto ? BarOptionsGlanceCopy.cryptoGreeksNote : BarOptionsGlanceCopy.nfoGreeksNote
        case .oi: return isCrypto ? BarOptionsGlanceCopy.cryptoOiNote : BarOptionsGlanceCopy.nfoOiNote
        case .margin: return BarOptionsGlanceCopy.marginNote
        }
    }
}
