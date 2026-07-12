//! Small, deterministic decision kernel for optical kerning.
//!
//! The host owns shaping, outline sampling, calibration, and caching. This
//! crate only maps normalized pair evidence to an `em` adjustment.

mod pair;
mod preservation;
mod run;
mod types;

pub use pair::{compact_guarded, fallback_only, nearest_contour};
pub use run::compact_guarded_run;
pub use types::{GlyphClass, PairEvidence, RunPair, SideShape};

pub(crate) const DEAD_ZONE: f32 = 0.006;
pub(crate) const MAX_DELTA: f32 = 0.16;

pub(crate) fn normalize(value: f32) -> f32 {
    let value = value.clamp(-MAX_DELTA, MAX_DELTA);
    if value.abs() < DEAD_ZONE { 0.0 } else { value }
}
