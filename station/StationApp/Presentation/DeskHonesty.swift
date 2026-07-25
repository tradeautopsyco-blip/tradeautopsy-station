import Foundation
import Notch

/// Dual-desk honesty resolution against `BrokerCatalog` (R7 Phase 5).
/// Mixed USD+INR → `.dualNoBlend` — never FX-blend into one hero number.
public enum DeskHonestyResolution: Equatable, Sendable {
    case single(quoteCurrency: String, calcProfileId: String, brokerSlug: String)
    case dualNoBlend(profiles: [DeskConnectionProfile])
    case none
}

public struct DeskConnectionProfile: Equatable, Sendable {
    public let brokerSlug: String
    public let quoteCurrency: String
    public let calcProfileId: String

    public init(brokerSlug: String, quoteCurrency: String, calcProfileId: String) {
        self.brokerSlug = brokerSlug
        self.quoteCurrency = quoteCurrency
        self.calcProfileId = calcProfileId
    }
}

public enum DeskHonesty {
    public static func profile(forBrokerSlug slug: String?) -> DeskConnectionProfile? {
        guard let slug, !slug.isEmpty else { return nil }
        if let descriptor = BrokerCatalog.descriptor(for: slug) {
            return DeskConnectionProfile(
                brokerSlug: descriptor.slug,
                quoteCurrency: descriptor.quoteCurrency,
                calcProfileId: descriptor.calcProfileId
            )
        }
        guard let ccy = DeskMoneyFormatting.quoteCurrency(forBrokerSlug: slug),
              let calc = DeskMoneyFormatting.calcProfileId(forBrokerSlug: slug)
        else {
            return nil
        }
        return DeskConnectionProfile(brokerSlug: slug, quoteCurrency: ccy, calcProfileId: calc)
    }

    public static func resolve(activeSlugs: [String]) -> DeskHonestyResolution {
        var profiles: [DeskConnectionProfile] = []
        var seen = Set<String>()
        for raw in activeSlugs {
            let slug = raw.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
            guard !slug.isEmpty, !seen.contains(slug) else { continue }
            seen.insert(slug)
            if let p = profile(forBrokerSlug: slug) {
                profiles.append(p)
            }
        }
        guard !profiles.isEmpty else { return .none }
        if profiles.count == 1 {
            let p = profiles[0]
            return .single(
                quoteCurrency: p.quoteCurrency,
                calcProfileId: p.calcProfileId,
                brokerSlug: p.brokerSlug
            )
        }
        let currencies = Set(profiles.map(\.quoteCurrency))
        if currencies.count == 1 {
            let p = profiles[0]
            return .single(
                quoteCurrency: p.quoteCurrency,
                calcProfileId: p.calcProfileId,
                brokerSlug: p.brokerSlug
            )
        }
        return .dualNoBlend(profiles: profiles)
    }

    /// Currency for a single hero/pulse surface. Dual mixed → nil (no blend).
    public static func heroQuoteCurrency(activeSlugs: [String]) -> String? {
        switch resolve(activeSlugs: activeSlugs) {
        case let .single(quoteCurrency, _, _):
            return quoteCurrency
        case .dualNoBlend, .none:
            return nil
        }
    }
}
