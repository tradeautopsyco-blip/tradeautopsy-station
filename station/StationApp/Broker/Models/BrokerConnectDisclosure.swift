import Foundation

public enum BrokerConnectDisclosure {
    public static func message(behavioralAnalysisOptedOut: Bool) -> String {
        if behavioralAnalysisOptedOut {
            return "Connecting starts broker sync. Behavioral and account-metadata upload stays off because behavioral analysis is opted out."
        }
        return "Connecting starts broker sync and uploads behavioral and account metadata unless you opt out of behavioral analysis."
    }
}
