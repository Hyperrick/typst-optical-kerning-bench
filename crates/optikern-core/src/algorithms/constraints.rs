use crate::class::PairClass;
use crate::profile::GapStats;

use super::geometry::PairGeometry;
use super::math::{clamp_delta, normalized_delta};
use super::types::EvaluationConfig;

#[derive(Debug, Clone, Copy)]
pub(super) struct KerningFacts {
    pub(super) metric_delta: f32,
    pub(super) optical_delta: f32,
    pub(super) nearest_delta: f32,
    pub(super) stats: GapStats,
    pub(super) config: EvaluationConfig,
    pub(super) pair_class: PairClass,
    pub(super) pair_geometry: PairGeometry,
}

impl KerningFacts {
    pub(super) fn nearest_guard(self) -> f32 {
        (self.config.target_gap_em * 0.08).clamp(0.012, 0.020)
    }

    pub(super) fn spread_upper(self) -> f32 {
        let spread = (self.config.gap_mad_em * 1.35).clamp(0.035, 0.14);
        self.config.target_gap_em + spread
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct DeltaPlan {
    desired_delta: f32,
    lower_bound: f32,
    upper_bound: f32,
}

impl DeltaPlan {
    pub(super) fn new(desired_delta: f32) -> Self {
        Self {
            desired_delta,
            lower_bound: -0.16,
            upper_bound: 0.16,
        }
    }

    pub(super) fn desired_delta(self) -> f32 {
        self.desired_delta
    }

    pub(super) fn tighten_to(&mut self, target_delta: f32) {
        self.desired_delta = self.desired_delta.min(target_delta);
    }

    pub(super) fn require_at_least(&mut self, lower_bound: f32) {
        self.lower_bound = self.lower_bound.max(lower_bound);
    }

    pub(super) fn limit_to_at_most(&mut self, upper_bound: f32) {
        self.upper_bound = self.upper_bound.min(upper_bound);
    }

    pub(super) fn finish(self) -> f32 {
        if self.lower_bound > self.upper_bound {
            return normalized_delta(clamp_delta(self.upper_bound));
        }
        normalized_delta(clamp_delta(
            self.desired_delta.clamp(self.lower_bound, self.upper_bound),
        ))
    }
}
