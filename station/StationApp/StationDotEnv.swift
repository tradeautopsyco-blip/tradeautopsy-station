import Foundation

/// Loads untracked Station ops env (`~/.tradeautopsy/station.env`) into the spawned agent.
/// GUI `open` does not inherit a shell profile, so WorkOS / Console URLs must come from this file.
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
}
