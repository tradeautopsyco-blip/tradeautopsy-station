import Foundation

/// Local playbooks (Console authoring TBD — `founder-backlog.md` §5).
struct BarPlaybook: Equatable, Codable, Identifiable {
    var id: String
    var title: String
    var body: String
}

enum BarPlaybookStore {
    private static let key = "tradeautopsy.notch.playbooks"

    static func load() -> [BarPlaybook] {
        guard let data = UserDefaults.standard.data(forKey: key),
              let decoded = try? JSONDecoder().decode([BarPlaybook].self, from: data)
        else { return [] }
        return decoded
    }

    static func save(_ items: [BarPlaybook]) {
        guard let data = try? JSONEncoder().encode(items) else { return }
        UserDefaults.standard.set(data, forKey: key)
    }

    static func upsert(title: String, body: String) -> BarPlaybook {
        var all = load()
        let item = BarPlaybook(id: UUID().uuidString, title: title, body: body)
        all.append(item)
        save(all)
        return item
    }
}
