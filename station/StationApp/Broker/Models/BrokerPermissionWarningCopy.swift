import Foundation

public enum BrokerPermissionWarningCopy {
    public static func label(for warning: BrokerPermissionWarning) -> String {
        switch warning {
        case .tradeEnabled:
            return "Trade-enabled API key — read-only keys are safer."
        case .unverifiable:
            return "Key permissions could not be fully verified."
        }
    }
}
