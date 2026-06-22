use crate::class::PairClass;

use super::math::{dead_zone, normalized_delta};
use super::run_context::RunContext;
use super::types::EvaluationConfig;

pub(super) fn serif_ligature_lower_run_delta(
    adjusted_delta: f32,
    metric_delta: f32,
    gap_min_em: f32,
    gap_robust_mean_em: f32,
    pair_class: PairClass,
    right_cluster_chars: usize,
    context: RunContext,
    config: EvaluationConfig,
) -> f32 {
    if let Some(delta) = short_serif_ligature_compaction_relief(
        adjusted_delta,
        metric_delta,
        gap_robust_mean_em,
        pair_class,
        context,
        config,
    ) {
        return delta;
    }

    if let Some(delta) = wide_serif_metric_opening_delta(
        adjusted_delta,
        metric_delta,
        gap_min_em,
        pair_class,
        context,
        config,
    ) {
        return delta;
    }

    if !context.serif_ligature_lower_run_like
        || !pair_class.is_lower_lower()
        || metric_delta.abs() >= dead_zone()
        || right_cluster_chars > 1
    {
        return 0.0;
    }

    let safe_min = (config.target_gap_em * 0.34).clamp(0.075, 0.110);
    if gap_min_em <= safe_min {
        return 0.0;
    }

    let target = -serif_ligature_lower_run_amount(config);
    if target < adjusted_delta {
        normalized_delta(target - adjusted_delta)
    } else {
        0.0
    }
}

fn short_serif_ligature_compaction_relief(
    adjusted_delta: f32,
    metric_delta: f32,
    gap_robust_mean_em: f32,
    pair_class: PairClass,
    context: RunContext,
    config: EvaluationConfig,
) -> Option<f32> {
    if !context.short_serif_ligature_lower_run_like
        || !pair_class.is_lower_lower()
        || metric_delta.abs() >= dead_zone()
        || adjusted_delta >= -dead_zone()
    {
        return None;
    }

    let compact_gap = config.target_gap_em * 0.74;
    (gap_robust_mean_em <= compact_gap).then_some(normalized_delta(-adjusted_delta))
}

fn serif_ligature_lower_run_amount(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.098).clamp(0.024, 0.031)
}

fn wide_serif_metric_opening_delta(
    adjusted_delta: f32,
    metric_delta: f32,
    gap_min_em: f32,
    pair_class: PairClass,
    context: RunContext,
    config: EvaluationConfig,
) -> Option<f32> {
    if !context.wide_serif_lower_run_like
        || !pair_class.is_lower_lower()
        || metric_delta <= dead_zone()
        || adjusted_delta <= 0.0
    {
        return None;
    }

    let near_touch_limit = (config.target_gap_em * 0.060).clamp(0.010, 0.018);
    if !(0.0..=near_touch_limit).contains(&gap_min_em) {
        return None;
    }

    Some(normalized_delta(-adjusted_delta))
}

pub(super) fn wide_serif_spacing_profile(config: EvaluationConfig) -> bool {
    config.target_gap_em >= 0.240 && config.profile.x_height / config.profile.cap_height < 0.72
}
