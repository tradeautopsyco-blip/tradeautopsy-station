import Foundation

/// PLAN protective SL chrome. Status + Cancel stay; Set SL send is cut (T5 K3 / PRD Q2 B).
enum BarProtectiveSlPlanChrome {
    struct Presentation: Equatable {
        /// “not placed” / “live” copy. Not a CTA.
        let statusText: String?
        let rejectedText: String?
        let showsSetSlButton: Bool
        let showsCancelSlButton: Bool
    }

    static func presentation(
        slStatus: String?,
        slPrice: Double?,
        slFailureReason: String?,
        formattedPrice: String? = nil
    ) -> Presentation {
        let st = slStatus?.trimmingCharacters(in: .whitespacesAndNewlines).lowercased() ?? ""
        let rejected: String? = {
            guard let reason = slFailureReason?.trimmingCharacters(in: .whitespacesAndNewlines), !reason.isEmpty else {
                return nil
            }
            return "SL rejected: \(reason)"
        }()

        if st == "missing" {
            let status: String
            if let formattedPrice, !formattedPrice.isEmpty {
                status = "Plan stop at ₹\(formattedPrice) — on the plan only, not a broker order."
            } else if slPrice != nil {
                status = "Plan stop not recorded at broker"
            } else {
                status = "Plan stop not recorded at broker"
            }
            return Presentation(
                statusText: status,
                rejectedText: rejected,
                showsSetSlButton: false,
                showsCancelSlButton: false
            )
        }

        if st == "placed" {
            return Presentation(
                statusText: "Protective SL is live at the broker.",
                rejectedText: rejected,
                showsSetSlButton: false,
                showsCancelSlButton: true
            )
        }

        return Presentation(
            statusText: nil,
            rejectedText: rejected,
            showsSetSlButton: false,
            showsCancelSlButton: false
        )
    }

    /// PLAN must not emit `place_sl`. Always `nil` (T5 K3). Hosted `/api/bar/v1/protective` stays.
    static func placeSlJSONFromPlan() -> Data? {
        nil
    }
}
