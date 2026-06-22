use super::*;

#[test]
fn sans_run_context_tightens_kerned_uppercase_runs() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Upper,
    };
    let context = RunContext {
        sans_like: true,
        upper_pairs: 5,
        strong_upper_metric_pairs: 4,
        ..RunContext::default()
    };

    let delta = sans_run_context_delta(-0.085, -0.085, class, context, test_config(0.220, 0.050));
    assert!((delta + 0.036).abs() < 0.001);
}

#[test]
fn sans_run_context_tightens_partial_long_uppercase_runs() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Upper,
    };
    let context = RunContext {
        sans_like: true,
        upper_pairs: 5,
        strong_upper_metric_pairs: 3,
        ..RunContext::default()
    };

    let delta = sans_run_context_delta(-0.087, -0.087, class, context, test_config(0.214, 0.046));

    assert!((delta + 0.024).abs() < 0.001);
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

    let delta = sans_run_context_delta(-0.013, 0.0, class, context, test_config(0.220, 0.050));
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

    let delta = sans_run_context_delta(-0.092, -0.063, class, context, test_config(0.220, 0.050));
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
        sans_run_context_delta(
            -0.018,
            0.0,
            class,
            pure_lower_context,
            test_config(0.220, 0.050)
        ),
        0.0
    );
    let delta =
        sans_run_context_delta(-0.018, 0.0, class, mixed_context, test_config(0.220, 0.050));
    assert!((delta + 0.012).abs() < 0.001);
}

#[test]
fn sans_run_context_relaxes_pure_compact_lowercase_runs_without_optical_tightening() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        sans_like: true,
        sans_lower_run_like: true,
        lower_pairs: 7,
        metricless_lower_pairs: 7,
        ..RunContext::default()
    };
    let mut config = test_config(0.203, 0.046);
    config.profile.x_height = 0.52;
    config.profile.cap_height = 0.70;

    let delta = sans_run_context_delta(-0.008, 0.0, class, context, config);

    assert!((delta - 0.019).abs() < 0.001);
}

#[test]
fn sans_run_context_relaxes_severe_compact_lowercase_tightening_only() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        sans_like: true,
        sans_lower_run_like: true,
        lower_pairs: 5,
        metricless_lower_pairs: 5,
        optical_tightening_lower_pairs: 1,
        ..RunContext::default()
    };
    let mut config = test_config(0.203, 0.046);
    config.profile.x_height = 0.52;
    config.profile.cap_height = 0.70;

    let severe_delta = sans_run_context_delta(-0.123, 0.0, class, context, config);
    let mild_delta = sans_run_context_delta(-0.008, 0.0, class, context, config);

    assert!((severe_delta - 0.088).abs() < 0.001);
    assert_eq!(mild_delta, 0.0);
}

#[test]
fn sans_run_context_relaxes_long_noncompact_lowercase_runs_moderately() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        sans_like: true,
        sans_lower_run_like: true,
        lower_pairs: 8,
        metricless_lower_pairs: 7,
        optical_tightening_lower_pairs: 1,
        ..RunContext::default()
    };

    let delta = sans_run_context_delta(-0.018, 0.0, class, context, test_config(0.218, 0.050));

    assert!((delta - 0.004).abs() < 0.001);
}

#[test]
fn sans_run_context_relaxes_medium_noncompact_lowercase_runs_moderately() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        sans_like: true,
        sans_lower_run_like: true,
        lower_pairs: 5,
        metricless_lower_pairs: 4,
        optical_tightening_lower_pairs: 1,
        ..RunContext::default()
    };

    let delta = sans_run_context_delta(-0.018, 0.0, class, context, test_config(0.218, 0.050));

    assert!((delta - 0.008).abs() < 0.001);
}

#[test]
fn sans_run_context_preserves_strong_lowercase_metric_pairs() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        sans_like: true,
        sans_lower_run_like: true,
        lower_pairs: 7,
        metricless_lower_pairs: 6,
        ..RunContext::default()
    };

    let delta = sans_run_context_delta(-0.060, -0.060, class, context, test_config(0.218, 0.050));

    assert_eq!(delta, 0.0);
}

#[test]
fn sans_run_context_preserves_moderate_lowercase_metric_pairs() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        sans_like: true,
        sans_lower_run_like: true,
        lower_pairs: 7,
        metricless_lower_pairs: 5,
        ..RunContext::default()
    };

    let delta = sans_run_context_delta(-0.039, -0.039, class, context, test_config(0.218, 0.050));

    assert_eq!(delta, 0.0);
}

#[test]
fn sans_run_context_keeps_compact_sans_mixed_metrics() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        sans_like: true,
        strong_mixed_metric_pairs: 2,
        ..RunContext::default()
    };
    let mut config = test_config(0.203, 0.046);
    config.profile.x_height = 0.52;
    config.profile.cap_height = 0.70;

    let delta = sans_run_context_delta(-0.105, -0.093, class, context, config);

    assert_eq!(delta, 0.0);
}

#[test]
fn sans_run_context_restores_compact_sans_mixed_lower_bridges() {
    let lower_class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let upper_lower_class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        sans_like: true,
        mixed_case_pairs: 2,
        lower_pairs: 4,
        ..RunContext::default()
    };
    let mut config = test_config(0.203, 0.046);
    config.profile.x_height = 0.52;
    config.profile.cap_height = 0.70;

    let lower_delta = sans_run_context_delta(-0.008, 0.0, lower_class, context, config);
    let upper_lower_delta = sans_run_context_delta(-0.064, 0.0, upper_lower_class, context, config);

    assert!((lower_delta + 0.010).abs() < 0.001);
    assert!((upper_lower_delta + 0.010).abs() < 0.001);
}
