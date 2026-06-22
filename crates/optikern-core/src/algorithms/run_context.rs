use crate::class::PairClass;

use super::capital_context::{CapitalRunContext, serif_cap_run_delta};
use super::digit_context::{DigitRunContext, digit_run_context_delta};
use super::math::{dead_zone, normalized_delta};
use super::sans_context::{
    compact_sans_spacing_profile, sans_like_spacing_profile, sans_run_context_delta,
};
use super::script_context::{
    CONNECTED_JOIN_GAP_EM, CONNECTED_JOIN_MIN_POSITIVE_SUM_EM, CONNECTED_JOIN_OPENING_EM,
    SCRIPT_LIGATURE_RUN_MIN_PAIRS, SCRIPT_LOWER_RUN_MIN_PAIRS, SCRIPT_MIXED_MIN_PAIRS,
    SCRIPT_UPPER_RUN_MIN_OPENINGS, SCRIPT_UPPER_RUN_MIN_PAIRS, apply_script_residual_balancer,
    connected_script_delta, script_ligature_run_delta, script_lower_run_delta,
    script_mixed_case_delta, script_spacing_profile, script_upper_run_delta,
};
use super::serif_context::{serif_ligature_lower_run_delta, wide_serif_spacing_profile};
use super::types::{Algorithm, AlgorithmSet, EvaluationConfig};

pub(super) fn apply_run_context_adjustments(
    results: &mut [AlgorithmSet],
    config: EvaluationConfig,
) {
    let context = RunContext::from_results(results, config);
    if !context.has_adjustments(config) {
        return;
    }

    for result in results.iter_mut() {
        let pair_class = result.pair_class();
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
            delta
                + sans_run_context_delta(
                    delta,
                    output.metric_delta_em,
                    pair_class,
                    context,
                    config,
                ),
        );
        delta = normalized_delta(
            delta
                + digit_run_context_delta(
                    delta,
                    output.metric_delta_em,
                    pair_class,
                    context.digit_run,
                    context.sans_like,
                    config,
                ),
        );
        delta = normalized_delta(
            delta
                + serif_cap_run_delta(
                    delta,
                    output.metric_delta_em,
                    pair_class,
                    context.capital_run,
                    context.sans_like,
                    config,
                ),
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
                + serif_ligature_lower_run_delta(
                    delta,
                    output.metric_delta_em,
                    output.gap_min_em,
                    pair_class,
                    result.right_cluster.chars().count(),
                    context,
                    config,
                ),
        );
        delta = normalized_delta(
            delta
                + script_ligature_run_delta(
                    delta,
                    output.metric_delta_em,
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
    pub(super) script_ligature_run_like: bool,
    pub(super) script_lower_run_like: bool,
    pub(super) script_upper_run_like: bool,
    pub(super) sans_lower_run_like: bool,
    pub(super) wide_serif_lower_run_like: bool,
    pub(super) serif_ligature_lower_run_like: bool,
    pub(super) mixed_case_pairs: usize,
    pub(super) upper_pairs: usize,
    pub(super) metricless_upper_pairs: usize,
    pub(super) connected_upper_pairs: usize,
    pub(super) opened_connected_upper_pairs: usize,
    pub(super) strong_upper_metric_pairs: usize,
    pub(super) strong_mixed_metric_pairs: usize,
    pub(super) lower_pairs: usize,
    pub(super) multi_char_letter_pairs: usize,
    pub(super) connected_multi_char_letter_pairs: usize,
    pub(super) optical_opening_letter_pairs: usize,
    pub(super) optical_tightening_lower_pairs: usize,
    pub(super) metric_tightened_letter_pairs: usize,
    pub(super) max_cluster_chars: usize,
    pub(super) metricless_lower_pairs: usize,
    pub(super) connected_lower_pairs: usize,
    pub(super) opening_lower_pairs: usize,
    pub(super) digit_run: DigitRunContext,
    pub(super) capital_run: CapitalRunContext,
}

impl RunContext {
    fn from_results(results: &[AlgorithmSet], config: EvaluationConfig) -> Self {
        let mut context = Self {
            sans_like: sans_like_spacing_profile(config),
            ..Self::default()
        };

        for result in results {
            let class = result.pair_class();
            let Some(output) = result
                .outputs
                .iter()
                .find(|output| output.algorithm == Algorithm::GuardedProfileHybrid)
            else {
                continue;
            };

            if is_lower_involved_letter_pair(class) {
                context.letter_pairs += 1;
                context.max_cluster_chars = context.max_cluster_chars.max(
                    result
                        .left_cluster
                        .chars()
                        .count()
                        .max(result.right_cluster.chars().count()),
                );
                if output.optical_delta_em > dead_zone() {
                    context.optical_opening_letter_pairs += 1;
                }
                if output.metric_delta_em < -dead_zone() {
                    context.metric_tightened_letter_pairs += 1;
                }
                if class.is_upper_lower() || class.is_lower_upper() {
                    context.mixed_case_pairs += 1;
                }
                if result.has_multi_char_cluster() {
                    context.multi_char_letter_pairs += 1;
                    if output.gap_min_em < CONNECTED_JOIN_GAP_EM {
                        context.connected_multi_char_letter_pairs += 1;
                    }
                }
                if class.is_lower_lower() {
                    context.lower_pairs += 1;
                    if output.metric_delta_em.abs() < dead_zone() {
                        context.metricless_lower_pairs += 1;
                    }
                    if output.optical_delta_em < -dead_zone() {
                        context.optical_tightening_lower_pairs += 1;
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

            context
                .digit_run
                .record(class, output.metric_delta_em, output.gap_min_em, config);
            context.capital_run.record(class, output.metric_delta_em);
        }

        context.connected_script_like = context.letter_pairs >= 3
            && context.connected_letter_pairs >= 2
            && context.opened_connected_letter_pairs >= 2
            && context.positive_connected_opening_em >= CONNECTED_JOIN_MIN_POSITIVE_SUM_EM;
        context.script_mixed_case_like = context.mixed_case_pairs >= SCRIPT_MIXED_MIN_PAIRS
            && !context.sans_like
            && script_spacing_profile(config);
        context.script_ligature_run_like = context.letter_pairs >= SCRIPT_LIGATURE_RUN_MIN_PAIRS
            && context.multi_char_letter_pairs > 0
            && context.connected_letter_pairs >= context.letter_pairs.saturating_sub(1)
            && context.connected_multi_char_letter_pairs > 0
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
        context.sans_lower_run_like = context.sans_like
            && context.lower_pairs >= 5
            && context.lower_pairs == context.letter_pairs
            && context.metricless_lower_pairs >= context.lower_pairs.saturating_sub(2);
        context.wide_serif_lower_run_like = !context.sans_like
            && wide_serif_spacing_profile(config)
            && context.lower_pairs >= 4
            && context.connected_lower_pairs == 0;
        context.serif_ligature_lower_run_like = !context.sans_like
            && wide_serif_spacing_profile(config)
            && context.lower_pairs >= 6
            && context.multi_char_letter_pairs > 0
            && context.metricless_lower_pairs >= context.lower_pairs.saturating_sub(1)
            && context.connected_lower_pairs == 0;

        context
    }

    fn has_adjustments(self, config: EvaluationConfig) -> bool {
        self.connected_script_like
            || self.script_mixed_case_like
            || self.script_ligature_run_like
            || self.script_lower_run_like
            || self.script_upper_run_like
            || self.sans_lower_run_like
            || self.wide_serif_lower_run_like
            || self.serif_ligature_lower_run_like
            || self.digit_run.has_adjustments(self.sans_like, config)
            || self.capital_run.has_adjustments(self.sans_like, config)
            || (compact_sans_spacing_profile(config)
                && self.mixed_case_pairs >= 2
                && self.lower_pairs >= 3)
            || (self.sans_like
                && (self.strong_upper_metric_pairs >= 2 || self.strong_mixed_metric_pairs >= 2))
    }
}

fn is_lower_involved_letter_pair(pair_class: PairClass) -> bool {
    pair_class.is_upper_lower() || pair_class.is_lower_upper() || pair_class.is_lower_lower()
}
