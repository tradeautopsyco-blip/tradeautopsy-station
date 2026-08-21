import Foundation
import Testing
@testable import Station

struct StationDotEnvTests {
    @Test func parseIgnoresCommentsAndStripsQuotes() {
        let text = """
        # comment
        WORKOS_STATION_CLIENT_ID="client_test"
        TRADEAUTOPSY_SERVER_BASE_URL=https://tradeautopsy.in

        EMPTY=
        """
        let parsed = StationDotEnv.parse(text)
        #expect(parsed["WORKOS_STATION_CLIENT_ID"] == "client_test")
        #expect(parsed["TRADEAUTOPSY_SERVER_BASE_URL"] == "https://tradeautopsy.in")
        #expect(parsed["EMPTY"] == "")
        #expect(parsed["# comment"] == nil)
    }

    @Test func mergeDoesNotOverrideSetKeys() {
        let dir = FileManager.default.temporaryDirectory
        let url = dir.appendingPathComponent("station-dotenv-\(UUID().uuidString).env")
        try? "WORKOS_STATION_CLIENT_ID=from_file\nOTHER=file_only\n".write(to: url, atomically: true, encoding: .utf8)
        defer { try? FileManager.default.removeItem(at: url) }

        let merged = StationDotEnv.merge(
            into: ["WORKOS_STATION_CLIENT_ID": "from_process"],
            from: url
        )
        #expect(merged["WORKOS_STATION_CLIENT_ID"] == "from_process")
        #expect(merged["OTHER"] == "file_only")
    }
}
