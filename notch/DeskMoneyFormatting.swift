import Foundation

/// Desk money formatting keyed by active connection `quoteCurrency` (R7).
/// Never FX-blends USD+INR — callers must choose a single currency or dual strip.
public enum DeskMoneyFormatting {
    /// Format a signed PnL in the given ISO currency (`USD`, `INR`, …).
    public static func formatSigned(
        _ value: Double,
        quoteCurrency: String,
        fractionDigits: Int? = nil
    ) -> String {
        let code = quoteCurrency.trimmingCharacters(in: .whitespacesAndNewlines).uppercased()
        let digits = fractionDigits ?? defaultFractionDigits(for: code)
        let formatter = NumberFormatter()
        formatter.numberStyle = .currency
        formatter.currencyCode = code.isEmpty ? "USD" : code
        formatter.maximumFractionDigits = digits
        formatter.minimumFractionDigits = min(2, digits)
        let absText = formatter.string(from: NSNumber(value: abs(value)))
            ?? fallback(abs(value), code: code)
        if value > 0 { return "+\(absText)" }
        if value < 0 { return "-\(absText)" }
        return absText
    }

    /// Whole-currency (no cents) — collapsed notch / exposure style.
    public static func formatWhole(_ value: Double, quoteCurrency: String) -> String {
        let signed = formatSigned(value, quoteCurrency: quoteCurrency, fractionDigits: 0)
        if signed.hasPrefix("+") {
            return String(signed.dropFirst())
        }
        return signed
    }

    public static func defaultFractionDigits(for currencyCode: String) -> Int {
        switch currencyCode.uppercased() {
        case "INR":
            return 0
        default:
            return 2
        }
    }

    /// First-pair slug → quote currency (mirrors BrokerCatalog / R7).
    public static func quoteCurrency(forBrokerSlug slug: String?) -> String? {
        guard let raw = slug?.trimmingCharacters(in: .whitespacesAndNewlines), !raw.isEmpty else {
            return nil
        }
        switch raw.lowercased() {
        case "binance_com", "binance", "binance_us":
            return "USD"
        case "kotak_neo", "kotak", "zerodha_kite", "zerodha", "kite":
            return "INR"
        default:
            return nil
        }
    }

    public static func calcProfileId(forBrokerSlug slug: String?) -> String? {
        guard let raw = slug?.trimmingCharacters(in: .whitespacesAndNewlines), !raw.isEmpty else {
            return nil
        }
        switch raw.lowercased() {
        case "binance_com", "binance", "binance_us":
            return "crypto_spot_usd"
        case "kotak_neo", "kotak", "zerodha_kite", "zerodha", "kite":
            return "equities_inr_cash"
        default:
            return nil
        }
    }

    private static func fallback(_ value: Double, code: String) -> String {
        switch code {
        case "INR":
            return "₹\(Int(value.rounded()))"
        case "USD":
            return String(format: "$%.2f", value)
        default:
            return String(format: "%.2f %@", value, code)
        }
    }
}
