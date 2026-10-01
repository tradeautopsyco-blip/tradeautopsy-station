import Foundation

/// Debrief A cites Station Today trip net. NFO/options stay a dash — Notch does not invent a second PnL.
enum BarDebriefCitedNet {
    static func display(optionsOrNfo: Bool, net: Double?, currency: String?) -> String {
        if optionsOrNfo { return "—" }
        guard let net, net.isFinite, let ccy = currency?.trimmingCharacters(in: .whitespacesAndNewlines), !ccy.isEmpty
        else { return "—" }
        return DeskMoneyFormatting.formatSigned(net, quoteCurrency: ccy)
    }

    static func net(
        symbol: String?,
        trips: [(symbol: String, net: Double)]
    ) -> Double? {
        let needle = symbol?.trimmingCharacters(in: .whitespacesAndNewlines).uppercased() ?? ""
        guard !needle.isEmpty else { return nil }
        return trips.last { $0.symbol.uppercased() == needle }?.net
    }

    /// Unique symbol match only — multiple same-symbol trips stay a dash (`harness-trade-arc.md` §6).
    static func netUniqueSymbol(
        symbol: String?,
        trips: [(symbol: String, net: Double)]
    ) -> Double? {
        let needle = symbol?.trimmingCharacters(in: .whitespacesAndNewlines).uppercased() ?? ""
        guard !needle.isEmpty else { return nil }
        let matches = trips.filter { $0.symbol.uppercased() == needle }
        guard matches.count == 1 else { return nil }
        return matches[0].net
    }

    static func displayUniqueOrAmbiguous(
        optionsOrNfo: Bool,
        symbol: String?,
        trips: [(symbol: String, net: Double)],
        currency: String?
    ) -> String {
        if optionsOrNfo { return "—" }
        guard let net = netUniqueSymbol(symbol: symbol, trips: trips) else {
            let needle = symbol?.trimmingCharacters(in: .whitespacesAndNewlines).uppercased() ?? ""
            if !needle.isEmpty,
               trips.filter({ $0.symbol.uppercased() == needle }).count > 1
            {
                return "— · multiple trips for symbol"
            }
            return "—"
        }
        return display(optionsOrNfo: false, net: net, currency: currency)
    }
}
