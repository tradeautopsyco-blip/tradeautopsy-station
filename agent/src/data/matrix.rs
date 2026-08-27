//! Closed family × physics allow-list (#357) and known-ID physics (#363).
//!
//! Copied from `scripts/research/representative-e2e-capabilities.ts`. Do not invent pairs.

use super::identity::{Family, Physics};

pub fn allowed_physics(family: Family) -> &'static [Physics] {
    match family {
        Family::Reference => &[Physics::VersionedSnapshot, Physics::BoundedSnapshot],
        Family::Market => &[
            Physics::LatestState,
            Physics::OrderedState,
            Physics::CompleteEventSequence,
            Physics::LossyEventObservation,
            Physics::HistoricalSeries,
            Physics::BoundedSnapshot,
        ],
        Family::Economic => &[Physics::HistoricalSeries, Physics::BoundedSnapshot],
        Family::Fundamentals => &[Physics::BoundedSnapshot, Physics::HistoricalSeries],
        Family::News => &[
            Physics::CompleteEventSequence,
            Physics::LossyEventObservation,
            Physics::BoundedSnapshot,
        ],
        Family::Derived => &[
            Physics::BoundedSnapshot,
            Physics::HistoricalSeries,
            Physics::LatestState,
        ],
        Family::Account => &[Physics::BoundedSnapshot, Physics::CompleteEventSequence],
    }
}

pub fn pair_allowed(family: Family, physics: Physics) -> bool {
    allowed_physics(family).contains(&physics)
}

/// Market IDs that must not bind on family `account` (and the reverse).
pub fn is_market_capability_id(id: &str) -> bool {
    matches!(
        id,
        "quote"
            | "ohlcv"
            | "order_book"
            | "public_trade"
            | "force_order"
            | "option_chain"
            | "open_interest"
    )
}

pub fn is_account_capability_id(id: &str) -> bool {
    matches!(
        id,
        "funds"
            | "session"
            | "positions"
            | "holdings"
            | "orders"
            | "fills"
            | "gtt_orders"
            | "margin_estimate"
            | "order_events"
    )
}

pub fn market_account_id_collision(family: Family, capability_id: &str) -> bool {
    match family {
        Family::Market => is_account_capability_id(capability_id),
        Family::Account => is_market_capability_id(capability_id),
        _ => false,
    }
}

/// Known product-vocabulary IDs. `None` means the ID is unknown (snake check only).
pub fn known_physics(family: Family, capability_id: &str) -> Option<&'static [Physics]> {
    match (family, capability_id) {
        (Family::Market, "quote") => Some(&[Physics::LatestState]),
        (Family::Market, "ohlcv") => Some(&[Physics::HistoricalSeries]),
        (Family::Market, "order_book") => Some(&[Physics::OrderedState, Physics::BoundedSnapshot]),
        (Family::Market, "public_trade") => Some(&[Physics::CompleteEventSequence]),
        (Family::Market, "force_order") => Some(&[Physics::LossyEventObservation]),
        (Family::Market, "option_chain") => Some(&[Physics::BoundedSnapshot]),
        (Family::Market, "open_interest") => Some(&[Physics::LatestState]),
        (Family::Account, "funds")
        | (Family::Account, "session")
        | (Family::Account, "positions")
        | (Family::Account, "holdings")
        | (Family::Account, "orders")
        | (Family::Account, "gtt_orders")
        | (Family::Account, "margin_estimate") => Some(&[Physics::BoundedSnapshot]),
        (Family::Account, "fills") => {
            Some(&[Physics::BoundedSnapshot, Physics::CompleteEventSequence])
        }
        (Family::Account, "order_events") => Some(&[Physics::CompleteEventSequence]),
        (Family::Reference, "trading_calendar") => Some(&[Physics::VersionedSnapshot]),
        (Family::Reference, "instrument_master") => {
            Some(&[Physics::VersionedSnapshot, Physics::BoundedSnapshot])
        }
        (Family::Reference, "intervals") => Some(&[Physics::BoundedSnapshot]),
        (Family::Reference, "instrument_search") => Some(&[Physics::BoundedSnapshot]),
        (Family::Reference, "expiry") => Some(&[Physics::BoundedSnapshot]),
        (Family::Reference, "option_symbol") => Some(&[Physics::BoundedSnapshot]),
        (Family::Reference, "market_timings") => Some(&[Physics::BoundedSnapshot]),
        (Family::Derived, "ohlcv") => Some(&[Physics::HistoricalSeries]),
        (Family::Derived, "greeks") => Some(&[Physics::BoundedSnapshot]),
        (Family::Derived, "synthetic_future") => Some(&[Physics::BoundedSnapshot]),
        _ => None,
    }
}

pub fn known_id_physics_ok(family: Family, capability_id: &str, physics: Physics) -> bool {
    match known_physics(family, capability_id) {
        Some(allowed) => allowed.contains(&physics),
        None => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_matches_representative_e2e_allow_list() {
        assert_eq!(
            allowed_physics(Family::Reference),
            &[Physics::VersionedSnapshot, Physics::BoundedSnapshot]
        );
        assert_eq!(
            allowed_physics(Family::Market),
            &[
                Physics::LatestState,
                Physics::OrderedState,
                Physics::CompleteEventSequence,
                Physics::LossyEventObservation,
                Physics::HistoricalSeries,
                Physics::BoundedSnapshot,
            ]
        );
        assert_eq!(
            allowed_physics(Family::Economic),
            &[Physics::HistoricalSeries, Physics::BoundedSnapshot]
        );
        assert_eq!(
            allowed_physics(Family::Fundamentals),
            &[Physics::BoundedSnapshot, Physics::HistoricalSeries]
        );
        assert_eq!(
            allowed_physics(Family::News),
            &[
                Physics::CompleteEventSequence,
                Physics::LossyEventObservation,
                Physics::BoundedSnapshot,
            ]
        );
        assert_eq!(
            allowed_physics(Family::Derived),
            &[
                Physics::BoundedSnapshot,
                Physics::HistoricalSeries,
                Physics::LatestState,
            ]
        );
        assert_eq!(
            allowed_physics(Family::Account),
            &[Physics::BoundedSnapshot, Physics::CompleteEventSequence]
        );
    }

    #[test]
    fn account_latest_state_is_not_on_the_matrix() {
        assert!(!pair_allowed(Family::Account, Physics::LatestState));
    }

    #[test]
    fn quote_ordered_state_fails_known_id_gate() {
        assert!(pair_allowed(Family::Market, Physics::OrderedState));
        assert!(!known_id_physics_ok(
            Family::Market,
            "quote",
            Physics::OrderedState
        ));
        assert!(known_id_physics_ok(
            Family::Market,
            "quote",
            Physics::LatestState
        ));
    }

    #[test]
    fn fills_accepts_snapshot_or_complete_replay() {
        assert!(known_id_physics_ok(
            Family::Account,
            "fills",
            Physics::BoundedSnapshot
        ));
        assert!(known_id_physics_ok(
            Family::Account,
            "fills",
            Physics::CompleteEventSequence
        ));
    }

    #[test]
    fn market_must_not_bind_funds() {
        assert!(market_account_id_collision(Family::Market, "funds"));
        assert!(market_account_id_collision(Family::Account, "quote"));
        assert!(!market_account_id_collision(Family::Market, "quote"));
        assert!(!market_account_id_collision(Family::Derived, "ohlcv"));
    }

    #[test]
    fn option_chain_is_bounded_snapshot_not_order_book() {
        assert!(pair_allowed(Family::Market, Physics::BoundedSnapshot));
        assert!(known_id_physics_ok(
            Family::Market,
            "option_chain",
            Physics::BoundedSnapshot
        ));
        assert!(!known_id_physics_ok(
            Family::Market,
            "option_chain",
            Physics::OrderedState
        ));
        assert!(known_id_physics_ok(
            Family::Market,
            "order_book",
            Physics::OrderedState
        ));
        assert!(known_id_physics_ok(
            Family::Market,
            "order_book",
            Physics::BoundedSnapshot
        ));
        assert!(!known_id_physics_ok(
            Family::Market,
            "order_book",
            Physics::LatestState
        ));
    }

    #[test]
    fn open_interest_is_latest_state() {
        assert!(known_id_physics_ok(
            Family::Market,
            "open_interest",
            Physics::LatestState
        ));
        assert!(!known_id_physics_ok(
            Family::Market,
            "open_interest",
            Physics::OrderedState
        ));
    }
}
