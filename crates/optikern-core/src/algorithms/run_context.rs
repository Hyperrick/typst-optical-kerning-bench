use crate::class::PairClass;

use super::math::{dead_zone, normalized_delta};
use super::types::{Algorithm, AlgorithmSet, EvaluationConfig};

pub(super) fn apply_run_context_adjustments(
    results: &mut [AlgorithmSet],
    config: EvaluationConfig,
) {
    let context = RunContext::from_results(results, config);
    if !context.has_adjustments() {
        return;
    }

    for result in results {
        let pair_class = PairClass::from_chars(result.left, result.right);
        let Some(output) = result
            .outputs
            .iter_mut()
            .find(|output| output.algorithm == Algorithm::GuardedProfileHybrid)
        else {
            continue;
        };
        output.delta_em = normalized_delta(
            output.delta_em
                + sans_run_context_delta(
                    output.delta_em,
                    output.metric_delta_em,
                    pair_class,
                    context,
                ),
        );
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct RunContext {
    pub(super) sans_like: bool,
    pub(super) strong_upper_metric_pairs: usize,
    pub(super) strong_mixed_metric_pairs: usize,
    pub(super) lower_pairs: usize,
}

impl RunContext {
    fn from_results(results: &[AlgorithmSet], config: EvaluationConfig) -> Self {
        let mut context = Self {
            sans_like: sans_like_spacing_profile(config),
            ..Self::default()
        };
        if !context.sans_like {
            return context;
        }

        for result in results {
            let class = PairClass::from_chars(result.left, result.right);
            let Some(output) = result
                .outputs
                .iter()
                .find(|output| output.algorithm == Algorithm::GuardedProfileHybrid)
            else {
                continue;
            };
            if class.is_upper_upper() && output.metric_delta_em < -0.050 {
                context.strong_upper_metric_pairs += 1;
            }
            if (class.is_upper_lower() || class.is_lower_upper()) && output.metric_delta_em < -0.050
            {
                context.strong_mixed_metric_pairs += 1;
            }
            if class.is_lower_lower() {
                context.lower_pairs += 1;
            }
        }

        context
    }

    fn has_adjustments(self) -> bool {
        self.sans_like
            && (self.strong_upper_metric_pairs >= 2 || self.strong_mixed_metric_pairs >= 2)
    }
}

pub(super) fn sans_run_context_delta(
    adjusted_delta: f32,
    metric_delta: f32,
    pair_class: PairClass,
    context: RunContext,
) -> f32 {
    if !context.sans_like {
        return 0.0;
    }

    if pair_class.is_upper_upper()
        && metric_delta < -0.050
        && context.strong_upper_metric_pairs >= 2
    {
        let amount = if context.strong_upper_metric_pairs >= 4 {
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
        return clamp_tightening(adjusted_delta, 0.024, -0.130);
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

fn clamp_tightening(adjusted_delta: f32, amount: f32, lower_bound: f32) -> f32 {
    let target = (adjusted_delta - amount).max(lower_bound);
    normalized_delta(target - adjusted_delta)
}

pub(super) fn sans_like_spacing_profile(config: EvaluationConfig) -> bool {
    config.target_gap_em <= 0.235 && config.profile.x_height / config.profile.cap_height >= 0.72
}
