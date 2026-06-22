use crate::class::PairClass;

use super::math::normalized_delta;
use super::types::EvaluationConfig;

const SERIF_CAP_RUN_MIN_PAIRS: usize = 4;
const SERIF_CAP_RUN_MIN_STRONG_METRICS: usize = 2;
const SERIF_CAP_STRONG_METRIC_EM: f32 = -0.050;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct CapitalRunContext {
    upper_pairs: usize,
    strong_metric_upper_pairs: usize,
}

impl CapitalRunContext {
    pub(super) fn record(&mut self, pair_class: PairClass, metric_delta_em: f32) {
        if !pair_class.is_upper_upper() {
            return;
        }

        self.upper_pairs += 1;
        if metric_delta_em < SERIF_CAP_STRONG_METRIC_EM {
            self.strong_metric_upper_pairs += 1;
        }
    }

    pub(super) fn has_adjustments(self, sans_like: bool, config: EvaluationConfig) -> bool {
        self.serif_cap_run(sans_like, config)
    }

    fn serif_cap_run(self, sans_like: bool, config: EvaluationConfig) -> bool {
        !sans_like
            && config.target_gap_em >= 0.225
            && self.upper_pairs >= SERIF_CAP_RUN_MIN_PAIRS
            && self.strong_metric_upper_pairs >= SERIF_CAP_RUN_MIN_STRONG_METRICS
    }
}

pub(super) fn serif_cap_run_delta(
    adjusted_delta: f32,
    metric_delta_em: f32,
    pair_class: PairClass,
    context: CapitalRunContext,
    sans_like: bool,
    config: EvaluationConfig,
) -> f32 {
    if !context.serif_cap_run(sans_like, config)
        || !pair_class.is_upper_upper()
        || metric_delta_em >= SERIF_CAP_STRONG_METRIC_EM
    {
        return 0.0;
    }

    let floor = serif_cap_run_floor(config);
    if adjusted_delta < floor {
        normalized_delta(floor - adjusted_delta)
    } else {
        0.0
    }
}

fn serif_cap_run_floor(config: EvaluationConfig) -> f32 {
    let extra_width = (config.target_gap_em - 0.242).max(0.0);
    let magnitude = (0.105 - extra_width * 0.20).clamp(0.098, 0.105);
    -magnitude
}
