use crate::class::PairClass;
use crate::profile::GapStats;

use super::basic::metric_prior_hybrid_for_class;
use super::geometry::PairGeometry;
use super::math::{dead_zone, normalized_delta};
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

    let proposed = metric_prior_hybrid_for_class(metric_delta, optical_delta, pair_class);
    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);
    let mut adjusted = if proposed > 0.0 && metric_delta.abs() < dead_zone() {
        if nearest_delta > nearest_guard {
            proposed
        } else {
            0.0
        }
    } else {
        proposed
    };

    if adjusted < 0.0
        && !pair_class.allows_tight_nearest_override()
        && (aperture_risk(stats, config) || nearest_delta > nearest_guard)
    {
        adjusted = if metric_delta.abs() >= dead_zone() {
            metric_delta
        } else {
            0.0
        };
    }

    adjusted += metricless_upper_lower_aperture_guard_delta(
        metric_delta,
        adjusted,
        nearest_delta,
        stats,
        config,
        pair_class,
        pair_geometry,
    );
    adjusted += lower_upper_overhang_delta(
        metric_delta,
        optical_delta,
        adjusted,
        nearest_delta,
        stats,
        config,
        pair_class,
        pair_geometry,
    );
    adjusted += side_shape_delta(
        metric_delta,
        adjusted,
        nearest_delta,
        stats,
        config,
        pair_class,
        pair_geometry,
    );
    adjusted += collision_opening_delta(adjusted, nearest_delta, stats, config, pair_class);
    adjusted += punctuation_spacing_delta(
        metric_delta,
        adjusted,
        nearest_delta,
        stats,
        config,
        pair_class,
    );
    adjusted = suppress_false_diagonal_opening(
        adjusted,
        metric_delta,
        stats,
        config,
        pair_class,
        pair_geometry,
    );
    adjusted += wide_serif_display_delta(
        metric_delta,
        adjusted,
        nearest_delta,
        stats,
        config,
        pair_class,
        pair_geometry,
    );
    adjusted += sans_lowercase_compaction_delta(
        metric_delta,
        adjusted,
        nearest_delta,
        stats,
        config,
        pair_class,
    );

    normalized_delta(
        adjusted
            + spacing_compaction_delta(
                metric_delta,
                adjusted,
                nearest_delta,
                stats,
                config,
                pair_class,
            ),
    )
}

fn metricless_upper_lower_aperture_guard_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
    pair_geometry: PairGeometry,
) -> f32 {
    if !pair_class.is_upper_lower()
        || metric_delta.abs() >= dead_zone()
        || adjusted_delta >= -dead_zone()
        || !pair_geometry.right_left_side.is_round_like()
    {
        return 0.0;
    }

    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);
    let safe_min = (config.target_gap_em * 0.42).clamp(0.070, 0.120);
    if nearest_delta <= nearest_guard && !aperture_risk(stats, config) {
        return 0.0;
    }
    if stats.min_gap > safe_min {
        return 0.0;
    }

    let lower_bound = -(config.gap_mad_em * 1.05).clamp(0.045, 0.065);
    let target = adjusted_delta.max(lower_bound);
    normalized_delta(target - adjusted_delta)
}

pub(super) fn suppress_false_diagonal_opening(
    adjusted_delta: f32,
    metric_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
    pair_geometry: PairGeometry,
) -> f32 {
    if adjusted_delta <= 0.0 || metric_delta > dead_zone() || !pair_class.is_upper_upper() {
        return adjusted_delta;
    }

    if config.target_gap_em < 0.255 || config.profile.x_height / config.profile.cap_height > 0.72 {
        return adjusted_delta;
    }

    if !pair_geometry.has_diagonal_pair() {
        return adjusted_delta;
    }

    let spread = (config.gap_mad_em * 1.35).clamp(0.035, 0.14);
    let upper = config.target_gap_em + spread;
    if stats.min_gap > -0.020 && stats.robust_mean_gap > upper {
        0.0
    } else {
        adjusted_delta
    }
}

pub(super) fn wide_serif_display_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
    pair_geometry: PairGeometry,
) -> f32 {
    if config.target_gap_em < 0.255 || config.profile.x_height / config.profile.cap_height > 0.72 {
        return 0.0;
    }

    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);
    let safe_min = (config.target_gap_em * 0.48).clamp(0.11, 0.16);
    if nearest_delta > nearest_guard || stats.min_gap <= safe_min || aperture_risk(stats, config) {
        return 0.0;
    }

    if pair_class.is_upper_upper() {
        return serif_diagonal_upper_delta(
            metric_delta,
            adjusted_delta,
            stats,
            config,
            pair_geometry,
        );
    }

    if pair_class.is_upper_lower() || pair_class.is_lower_upper() {
        return serif_mixed_case_delta(metric_delta, adjusted_delta, stats, config, pair_geometry);
    }

    0.0
}

fn serif_diagonal_upper_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_geometry: PairGeometry,
) -> f32 {
    if !pair_geometry.has_diagonal_pair() || metric_delta < -0.105 || adjusted_delta < -0.120 {
        return 0.0;
    }

    let spread = (config.gap_mad_em * 1.35).clamp(0.035, 0.14);
    let upper = config.target_gap_em + spread;
    let gap_bonus = ((stats.robust_mean_gap - upper).max(0.0) * 0.18).clamp(0.0, 0.014);
    let base = if metric_delta.abs() < dead_zone() {
        0.030
    } else {
        0.022
    };
    let target = (adjusted_delta.min(metric_delta.min(0.0)) - base - gap_bonus)
        .clamp(-0.125, adjusted_delta);
    normalized_delta(target - adjusted_delta)
}

fn serif_mixed_case_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_geometry: PairGeometry,
) -> f32 {
    if metric_delta > -dead_zone() || adjusted_delta < -0.135 {
        return 0.0;
    }

    let has_round_or_overhang = pair_geometry.left_right_side.is_round_like()
        || pair_geometry.right_left_side.is_round_like()
        || pair_geometry.right_top_left_overhang > 0.10;
    if !has_round_or_overhang {
        return 0.0;
    }

    let spread = (config.gap_mad_em * 1.35).clamp(0.035, 0.14);
    let upper = config.target_gap_em + spread;
    let gap_bonus = ((stats.robust_mean_gap - upper).max(0.0) * 0.16).clamp(0.0, 0.014);
    let target =
        (adjusted_delta.min(metric_delta) - 0.018 - gap_bonus).clamp(-0.140, adjusted_delta);
    normalized_delta(target - adjusted_delta)
}

pub(super) fn sans_lowercase_compaction_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
) -> f32 {
    if !sans_like_spacing_profile(config) {
        return 0.0;
    }

    if !(pair_class.is_lower_lower() || pair_class.is_upper_lower()) {
        return 0.0;
    }

    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);
    let safe_min = (config.target_gap_em * 0.36).clamp(0.070, 0.100);
    if nearest_delta > nearest_guard || stats.min_gap <= safe_min || aperture_risk(stats, config) {
        return 0.0;
    }

    if pair_class.is_lower_lower() && metric_delta.abs() >= 0.025 {
        return 0.0;
    }

    let amount = if pair_class.is_upper_lower() && metric_delta < -dead_zone() {
        0.030
    } else if pair_class.is_upper_lower() {
        0.020
    } else {
        0.018
    };
    let target = (adjusted_delta - amount).clamp(-0.105, adjusted_delta);
    normalized_delta(target - adjusted_delta)
}

pub(super) fn side_shape_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
    pair_geometry: PairGeometry,
) -> f32 {
    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);

    if pair_class.is_upper_lower()
        && metric_delta < -dead_zone()
        && pair_geometry.right_left_side.roundness > 0.040
        && nearest_delta <= nearest_guard
    {
        let spread = (config.gap_mad_em * 1.35).clamp(0.035, 0.14);
        let upper = config.target_gap_em + spread;
        if stats.robust_mean_gap > upper + 0.012 {
            let target = (metric_delta * 0.94).clamp(metric_delta, adjusted_delta);
            return normalized_delta(target - adjusted_delta);
        }
    }

    if !pair_class.has_digit() {
        return 0.0;
    }

    let safe_min = if pair_class.is_digit_digit() {
        (config.target_gap_em * 0.24).clamp(0.045, 0.075)
    } else {
        (config.target_gap_em * 0.32).clamp(0.060, 0.105)
    };
    if nearest_delta > nearest_guard || stats.min_gap <= safe_min {
        return 0.0;
    }

    let target = if pair_class.is_digit_digit() {
        digit_digit_target(pair_geometry)
    } else if pair_class.is_digit_punctuation() || pair_class.is_punctuation_digit() {
        digit_punctuation_target(pair_geometry)
    } else {
        0.0
    };

    if target >= adjusted_delta {
        return 0.0;
    }

    normalized_delta(target - adjusted_delta)
}

pub(super) fn collision_opening_delta(
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
) -> f32 {
    if !pair_class.allows_collision_opening() {
        return 0.0;
    }

    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);
    if stats.min_gap > 0.0 || nearest_delta <= nearest_guard {
        return 0.0;
    }

    let penetration = (-stats.min_gap).max(0.0);
    let target = (nearest_delta * 0.78 + penetration * 0.22).clamp(nearest_guard, 0.055);
    if target <= adjusted_delta {
        return 0.0;
    }

    normalized_delta(target - adjusted_delta)
}

pub(super) fn punctuation_spacing_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
) -> f32 {
    if !pair_class.is_upper_punctuation() || metric_delta >= -dead_zone() {
        return 0.0;
    }

    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);
    let safe_min = (config.target_gap_em * 0.65).clamp(0.12, 0.18);
    if nearest_delta > nearest_guard || stats.min_gap <= safe_min {
        return 0.0;
    }

    let base = (config.gap_mad_em * 0.46).clamp(0.018, 0.035);
    let gap_excess = (stats.robust_mean_gap - config.target_gap_em).max(0.0);
    let gap_bonus = (gap_excess * 0.12).clamp(0.0, 0.014);
    let target =
        (metric_delta.min(adjusted_delta) - base - gap_bonus).clamp(-0.120, adjusted_delta);

    normalized_delta(target - adjusted_delta)
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

fn lower_upper_overhang_delta(
    metric_delta: f32,
    optical_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
    pair_geometry: PairGeometry,
) -> f32 {
    if !pair_class.is_lower_upper()
        || metric_delta.abs() >= dead_zone()
        || optical_delta >= -dead_zone()
        || adjusted_delta <= -0.090
    {
        return 0.0;
    }

    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);
    let safe_min = (config.target_gap_em * 0.58).clamp(0.10, 0.18);
    if nearest_delta > nearest_guard || stats.min_gap <= safe_min || aperture_risk(stats, config) {
        return 0.0;
    }

    let overhang = pair_geometry.right_top_left_overhang;
    if overhang <= 0.10 {
        return 0.0;
    }

    let spread = (config.gap_mad_em * 1.35).clamp(0.035, 0.14);
    let upper = config.target_gap_em + spread;
    let gap_excess = (stats.robust_mean_gap - upper).max(0.0);
    let shape_bonus = ((overhang - 0.10) * 0.24).clamp(0.0, 0.040);
    let gap_bonus = (gap_excess * 0.40).clamp(0.0, 0.030);
    let round_bonus = if pair_geometry.left_right_side.is_round_like() && overhang > 0.18 {
        let curvature_bonus =
            ((pair_geometry.left_right_side.roundness - 0.030) * 0.70).clamp(0.0, 0.024);
        let overhang_bonus = ((overhang - 0.18) * 0.16).clamp(0.0, 0.020);
        (curvature_bonus + overhang_bonus).clamp(0.0, 0.034)
    } else {
        0.0
    };
    let lower_bound = if pair_geometry.left_right_side.is_round_like() && overhang > 0.18 {
        -0.120
    } else {
        -0.095
    };
    let target =
        (adjusted_delta - shape_bonus - gap_bonus - round_bonus).clamp(lower_bound, adjusted_delta);
    normalized_delta(target - adjusted_delta)
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

fn spacing_compaction_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
) -> f32 {
    if !pair_class.allows_safe_compaction() {
        return 0.0;
    }

    if adjusted_delta.abs() >= 0.045 {
        return 0.0;
    }

    if adjusted_delta < -dead_zone() && metric_delta.abs() < dead_zone() {
        return 0.0;
    }

    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);
    if nearest_delta > nearest_guard || aperture_risk(stats, config) {
        return 0.0;
    }

    let safe_min = (config.target_gap_em * 0.22).clamp(0.045, 0.065);
    if stats.min_gap <= safe_min {
        return 0.0;
    }

    let amount = (config.gap_mad_em * 0.25).clamp(0.008, 0.016);
    -amount
}
