//! Capability identity: family + capability ID + physics (#357, #363).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Closed capability families. Serialize as snake_case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Family {
    Reference,
    Market,
    Economic,
    Fundamentals,
    News,
    Derived,
    Account,
}

impl Family {
    pub fn as_str(self) -> &'static str {
        match self {
            Family::Reference => "reference",
            Family::Market => "market",
            Family::Economic => "economic",
            Family::Fundamentals => "fundamentals",
            Family::News => "news",
            Family::Derived => "derived",
            Family::Account => "account",
        }
    }
}

impl fmt::Display for Family {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Closed physics kinds. Serialize as snake_case.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Physics {
    BoundedSnapshot,
    VersionedSnapshot,
    LatestState,
    OrderedState,
    CompleteEventSequence,
    LossyEventObservation,
    HistoricalSeries,
}

impl Physics {
    pub fn as_str(self) -> &'static str {
        match self {
            Physics::BoundedSnapshot => "bounded_snapshot",
            Physics::VersionedSnapshot => "versioned_snapshot",
            Physics::LatestState => "latest_state",
            Physics::OrderedState => "ordered_state",
            Physics::CompleteEventSequence => "complete_event_sequence",
            Physics::LossyEventObservation => "lossy_event_observation",
            Physics::HistoricalSeries => "historical_series",
        }
    }
}

impl fmt::Display for Physics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Why [`CapabilityId::new`] refused a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapabilityIdError {
    InvalidPattern(String),
    EncodedVendorOrInterval(String),
}

impl fmt::Display for CapabilityIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapabilityIdError::InvalidPattern(id) => {
                write!(
                    f,
                    "capability_id `{id}` is not lowercase snake (`^[a-z][a-z0-9]*(_[a-z0-9]+)*$`)"
                )
            }
            CapabilityIdError::EncodedVendorOrInterval(id) => {
                write!(
                    f,
                    "capability_id `{id}` encodes vendor, interval, or symbol"
                )
            }
        }
    }
}

impl std::error::Error for CapabilityIdError {}

/// Product capability ID. Not a vendor, interval, or ticker.
///
/// Serde wraps the JSON string as-is so [`super::Registry::load`] can return a
/// structured [`super::Reject`] instead of a deserialize error.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CapabilityId(pub(crate) String);

impl CapabilityId {
    /// Fallible constructor for programmatic IDs. Unknown IDs are allowed only
    /// when they are lowercase snake and do not encode vendor / interval / symbol.
    pub fn new(raw: impl AsRef<str>) -> Result<Self, CapabilityIdError> {
        let raw = raw.as_ref();
        validate_capability_id(raw)?;
        Ok(Self(raw.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CapabilityId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Three-field identity. Interval, symbol, and venue are request params, not IDs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Identity {
    pub family: Family,
    pub capability_id: CapabilityId,
    pub physics: Physics,
}

impl Identity {
    pub fn new(family: Family, capability_id: CapabilityId, physics: Physics) -> Self {
        Self {
            family,
            capability_id,
            physics,
        }
    }
}

impl fmt::Display for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}/{}", self.family, self.capability_id, self.physics)
    }
}

const VENDOR_TOKENS: &[&str] = &[
    "twelve_data",
    "yahoo",
    "alpha_vantage",
    "finnhub",
    "openbb",
    "odp",
    "kotak",
    "zerodha",
    "kite",
    "binance",
];

const INTERVAL_SEGMENTS: &[&str] = &["1d", "1m", "5m", "15m", "1h", "4h", "1w", "1mo"];

/// `^[a-z][a-z0-9]*(_[a-z0-9]+)*$`
pub(crate) fn is_snake_capability_id(s: &str) -> bool {
    let mut chars = s.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_lowercase() {
        return false;
    }
    let mut prev_underscore = false;
    for c in chars {
        if c == '_' {
            if prev_underscore {
                return false;
            }
            prev_underscore = true;
            continue;
        }
        if !(c.is_ascii_lowercase() || c.is_ascii_digit()) {
            return false;
        }
        prev_underscore = false;
    }
    !prev_underscore
}

pub(crate) fn encodes_vendor_interval_or_symbol(id: &str) -> bool {
    let wrapped = format!("_{id}_");
    if VENDOR_TOKENS
        .iter()
        .any(|tok| wrapped.contains(&format!("_{tok}_")))
    {
        return true;
    }
    if id.split('_').any(|seg| INTERVAL_SEGMENTS.contains(&seg)) {
        return true;
    }
    false
}

pub(crate) fn validate_capability_id(id: &str) -> Result<(), CapabilityIdError> {
    if !is_snake_capability_id(id) {
        return Err(CapabilityIdError::InvalidPattern(id.to_string()));
    }
    if encodes_vendor_interval_or_symbol(id) {
        return Err(CapabilityIdError::EncodedVendorOrInterval(id.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_accepts_canonical_quote() {
        assert_eq!(CapabilityId::new("quote").unwrap().as_str(), "quote");
    }

    #[test]
    fn new_rejects_vendor_interval_symbol() {
        let err = CapabilityId::new("twelve_data_aapl_1d").unwrap_err();
        assert!(matches!(err, CapabilityIdError::EncodedVendorOrInterval(_)));
    }

    #[test]
    fn new_rejects_uppercase() {
        let err = CapabilityId::new("Quote").unwrap_err();
        assert!(matches!(err, CapabilityIdError::InvalidPattern(_)));
    }

    #[test]
    fn serde_wraps_invalid_id_for_load_gate() {
        let id: CapabilityId = serde_json::from_str("\"twelve_data_aapl_1d\"").unwrap();
        assert_eq!(id.as_str(), "twelve_data_aapl_1d");
    }
}
