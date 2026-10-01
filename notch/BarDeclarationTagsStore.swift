import Foundation

/// Per-`declaration_id` tags (Station-local until Console contract exists).
final class BarDeclarationTagsStore {
    static let shared = BarDeclarationTagsStore()
    private let key = "tradeautopsy.notch.declaration_tags"

    private init() {}

    func tags(for declarationId: String) -> [String] {
        let id = declarationId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !id.isEmpty else { return [] }
        guard let map = UserDefaults.standard.dictionary(forKey: key) as? [String: [String]] else {
            return []
        }
        return map[id] ?? []
    }

    func setTags(_ tags: [String], for declarationId: String) {
        let id = declarationId.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !id.isEmpty else { return }
        var map = (UserDefaults.standard.dictionary(forKey: key) as? [String: [String]]) ?? [:]
        let cleaned = tags.map { $0.trimmingCharacters(in: .whitespacesAndNewlines) }.filter { !$0.isEmpty }
        if cleaned.isEmpty {
            map.removeValue(forKey: id)
        } else {
            map[id] = cleaned
        }
        UserDefaults.standard.set(map, forKey: key)
    }
}
