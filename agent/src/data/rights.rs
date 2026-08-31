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

    /// Venue-published numbers Station is licensed to show on the desk.
    ///
    /// Distinct from [`Self::research_fetch_only`] — do not mutate that one; other
    /// bindings depend on it. Display is granted; store, compose and redistribute
    /// are not: Station may show what the venue published, not resell it.
    pub fn desk_display() -> Self {
        Self {
            research_fetch: true,
            store: false,
            display: true,
            compose: false,
            redistribute: false,
            derived: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desk_display_grants_display_without_redistribution() {
        let rights = Rights::desk_display();
        assert!(rights.research_fetch);
        assert!(rights.display, "this is the whole point of the helper");
        assert!(rights.derived, "greeks are a Derived-family number");
        assert!(!rights.store);
        assert!(!rights.compose);
        assert!(!rights.redistribute);
    }

    #[test]
    fn research_fetch_only_is_untouched_by_the_new_grant() {
        // Other bindings depend on this one. Adding a desk grant must not widen it.
        let research = Rights::research_fetch_only();
        assert!(research.research_fetch);
        assert!(!research.display, "research fixtures still never render");
        assert!(!research.derived);
        assert_ne!(research, Rights::desk_display());
    }
}
