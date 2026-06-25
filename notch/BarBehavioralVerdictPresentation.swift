import Foundation

/// Server-owned behavioral verdict bands from `notch.behavioral_verdict` (#180).
enum BarBehavioralVerdictBand: String, Equatable {
    case flow = "FLOW"
    case calm = "CALM"
    case caution = "CAUTION"
    case softBlock = "SOFT_BLOCK"
    case danger = "DANGER"

    init?(wire: String?) {
        guard let raw = wire?.trimmingCharacters(in: .whitespacesAndNewlines).uppercased(), !raw.isEmpty else {
            return nil
        }
        switch raw {
        case "FLOW":
            self = .flow
        case "CALM":
            self = .calm
        case "CAUTION":
            self = .caution
        case "SOFT_BLOCK", "SOFT BLOCK", "SOFT-BLOCK":
            self = .softBlock
        case "DANGER", "HIGH", "HIGH_RISK":
            self = .danger
        default:
            return nil
        }
    }
}

enum BarBehavioralVerdictPresentation {
    /// Collapsed / metric strip label aligned to validated ladder.
    static func displayLabel(score: Double, verdict: BarBehavioralVerdictBand?) -> String {
        if let verdict {
            switch verdict {
            case .flow:
                return "FLOW"
            case .calm:
                return "CALM"
            case .caution:
                return "CAUTION"
            case .softBlock:
                return "SOFT BLOCK"
            case .danger:
                return "DANGER"
            }
        }
        if score >= 0.45 { return "DANGER" }
        if score >= 0.25 { return "CAUTION" }
        if score < 0.15 { return "FLOW" }
        return "CALM"
    }

    /// Maps hosted verdict to legacy `behavioralState` string consumed by existing chrome.
    static func behavioralStateLabel(score: Double, verdict: BarBehavioralVerdictBand?) -> String {
        displayLabel(score: score, verdict: verdict)
    }
}

struct BarBehaviorSignalRow: Decodable, Equatable {
    let signal: String
    let value: Double
}

extension BarLiveStateResponse {
    /// Applies hosted live-state behavioral projection to Notch view model fields (#180).
    @MainActor
    func applyBehavioralToViewModel(_ viewModel: NotchViewModel) {
        guard let score = behavioralScore, score.isFinite else { return }
        viewModel.compositeScore = max(0, min(1, score))
        if let verdict = BarBehavioralVerdictBand(wire: behavioralVerdict) {
            viewModel.behavioralState = BarBehavioralVerdictPresentation.behavioralStateLabel(
                score: viewModel.compositeScore,
                verdict: verdict
            )
        } else {
            viewModel.behavioralState = BarBehavioralVerdictPresentation.behavioralStateLabel(
                score: viewModel.compositeScore,
                verdict: nil
            )
        }
    }
}
