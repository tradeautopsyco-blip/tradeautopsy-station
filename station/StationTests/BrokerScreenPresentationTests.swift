import Foundation
import Testing
@testable import Station

@MainActor
struct BrokerScreenPresentationTests {
    private let binanceIdentity = BrokerConnectionIdentity.binanceComProd

    private func binanceConnection(
        lastValidatedAt: Date? = Date(timeIntervalSince1970: 1_700_000_000),
        lastSyncSummary: String? = nil
    ) -> BrokerConfiguredConnection {
        BrokerConfiguredConnection(
            identity: binanceIdentity,
            displayName: "Binance.com",
            lastValidatedAt: lastValidatedAt,
            lastSyncSummary: lastSyncSummary
        )
    }

    @Test func notConfiguredWhenNoCredentialsSaved() {
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [],
            agentAvailable: true,
            runtimeStatusByConnectionID: [:]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        let binance = cards.first { $0.id == "binance_com" }

        #expect(binance != nil)
        #expect(binance?.status == .notConfigured)
        #expect(binance?.isConnectable == true)
        #expect(binance?.identity == nil)
    }

    @Test func agentOfflineShowsUnavailableWithMetadata() {
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [
                binanceConnection(lastSyncSummary: "Last sync: 2 fills imported")
            ],
            agentAvailable: false,
            runtimeStatusByConnectionID: [:]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        let binance = cards.first { $0.id == "binance_com" }

        #expect(binance?.status == .unavailableAgentOffline)
        #expect(binance?.statusLabel == "Unavailable: Agent Offline")
        #expect(binance?.isStartEnabled == false)
        #expect(binance?.isStopEnabled == false)
        #expect(binance?.lastValidatedAtText != nil)
        #expect(binance?.lastSyncSummary == "Last sync: 2 fills imported")
        #expect(binance?.identity == binanceIdentity)
    }

    @Test func catalogShowsEnabledPairAndPlannedIndiaCash() {
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [],
            agentAvailable: true,
            runtimeStatusByConnectionID: [:]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        #expect(cards.count == BrokerCatalog.v1.count)
        #expect(cards.first { $0.id == "kotak_neo" }?.isConnectable == true)
        #expect(cards.first { $0.id == "binance_com" }?.isConnectable == true)
        #expect(cards.first { $0.id == "binance_com" }?.quoteCurrency == "USD")
        #expect(cards.first { $0.id == "kotak_neo" }?.quoteCurrency == "INR")
        #expect(cards.first { $0.id == "upstox" }?.isConnectable == false)
        #expect(cards.first { $0.id == "upstox" }?.plannedLabel == "Planned")
    }

    @Test func plannedZerodhaDogfoodShowsConnectWhenAgentOnline() {
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [],
            agentAvailable: true,
            runtimeStatusByConnectionID: [:]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        let kite = cards.first { $0.id == "zerodha_kite" }

        #expect(kite?.plannedLabel == "Planned")
        #expect(kite?.isConnectable == true)
        #expect(kite?.isStartEnabled == false)
    }

    @Test func connectionMetadataCarriesIdentityFields() {
        let connection = binanceConnection()
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [connection],
            agentAvailable: true,
            runtimeStatusByConnectionID: [:]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        let binance = cards.first { $0.id == "binance_com" }

        #expect(binance?.identity?.brokerConnectionID == binanceIdentity.brokerConnectionID)
        #expect(binance?.identity?.brokerSlug == "binance_com")
        #expect(binance?.identity?.assetClass == "crypto_spot")
        #expect(binance?.identity?.environment == "prod")
    }

    @Test func neverShowsSyncingWithoutAgentReportedStatus() {
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [binanceConnection()],
            agentAvailable: true,
            runtimeStatusByConnectionID: [:]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        let binance = cards.first { $0.id == "binance_com" }

        #expect(binance?.status == .readyToStart)
        #expect(binance?.status.impliesAgentConnectedSync == false)
    }

    @Test func showsAgentReportedConnectedWithLastSynced() {
        let connectionID = binanceIdentity.brokerConnectionID.uuidString
        let now = Date(timeIntervalSince1970: 1_700_000_012)
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [
                BrokerConfiguredConnection(
                    identity: binanceIdentity,
                    displayName: "Binance.com",
                    lastValidatedAt: Date(timeIntervalSince1970: 1_700_000_000),
                    lastSyncedAtMs: 1_700_000_000_000
                )
            ],
            agentAvailable: true,
            runtimeStatusByConnectionID: [connectionID: .connected]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1, now: now)
        let binance = cards.first { $0.id == "binance_com" }
        let kotak = cards.first { $0.id == "kotak_neo" }

        #expect(binance?.status == .connected)
        #expect(binance?.statusLabel == "Connected")
        #expect(binance?.status.impliesAgentConnectedSync == true)
        #expect(binance?.isStopEnabled == true)
        #expect(binance?.lastSyncedAtText == "12s ago")
        #expect(kotak?.status == .notConfigured)
        #expect(kotak?.lastSyncedAtText == nil)
    }

    @Test func idleConfiguredBrokerHasNoLastSyncedOrConnected() {
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [binanceConnection()],
            agentAvailable: true,
            runtimeStatusByConnectionID: [:]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        let binance = cards.first { $0.id == "binance_com" }

        #expect(binance?.status == .readyToStart)
        #expect(binance?.lastSyncedAtText == nil)
        #expect(binance?.isStopEnabled == false)
    }

    @Test func degradedKeepsLastSyncedWhenMsPresent() {
        let connectionID = binanceIdentity.brokerConnectionID.uuidString
        let now = Date(timeIntervalSince1970: 1_700_000_180)
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [
                BrokerConfiguredConnection(
                    identity: binanceIdentity,
                    displayName: "Binance.com",
                    lastSyncedAtMs: 1_700_000_000_000
                )
            ],
            agentAvailable: true,
            runtimeStatusByConnectionID: [connectionID: .degraded]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1, now: now)
        let binance = cards.first { $0.id == "binance_com" }

        #expect(binance?.status == .degraded)
        #expect(binance?.lastSyncedAtText == "3m ago")
        #expect(binance?.isStopEnabled == true)
    }

    @Test func syncStateMappingPrefersSyncedOverRuntimeSyncing() {
        #expect(
            BrokerAgentSyncStateMapping.cardStatus(syncState: "synced", runtimeStatus: "syncing")
                == .connected
        )
        #expect(
            BrokerAgentSyncStateMapping.cardStatus(syncState: "syncing", runtimeStatus: "syncing")
                == .syncing
        )
        #expect(
            BrokerAgentSyncStateMapping.cardStatus(syncState: "stale", runtimeStatus: "syncing")
                == .degraded
        )
        #expect(
            BrokerAgentSyncStateMapping.cardStatus(syncState: "disconnected", runtimeStatus: "paused")
                == .paused
        )
        #expect(
            BrokerAgentSyncStateMapping.cardStatus(syncState: nil, runtimeStatus: "ready_to_start")
                == .readyToStart
        )
    }

    @Test func formatRelativeSyncedAtBuckets() {
        let now = Date(timeIntervalSince1970: 1_000)
        #expect(BrokerScreenPresentation.formatRelativeSyncedAt(999_000, now: now) == "just now")
        #expect(BrokerScreenPresentation.formatRelativeSyncedAt(970_000, now: now) == "30s ago")
        #expect(BrokerScreenPresentation.formatRelativeSyncedAt(820_000, now: now) == "3m ago")
        #expect(BrokerScreenPresentation.formatRelativeSyncedAt(nil, now: now) == nil)
    }

    @Test func showsAgentReportedSyncingStatus() {
        let connectionID = binanceIdentity.brokerConnectionID.uuidString
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [binanceConnection()],
            agentAvailable: true,
            runtimeStatusByConnectionID: [connectionID: .syncing]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        let binance = cards.first { $0.id == "binance_com" }

        #expect(binance?.status == .syncing)
        #expect(binance?.isStopEnabled == true)
        #expect(binance?.isStartEnabled == true)
        #expect(binance?.isEditEnabled == true)
    }

    @Test func degradedKeepsStopEnabledBecauseSyncIsActive() {
        let connectionID = binanceIdentity.brokerConnectionID.uuidString
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [binanceConnection()],
            agentAvailable: true,
            runtimeStatusByConnectionID: [connectionID: .degraded]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        let binance = cards.first { $0.id == "binance_com" }

        #expect(binance?.status == .degraded)
        #expect(binance?.isStartEnabled == true)
        #expect(binance?.isStopEnabled == true)
    }

    @Test func brokerCardPresentationNeverContainsSecretMaterial() {
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [
                BrokerConfiguredConnection(
                    identity: .binanceComProd,
                    displayName: "Binance.com",
                    permissionWarning: .tradeEnabled,
                    lastValidatedAt: Date(timeIntervalSince1970: 1_700_000_000),
                    lastSyncSummary: "Last sync: 2 fills imported"
                )
            ],
            agentAvailable: true,
            runtimeStatusByConnectionID: [:]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        let credentials = BrokerCredentials(apiKey: "leak-key", apiSecret: "leak-secret")

        for card in cards {
            let values = [
                card.id,
                card.displayName,
                card.assetClass,
                card.statusLabel,
                card.lastValidatedAtText,
                card.lastSyncSummary,
                card.lastSyncedAtText,
                card.plannedLabel,
            ].compactMap { $0 }
            for value in values {
                #expect(!BrokerSecretGuard.containsSecretMaterial(value, credentials: credentials))
            }
        }
    }

    @Test func tradeEnabledWarningSurfacesOnConfiguredCard() {
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [
                BrokerConfiguredConnection(
                    identity: .binanceComProd,
                    displayName: "Binance.com",
                    permissionWarning: .tradeEnabled,
                    lastValidatedAt: Date(timeIntervalSince1970: 1_700_000_000)
                )
            ],
            agentAvailable: true,
            runtimeStatusByConnectionID: [:]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        let binance = cards.first { $0.id == "binance_com" }

        #expect(binance?.permissionWarning == .tradeEnabled)
    }
}
