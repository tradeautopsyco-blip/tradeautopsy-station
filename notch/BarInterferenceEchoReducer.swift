import Foundation

/// Echo copy for live-trade interference chips (#113 / Mockup 5).
enum BarInterferenceEchoReducer {
    /// Spec / issue naming — identical to `echoText`.
    static func next(choice: String, planIsRed: Bool) -> String? {
        echoText(choice: choice, planIsRed: planIsRed)
    }

    static func echoText(choice: String, planIsRed: Bool) -> String? {
        let c = choice.trimmingCharacters(in: .whitespacesAndNewlines).lowercased()
        switch c {
        case "no":
            return "Good. Nothing to act on. Let the plan run."
        case "maybe":
            return "Has any rule triggered? If not — do nothing. Write the feeling below."
        case "yes":
            return planIsRed
                ? "Your thesis is already dead. This exit is the plan. Execute it."
                : "This urge is fear or greed. Write it below and wait."
        default:
            return nil
        }
    }

    static func planIsRed(planState: String?, isRedTerminal: Bool) -> Bool {
        if isRedTerminal { return true }
        let u = planState?.trimmingCharacters(in: .whitespacesAndNewlines).uppercased() ?? ""
        return u == "RED"
    }
}
