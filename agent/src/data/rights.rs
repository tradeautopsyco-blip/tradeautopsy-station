//! Binding rights (#367). All six flags required; absence is a registration reject.

use serde::{Deserialize, Serialize};

/// License / product-use flags declared on a binding.
///
/// No `Default`. Missing object must not become all-false; that is [`super::Reject::RightsMissing`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rights {
    pub research_fetch: bool,
    pub store: bool,
    pub display: bool,
    pub compose: bool,
    pub redistribute: bool,
    pub derived: bool,
}

impl Rights {
    /// Research fixture: fetch allowed, no canonical store/display/compose.
    pub fn research_fetch_only() -> Self {
        Self {
            research_fetch: true,
            store: false,
            display: false,
            compose: false,
            redistribute: false,
            derived: false,
        }
    }
}
