import Foundation

/// Pure state step for `@Published` declaration id sync with optimistic armed flow (#119).
enum BarDeclarationIdEvent: Equatable {
    case declareSucceeded(String)
    case declarationClosed
}

enum BarDeclarationIdReducer {
    /// - Parameter state: Previous published id, if any.
    static func apply(event: BarDeclarationIdEvent, state: String?) -> String? {
        switch event {
        case .declareSucceeded(let id):
            if id.isEmpty { return state }
            return id
        case .declarationClosed:
            return nil
        }
    }
}
