use crate::calibration::ClassGapCalibration;
use crate::class::PairClass;
use crate::profile::{GapStats, ProfileConfig};

use super::math::{dead_zone, normalized_delta};
use super::types::EvaluationConfig;

pub(super) fn nearest_distance_delta(stats: GapStats, target_gap: f32) -> f32 {
    let desired_min = (target_gap * 0.38).clamp(0.018, 0.040);
    if stats.min_gap < desired_min {
        normalized_delta(desired_min - stats.min_gap)
    } else {
        let config = EvaluationConfig {
            profile: ProfileConfig::default(),
            target_gap_em: target_gap,
            gap_mad_em: (target_gap * 0.32).clamp(0.025, 0.08),
            preserve_monospace: false,
            class_gap_calibration: ClassGapCalibration::empty(),
        };
        distribution_delta(stats.min_gap, config) * 0.5
    }
}

pub(super) fn distribution_delta(gap: f32, config: EvaluationConfig) -> f32 {
    let spread = (config.gap_mad_em * 1.35).clamp(0.035, 0.14);
    let lower = (config.target_gap_em - spread * 1.15).max(0.020);
    let upper = config.target_gap_em + spread;
    if gap > upper {
        normalized_delta((upper - gap) * 0.85)
    } else if gap < lower {
        normalized_delta((lower - gap) * 0.65)
    } else {
        0.0
    }
}

pub(super) fn metric_prior_hybrid_for_class(
    metric_delta: f32,
    optical_delta: f32,
    pair_class: PairClass,
) -> f32 {
    if metric_delta.abs() < dead_zone() {
        return zero_metric_delta(optical_delta, pair_class);
    }

    let disagreement = (optical_delta - metric_delta).abs();
    if disagreement <= 0.045 {
        metric_delta
    } else {
        let pull = metric_optical_pull(metric_delta, pair_class);
        normalized_delta(metric_delta + pull * (optical_delta - metric_delta))
    }
}

fn metric_optical_pull(metric_delta: f32, pair_class: PairClass) -> f32 {
    if metric_delta < -dead_zone() {
        if pair_class.is_upper_lower()
            || pair_class.is_upper_punctuation()
            || pair_class.is_upper_upper()
        {
            return 0.25;
        }
    }
    0.80
}

fn zero_metric_delta(optical_delta: f32, pair_class: PairClass) -> f32 {
    if pair_class.is_upper_digit() {
        return normalized_delta(optical_delta.clamp(-0.055, 0.030));
    }

    if pair_class.is_upper_upper() {
        return normalized_delta(optical_delta.clamp(-0.070, 0.030));
    }

    if pair_class.is_digit_punctuation() || pair_class.is_punctuation_digit() {
        return normalized_delta(optical_delta.clamp(-0.035, 0.035));
    }

    if pair_class.is_digit_digit() {
        return normalized_delta(optical_delta.min(0.0).max(-0.055));
    }

    if pair_class.is_lower_upper() {
        return normalized_delta(optical_delta.clamp(-0.060, 0.030));
    }

    optical_delta
}
