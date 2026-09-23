import AppKit
import Foundation

public enum BrokerConnectBrowser {
    public static let openURL: @Sendable (URL) -> Void = { url in
        NSWorkspace.shared.open(url)
    }
}
