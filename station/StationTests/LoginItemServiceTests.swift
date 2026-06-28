import Foundation
import Testing
@testable import Station

@MainActor
struct LoginItemServiceTests {
    private func makeService(
        backend: FakeLoginItemBackend = FakeLoginItemBackend(),
        defaults: UserDefaults? = nil
    ) -> (LoginItemService, FakeLoginItemBackend, UserDefaults) {
        let suiteName = "StationTests.LoginItem.\(UUID().uuidString)"
        let userDefaults = defaults ?? UserDefaults(suiteName: suiteName)!
        let service = LoginItemService(backend: backend, defaults: userDefaults)
        return (service, backend, userDefaults)
    }

    // T_default_off: fresh install → isRegistered == false
    @Test func defaultOffOnFreshInstall() {
        let (service, _, _) = makeService()
        #expect(service.isRegistered == false)
    }

    // T_toggle_on: setRegistered(true) → SMAppService registers (fake verifies call)
    @Test func toggleOnRegistersLoginItem() throws {
        let (service, backend, _) = makeService()
        try service.setRegistered(true)
        #expect(backend.registerCallCount == 1)
        #expect(backend.unregisterCallCount == 0)
        #expect(service.isRegistered == true)
    }

    // T_toggle_off: setRegistered(false) → SMAppService unregisters
    @Test func toggleOffUnregistersLoginItem() throws {
        let fakeBackend = FakeLoginItemBackend()
        fakeBackend.registrationStatus = .enabled
        let (service, backend, _) = makeService(backend: fakeBackend)
        service.syncStatusOnLaunch()
        #expect(service.isRegistered == true)

        try service.setRegistered(false)
        #expect(backend.unregisterCallCount == 1)
        #expect(service.isRegistered == false)
    }

    // T_resync_on_launch: user disabled in System Settings → syncStatusOnLaunch corrects state
    @Test func resyncOnLaunchDetectsSystemSettingsMismatch() {
        let fakeBackend = FakeLoginItemBackend()
        fakeBackend.registrationStatus = .enabled
        let (service, backend, _) = makeService(backend: fakeBackend)
        service.syncStatusOnLaunch()
        #expect(service.isRegistered == true)

        fakeBackend.registrationStatus = .notRegistered
        service.syncStatusOnLaunch()
        #expect(service.isRegistered == false)
    }

    // T_prompt_once: first-run prompt dismissed once → not shown again on second launch
    @Test func promptDismissedPersistsAcrossServiceInstances() {
        let suiteName = "StationTests.LoginItem.prompt.\(UUID().uuidString)"
        let defaults = UserDefaults(suiteName: suiteName)!
        defer { defaults.removePersistentDomain(forName: suiteName) }

        let backend = FakeLoginItemBackend()
        let first = LoginItemService(backend: backend, defaults: defaults)
        #expect(first.isPromptDismissed == false)

        first.markPromptDismissed()
        #expect(first.isPromptDismissed == true)

        let second = LoginItemService(backend: backend, defaults: defaults)
        #expect(second.isPromptDismissed == true)
    }
}
