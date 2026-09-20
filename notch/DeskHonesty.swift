import Foundation

/// Dual-desk honesty for Notch. Mirrors Station `DeskHonesty` without `BrokerCatalog`
/// (Notch cannot import Station). Mixed USD+INR → no hero number.
enum DeskHonestyResolution: Equatable, Sendable {
    case single(quoteCurrency: String, calcProfileId: String, brokerSlug: String)
    case dualNoBlend(profiles: [DeskConnectionProfile])
    case none
}

struct DeskConnectionProfile: Equatable, Sendable {
    let brokerSlug: String
    let quoteCurrency: String
    let calcProfileId: String

    init(brokerSlug: String, quoteCurrency: String, calcProfileId: String) {
        self.brokerSlug = brokerSlug
        self.quoteCurrency = quoteCurrency
        self.calcProfileId = calcProfileId
    }
}

enum DeskHonesty {
    static func profile(forBrokerSlug slug: String?) -> DeskConnectionProfile? {
        guard let slug, !slug.isEmpty else { return nil }
        guard let ccy = DeskMoneyFormatting.quoteCurrency(forBrokerSlug: slug),
              let calc = DeskMoneyFormatting.calcProfileId(forBrokerSlug: slug)
        else {
            return nil
        }
        return DeskConnectionProfile(brokerSlug: slug, quoteCurrency: ccy, calcProfileId: calc)
    }

    static func resolve(activeSlugs: [String]) -> DeskHonestyResolution {
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
    static func heroQuoteCurrency(activeSlugs: [String]) -> String? {
        switch resolve(activeSlugs: activeSlugs) {
        case let .single(quoteCurrency, _, _):
            return quoteCurrency
        case .dualNoBlend, .none:
            return nil
        }
    }
}
