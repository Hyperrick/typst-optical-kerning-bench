use crate::class::PairClass;

use super::math::{dead_zone, normalized_delta};
use super::run_context::RunContext;
use super::types::EvaluationConfig;

pub(super) fn sans_run_context_delta(
    adjusted_delta: f32,
    metric_delta: f32,
    pair_class: PairClass,
    context: RunContext,
    config: EvaluationConfig,
) -> f32 {
    if !context.sans_like {
        return 0.0;
    }

    if pair_class.is_upper_upper()
        && metric_delta < -0.050
        && context.strong_upper_metric_pairs >= 2
    {
        let amount = if context.upper_pairs >= 4 && context.strong_upper_metric_pairs >= 4 {
            0.036
        } else if context.upper_pairs >= 4 && context.strong_upper_metric_pairs >= 3 {
            0.024
        } else if context.strong_upper_metric_pairs >= 4 {
            0.026
        } else {
            0.012
        };
        return clamp_tightening(adjusted_delta, amount, -0.125);
    }

    if (pair_class.is_upper_lower() || pair_class.is_lower_upper())
        && metric_delta < -0.050
        && context.strong_mixed_metric_pairs >= 2
    {
        if compact_sans_spacing_profile(config) {
            return 0.0;
        }
        return clamp_tightening(adjusted_delta, 0.024, -0.130);
    }

    if compact_sans_spacing_profile(config)
        && context.mixed_case_pairs >= 2
        && context.lower_pairs >= 3
    {
        if pair_class.is_lower_lower() && adjusted_delta > -0.018 {
            return clamp_tightening(adjusted_delta, 0.010, -0.018);
        }
        if pair_class.is_upper_lower()
            && metric_delta.abs() < dead_zone()
            && adjusted_delta > -0.074
        {
            return clamp_tightening(adjusted_delta, 0.010, -0.074);
        }
    }

    if context.strong_mixed_metric_pairs >= 2 {
        if pair_class.is_lower_lower() && adjusted_delta > -0.040 {
            return clamp_tightening(adjusted_delta, 0.012, -0.040);
        }
        if pair_class.is_upper_lower()
            && metric_delta.abs() < dead_zone()
            && adjusted_delta > -0.045
        {
            return clamp_tightening(adjusted_delta, 0.010, -0.045);
        }
    }

    0.0
}

pub(super) fn sans_like_spacing_profile(config: EvaluationConfig) -> bool {
    config.target_gap_em <= 0.235 && config.profile.x_height / config.profile.cap_height >= 0.72
}

pub(super) fn compact_sans_spacing_profile(config: EvaluationConfig) -> bool {
    sans_like_spacing_profile(config) && config.target_gap_em <= 0.210
}

fn clamp_tightening(adjusted_delta: f32, amount: f32, lower_bound: f32) -> f32 {
    let target = (adjusted_delta - amount).max(lower_bound);
    normalized_delta(target - adjusted_delta)
}
