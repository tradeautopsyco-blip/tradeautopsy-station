//! OpenAlgo Data + Accounts nouns as Station operations. Not literal routes.
//! Execution nouns are catalogued only to refuse them.

#![allow(dead_code)]

use super::host_policy::AuthMode;
use super::identity::{Family, Physics};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationKind {
    Read,
    ExecutionForbidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation {
    pub noun: &'static str,
    pub kind: OperationKind,
    pub family: Option<Family>,
    pub capability_id: Option<&'static str>,
    pub physics: Option<Physics>,
    pub auth_mode: Option<AuthMode>,
}

macro_rules! read_op {
    ($noun:expr, $family:expr, $id:expr, $physics:expr, $auth:expr) => {
        Operation {
            noun: $noun,
            kind: OperationKind::Read,
            family: Some($family),
            capability_id: Some($id),
            physics: Some($physics),
            auth_mode: Some($auth),
        }
    };
}

const fn forbidden(noun: &'static str) -> Operation {
    Operation {
        noun,
        kind: OperationKind::ExecutionForbidden,
        family: None,
        capability_id: None,
        physics: None,
        auth_mode: None,
    }
}

pub const OPENALGO_OPERATIONS: &[Operation] = &[
    read_op!(
        "quotes",
        Family::Market,
        "quote",
        Physics::LatestState,
        AuthMode::Public
    ),
    read_op!(
        "multiquotes",
        Family::Market,
        "quote",
        Physics::LatestState,
        AuthMode::Public
    ),
    read_op!(
        "depth",
        Family::Market,
        "order_book",
        Physics::BoundedSnapshot,
        AuthMode::Public
    ),
    read_op!(
        "depth_stream",
        Family::Market,
        "order_book",
        Physics::OrderedState,
        AuthMode::Public
    ),
    read_op!(
        "history",
        Family::Market,
        "ohlcv",
        Physics::HistoricalSeries,
        AuthMode::Public
    ),
    read_op!(
        "ticker",
        Family::Market,
        "ohlcv",
        Physics::HistoricalSeries,
        AuthMode::Public
    ),
    read_op!(
        "intervals",
        Family::Reference,
        "intervals",
        Physics::BoundedSnapshot,
        AuthMode::Public
    ),
    read_op!(
        "search",
        Family::Reference,
        "instrument_search",
        Physics::BoundedSnapshot,
        AuthMode::Public
    ),
    read_op!(
        "symbol",
        Family::Reference,
        "instrument_master",
        Physics::BoundedSnapshot,
        AuthMode::Public
    ),
    read_op!(
        "instruments",
        Family::Reference,
        "instrument_master",
        Physics::VersionedSnapshot,
        AuthMode::Public
    ),
    read_op!(
        "expiry",
        Family::Reference,
        "expiry",
        Physics::BoundedSnapshot,
        AuthMode::Public
    ),
    read_op!(
        "optionchain",
        Family::Market,
        "option_chain",
        Physics::BoundedSnapshot,
        AuthMode::Public
    ),
    read_op!(
        "optiongreeks",
        Family::Derived,
        "greeks",
        Physics::BoundedSnapshot,
        AuthMode::Public
    ),
    read_op!(
        "multioptiongreeks",
        Family::Derived,
        "greeks",
        Physics::BoundedSnapshot,
        AuthMode::Public
    ),
    read_op!(
        "optionsymbol",
        Family::Reference,
        "option_symbol",
        Physics::BoundedSnapshot,
        AuthMode::Public
    ),
    read_op!(
        "syntheticfuture",
        Family::Derived,
        "synthetic_future",
        Physics::BoundedSnapshot,
        AuthMode::Public
    ),
    read_op!(
        "market/timings",
        Family::Reference,
        "market_timings",
        Physics::BoundedSnapshot,
        AuthMode::Public
    ),
    read_op!(
        "market/holidays",
        Family::Reference,
        "trading_calendar",
        Physics::VersionedSnapshot,
        AuthMode::Public
    ),
    read_op!(
        "ping",
        Family::Account,
        "session",
        Physics::BoundedSnapshot,
        AuthMode::PrivateRead
    ),
    read_op!(
        "funds",
        Family::Account,
        "funds",
        Physics::BoundedSnapshot,
        AuthMode::PrivateRead
    ),
    read_op!(
        "orderbook",
        Family::Account,
        "orders",
        Physics::BoundedSnapshot,
        AuthMode::PrivateRead
    ),
    read_op!(
        "tradebook",
        Family::Account,
        "fills",
        Physics::BoundedSnapshot,
        AuthMode::PrivateRead
    ),
    read_op!(
        "positionbook",
        Family::Account,
        "positions",
        Physics::BoundedSnapshot,
        AuthMode::PrivateRead
    ),
    read_op!(
        "holdings",
        Family::Account,
        "holdings",
        Physics::BoundedSnapshot,
        AuthMode::PrivateRead
    ),
    forbidden("placeorder"),
    forbidden("placesmartorder"),
    forbidden("modifyorder"),
    forbidden("cancelorder"),
    forbidden("cancelallorder"),
    forbidden("closeposition"),
    forbidden("basketorder"),
    forbidden("splitorder"),
];

pub fn catalog_row(noun: &str) -> Option<&'static Operation> {
    OPENALGO_OPERATIONS.iter().find(|row| row.noun == noun)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::matrix::{known_id_physics_ok, pair_allowed};

    #[test]
    fn every_read_identity_is_on_the_matrix() {
        for row in OPENALGO_OPERATIONS {
            if row.kind != OperationKind::Read {
                continue;
            }
            let family = row.family.expect("read family");
            let physics = row.physics.expect("read physics");
            let id = row.capability_id.expect("read id");
            assert!(pair_allowed(family, physics), "{}", row.noun);
            assert!(known_id_physics_ok(family, id, physics), "{}", row.noun);
        }
    }

    #[test]
    fn optionchain_is_not_order_book() {
        let chain = catalog_row("optionchain").unwrap();
        assert_eq!(chain.capability_id, Some("option_chain"));
        assert_eq!(chain.physics, Some(Physics::BoundedSnapshot));
        let orders = catalog_row("orderbook").unwrap();
        assert_eq!(orders.family, Some(Family::Account));
        assert_eq!(orders.capability_id, Some("orders"));
    }

    #[test]
    fn placeorder_is_forbidden() {
        assert_eq!(
            catalog_row("placeorder").unwrap().kind,
            OperationKind::ExecutionForbidden
        );
    }
}
