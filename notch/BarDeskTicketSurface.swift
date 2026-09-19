import Foundation

/// Routes the venue ticket. Orthogonal to `BarOptionsDeclareSurface` (NFO vs crypto vs standard form).
/// Equity / Kotak cash have no COM ticket widgets.
enum BarDeskTicketSurface {
    enum Surface: Equatable {
        case none
        case spot
        case usdm
        case coinm
        case cryptoOptions
    }

    static func surface(
        for asset: BarDeclareAssetClass,
        slug: String?,
        instrumentId: String
    ) -> Surface {
        switch asset {
        case .equity:
            return .none
        case .usdm:
            return DeskCatalogAllowlist.isBinanceDesk(slug) ? .usdm : .none
        case .coinm:
            return DeskCatalogAllowlist.isBinanceDesk(slug) ? .coinm : .none
        case .spot:
            return DeskCatalogAllowlist.isBinanceDesk(slug) ? .spot : .none
        case .options:
            if BarDeskTemplate.isBinanceOptionsSelection(assetClass: asset, instrumentId: instrumentId) {
                return .cryptoOptions
            }
            return .none
        }
    }

    static func bookId(
        for asset: BarDeclareAssetClass,
        slug: String?,
        instrumentId: String
    ) -> String? {
        switch surface(for: asset, slug: slug, instrumentId: instrumentId) {
        case .none:
            return nil
        case .spot:
            return BarDeskTemplate.binanceComSpotBookId
        case .usdm:
            return BarDeskTemplate.binanceComUsdmBookId
        case .coinm:
            return BarDeskTemplate.binanceComCoinmBookId
        case .cryptoOptions:
            return BarDeskTemplate.binanceComOptionsBookId
        }
    }

    static func usesVenueTicket(
        for asset: BarDeclareAssetClass,
        slug: String?,
        instrumentId: String
    ) -> Bool {
        surface(for: asset, slug: slug, instrumentId: instrumentId) != .none
    }
}
