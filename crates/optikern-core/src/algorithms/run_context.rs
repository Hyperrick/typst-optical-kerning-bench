use crate::class::PairClass;

use super::math::{dead_zone, normalized_delta};
use super::types::{Algorithm, AlgorithmSet, EvaluationConfig};

const CONNECTED_JOIN_GAP_EM: f32 = -0.020;
const CONNECTED_JOIN_OPENING_EM: f32 = 0.030;
const CONNECTED_JOIN_MIN_POSITIVE_SUM_EM: f32 = 0.080;
const SCRIPT_MIXED_MIN_PAIRS: usize = 2;
const SCRIPT_RESIDUAL_MIN_LOWER_PAIRS: usize = 2;
const SCRIPT_RESIDUAL_SEVERE_DELTA_EM: f32 = -0.080;
const SCRIPT_LOWER_RUN_MIN_PAIRS: usize = 5;
const SCRIPT_UPPER_RUN_MIN_PAIRS: usize = 5;
const SCRIPT_UPPER_RUN_MIN_OPENINGS: usize = 2;

pub(super) fn apply_run_context_adjustments(
    results: &mut [AlgorithmSet],
    config: EvaluationConfig,
) {
    let context = RunContext::from_results(results, config);
    if !context.has_adjustments() {
        return;
    }

    for result in results.iter_mut() {
        let pair_class = PairClass::from_chars(result.left, result.right);
        let Some(output) = result
            .outputs
            .iter_mut()
            .find(|output| output.algorithm == Algorithm::GuardedProfileHybrid)
        else {
            continue;
        };
        let mut delta = output.delta_em;
        delta = normalized_delta(
            delta + connected_script_delta(delta, output.gap_min_em, pair_class, context, config),
        );
        delta = normalized_delta(
            delta
                + script_mixed_case_delta(
                    delta,
                    output.gap_min_em,
                    output.metric_delta_em,
                    output.optical_delta_em,
                    pair_class,
                    context,
                    config,
                ),
        );
        delta = normalized_delta(
            delta + sans_run_context_delta(delta, output.metric_delta_em, pair_class, context),
        );
        delta = normalized_delta(
            delta
                + script_lower_run_delta(
                    delta,
                    output.metric_delta_em,
                    output.optical_delta_em,
                    output.gap_min_em,
                    pair_class,
                    context,
                    config,
                ),
        );
        delta = normalized_delta(
            delta
                + script_upper_run_delta(
                    delta,
                    output.metric_delta_em,
                    output.gap_min_em,
                    pair_class,
                    context,
                    config,
                ),
        );
        output.delta_em = delta;
    }
    apply_script_residual_balancer(results, context, config);
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct RunContext {
    pub(super) sans_like: bool,
    pub(super) connected_script_like: bool,
    pub(super) letter_pairs: usize,
    pub(super) connected_letter_pairs: usize,
    pub(super) opened_connected_letter_pairs: usize,
    pub(super) positive_connected_opening_em: f32,
    pub(super) script_mixed_case_like: bool,
    pub(super) script_lower_run_like: bool,
    pub(super) script_upper_run_like: bool,
    pub(super) mixed_case_pairs: usize,
    pub(super) upper_pairs: usize,
    pub(super) metricless_upper_pairs: usize,
    pub(super) connected_upper_pairs: usize,
    pub(super) opened_connected_upper_pairs: usize,
    pub(super) strong_upper_metric_pairs: usize,
    pub(super) strong_mixed_metric_pairs: usize,
    pub(super) lower_pairs: usize,
    pub(super) metricless_lower_pairs: usize,
    pub(super) connected_lower_pairs: usize,
    pub(super) opening_lower_pairs: usize,
}

impl RunContext {
    fn from_results(results: &[AlgorithmSet], config: EvaluationConfig) -> Self {
        let mut context = Self {
            sans_like: sans_like_spacing_profile(config),
            ..Self::default()
        };

        for result in results {
            let class = PairClass::from_chars(result.left, result.right);
            let Some(output) = result
                .outputs
                .iter()
                .find(|output| output.algorithm == Algorithm::GuardedProfileHybrid)
            else {
                continue;
            };

            if is_lower_involved_letter_pair(class) {
                context.letter_pairs += 1;
                if class.is_upper_lower() || class.is_lower_upper() {
                    context.mixed_case_pairs += 1;
                }
                if class.is_lower_lower() {
                    context.lower_pairs += 1;
                    if output.metric_delta_em.abs() < dead_zone() {
                        context.metricless_lower_pairs += 1;
                    }
                    if output.gap_min_em < CONNECTED_JOIN_GAP_EM {
                        context.connected_lower_pairs += 1;
                    }
                    if output.optical_delta_em > dead_zone() {
                        context.opening_lower_pairs += 1;
                    }
                }
                if output.gap_min_em < CONNECTED_JOIN_GAP_EM {
                    context.connected_letter_pairs += 1;
                    if output.delta_em > CONNECTED_JOIN_OPENING_EM {
                        context.opened_connected_letter_pairs += 1;
                        context.positive_connected_opening_em += output.delta_em;
                    }
                }
            }

            if class.is_upper_upper() {
                context.upper_pairs += 1;
                if output.metric_delta_em.abs() < dead_zone() {
                    context.metricless_upper_pairs += 1;
                }
                if output.gap_min_em < CONNECTED_JOIN_GAP_EM {
                    context.connected_upper_pairs += 1;
                    if output.delta_em > CONNECTED_JOIN_OPENING_EM {
                        context.opened_connected_upper_pairs += 1;
                    }
                }
            }

            if context.sans_like {
                if class.is_upper_upper() && output.metric_delta_em < -0.050 {
                    context.strong_upper_metric_pairs += 1;
                }
                if (class.is_upper_lower() || class.is_lower_upper())
                    && output.metric_delta_em < -0.050
                {
                    context.strong_mixed_metric_pairs += 1;
                }
            }
        }

        context.connected_script_like = context.letter_pairs >= 3
            && context.connected_letter_pairs >= 2
            && context.opened_connected_letter_pairs >= 2
            && context.positive_connected_opening_em >= CONNECTED_JOIN_MIN_POSITIVE_SUM_EM;
        context.script_mixed_case_like = context.mixed_case_pairs >= SCRIPT_MIXED_MIN_PAIRS
            && !context.sans_like
            && script_spacing_profile(config);
        context.script_lower_run_like = context.lower_pairs >= SCRIPT_LOWER_RUN_MIN_PAIRS
            && context.metricless_lower_pairs == context.lower_pairs
            && context.connected_lower_pairs >= context.lower_pairs.saturating_sub(1)
            && context.opening_lower_pairs == 0
            && !context.sans_like
            && script_spacing_profile(config);
        context.script_upper_run_like = context.upper_pairs >= SCRIPT_UPPER_RUN_MIN_PAIRS
            && context.metricless_upper_pairs == context.upper_pairs
            && context.connected_upper_pairs >= SCRIPT_UPPER_RUN_MIN_OPENINGS
            && context.opened_connected_upper_pairs >= SCRIPT_UPPER_RUN_MIN_OPENINGS
            && !context.sans_like
            && script_spacing_profile(config);

        context
    }

    fn has_adjustments(self) -> bool {
        self.connected_script_like
            || self.script_mixed_case_like
            || self.script_lower_run_like
            || self.script_upper_run_like
            || (self.sans_like
                && (self.strong_upper_metric_pairs >= 2 || self.strong_mixed_metric_pairs >= 2))
    }
}

fn apply_script_residual_balancer(
    results: &mut [AlgorithmSet],
    context: RunContext,
    config: EvaluationConfig,
) {
    let balance = ScriptResidualBalance::from_results(results, context, config);
    if !balance.should_apply(config) {
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
                + script_residual_balance_delta(
                    output.delta_em,
                    output.metric_delta_em,
                    output.optical_delta_em,
                    output.gap_min_em,
                    pair_class,
                    balance,
                    config,
                ),
        );
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct ScriptResidualBalance {
    pub(super) severe_metricless_mixed_pairs: usize,
    pub(super) metricless_excess_tightening_em: f32,
}

impl ScriptResidualBalance {
    fn from_results(
        results: &[AlgorithmSet],
        context: RunContext,
        config: EvaluationConfig,
    ) -> Self {
        if !context.script_mixed_case_like
            || context.lower_pairs < SCRIPT_RESIDUAL_MIN_LOWER_PAIRS
            || !script_spacing_profile(config)
        {
            return Self::default();
        }

        let mut balance = Self::default();
        for result in results {
            let pair_class = PairClass::from_chars(result.left, result.right);
            let Some(output) = result
                .outputs
                .iter()
                .find(|output| output.algorithm == Algorithm::GuardedProfileHybrid)
            else {
                continue;
            };

            if !is_lower_involved_letter_pair(pair_class) || output.metric_delta_em < -dead_zone() {
                continue;
            }

            let excess = output.optical_delta_em - output.delta_em;
            if excess > dead_zone() {
                balance.metricless_excess_tightening_em += excess;
            }
            if (pair_class.is_upper_lower() || pair_class.is_lower_upper())
                && output.delta_em <= SCRIPT_RESIDUAL_SEVERE_DELTA_EM
            {
                balance.severe_metricless_mixed_pairs += 1;
            }
        }

        balance
    }

    fn should_apply(self, config: EvaluationConfig) -> bool {
        self.severe_metricless_mixed_pairs > 0
            && self.metricless_excess_tightening_em >= script_residual_min_excess(config)
    }
}

pub(super) fn script_residual_balance_delta(
    adjusted_delta: f32,
    metric_delta: f32,
    optical_delta: f32,
    gap_min_em: f32,
    pair_class: PairClass,
    balance: ScriptResidualBalance,
    config: EvaluationConfig,
) -> f32 {
    if !balance.should_apply(config)
        || !is_lower_involved_letter_pair(pair_class)
        || metric_delta < -dead_zone()
    {
        return 0.0;
    }

    if pair_class.is_upper_lower() || pair_class.is_lower_upper() {
        if adjusted_delta < -dead_zone() && adjusted_delta > SCRIPT_RESIDUAL_SEVERE_DELTA_EM {
            let target = (adjusted_delta - script_residual_soft_mixed_amount(config))
                .max(-script_residual_soft_mixed_bound(config));
            return normalized_delta(target - adjusted_delta);
        }
    }

    if pair_class.is_lower_lower()
        && optical_delta > dead_zone()
        && gap_min_em < CONNECTED_JOIN_GAP_EM
    {
        let target = -script_residual_lower_compaction_amount(config);
        if adjusted_delta > target {
            return normalized_delta(target - adjusted_delta);
        }
    }

    0.0
}

pub(super) fn connected_script_delta(
    adjusted_delta: f32,
    gap_min_em: f32,
    pair_class: PairClass,
    context: RunContext,
    config: EvaluationConfig,
) -> f32 {
    if !context.connected_script_like
        || !is_lower_involved_letter_pair(pair_class)
        || gap_min_em >= CONNECTED_JOIN_GAP_EM
        || adjusted_delta <= 0.0
    {
        return 0.0;
    }

    let cap = connected_script_opening_cap(config);
    normalized_delta(adjusted_delta.min(cap) - adjusted_delta)
}

pub(super) fn script_mixed_case_delta(
    adjusted_delta: f32,
    gap_min_em: f32,
    metric_delta: f32,
    optical_delta: f32,
    pair_class: PairClass,
    context: RunContext,
    config: EvaluationConfig,
) -> f32 {
    if !context.script_mixed_case_like || !is_lower_involved_letter_pair(pair_class) {
        return 0.0;
    }

    if adjusted_delta > 0.0 && gap_min_em < CONNECTED_JOIN_GAP_EM {
        return normalized_delta(
            adjusted_delta.min(script_mixed_opening_cap(config)) - adjusted_delta,
        );
    }

    if !(pair_class.is_upper_lower() || pair_class.is_lower_upper()) {
        return 0.0;
    }

    let target = if metric_delta < -dead_zone() {
        if optical_delta > metric_delta {
            metric_delta
        } else {
            (metric_delta - 0.014).max(-0.125)
        }
    } else {
        -script_mixed_tightening_amount(config)
    };
    if target < adjusted_delta {
        normalized_delta(target - adjusted_delta)
    } else {
        0.0
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

pub(super) fn script_lower_run_delta(
    adjusted_delta: f32,
    metric_delta: f32,
    optical_delta: f32,
    gap_min_em: f32,
    pair_class: PairClass,
    context: RunContext,
    config: EvaluationConfig,
) -> f32 {
    if !context.script_lower_run_like
        || !pair_class.is_lower_lower()
        || metric_delta.abs() >= dead_zone()
        || optical_delta > dead_zone()
        || gap_min_em >= CONNECTED_JOIN_GAP_EM
    {
        return 0.0;
    }

    let target = -script_lower_run_compaction_amount(config);
    if adjusted_delta > target {
        normalized_delta(target - adjusted_delta)
    } else {
        0.0
    }
}

pub(super) fn script_upper_run_delta(
    adjusted_delta: f32,
    metric_delta: f32,
    gap_min_em: f32,
    pair_class: PairClass,
    context: RunContext,
    config: EvaluationConfig,
) -> f32 {
    if !context.script_upper_run_like
        || !pair_class.is_upper_upper()
        || metric_delta.abs() >= dead_zone()
    {
        return 0.0;
    }

    if gap_min_em < CONNECTED_JOIN_GAP_EM && adjusted_delta > CONNECTED_JOIN_OPENING_EM {
        let target = script_upper_run_opening_cap(config);
        return if target < adjusted_delta {
            normalized_delta(target - adjusted_delta)
        } else {
            0.0
        };
    }

    if adjusted_delta.abs() < dead_zone()
        && gap_min_em >= 0.0
        && gap_min_em <= script_upper_run_near_gap_limit(config)
    {
        return script_upper_run_near_gap_opening(config);
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

fn connected_script_opening_cap(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.035).clamp(0.004, 0.010)
}

fn script_mixed_opening_cap(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.025).clamp(0.003, 0.008)
}

fn script_mixed_tightening_amount(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.24).clamp(0.035, 0.055)
}

fn script_residual_min_excess(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.42).clamp(0.055, 0.085)
}

fn script_residual_soft_mixed_amount(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.08).clamp(0.010, 0.016)
}

fn script_residual_soft_mixed_bound(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.38).clamp(0.052, 0.070)
}

fn script_residual_lower_compaction_amount(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.085).clamp(0.010, 0.018)
}

fn script_lower_run_compaction_amount(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.065).clamp(0.008, 0.012)
}

fn script_upper_run_opening_cap(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.12).clamp(0.018, 0.026)
}

fn script_upper_run_near_gap_limit(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.14).clamp(0.018, 0.032)
}

fn script_upper_run_near_gap_opening(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.11).clamp(0.016, 0.022)
}

fn script_spacing_profile(config: EvaluationConfig) -> bool {
    config.target_gap_em <= 0.205 && config.profile.x_height / config.profile.cap_height < 0.72
}

fn is_lower_involved_letter_pair(pair_class: PairClass) -> bool {
    pair_class.is_upper_lower() || pair_class.is_lower_upper() || pair_class.is_lower_lower()
}
