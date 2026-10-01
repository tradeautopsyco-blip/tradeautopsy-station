import Foundation

enum BarOpenStartGate {
    /// Open requires one local rule only — calm/confidence live on Plan (`harness-open-plan-working.md` §1).
    static func unlocked(rule: String) -> Bool {
        !rule.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    @available(*, deprecated, message: "Use unlocked(rule:) — calm/confidence removed from Open.")
    static func unlocked(calm: Int, confidence: Int, rule: String) -> Bool {
        unlocked(rule: rule)
    }

    /// Pre-market index row only when the morning brief actually carries index prints.
    static func showsPreMarketIndices(morningBrief: MorningBrief?) -> Bool {
        guard let b = morningBrief else { return false }
        return b.niftyFutures > 0 || b.bankniftyFutures > 0 || b.vix > 0
    }

    /// COM / named futures never paint Nifty–BNF–VIX chips.
    static func hidesIndexChips(bookId: String?, assetClass: BarDeclareAssetClass, slug: String?) -> Bool {
        if assetClass.isNamedComFutures { return true }
        let book = bookId?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? ""
        if book.hasPrefix("binance-com") { return true }
        let s = slug?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? ""
        return s == "binance_com" || s == "binance"
    }
}
