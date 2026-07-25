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

    @Test func futureBrokersArePlannedAndNotConnectable() {
        let snapshot = BrokerControlSnapshot(
            configuredConnections: [],
            agentAvailable: true,
            runtimeStatusByConnectionID: [:]
        )

        let cards = BrokerScreenPresentation.build(snapshot: snapshot, catalog: BrokerCatalog.v1)
        let planned = cards.filter { $0.plannedLabel == "Planned" }

        #expect(planned.count == 2)
        for card in planned {
            #expect(card.isConnectable == false)
            #expect(card.isStartEnabled == false)
            #expect(card.isStopEnabled == false)
        }

        let parked = cards.filter { $0.plannedLabel == "Parked" }
        #expect(parked.map(\.id).contains("binance_us"))
        #expect(cards.first { $0.id == "kotak_neo" }?.isConnectable == true)
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
        #expect(binance?.isStartEnabled == false)
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
