import Testing
@testable import Station

struct DeskChromeTests {
    @Test func workspaceRoutesIncludeSingleToday() {
        let routes = StationRoute.workspaceRoutes.filter { $0 == .today }
        #expect(routes.count == 1)
    }

    @Test func deskTypeScaleUsesEightPointGrid() {
        #expect(DeskChrome.Space.unit == 8)
        #expect(DeskChrome.Space.pageInset == 24)
    }
}
