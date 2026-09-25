import Foundation
import Testing

struct StationAppInfoPlistTests {
    private static func repoInfoPlistURL() -> URL {
        let testFile = URL(fileURLWithPath: #filePath)
        return testFile
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("StationApp/Info.plist")
    }

    @Test func infoPlistDoesNotDeclareInputMonitoringUsage() throws {
        let url = Self.repoInfoPlistURL()
        let data = try Data(contentsOf: url)
        let plist = try PropertyListSerialization.propertyList(from: data, format: nil) as? [String: Any]
        #expect(plist != nil)
        #expect(plist?["NSInputMonitoringUsageDescription"] == nil)
    }

    @Test func infoPlistDeclaresSparkleFeedOnUpdatesDomain() throws {
        let url = Self.repoInfoPlistURL()
        let data = try Data(contentsOf: url)
        let plist = try PropertyListSerialization.propertyList(from: data, format: nil) as? [String: Any]
        let feed = plist?["SUFeedURL"] as? String
        #expect(feed == "https://updates.tradeautopsy.in/appcast.xml")
        #expect((plist?["SUPublicEDKey"] as? String)?.isEmpty == false)
    }
}
