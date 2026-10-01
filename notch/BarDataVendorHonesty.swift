import Foundation

/// When broker adapters lack chain/OI/history, prompt for a data vendor — never invent rows.
enum BarDataVendorHonesty {
    static let vendorPrompt =
        "This book’s adapter does not expose this lane. Add a licensed data vendor in Station Settings → Data sources — Station will not invent chain, OI, or history."

    static func panelTitle(for capability: String) -> String {
        switch capability {
        case "option_chain": return "Option chain unavailable"
        case "open_interest": return "Open interest unavailable"
        case "history": return "History unavailable"
        default: return "Market data unavailable"
        }
    }

    static func body(status: String, capability: String) -> String {
        let wire = status.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        if wire == "success" { return "" }
        if wire == "unlicensed" {
            return "\(panelTitle(for: capability)). Licensed vendor required — \(vendorPrompt)"
        }
        return "\(panelTitle(for: capability)). \(vendorPrompt)"
    }
}
