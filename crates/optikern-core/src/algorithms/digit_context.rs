use crate::class::PairClass;

use super::math::{dead_zone, normalized_delta};
use super::types::EvaluationConfig;

const DIGIT_RUN_MIN_PAIRS: usize = 5;
const SANS_DIGIT_RUN_MAX_METRIC_PAIRS: usize = 0;
const WIDE_SERIF_MIN_LOOSE_DIGIT_PAIRS: usize = 2;
const WIDE_SERIF_MIN_TARGET_GAP_EM: f32 = 0.255;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct DigitRunContext {
    digit_run_pairs: usize,
    metricless_digit_run_pairs: usize,
    loose_metricless_digit_pairs: usize,
}

impl DigitRunContext {
    pub(super) fn record(
        &mut self,
        pair_class: PairClass,
        metric_delta_em: f32,
        gap_min_em: f32,
        config: EvaluationConfig,
    ) {
        if !is_digit_run_pair(pair_class) {
            return;
        }

        self.digit_run_pairs += 1;
        if metric_delta_em.abs() >= dead_zone() {
            return;
        }

        self.metricless_digit_run_pairs += 1;
        if pair_class.is_digit_digit() && gap_min_em >= loose_digit_gap_min(config) {
            self.loose_metricless_digit_pairs += 1;
        }
    }

    pub(super) fn has_adjustments(self, sans_like: bool, config: EvaluationConfig) -> bool {
        self.sans_metricless_digit_run(sans_like)
            || self.wide_serif_metricless_digit_run(sans_like, config)
    }

    fn sans_metricless_digit_run(self, sans_like: bool) -> bool {
        sans_like
            && self.digit_run_pairs >= DIGIT_RUN_MIN_PAIRS
            && self.metricful_digit_run_pairs() <= SANS_DIGIT_RUN_MAX_METRIC_PAIRS
    }

    fn wide_serif_metricless_digit_run(self, sans_like: bool, config: EvaluationConfig) -> bool {
        !sans_like
            && config.target_gap_em >= WIDE_SERIF_MIN_TARGET_GAP_EM
            && self.digit_run_pairs >= DIGIT_RUN_MIN_PAIRS
            && self.loose_metricless_digit_pairs >= WIDE_SERIF_MIN_LOOSE_DIGIT_PAIRS
    }

    fn metricful_digit_run_pairs(self) -> usize {
        self.digit_run_pairs
            .saturating_sub(self.metricless_digit_run_pairs)
    }
}

pub(super) fn digit_run_context_delta(
    adjusted_delta: f32,
    metric_delta_em: f32,
    pair_class: PairClass,
    context: DigitRunContext,
    sans_like: bool,
    config: EvaluationConfig,
) -> f32 {
    if !is_digit_run_pair(pair_class) || metric_delta_em.abs() >= dead_zone() {
        return 0.0;
    }

    if context.sans_metricless_digit_run(sans_like) {
        return tighten_metricless_digit_run_pair(
            adjusted_delta,
            sans_digit_run_amount(config),
            sans_digit_run_lower_bound(pair_class, config),
        );
    }

    if context.wide_serif_metricless_digit_run(sans_like, config) && pair_class.is_digit_digit() {
        return tighten_metricless_digit_run_pair(
            adjusted_delta,
            wide_serif_digit_run_amount(config),
            wide_serif_digit_run_lower_bound(config),
        );
    }

    0.0
}

fn tighten_metricless_digit_run_pair(adjusted_delta: f32, amount: f32, lower_bound: f32) -> f32 {
    normalized_delta((adjusted_delta - amount).max(lower_bound) - adjusted_delta)
}

fn is_digit_run_pair(pair_class: PairClass) -> bool {
    pair_class.is_digit_digit()
        || pair_class.is_digit_punctuation()
        || pair_class.is_punctuation_digit()
}

fn loose_digit_gap_min(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.44).clamp(0.095, 0.125)
}

fn sans_digit_run_amount(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.083).clamp(0.014, 0.020)
}

fn sans_digit_run_lower_bound(pair_class: PairClass, config: EvaluationConfig) -> f32 {
    if pair_class.is_digit_digit() {
        (-config.target_gap_em * 0.30).clamp(-0.070, -0.055)
    } else {
        (-config.target_gap_em * 0.145).clamp(-0.032, -0.024)
    }
}

fn wide_serif_digit_run_amount(config: EvaluationConfig) -> f32 {
    (config.target_gap_em * 0.060).clamp(0.014, 0.018)
}

fn wide_serif_digit_run_lower_bound(config: EvaluationConfig) -> f32 {
    (-config.target_gap_em * 0.21).clamp(-0.060, -0.050)
}
