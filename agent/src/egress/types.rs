//! Core egress vocabulary. Shared by every slot; blended by none.

use std::sync::Arc;

use serde::Serialize;

/// Which reserve lane a call draws from. Lanes exist so a market-data flood can
/// never consume the headroom an exit order needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Lane {
    MarketData,
    PrivateRead,
    Execution,
}

impl Lane {
    pub fn as_str(self) -> &'static str {
        match self {
            Lane::MarketData => "market_data",
            Lane::PrivateRead => "private_read",
            Lane::Execution => "execution",
        }
    }

    /// Market data is the only lane that refuses at the reserve floor.
    pub fn is_reserved(self) -> bool {
        matches!(self, Lane::PrivateRead | Lane::Execution)
    }
}

/// What a venue is currently allowing. `Banned` is slot-wide (IP level);
/// `Backoff` is per meter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "posture", rename_all = "snake_case")]
pub enum Posture {
    Live,
    Backoff { until_ms: i64 },
    Banned { until_ms: i64 },
}

impl Posture {
    pub fn as_str(self) -> &'static str {
        match self {
            Posture::Live => "live",
            Posture::Backoff { .. } => "backoff",
            Posture::Banned { .. } => "banned",
        }
    }

    pub fn until_ms(self) -> Option<i64> {
        match self {
            Posture::Live => None,
            Posture::Backoff { until_ms } | Posture::Banned { until_ms } => Some(until_ms),
        }
    }

    /// Postures expire on their own; nothing needs to tick them.
    pub fn resolved(self, now_ms: i64) -> Posture {
        match self {
            Posture::Live => Posture::Live,
            Posture::Backoff { until_ms } | Posture::Banned { until_ms } if until_ms <= now_ms => {
                Posture::Live
            }
            other => other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RefuseKind {
    /// Host is not on this slot, or the book id is unknown.
    WrongBook,
    /// The R0/R6 fence refused this host, path, or method.
    HostBlocked,
    /// This meter is in 429 backoff.
    Frozen,
    /// The whole slot is IP-banned.
    Banned,
    /// Budget or concurrency exhausted for this lane.
    OverBudget,
}

impl RefuseKind {
    pub fn as_str(self) -> &'static str {
        match self {
            RefuseKind::WrongBook => "wrong_book",
            RefuseKind::HostBlocked => "host_blocked",
            RefuseKind::Frozen => "venue_frozen",
            RefuseKind::Banned => "venue_banned",
            RefuseKind::OverBudget => "rate_limited",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct RefuseReason {
    pub kind: RefuseKind,
    /// When the caller may reasonably try again. `None` means "not from a clock" —
    /// a fence refusal never becomes allowed by waiting.
    pub until_ms: Option<i64>,
}

impl RefuseReason {
    pub fn new(kind: RefuseKind, until_ms: Option<i64>) -> Self {
        Self { kind, until_ms }
    }

    pub fn as_str(&self) -> &'static str {
        self.kind.as_str()
    }
}

impl std::fmt::Display for RefuseReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.until_ms {
            Some(until) => write!(f, "{}:until_ms={until}", self.kind.as_str()),
            None => f.write_str(self.kind.as_str()),
        }
    }
}

/// Proof of admission. Deliberately **not** `Clone` and not constructible outside
/// this module: an outcome cannot be recorded against a meter it was not drawn
/// from, and a single admission cannot be spent twice.
pub struct Permit {
    pub(crate) slot: &'static str,
    pub(crate) meter: &'static str,
    pub(crate) lane: Lane,
    pub(crate) forecast_cost: u32,
    pub(crate) issued_at_ms: i64,
    /// Set by `VenueEgress::record`. A permit that is dropped without being
    /// recorded has leaked a concurrency slot and thrown away whatever the venue
    /// said — including a 429 or a 418. Loud, because it is silent otherwise.
    pub(crate) recorded: bool,
    /// Installed by the engine at admission: gives the concurrency slot back
    /// when this permit is dropped unrecorded — async cancellation kills a
    /// holder mid-send (axum drops handlers when the client goes away), and the
    /// slot must not leak with it. The forecast stays spent: the call may
    /// already have left the Mac, so it is not refunded, and no posture changes
    /// because an unobserved outcome is not a venue instruction.
    pub(crate) on_unrecorded_drop: Option<Arc<dyn Fn(&'static str, &'static str) + Send + Sync>>,
}

impl std::fmt::Debug for Permit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Permit")
            .field("slot", &self.slot)
            .field("meter", &self.meter)
            .field("lane", &self.lane)
            .field("forecast_cost", &self.forecast_cost)
            .field("issued_at_ms", &self.issued_at_ms)
            .field("recorded", &self.recorded)
            .finish()
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        if self.recorded {
            return;
        }
        if let Some(release) = self.on_unrecorded_drop.take() {
            release(self.slot, self.meter);
        }
        tracing::warn!(
            slot = self.slot,
            meter = self.meter,
            "VenueEgress: permit dropped without record() — slot released; the \
             venue's response was not accounted"
        );
    }
}

impl Permit {
    pub fn slot(&self) -> &'static str {
        self.slot
    }
    pub fn meter(&self) -> &'static str {
        self.meter
    }
    pub fn lane(&self) -> Lane {
        self.lane
    }
    pub fn forecast_cost(&self) -> u32 {
        self.forecast_cost
    }
    pub fn issued_at_ms(&self) -> i64 {
        self.issued_at_ms
    }
}

#[derive(Debug)]
pub enum Decision {
    Admit(Permit),
    Refuse(RefuseReason),
}

impl Decision {
    pub fn refused(&self) -> Option<RefuseReason> {
        match self {
            Decision::Admit(_) => None,
            Decision::Refuse(reason) => Some(*reason),
        }
    }
}

/// What actually came back. `Transport` is a network failure with no HTTP status —
/// it must never be confused with a venue saying stop.
#[derive(Debug, Clone)]
pub enum Outcome {
    Http {
        status: u16,
        /// Already passed through `ubi::http::redact_response_headers`.
        headers: Vec<(String, String)>,
        /// Enough of the body to read a venue's own ban expiry out of it.
        /// Truncate at the call site — see `Outcome::http`.
        body: String,
    },
    Transport,
}

/// How much response body the engine keeps. Binance ban messages are well under
/// this; anything longer cannot contain a ban expiry we would trust.
pub const BODY_SNIPPET_LIMIT: usize = 512;

impl Outcome {
    /// Build an HTTP outcome, truncating the body on a char boundary.
    pub fn http(status: u16, headers: Vec<(String, String)>, body: &str) -> Self {
        let mut end = body.len().min(BODY_SNIPPET_LIMIT);
        while end > 0 && !body.is_char_boundary(end) {
            end -= 1;
        }
        Outcome::Http {
            status,
            headers,
            body: body[..end].to_string(),
        }
    }

    pub fn status(&self) -> Option<u16> {
        match self {
            Outcome::Http { status, .. } => Some(*status),
            Outcome::Transport => None,
        }
    }

    pub fn body(&self) -> &str {
        match self {
            Outcome::Http { body, .. } => body.as_str(),
            Outcome::Transport => "",
        }
    }

    pub fn header(&self, name: &str) -> Option<&str> {
        match self {
            Outcome::Http { headers, .. } => headers
                .iter()
                .find(|(n, _)| n.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.as_str()),
            Outcome::Transport => None,
        }
    }
}

/// One call, described before it leaves the Mac.
#[derive(Debug, Clone)]
pub struct EgressRequest<'a> {
    pub book_id: &'a str,
    pub host: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    /// Raw query string without the leading `?`, used for weight forecasting.
    pub query: &'a str,
    pub lane: Lane,
    /// Overrides the policy weight table when a caller knows better. Rare.
    pub weight_hint: Option<u32>,
}

impl<'a> EgressRequest<'a> {
    pub fn new(
        book_id: &'a str,
        host: &'a str,
        method: &'a str,
        path: &'a str,
        lane: Lane,
    ) -> Self {
        Self {
            book_id,
            host,
            method,
            path,
            query: "",
            lane,
            weight_hint: None,
        }
    }

    pub fn with_query(mut self, query: &'a str) -> Self {
        self.query = query.trim_start_matches('?');
        self
    }

    pub fn with_weight_hint(mut self, weight: u32) -> Self {
        self.weight_hint = Some(weight);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn posture_expires_on_its_own() {
        let p = Posture::Backoff { until_ms: 1_000 };
        assert_eq!(p.resolved(999), p);
        assert_eq!(p.resolved(1_000), Posture::Live);
        assert_eq!(p.resolved(1_001), Posture::Live);
    }

    #[test]
    fn ban_expires_on_its_own() {
        let p = Posture::Banned { until_ms: 5_000 };
        assert_eq!(p.resolved(4_999), p);
        assert_eq!(p.resolved(5_000), Posture::Live);
    }

    #[test]
    fn body_truncation_never_splits_a_char() {
        let body = "\u{1F600}".repeat(400); // 1600 bytes of 4-byte chars
        let out = Outcome::http(418, vec![], &body);
        assert!(out.body().len() <= BODY_SNIPPET_LIMIT);
        assert!(body.starts_with(out.body()));
    }

    #[test]
    fn only_market_data_is_unreserved() {
        assert!(!Lane::MarketData.is_reserved());
        assert!(Lane::PrivateRead.is_reserved());
        assert!(Lane::Execution.is_reserved());
    }

    #[test]
    fn transport_failure_has_no_status() {
        assert_eq!(Outcome::Transport.status(), None);
        let http = Outcome::http(418, vec![("retry-after".into(), "120".into())], "banned");
        assert_eq!(http.status(), Some(418));
        assert_eq!(http.header("Retry-After"), Some("120"));
        assert_eq!(http.body(), "banned");
    }
}
