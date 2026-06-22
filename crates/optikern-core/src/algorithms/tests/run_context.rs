use super::*;

#[test]
fn sans_run_context_tightens_kerned_uppercase_runs() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Upper,
    };
    let context = RunContext {
        sans_like: true,
        strong_upper_metric_pairs: 4,
        ..RunContext::default()
    };

    let delta = sans_run_context_delta(-0.085, -0.085, class, context);
    assert!((delta + 0.026).abs() < 0.001);
}

#[test]
fn sans_run_context_leaves_metricless_uppercase_controls_alone() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Upper,
    };
    let context = RunContext {
        sans_like: true,
        strong_upper_metric_pairs: 4,
        ..RunContext::default()
    };

    let delta = sans_run_context_delta(-0.013, 0.0, class, context);
    assert_eq!(delta, 0.0);
}

#[test]
fn sans_run_context_tightens_mixed_case_runs() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        sans_like: true,
        strong_mixed_metric_pairs: 2,
        ..RunContext::default()
    };

    let delta = sans_run_context_delta(-0.092, -0.063, class, context);
    assert!((delta + 0.024).abs() < 0.001);
}

#[test]
fn sans_run_context_limits_lowercase_accumulation_to_mixed_runs() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let pure_lower_context = RunContext {
        sans_like: true,
        lower_pairs: 5,
        ..RunContext::default()
    };
    let mixed_context = RunContext {
        sans_like: true,
        strong_mixed_metric_pairs: 2,
        lower_pairs: 5,
        ..RunContext::default()
    };

    assert_eq!(
        sans_run_context_delta(-0.018, 0.0, class, pure_lower_context),
        0.0
    );
    let delta = sans_run_context_delta(-0.018, 0.0, class, mixed_context);
    assert!((delta + 0.012).abs() < 0.001);
}
