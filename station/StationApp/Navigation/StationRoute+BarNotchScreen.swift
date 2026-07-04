import Foundation
import Notch

extension StationRoute {
    /// Transitional mapping from legacy `BarNotchScreen` to `StationRoute`.
    /// Used while Notch is being migrated to the shared shell.
    init?(barNotchScreen: BarNotchScreen) {
        switch barNotchScreen {
        case .morning: self = .today
        case .pretrade: self = .preTrade
        case .live: self = .liveTrade
        case .posttrade: self = .postTrade
        case .escrow: self = .escrowMatch
        case .patterns: self = .patterns
        case .fidelity: self = .fidelityScore
        case .triage: self = .journal
        case .settings: self = .settings
        }
    }
}

extension BarNotchScreen {
    init?(stationRoute: StationRoute) {
        switch stationRoute {
        case .today: self = .morning
        case .preTrade: self = .pretrade
        case .liveTrade: self = .live
        case .postTrade: self = .posttrade
        case .escrowMatch: self = .escrow
        case .patterns: self = .patterns
        case .fidelityScore: self = .fidelity
        case .journal: self = .triage
        case .settings: self = .settings
        case .brokers: return nil
        }
    }
}
