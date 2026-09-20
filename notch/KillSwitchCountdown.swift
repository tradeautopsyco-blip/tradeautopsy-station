import Foundation

/// Remaining Kill overlay time from policy `expires_at_ms`. Never invent a local 90s.
public enum KillSwitchCountdown {
    public static func remainingSecs(expiresAtMs: Int64?, countdownSecs: Int?, nowMs: Int64) -> Int? {
        if let exp = expiresAtMs {
            return max(0, Int((exp - nowMs) / 1000))
        }
        return countdownSecs.map { max(0, $0) }
    }

    public static func parseMs(_ raw: Any?) -> Int64? {
        if let n = raw as? Int { return Int64(n) }
        if let n = raw as? Int64 { return n }
        if let n = raw as? Double, n.isFinite { return Int64(n) }
        if let n = raw as? NSNumber { return n.int64Value }
        return nil
    }

    public static func nowMs(_ date: Date = Date()) -> Int64 {
        Int64(date.timeIntervalSince1970 * 1000)
    }
}

/// DNS sinkhole hosts must match agent `dns_block::hosts_for_broker`. Empty → L3 refuse (R8).
public enum KillDnsHosts {
    public static func hosts(forBrokerSlug slug: String?) -> [(label: String, url: String)] {
        let key = slug?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? ""
        let urls: [String]
        switch key {
        case "kotak", "kotak_neo":
            urls = [
                "cis.kotaksecurities.com",
                "neo.kotaksecurities.com",
                "mis.kotaksecurities.com",
                "gw-napi.kotaksecurities.com",
                "mnapi.kotaksecurities.com",
                "cnapi.kotaksecurities.com",
                "napi.kotaksecurities.com",
                "e21.kotaksecurities.com",
                "e22.kotaksecurities.com",
                "e41.kotaksecurities.com",
                "e43.kotaksecurities.com",
                "lapi.kotaksecurities.com",
            ]
        case "binance", "binance_com":
            urls = [
                "api.binance.com",
                "eapi.binance.com",
                "fapi.binance.com",
                "dapi.binance.com",
                "stream.binance.com",
                "fstream.binance.com",
                "dstream.binance.com",
            ]
        default:
            urls = []
        }
        return urls.map { ($0, $0) }
    }
}
