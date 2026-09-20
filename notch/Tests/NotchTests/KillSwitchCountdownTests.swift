import Foundation
import Testing
@testable import Notch

struct KillSwitchCountdownTests {
    @Test func remainingUsesExpiresAtNotInventedNinety() {
        let now: Int64 = 1_700_000_000_000
        #expect(KillSwitchCountdown.remainingSecs(expiresAtMs: now + 45_000, countdownSecs: 90, nowMs: now) == 45)
        #expect(KillSwitchCountdown.remainingSecs(expiresAtMs: now - 1_000, countdownSecs: 90, nowMs: now) == 0)
        #expect(KillSwitchCountdown.remainingSecs(expiresAtMs: nil, countdownSecs: 60, nowMs: now) == 60)
        #expect(KillSwitchCountdown.remainingSecs(expiresAtMs: nil, countdownSecs: nil, nowMs: now) == nil)
    }

    @Test func dnsHostsFollowArmedBrokerNeverDefaultKotak() {
        let kotak = KillDnsHosts.hosts(forBrokerSlug: "kotak_neo").map(\.url)
        #expect(kotak.contains("cis.kotaksecurities.com"))
        #expect(kotak.contains("gw-napi.kotaksecurities.com"))
        let com = KillDnsHosts.hosts(forBrokerSlug: "binance_com").map(\.url)
        #expect(com.contains("api.binance.com"))
        #expect(com.contains("stream.binance.com"))
        #expect(KillDnsHosts.hosts(forBrokerSlug: nil).isEmpty)
        #expect(KillDnsHosts.hosts(forBrokerSlug: "unknown").isEmpty)
    }
}

@MainActor
struct KillSwitchExpiresAtPillTests {
    @Test func killStatePrefersExpiresAtMsOverLocalNinety() {
        let vm = NotchViewModel(planSurfaceOnly: true)
        let now = KillSwitchCountdown.nowMs()
        _ = vm.applyDaemonEventPayload(
            type: "kill_switch_state",
            payload: [
                "active": true,
                "level": "L3",
                "countdown_secs": 90,
                "expires_at_ms": now + 45_000,
                "requires_ack": true,
            ],
            immediateToolbarShow: false
        )
        #expect(vm.killSwitchActive)
        #expect(vm.killSwitchExpiresAtMs == now + 45_000)
        let remaining = vm.killSwitchCountdownSecs ?? -1
        #expect(remaining <= 45)
        #expect(remaining >= 44)
        #expect(remaining != 90)
    }
}
