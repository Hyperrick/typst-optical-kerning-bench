use crate::DEAD_ZONE;

const METRIC_EPSILON: f32 = 0.0001;
const MIN_ALLOWANCE: f32 = 0.012;
const MAX_ALLOWANCE: f32 = 0.030;

pub(crate) fn preserve_metric_prior(metric: f32, candidate: f32) -> f32 {
    if metric.abs() < METRIC_EPSILON {
        return candidate;
    }

    if metric.abs() < DEAD_ZONE {
        return metric;
    }

    let allowance = (metric.abs() * 0.5).clamp(MIN_ALLOWANCE, MAX_ALLOWANCE);
    let bounded = candidate.clamp(metric - allowance, metric + allowance);
    let sign_floor = metric.abs().min(DEAD_ZONE);

    if metric.is_sign_positive() {
        bounded.max(sign_floor)
    } else {
        bounded.min(-sign_floor)
    }
}
