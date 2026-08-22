//! In-memory LiveBook — Station live-read authority after one hosted snapshot.

use serde_json::Value;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Default)]
struct Inner {
    value: Option<Value>,
    hydrated_at: Option<Instant>,
}

#[derive(Default)]
pub struct LiveBook {
    inner: Mutex<Inner>,
}

impl LiveBook {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn hydrate(&self, value: Value) {
        let mut g = self.inner.lock().expect("live book");
        g.value = Some(value);
        g.hydrated_at = Some(Instant::now());
    }

    pub fn snapshot(&self) -> Option<Value> {
        self.inner.lock().expect("live book").value.clone()
    }
}
