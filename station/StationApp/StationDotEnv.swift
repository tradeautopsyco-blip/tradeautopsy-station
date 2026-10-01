import Foundation

/// Production values baked into every shipped Station. A `~/.tradeautopsy/station.env`
/// file is a developer override only — downloaders do not create one.
public enum StationShippedConfig {
    public static let consoleBaseURL = "https://www.tradeautopsy.in"
    /// Public WorkOS AuthKit client id (same app as Console). Not a secret.
    public static let workosStationClientID = "client_01KBEHG7XWN269N97M1EAKXV07"
}

/// Loads untracked Station ops env (`~/.tradeautopsy/station.env`) into the spawned agent.
/// Missing keys fall back to `StationShippedConfig`.
public enum StationDotEnv {
    public static var defaultURL: URL {
        FileManager.default.homeDirectoryForCurrentUser
            .appendingPathComponent(".tradeautopsy/station.env")
    }

    /// Parse KEY=VALUE lines. `#` comments and empty lines are ignored. Quotes around values are stripped.
    public static func parse(_ text: String) -> [String: String] {
        var out: [String: String] = [:]
        for rawLine in text.split(whereSeparator: \.isNewline) {
            let line = rawLine.trimmingCharacters(in: .whitespaces)
            if line.isEmpty || line.hasPrefix("#") { continue }
            guard let eq = line.firstIndex(of: "=") else { continue }
            let key = String(line[..<eq]).trimmingCharacters(in: .whitespaces)
            guard !key.isEmpty, !key.hasPrefix("#") else { continue }
            var value = String(line[line.index(after: eq)...]).trimmingCharacters(in: .whitespaces)
            if value.count >= 2 {
                let first = value.first
                let last = value.last
                if (first == "\"" && last == "\"") || (first == "'" && last == "'") {
                    value = String(value.dropFirst().dropLast())
                }
            }
            out[key] = value
        }
        return out
    }

    public static func load(from url: URL = defaultURL) -> [String: String] {
        guard let text = try? String(contentsOf: url, encoding: .utf8) else { return [:] }
        return parse(text)
    }

    /// Fill missing or empty keys from the file. Explicit process/scheme env wins.
    public static func merge(into environment: [String: String], from url: URL = defaultURL) -> [String: String] {
        var out = environment
        for (key, value) in load(from: url) {
            if out[key]?.isEmpty != false {
                out[key] = value
            }
        }
        return out
    }

    /// Production Console + WorkOS client when the process and `station.env` left them blank.
    public static func applyShippedDefaults(_ environment: inout [String: String]) {
        fillIfBlank(&environment, key: "TRADEAUTOPSY_SERVER_BASE_URL", value: StationShippedConfig.consoleBaseURL)
        fillIfBlank(&environment, key: "WORKOS_STATION_CLIENT_ID", value: StationShippedConfig.workosStationClientID)
    }

    private static func fillIfBlank(_ environment: inout [String: String], key: String, value: String) {
        let current = environment[key]?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        if current.isEmpty {
            environment[key] = value
        }
    }
}
