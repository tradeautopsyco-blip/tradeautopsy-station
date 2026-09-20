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
}
