import Foundation

enum BarOpenStartGate {
    static func unlocked(calm: Int, confidence: Int, rule: String) -> Bool {
        calm >= 1 && confidence >= 1 && !rule.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
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
