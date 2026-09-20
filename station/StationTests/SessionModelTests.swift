import Foundation
import Notch
import Testing
@testable import Station

@MainActor
struct SessionModelTests {
    @Test func positionsDecodeMapsTickerAliasesAndUnrealized() throws {
        let json = """
        {
          "kill_switch_active": false,
          "open_orders": 1,
          "positions": [
            {
              "trdSym": "RELIANCE-EQ",
              "quantity": 10,
              "unrealizedPnl": 125.5,
              "direction": "LONG"
            },
            {
              "symbol": "BTCUSDT",
              "qty": 1,
              "unrealized_pnl": -20.0,
              "side": "SHORT"
            }
          ]
        }
        """.data(using: .utf8)!

        let decoded = try SessionPositionsDecoder.decode(json)
        #expect(decoded.killSwitchActive == false)
        #expect(decoded.openOrders == 1)
        #expect(decoded.positions.count == 2)
        #expect(decoded.positions[0].symbol == "RELIANCE")
        #expect(decoded.positions[0].qty == 10)
        #expect(decoded.positions[0].unrealizedPnL == 125.5)
        #expect(decoded.positions[0].direction == "LONG")
        #expect(decoded.positions[1].symbol == "BTCUSDT")
        #expect(decoded.positions[1].unrealizedPnL == -20.0)
        #expect(decoded.positions[1].direction == "SHORT")
    }

    @Test func positionsDecodeOmitsUnknownMarkAndKeepsFractionalQty() throws {
        let json = """
        {
          "kill_switch_active": false,
          "open_orders": 0,
          "positions": [
            {
              "symbol": "BTCUSDT",
              "qty": 0.01,
              "side": "LONG"
            }
          ]
        }
        """.data(using: .utf8)!

        let decoded = try SessionPositionsDecoder.decode(json)
        #expect(decoded.positions.count == 1)
        #expect(decoded.positions[0].qty == 0.01)
        #expect(decoded.positions[0].unrealizedPnL == nil)
        #expect(decoded.positions[0].direction == "LONG")
        #expect(decoded.positions[0].firstFilledAt == nil)
    }

    @Test func positionsDecodeMapsFirstFilledAtAndDoesNotInventADate() throws {
        let json = """
        {
          "kill_switch_active": false,
          "open_orders": 0,
          "positions": [
            {
              "symbol": "RELIANCE",
              "qty": 50,
              "side": "LONG",
              "firstFilledAt": "2026-09-11T09:50:00.000Z"
            },
            {
              "symbol": "TCS",
              "qty": 10,
              "side": "LONG"
            }
          ]
        }
        """.data(using: .utf8)!

        let decoded = try SessionPositionsDecoder.decode(json)
        var utc = Calendar(identifier: .gregorian)
        utc.timeZone = TimeZone(secondsFromGMT: 0)!
        let expected = utc.date(from: DateComponents(year: 2026, month: 9, day: 11, hour: 9, minute: 50))
        #expect(decoded.positions[0].firstFilledAt == expected)
        #expect(decoded.positions[1].firstFilledAt == nil)
    }

    @Test func totalUnrealizedSumsPositions() {
        let session = SessionModel()
        session.positions = [
            DeskPosition(symbol: "A", qty: 1, unrealizedPnL: 100, direction: "LONG"),
            DeskPosition(symbol: "B", qty: 2, unrealizedPnL: -30, direction: "SHORT"),
        ]
        #expect(session.totalUnrealizedPnL == 70)
    }

    @Test func brokerSyncStateMapsClassAndDeskHonesty() {
        let session = SessionModel()
        let applied = session.applyDaemonEventPayload(
            type: "broker_sync_state",
            payload: [
                "class": "synced",
                "active_broker_slug": "binance_com",
            ]
        )
        #expect(applied)
        #expect(session.brokerSyncClass == "synced")
        #expect(session.brokerSessionActive == true)
        #expect(session.activeBrokerSlug == "binance_com")
        #expect(session.deskQuoteCurrency == "USD")
        #expect(session.deskCalcProfileId == "crypto_spot_usd")
        #expect(session.isBrokerSyncActiveForTodayMirror == true)
    }

    @Test func brokerSyncDisconnectedClearsActiveSlug() {
        let session = SessionModel()
        _ = session.applyDaemonEventPayload(
            type: "broker_sync_state",
            payload: [
                "class": "synced",
                "active_broker_slug": "kotak_neo",
            ]
        )
        #expect(session.activeBrokerSlug == "kotak_neo")
        #expect(session.deskQuoteCurrency == "INR")

        _ = session.applyDaemonEventPayload(
            type: "broker_sync_state",
            payload: [
                "class": "disconnected",
                "active_broker_slug": "kotak_neo",
            ]
        )
        #expect(session.brokerSessionActive == false)
        #expect(session.activeBrokerSlug == nil)
        #expect(session.isBrokerSyncActiveForTodayMirror == false)
    }

    @Test func killSwitchStateSetsCountdownAndAge() {
        let session = SessionModel()
        #expect(session.killSwitchStateAgeSecs == Int.max)

        let applied = session.applyDaemonEventPayload(
            type: "kill_switch_state",
            payload: [
                "active": true,
                "level": "L3",
                "countdown_secs": 90,
                "expires_at_ms": KillSwitchCountdown.nowMs() + 90_000,
                "requires_ack": true,
            ]
        )
        #expect(applied)
        #expect(session.killSwitchActive == true)
        let remaining = session.killSwitchCountdownSecs ?? -1
        #expect(remaining <= 90)
        #expect(remaining >= 89)
        #expect(session.killSwitchStateAgeSecs < 60)

        _ = session.applyDaemonEventPayload(
            type: "kill_switch_state",
            payload: [
                "active": false,
                "level": NSNull(),
                "countdown_secs": NSNull(),
                "requires_ack": false,
            ]
        )
        #expect(session.killSwitchActive == false)
        #expect(session.killSwitchCountdownSecs == nil)
    }

    @Test func syncingAndStaleCountAsTodayMirrorActive() {
        let session = SessionModel()
        for cls in ["syncing", "synced", "stale"] {
            _ = session.applyDaemonEventPayload(
                type: "broker_sync_state",
                payload: ["class": cls, "active_broker_slug": "binance_com"]
            )
            #expect(session.isBrokerSyncActiveForTodayMirror == true)
        }
        _ = session.applyDaemonEventPayload(
            type: "broker_sync_state",
            payload: ["class": "not_connected"]
        )
        #expect(session.isBrokerSyncActiveForTodayMirror == false)
    }

    @Test func configureStoresDaemonSettings() {
        let session = SessionModel()
        session.configure(secret: "sec", port: 9999, webBase: "127.0.0.1:3000")
        #expect(session.daemonSecret == "sec")
        #expect(session.daemonPort == 9999)
        #expect(session.webBaseURL == "https://127.0.0.1:3000")
    }

    @Test func unknownEventTypeIsIgnored() {
        let session = SessionModel()
        let applied = session.applyDaemonEventPayload(
            type: "toolbar_show",
            payload: ["reason": "x"]
        )
        #expect(applied == false)
    }
}
