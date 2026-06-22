use crate::class::PairClass;
use crate::profile::GapStats;

use super::basic::metric_prior_hybrid_for_class;
use super::constraints::{DeltaPlan, KerningFacts};
use super::geometry::PairGeometry;
use super::math::dead_zone;
#[cfg(test)]
use super::math::normalized_delta;
use super::run_context::sans_like_spacing_profile;
use super::types::EvaluationConfig;

pub(super) fn guarded_profile_hybrid(
    metric_delta: f32,
    optical_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
    pair_geometry: PairGeometry,
) -> f32 {
    if config.preserve_monospace {
        return metric_delta;
    }

    let facts = KerningFacts {
        metric_delta,
        optical_delta,
        nearest_delta,
        stats,
        config,
        pair_class,
        pair_geometry,
    };
    let mut plan = DeltaPlan::new(base_guarded_delta(facts));
    apply_guard_bounds(facts, &mut plan);
    add_tightening_targets(facts, &mut plan);
    plan.finish()
}

fn base_guarded_delta(facts: KerningFacts) -> f32 {
    let proposed =
        metric_prior_hybrid_for_class(facts.metric_delta, facts.optical_delta, facts.pair_class);
    if proposed > 0.0 && facts.metric_delta.abs() < dead_zone() {
        if facts.nearest_delta > facts.nearest_guard() {
            proposed
        } else {
            0.0
        }
    } else {
        proposed
    }
}

fn apply_guard_bounds(facts: KerningFacts, plan: &mut DeltaPlan) {
    let desired = plan.desired_delta();
    if desired < 0.0
        && !facts.pair_class.allows_tight_nearest_override()
        && (aperture_risk(facts.stats, facts.config) || facts.nearest_delta > facts.nearest_guard())
    {
        plan.require_at_least(if facts.metric_delta.abs() >= dead_zone() {
            facts.metric_delta
        } else {
            0.0
        });
    }

    if let Some(bound) = metricless_upper_lower_aperture_lower_bound(facts, desired) {
        plan.require_at_least(bound);
    }
    if let Some(bound) = collision_opening_lower_bound(facts, desired) {
        plan.require_at_least(bound);
    }
    if suppress_false_diagonal_opening_target(facts, desired) {
        plan.limit_to_at_most(0.0);
    }
}

fn add_tightening_targets(facts: KerningFacts, plan: &mut DeltaPlan) {
    for target in [
        lower_upper_overhang_target,
        side_shape_target,
        punctuation_spacing_target,
        wide_serif_display_target,
        sans_lowercase_compaction_target,
        spacing_compaction_target,
    ] {
        let Some(target) = target(facts, plan.desired_delta()) else {
            continue;
        };
        plan.tighten_to(target);
    }
}

fn metricless_upper_lower_aperture_lower_bound(
    facts: KerningFacts,
    desired_delta: f32,
) -> Option<f32> {
    if !facts.pair_class.is_upper_lower()
        || facts.metric_delta.abs() >= dead_zone()
        || desired_delta >= -dead_zone()
        || !facts.pair_geometry.right_left_side.is_round_like()
    {
        return None;
    }

    let safe_min = (facts.config.target_gap_em * 0.42).clamp(0.070, 0.120);
    if facts.nearest_delta <= facts.nearest_guard() && !aperture_risk(facts.stats, facts.config) {
        return None;
    }
    if facts.stats.min_gap > safe_min {
        return None;
    }

    Some(-(facts.config.gap_mad_em * 1.05).clamp(0.045, 0.065))
}

#[cfg(test)]
pub(super) fn suppress_false_diagonal_opening(
    adjusted_delta: f32,
    metric_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
    pair_geometry: PairGeometry,
) -> f32 {
    let facts = KerningFacts {
        metric_delta,
        optical_delta: 0.0,
        nearest_delta: 0.0,
        stats,
        config,
        pair_class,
        pair_geometry,
    };
    if suppress_false_diagonal_opening_target(facts, adjusted_delta) {
        0.0
    } else {
        adjusted_delta
    }
}

fn suppress_false_diagonal_opening_target(facts: KerningFacts, _desired_delta: f32) -> bool {
    if facts.metric_delta > dead_zone() || !facts.pair_class.is_upper_upper() {
        return false;
    }

    if facts.config.target_gap_em < 0.255
        || facts.config.profile.x_height / facts.config.profile.cap_height > 0.72
    {
        return false;
    }

    facts.pair_geometry.has_diagonal_pair()
        && facts.stats.min_gap > -0.020
        && facts.stats.robust_mean_gap > facts.spread_upper()
}

#[cfg(test)]
pub(super) fn wide_serif_display_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
    pair_geometry: PairGeometry,
) -> f32 {
    let facts = KerningFacts {
        metric_delta,
        optical_delta: 0.0,
        nearest_delta,
        stats,
        config,
        pair_class,
        pair_geometry,
    };
    if let Some(target) = wide_serif_display_target(facts, adjusted_delta) {
        normalized_delta(target - adjusted_delta)
    } else {
        0.0
    }
}

fn wide_serif_display_target(facts: KerningFacts, desired_delta: f32) -> Option<f32> {
    if facts.config.target_gap_em < 0.255
        || facts.config.profile.x_height / facts.config.profile.cap_height > 0.72
    {
        return None;
    }

    let safe_min = (facts.config.target_gap_em * 0.48).clamp(0.11, 0.16);
    if facts.nearest_delta > facts.nearest_guard()
        || facts.stats.min_gap <= safe_min
        || aperture_risk(facts.stats, facts.config)
    {
        return None;
    }

    if facts.pair_class.is_upper_upper() {
        return serif_diagonal_upper_target(facts, desired_delta);
    }

    if facts.pair_class.is_upper_lower() || facts.pair_class.is_lower_upper() {
        return serif_mixed_case_target(facts, desired_delta);
    }

    None
}

fn serif_diagonal_upper_target(facts: KerningFacts, desired_delta: f32) -> Option<f32> {
    if !facts.pair_geometry.has_diagonal_pair()
        || facts.metric_delta < -0.105
        || desired_delta < -0.120
    {
        return None;
    }

    let gap_bonus =
        ((facts.stats.robust_mean_gap - facts.spread_upper()).max(0.0) * 0.18).clamp(0.0, 0.014);
    let base = if facts.metric_delta.abs() < dead_zone() {
        0.030
    } else {
        0.022
    };
    bounded_tightening_target(
        desired_delta.min(facts.metric_delta.min(0.0)) - base - gap_bonus,
        -0.125,
        desired_delta,
    )
}

fn serif_mixed_case_target(facts: KerningFacts, desired_delta: f32) -> Option<f32> {
    if facts.metric_delta > -dead_zone() || desired_delta < -0.135 {
        return None;
    }

    let has_round_or_overhang = facts.pair_geometry.left_right_side.is_round_like()
        || facts.pair_geometry.right_left_side.is_round_like()
        || facts.pair_geometry.right_top_left_overhang > 0.10;
    if !has_round_or_overhang {
        return None;
    }

    let gap_bonus =
        ((facts.stats.robust_mean_gap - facts.spread_upper()).max(0.0) * 0.16).clamp(0.0, 0.014);
    bounded_tightening_target(
        desired_delta.min(facts.metric_delta) - 0.018 - gap_bonus,
        -0.140,
        desired_delta,
    )
}

#[cfg(test)]
pub(super) fn sans_lowercase_compaction_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
) -> f32 {
    let facts = KerningFacts {
        metric_delta,
        optical_delta: 0.0,
        nearest_delta,
        stats,
        config,
        pair_class,
        pair_geometry: PairGeometry::default(),
    };
    if let Some(target) = sans_lowercase_compaction_target(facts, adjusted_delta) {
        normalized_delta(target - adjusted_delta)
    } else {
        0.0
    }
}

fn sans_lowercase_compaction_target(facts: KerningFacts, desired_delta: f32) -> Option<f32> {
    if !sans_like_spacing_profile(facts.config) {
        return None;
    }

    if !(facts.pair_class.is_lower_lower() || facts.pair_class.is_upper_lower()) {
        return None;
    }

    let safe_min = (facts.config.target_gap_em * 0.36).clamp(0.070, 0.100);
    if facts.nearest_delta > facts.nearest_guard()
        || facts.stats.min_gap <= safe_min
        || aperture_risk(facts.stats, facts.config)
    {
        return None;
    }

    if facts.pair_class.is_lower_lower() && facts.metric_delta.abs() >= 0.025 {
        return None;
    }

    let amount = if facts.pair_class.is_upper_lower() && facts.metric_delta < -dead_zone() {
        0.030
    } else if facts.pair_class.is_upper_lower() {
        0.020
    } else {
        0.018
    };
    bounded_tightening_target(desired_delta - amount, -0.105, desired_delta)
}

#[cfg(test)]
pub(super) fn side_shape_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
    pair_geometry: PairGeometry,
) -> f32 {
    let facts = KerningFacts {
        metric_delta,
        optical_delta: 0.0,
        nearest_delta,
        stats,
        config,
        pair_class,
        pair_geometry,
    };
    if let Some(target) = side_shape_target(facts, adjusted_delta) {
        normalized_delta(target - adjusted_delta)
    } else {
        return 0.0;
    }
}

fn side_shape_target(facts: KerningFacts, desired_delta: f32) -> Option<f32> {
    if facts.pair_class.is_upper_lower()
        && facts.metric_delta < -dead_zone()
        && facts.pair_geometry.right_left_side.roundness > 0.040
        && facts.nearest_delta <= facts.nearest_guard()
        && facts.stats.robust_mean_gap > facts.spread_upper() + 0.012
    {
        return bounded_tightening_target(
            facts.metric_delta * 0.94,
            facts.metric_delta,
            desired_delta,
        );
    }

    if !facts.pair_class.has_digit() {
        return None;
    }

    let safe_min = if facts.pair_class.is_digit_digit() {
        (facts.config.target_gap_em * 0.24).clamp(0.045, 0.075)
    } else {
        (facts.config.target_gap_em * 0.32).clamp(0.060, 0.105)
    };
    if facts.nearest_delta > facts.nearest_guard() || facts.stats.min_gap <= safe_min {
        return None;
    }

    let target = if facts.pair_class.is_digit_digit() {
        digit_digit_target(facts.pair_geometry)
    } else if facts.pair_class.is_digit_punctuation() || facts.pair_class.is_punctuation_digit() {
        digit_punctuation_target(facts.pair_geometry)
    } else {
        0.0
    };

    (target < desired_delta).then_some(target)
}

#[cfg(test)]
pub(super) fn collision_opening_delta(
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
) -> f32 {
    let facts = KerningFacts {
        metric_delta: 0.0,
        optical_delta: 0.0,
        nearest_delta,
        stats,
        config,
        pair_class,
        pair_geometry: PairGeometry::default(),
    };
    if let Some(bound) = collision_opening_lower_bound(facts, adjusted_delta) {
        normalized_delta(bound - adjusted_delta)
    } else {
        0.0
    }
}

fn collision_opening_lower_bound(facts: KerningFacts, desired_delta: f32) -> Option<f32> {
    if !facts.pair_class.allows_collision_opening() {
        return None;
    }

    if facts.stats.min_gap > 0.0 || facts.nearest_delta <= facts.nearest_guard() {
        return None;
    }

    let penetration = (-facts.stats.min_gap).max(0.0);
    let target =
        (facts.nearest_delta * 0.78 + penetration * 0.22).clamp(facts.nearest_guard(), 0.055);
    (target > desired_delta).then_some(target)
}

#[cfg(test)]
pub(super) fn punctuation_spacing_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
) -> f32 {
    let facts = KerningFacts {
        metric_delta,
        optical_delta: 0.0,
        nearest_delta,
        stats,
        config,
        pair_class,
        pair_geometry: PairGeometry::default(),
    };
    if let Some(target) = punctuation_spacing_target(facts, adjusted_delta) {
        normalized_delta(target - adjusted_delta)
    } else {
        0.0
    }
}

fn punctuation_spacing_target(facts: KerningFacts, desired_delta: f32) -> Option<f32> {
    if !facts.pair_class.is_upper_punctuation() || facts.metric_delta >= -dead_zone() {
        return None;
    }

    let safe_min = (facts.config.target_gap_em * 0.65).clamp(0.12, 0.18);
    if facts.nearest_delta > facts.nearest_guard() || facts.stats.min_gap <= safe_min {
        return None;
    }

    let base = (facts.config.gap_mad_em * 0.46).clamp(0.018, 0.035);
    let gap_excess = (facts.stats.robust_mean_gap - facts.config.target_gap_em).max(0.0);
    let gap_bonus = (gap_excess * 0.12).clamp(0.0, 0.014);
    bounded_tightening_target(
        facts.metric_delta.min(desired_delta) - base - gap_bonus,
        -0.120,
        desired_delta,
    )
}

fn digit_digit_target(pair_geometry: PairGeometry) -> f32 {
    let left = pair_geometry.left_right_side;
    let right = pair_geometry.right_left_side;
    let stem_round = left.stemness > 0.62 && right.is_round_like();
    let round_stem = left.is_round_like() && right.stemness > 0.62;
    let round_round = left.is_round_like() && right.is_round_like();

    if stem_round || round_stem {
        -0.040
    } else if round_round {
        -0.024
    } else {
        0.0
    }
}

fn digit_punctuation_target(pair_geometry: PairGeometry) -> f32 {
    let digit_side = if pair_geometry.left_right_side.has_shape() {
        pair_geometry.left_right_side
    } else {
        pair_geometry.right_left_side
    };

    if digit_side.is_round_like() || digit_side.stemness > 0.62 {
        -0.010
    } else {
        0.0
    }
}

fn lower_upper_overhang_target(facts: KerningFacts, desired_delta: f32) -> Option<f32> {
    if !facts.pair_class.is_lower_upper()
        || facts.metric_delta.abs() >= dead_zone()
        || facts.optical_delta >= -dead_zone()
        || desired_delta <= -0.090
    {
        return None;
    }

    let safe_min = (facts.config.target_gap_em * 0.58).clamp(0.10, 0.18);
    if facts.nearest_delta > facts.nearest_guard()
        || facts.stats.min_gap <= safe_min
        || aperture_risk(facts.stats, facts.config)
    {
        return None;
    }

    let overhang = facts.pair_geometry.right_top_left_overhang;
    if overhang <= 0.10 {
        return None;
    }

    let gap_excess = (facts.stats.robust_mean_gap - facts.spread_upper()).max(0.0);
    let shape_bonus = ((overhang - 0.10) * 0.24).clamp(0.0, 0.040);
    let gap_bonus = (gap_excess * 0.40).clamp(0.0, 0.030);
    let round_bonus = if facts.pair_geometry.left_right_side.is_round_like() && overhang > 0.18 {
        let curvature_bonus =
            ((facts.pair_geometry.left_right_side.roundness - 0.030) * 0.70).clamp(0.0, 0.024);
        let overhang_bonus = ((overhang - 0.18) * 0.16).clamp(0.0, 0.020);
        (curvature_bonus + overhang_bonus).clamp(0.0, 0.034)
    } else {
        0.0
    };
    let lower_bound = if facts.pair_geometry.left_right_side.is_round_like() && overhang > 0.18 {
        -0.120
    } else {
        -0.095
    };
    bounded_tightening_target(
        desired_delta - shape_bonus - gap_bonus - round_bonus,
        lower_bound,
        desired_delta,
    )
}

fn bounded_tightening_target(
    proposed_delta: f32,
    lower_bound: f32,
    desired_delta: f32,
) -> Option<f32> {
    if !proposed_delta.is_finite() || !lower_bound.is_finite() || !desired_delta.is_finite() {
        return None;
    }
    if lower_bound >= desired_delta {
        return None;
    }

    let target = proposed_delta.clamp(lower_bound, desired_delta);
    (target < desired_delta).then_some(target)
}

fn aperture_risk(stats: GapStats, config: EvaluationConfig) -> bool {
    if stats.min_gap <= 0.0 {
        return false;
    }

    let close_min = (config.target_gap_em * 0.42).clamp(0.040, 0.120);
    let spread = (config.gap_mad_em * 1.35).clamp(0.035, 0.14);
    let upper = config.target_gap_em + spread;
    let mean_to_min_ratio = stats.robust_mean_gap / stats.min_gap;

    stats.min_gap <= close_min && stats.robust_mean_gap > upper && mean_to_min_ratio >= 3.2
}

fn spacing_compaction_target(facts: KerningFacts, desired_delta: f32) -> Option<f32> {
    if !facts.pair_class.allows_safe_compaction() {
        return None;
    }

    if desired_delta.abs() >= 0.045 {
        return None;
    }

    if desired_delta < -dead_zone() && facts.metric_delta.abs() < dead_zone() {
        return None;
    }

    if facts.nearest_delta > facts.nearest_guard() || aperture_risk(facts.stats, facts.config) {
        return None;
    }

    let safe_min = (facts.config.target_gap_em * 0.22).clamp(0.045, 0.065);
    if facts.stats.min_gap <= safe_min {
        return None;
    }

    let amount = (facts.config.gap_mad_em * 0.25).clamp(0.008, 0.016);
    Some(desired_delta - amount)
}
