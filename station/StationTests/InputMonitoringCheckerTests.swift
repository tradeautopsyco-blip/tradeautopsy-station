import Testing
@testable import Station

struct InputMonitoringCheckerTests {
    @Test func defaultCheckerTreatsInputMonitoringAsNotRequired() {
        let checker = DefaultInputMonitoringChecker()
        #expect(checker.isInputMonitoringGranted())
        checker.requestInputMonitoringAccess()
        #expect(checker.isInputMonitoringGranted())
    }
}
