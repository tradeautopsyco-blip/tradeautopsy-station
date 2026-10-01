//! Shared pre-trade risk preview (Option A in `docs/design/risk-engine.md`).
//! Preview only — no broker orders, no invented sizing formula.

mod preview;

pub use preview::{compute_preview, RiskPreviewInput};
