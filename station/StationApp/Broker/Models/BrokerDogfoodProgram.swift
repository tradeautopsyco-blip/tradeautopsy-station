import Foundation

/// Founder G7 dogfood while catalog rows stay **Planned** (decision 8).
/// Locks are SHIPPING; catalog flips to **Enabled** only after the signed dogfood plan.
public enum BrokerDogfoodProgram {
    public static let connectWhilePlannedSlugs: Set<String> = []

    public static func allowsConnectWhilePlanned(slug: String) -> Bool {
        connectWhilePlannedSlugs.contains(slug)
    }
}
