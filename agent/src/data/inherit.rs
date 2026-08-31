//! Derived-envelope honesty: dark iff any named input is dark.

use super::honesty::{HonestyStatus, InputHonesty};

/// Derived envelope is dark iff ANY named input is dark.
/// Then honesty = inherited_dark and data = None.
pub fn inherit(inputs: &[InputHonesty]) -> Option<HonestyStatus> {
    if inputs.iter().any(|h| matches!(h, InputHonesty::Dark(_))) {
        Some(HonestyStatus::InheritedDark)
    } else {
        None
    }
}

/// Capital (funds / margin) may light independently of the chain quote.
pub fn capital_may_light(funds: InputHonesty, _chain: InputHonesty) -> bool {
    matches!(funds, InputHonesty::Lit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::greeks::extract_greeks;

    #[test]
    fn chain_unavailable_greeks_are_inherited_dark_with_data_none() {
        let envelope = extract_greeks(
            Some(crate::data::KOTAK_NSE_NFO_BOOK_ID),
            InputHonesty::Dark(HonestyStatus::Unavailable),
        );
        assert_eq!(envelope.status, HonestyStatus::InheritedDark);
        assert!(envelope.data.is_none());
        assert!(envelope.status.requires_data_none());
    }

    #[test]
    fn all_lit_inputs_leave_caller_to_compute() {
        assert_eq!(inherit(&[InputHonesty::Lit, InputHonesty::Lit]), None);
    }

    #[test]
    fn any_dark_input_is_inherited_dark() {
        assert_eq!(
            inherit(&[InputHonesty::Lit, InputHonesty::Dark(HonestyStatus::Empty)]),
            Some(HonestyStatus::InheritedDark)
        );
        assert_eq!(
            inherit(&[InputHonesty::Dark(HonestyStatus::Unusable)]),
            Some(HonestyStatus::InheritedDark)
        );
    }

    #[test]
    fn funds_lit_chain_dark_capital_may_light() {
        assert!(capital_may_light(
            InputHonesty::Lit,
            InputHonesty::Dark(HonestyStatus::Unavailable)
        ));
    }

    #[test]
    fn funds_dark_chain_lit_capital_must_not_light() {
        assert!(!capital_may_light(
            InputHonesty::Dark(HonestyStatus::Unavailable),
            InputHonesty::Lit
        ));
    }
}
