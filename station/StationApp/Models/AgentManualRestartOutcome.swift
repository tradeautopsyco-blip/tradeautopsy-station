import Foundation

/// Result of the trader-facing one-button agent restart (Wave 6).
public enum AgentManualRestartOutcome: Equatable {
    case restarted
    case blockedKillLatched
}
