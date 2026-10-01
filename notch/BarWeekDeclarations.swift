import Foundation

struct BarWeekDeclarationRow: Equatable, Identifiable, Sendable {
    var id: String
    var symbol: String
    var side: String
    var status: String
    var localDate: String?
    var isClosed: Bool
}

enum BarWeekDeclarations {
    static func parseWeekList(_ data: Data) -> [BarWeekDeclarationRow] {
        guard let root = try? JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            return []
        }
        let items = (root["items"] as? [[String: Any]]) ?? []
        return items.compactMap { row in
            guard let id = row["id"] as? String, !id.isEmpty else { return nil }
            let status = (row["status"] as? String ?? "").uppercased()
            let closed = status.contains("CLOSED") || status.contains("CANCEL") || status == "FILLED"
            return BarWeekDeclarationRow(
                id: id,
                symbol: row["symbol"] as? String ?? "—",
                side: row["side"] as? String ?? "",
                status: status,
                localDate: row["local_date"] as? String,
                isClosed: closed
            )
        }
        .filter(\.isClosed)
    }
}
