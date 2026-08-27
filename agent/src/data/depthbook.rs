//! In-memory REST depth snapshots. Not TickBook, not an ordered replica.

use super::kotak_depth::DepthSnapshot;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct DepthBook {
    rows: HashMap<String, DepthSnapshot>,
}

impl DepthBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, instrument_id: &str) -> Option<&DepthSnapshot> {
        self.rows.get(instrument_id)
    }

    pub fn upsert(&mut self, snapshot: DepthSnapshot) {
        if !snapshot.completeness {
            return;
        }
        self.rows.insert(snapshot.instrument_id.clone(), snapshot);
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &DepthSnapshot)> {
        self.rows.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}
