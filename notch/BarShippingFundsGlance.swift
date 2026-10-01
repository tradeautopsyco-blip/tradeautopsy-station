import Foundation

/// Obtain `funds` on the **shipping** AccountBook (Kotak cash, Binance spot) — not named USDM/NFO books.
enum BarShippingFundsGlance {
    struct Presentation: Equatable, Sendable {
        var freeText: String
        var status: String
        var bookId: String

        static let dark = Presentation(freeText: "—", status: "unavailable", bookId: "")

        var isLit: Bool { status == "success" }
    }

    static func present(
        fundsEnvelope: [String: Any]?,
        shippingBookId: String,
        quoteCurrency: String
    ) -> Presentation {
        let snap = BarAccountChrome.compose(
            shippingBookId: shippingBookId,
            quoteCurrency: quoteCurrency,
            funds: fundsEnvelope,
            holdings: nil,
            positions: nil,
            orders: nil
        )
        return Presentation(
            freeText: snap.freeText,
            status: snap.fundsStatus,
            bookId: shippingBookId
        )
    }

    static func shippingContext(startSlug: String?) -> (adapter: String, bookId: String)? {
        guard let book = BarAccountChrome.shippingBookId(forStartSlug: startSlug),
              let adapter = BarAccountChrome.obtainAdapterId(forStartSlug: startSlug)
        else { return nil }
        return (adapter, book)
    }
}
