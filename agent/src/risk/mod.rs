//! Shared pre-trade risk preview (Option A in `docs/design/risk-engine.md`).
//! Fixed-risk quantity follows Nautilus `calculate_fixed_risk_position_size`.
//! Preview only — no broker orders.

mod fixed_risk;
mod preview;

pub use preview::{compute_preview, RiskPreviewInput};
