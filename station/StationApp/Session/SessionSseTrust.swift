import CryptoKit
import Foundation

/// Ed25519 SSE envelope verification + agent pubkey pinning helpers for SessionModel.
enum SessionSseTrust {
    /// Raw UTF-8 slice of the JSON object for `"payload"` — must match agent
    /// `serde_json::to_vec` bytes for hashing.
    static func extractJsonObjectUtf8(forKey key: String, inEnvelopeJsonLine line: String) -> Data? {
        let needle = "\"\(key)\":"
        guard let range = line.range(of: needle) else { return nil }
        var idx = range.upperBound
        while idx < line.endIndex, line[idx].isWhitespace {
            line.formIndex(after: &idx)
        }
        guard idx < line.endIndex, line[idx] == "{" else { return nil }
        let start = idx
        var depth = 0
        while idx < line.endIndex {
            let ch = line[idx]
            if ch == "{" { depth += 1 }
            else if ch == "}" {
                depth -= 1
                if depth == 0 {
                    let end = line.index(after: idx)
                    let sub = line[start..<end]
                    return String(sub).data(using: .utf8)
                }
            }
            line.formIndex(after: &idx)
        }
        return nil
    }

    static func verifySseEd25519(
        rawEnvelopeJson: String,
        eventId: String,
        typeStr: String,
        sigB64: String,
        pubKeyB64: String
    ) -> Bool {
        guard let payloadUtf8 = extractJsonObjectUtf8(forKey: "payload", inEnvelopeJsonLine: rawEnvelopeJson) else {
            return false
        }
        let digest = SHA256.hash(data: payloadUtf8)
        let digestHex = digest.map { String(format: "%02x", $0) }.joined()
        let msg = "\(eventId)\n\(typeStr)\n\(digestHex)"
        guard let sigData = Data(base64Encoded: sigB64, options: [.ignoreUnknownCharacters]) else { return false }
        guard let pkData = Data(base64Encoded: pubKeyB64, options: [.ignoreUnknownCharacters]),
              pkData.count == 32 else { return false }
        guard let pk = try? Curve25519.Signing.PublicKey(rawRepresentation: pkData) else { return false }
        guard let msgData = msg.data(using: .utf8) else { return false }
        return pk.isValidSignature(sigData, for: msgData)
    }
}
