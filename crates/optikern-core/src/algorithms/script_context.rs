use crate::class::PairClass;

use super::math::{dead_zone, normalized_delta};
use super::run_context::RunContext;
use super::types::EvaluationConfig;

pub(super) const CONNECTED_JOIN_GAP_EM: f32 = -0.020;
pub(super) const CONNECTED_JOIN_OPENING_EM: f32 = 0.030;
pub(super) const CONNECTED_JOIN_MIN_POSITIVE_SUM_EM: f32 = 0.080;
pub(super) const SCRIPT_MIXED_MIN_PAIRS: usize = 2;
pub(super) const SCRIPT_LIGATURE_RUN_MIN_PAIRS: usize = 3;
pub(super) const SCRIPT_RESIDUAL_MIN_LOWER_PAIRS: usize = 2;
pub(super) const SCRIPT_RESIDUAL_SEVERE_DELTA_EM: f32 = -0.080;
pub(super) const SCRIPT_LOWER_RUN_MIN_PAIRS: usize = 4;
pub(super) const SCRIPT_UPPER_RUN_MIN_PAIRS: usize = 5;
pub(super) const SCRIPT_UPPER_RUN_MIN_OPENINGS: usize = 2;

pub(super) fn apply_script_residual_balancer(
    results: &mut [super::types::AlgorithmSet],
    context: RunContext,
    config: EvaluationConfig,
) {
    let balance = ScriptResidualBalance::from_results(results, context, config);
    if !balance.should_apply(config) {
        return;
    }

    for result in results {
        let pair_class = result.pair_class();
        let Some(output) = result
            .outputs
            .iter_mut()
            .find(|output| output.algorithm == super::Algorithm::GuardedProfileHybrid)
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
        results: &[super::types::AlgorithmSet],
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
            let pair_class = result.pair_class();
            let Some(output) = result
                .outputs
                .iter()
                .find(|output| output.algorithm == super::Algorithm::GuardedProfileHybrid)
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

    if (pair_class.is_upper_lower() || pair_class.is_lower_upper())
        && adjusted_delta < -dead_zone()
        && adjusted_delta > SCRIPT_RESIDUAL_SEVERE_DELTA_EM
    {
        let target = (adjusted_delta - script_residual_soft_mixed_amount(config))
            .max(-script_residual_soft_mixed_bound(config));
        return normalized_delta(target - adjusted_delta);
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

pub(super) fn script_ligature_run_delta(
    adjusted_delta: f32,
    metric_delta: f32,
    gap_min_em: f32,
    pair_class: PairClass,
    context: RunContext,
    config: EvaluationConfig,
) -> f32 {
    if !context.script_ligature_run_like
        || !is_lower_involved_letter_pair(pair_class)
        || gap_min_em >= CONNECTED_JOIN_GAP_EM
        || metric_delta < -script_ligature_metric_floor(config)
    {
        return 0.0;
    }

    let target = script_ligature_run_opening_amount(config, context);
    if adjusted_delta < target {
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

fn script_ligature_metric_floor(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.14).clamp(0.020, 0.028)
}

fn script_ligature_run_opening_amount(config: EvaluationConfig, context: RunContext) -> f32 {
    let connected_ratio = if context.letter_pairs == 0 {
        1.0
    } else {
        context.connected_letter_pairs as f32 / context.letter_pairs as f32
    };
    let base = (config.target_gap_em * 0.150 * connected_ratio).clamp(0.018, 0.027);

    if context.letter_pairs <= 3
        && context.max_cluster_chars >= 3
        && context.optical_opening_letter_pairs == 0
    {
        return base.max((config.target_gap_em * 0.240).clamp(0.034, 0.040));
    }

    if context.letter_pairs >= 6
        && context.metric_tightened_letter_pairs > 0
        && context.optical_opening_letter_pairs * 2 >= context.letter_pairs
    {
        return base.min((config.target_gap_em * 0.090).clamp(0.013, 0.018));
    }

    if context.letter_pairs >= 6
        && context.metric_tightened_letter_pairs == 0
        && context.optical_opening_letter_pairs == 0
        && context.connected_letter_pairs == context.letter_pairs
    {
        return base.min((config.target_gap_em * 0.115).clamp(0.016, 0.020));
    }

    base
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

pub(super) fn script_spacing_profile(config: EvaluationConfig) -> bool {
    config.target_gap_em <= 0.205 && config.profile.x_height / config.profile.cap_height < 0.72
}

fn is_lower_involved_letter_pair(pair_class: PairClass) -> bool {
    pair_class.is_upper_lower() || pair_class.is_lower_upper() || pair_class.is_lower_lower()
}
