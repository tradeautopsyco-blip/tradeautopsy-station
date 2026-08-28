//! Shared honesty dialect. Lit is not a fifth variant — see [`InputHonesty`].
//! Freshness (`QuoteStatus` stale/fresh) is a different axis and must not collapse here.

use serde::Serialize;

/// Closed dark/hole states. Success is [`InputHonesty::Lit`], not a fifth variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum HonestyStatus {
    /// Capability live, result set legitimately zero (no contracts).
    Empty,
    /// No snapshot / hole / not implemented.
    Unavailable,
    /// Snapshot exists but fails physics (incomplete).
    Unusable,
    /// This extract did not fail; a named input is dark. `data` MUST be None.
    InheritedDark,
}

impl HonestyStatus {
    pub const ALL: [HonestyStatus; 4] = [
        Self::Empty,
        Self::Unavailable,
        Self::Unusable,
        Self::InheritedDark,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Unavailable => "unavailable",
            Self::Unusable => "unusable",
            Self::InheritedDark => "inherited_dark",
        }
    }

    /// Inherited-dark extracts must not carry a payload.
    pub fn requires_data_none(self) -> bool {
        matches!(self, Self::InheritedDark)
    }
}

/// Wrapper so lit/success is not a fifth [`HonestyStatus`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputHonesty {
    Lit,
    Dark(HonestyStatus),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_snake_case_states_and_no_fifth() {
        let wire: Vec<String> = HonestyStatus::ALL
            .iter()
            .map(|s| {
                serde_json::to_value(s)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect();
        assert_eq!(wire, ["empty", "unavailable", "unusable", "inherited_dark"]);
        assert_eq!(HonestyStatus::ALL.len(), 4);
        for forbidden in ["lit", "success", "fresh", "stale", "unknown"] {
            assert!(
                !wire.iter().any(|v| v == forbidden),
                "honesty must not include {forbidden}"
            );
        }
    }

    #[test]
    fn empty_is_live_zero_not_a_hole() {
        // empty: capability live, result set legitimately zero (no contracts)
        assert_eq!(HonestyStatus::Empty.as_str(), "empty");
        assert_ne!(HonestyStatus::Empty, HonestyStatus::Unavailable);
        assert!(!HonestyStatus::Empty.requires_data_none());
    }

    #[test]
    fn unavailable_is_missing_snapshot_hole() {
        // unavailable: no snapshot / hole / not implemented
        assert_eq!(HonestyStatus::Unavailable.as_str(), "unavailable");
        assert_ne!(HonestyStatus::Unavailable, HonestyStatus::Unusable);
        assert_ne!(HonestyStatus::Unavailable, HonestyStatus::Empty);
    }

    #[test]
    fn unusable_is_physics_fail_not_a_hole() {
        // unusable: snapshot exists but fails physics (incomplete)
        assert_eq!(HonestyStatus::Unusable.as_str(), "unusable");
        assert_ne!(HonestyStatus::Unusable, HonestyStatus::Unavailable);
        assert_ne!(HonestyStatus::Unusable, HonestyStatus::InheritedDark);
    }

    #[test]
    fn inherited_dark_is_named_input_dark_and_requires_data_none() {
        // inherited_dark: this extract did not fail; a named input is dark. data MUST be None.
        assert_eq!(HonestyStatus::InheritedDark.as_str(), "inherited_dark");
        assert!(HonestyStatus::InheritedDark.requires_data_none());
        assert_ne!(HonestyStatus::InheritedDark, HonestyStatus::Unusable);
        assert_ne!(HonestyStatus::InheritedDark, HonestyStatus::Unavailable);
    }

    #[test]
    fn lit_is_wrapper_not_honesty_status() {
        assert!(matches!(InputHonesty::Lit, InputHonesty::Lit));
        assert!(matches!(
            InputHonesty::Dark(HonestyStatus::Unavailable),
            InputHonesty::Dark(_)
        ));
        assert_ne!(
            InputHonesty::Lit,
            InputHonesty::Dark(HonestyStatus::Unavailable)
        );
    }
}
