//! VenueEgress — one chokepoint for every venue call this process makes.
//!
//! Universal: no call leaves the Mac without `admit`, every response goes through
//! `record`, and the engine owns the clock. Per venue and never blended: budgets,
//! ban semantics, header names — all in `policy`.
//!
//! Adapters do not sleep and retry. They are refused with `rate_limited` /
//! `venue_frozen` / `venue_banned` and stop.

pub mod ban;
pub mod clock;
pub mod engine;
pub mod meter;
pub mod policy;
pub mod transport;
pub mod types;

pub use engine::{VenueEgress, VenuePosture};
pub use transport::{split_url, EgressCall, EgressError, EgressResponse, EgressTransport};
pub use types::{Decision, EgressRequest, Lane, Outcome, RefuseKind, RefuseReason};

use std::sync::{Arc, OnceLock};

/// The process-wide egress. This is the only owner of a `reqwest::Client` that
/// may reach a venue host — a test in `agent/tests/egress_chokepoint.rs` asserts
/// no other module in `src/` builds one.
pub fn shared() -> Arc<EgressTransport> {
    static SHARED: OnceLock<Arc<EgressTransport>> = OnceLock::new();
    SHARED
        .get_or_init(|| Arc::new(EgressTransport::new_system()))
        .clone()
}

/// Is this host one of the venues the engine knows? Callers that can legitimately
/// be pointed at a non-venue URL (a local test server) use this to decide whether
/// there is anything to meter. It never widens what a venue host may do:
/// `admit_auth` still refuses any host that is not a venue.
pub fn is_venue_host(host: &str) -> bool {
    policy::slot_for_host(host).is_some()
}

/// Admission state only. The blocking Wasm transport needs this without needing
/// the async send path.
pub fn shared_engine() -> Arc<VenueEgress> {
    shared().engine().clone()
}

/// A handle on the single connection pool, for the one caller that must own its
/// own runtime (`ubi::http::ReqwestBrokerHttpTransport`, whose `send` is
/// synchronous because the Wasm import blocks).
pub fn shared_client() -> reqwest::Client {
    shared().client()
}
