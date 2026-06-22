use super::*;

fn guarded_run_result(left: char, right: char, delta_em: f32, gap_min_em: f32) -> AlgorithmSet {
    guarded_run_result_with_metrics(left, right, delta_em, 0.0, delta_em, gap_min_em)
}

fn guarded_run_result_with_metrics(
    left: char,
    right: char,
    delta_em: f32,
    metric_delta_em: f32,
    optical_delta_em: f32,
    gap_min_em: f32,
) -> AlgorithmSet {
    AlgorithmSet {
        font_id: "test".to_owned(),
        pair: format!("{left}{right}"),
        left,
        right,
        display: format!("{left}{right}"),
        shaping_text: format!("{left}{right}"),
        left_glyph_id: 1,
        right_glyph_id: 2,
        left_cluster: left.to_string(),
        right_cluster: right.to_string(),
        outputs: vec![AlgorithmOutput {
            algorithm: Algorithm::GuardedProfileHybrid,
            delta_em,
            metric_delta_em,
            optical_delta_em,
            target_gap_em: 0.231,
            gap_distribution_mad_em: 0.056,
            gap_min_em,
            gap_weighted_mean_em: 0.100,
            gap_robust_mean_em: 0.100,
            gap_mad_em: 0.030,
            samples: 80,
        }],
    }
}

fn guarded_delta(result: &AlgorithmSet) -> f32 {
    result
        .outputs
        .iter()
        .find(|output| output.algorithm == Algorithm::GuardedProfileHybrid)
        .map(|output| output.delta_em)
        .unwrap()
}

#[test]
fn run_context_caps_connected_script_openings() {
    let mut results = vec![
        guarded_run_result('G', 'o', 0.055, -0.034),
        guarded_run_result('o', 'l', 0.055, -0.041),
        guarded_run_result('l', 'd', 0.055, -0.030),
        guarded_run_result('d', 'f', 0.055, -0.036),
    ];

    apply_run_context_adjustments(&mut results, test_config(0.231, 0.056));

    for result in &results {
        let delta = guarded_delta(result);
        assert!(delta > 0.0);
        assert!(delta <= 0.009);
    }
}

#[test]
fn run_context_keeps_isolated_letter_openings() {
    let mut results = vec![
        guarded_run_result('W', 'A', 0.055, -0.034),
        guarded_run_result('A', 'V', 0.0, 0.044),
        guarded_run_result('V', 'Y', 0.0, 0.038),
    ];

    apply_run_context_adjustments(&mut results, test_config(0.231, 0.056));

    assert!((guarded_delta(&results[0]) - 0.055).abs() < 0.001);
}

#[test]
fn run_context_keeps_uppercase_script_openings() {
    let mut results = vec![
        guarded_run_result('W', 'A', 0.055, -0.034),
        guarded_run_result('A', 'V', 0.055, -0.041),
        guarded_run_result('V', 'Y', 0.055, -0.030),
    ];

    apply_run_context_adjustments(&mut results, test_config(0.231, 0.056));

    for result in &results {
        assert!((guarded_delta(result) - 0.055).abs() < 0.001);
    }
}

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

#[test]
fn connected_script_delta_is_zero_without_script_run_context() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };

    let delta = connected_script_delta(
        0.055,
        -0.034,
        class,
        RunContext::default(),
        test_config(0.231, 0.056),
    );

    assert_eq!(delta, 0.0);
}

#[test]
fn script_mixed_case_caps_connected_openings() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Upper,
    };
    let context = RunContext {
        script_mixed_case_like: true,
        mixed_case_pairs: 2,
        ..RunContext::default()
    };
    let mut config = test_config(0.170, 0.050);
    config.profile.x_height = 0.48;
    config.profile.cap_height = 0.78;

    let delta = script_mixed_case_delta(0.055, -0.063, 0.0, class, context, config);

    assert!(delta < -0.045);
}

#[test]
fn script_mixed_case_tightens_metricless_mixed_pairs() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        script_mixed_case_like: true,
        mixed_case_pairs: 2,
        ..RunContext::default()
    };
    let mut config = test_config(0.170, 0.050);
    config.profile.x_height = 0.48;
    config.profile.cap_height = 0.78;

    let delta = script_mixed_case_delta(-0.013, 0.124, 0.0, class, context, config);

    assert!(delta < -0.025);
}

#[test]
fn script_mixed_case_ignores_sans_contexts() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        sans_like: true,
        script_mixed_case_like: false,
        mixed_case_pairs: 2,
        ..RunContext::default()
    };

    let delta = script_mixed_case_delta(
        -0.013,
        0.124,
        0.0,
        class,
        context,
        test_config(0.170, 0.050),
    );

    assert_eq!(delta, 0.0);
}

#[test]
fn script_residual_balancer_tightens_script_lowercase_bridges() {
    let mut results = vec![
        guarded_run_result_with_metrics('O', 'p', -0.041, 0.0, 0.0, 0.086),
        guarded_run_result_with_metrics('p', 'e', 0.0, 0.0, 0.017, -0.095),
        guarded_run_result_with_metrics('e', 'n', 0.0, 0.0, 0.030, -0.079),
        guarded_run_result_with_metrics('n', 'T', -0.120, 0.0, -0.055, 0.184),
        guarded_run_result_with_metrics('T', 'y', -0.044, -0.030, -0.045, 0.282),
        guarded_run_result_with_metrics('y', 'p', 0.0, 0.010, 0.024, -0.074),
        guarded_run_result_with_metrics('p', 'e', 0.0, 0.0, 0.017, -0.095),
    ];
    let mut config = test_config(0.170, 0.050);
    config.profile.x_height = 0.48;
    config.profile.cap_height = 0.78;

    apply_run_context_adjustments(&mut results, config);

    assert!(guarded_delta(&results[0]) < -0.050);
    assert!(guarded_delta(&results[1]) < 0.0);
    assert!((guarded_delta(&results[3]) + 0.120).abs() < 0.001);
    assert!((guarded_delta(&results[4]) + 0.044).abs() < 0.001);
}

#[test]
fn script_residual_balancer_keeps_alternating_mixed_case_run() {
    let mut results = vec![
        guarded_run_result_with_metrics('T', 'o', -0.104, -0.090, -0.048, 0.264),
        guarded_run_result_with_metrics('o', 'T', -0.120, 0.0, -0.082, 0.191),
        guarded_run_result_with_metrics('T', 'a', -0.109, -0.095, -0.058, 0.273),
        guarded_run_result_with_metrics('a', 'L', 0.0, 0.0, 0.007, -0.148),
    ];
    let mut config = test_config(0.170, 0.050);
    config.profile.x_height = 0.48;
    config.profile.cap_height = 0.78;

    apply_run_context_adjustments(&mut results, config);

    assert!((guarded_delta(&results[1]) + 0.120).abs() < 0.001);
}

#[test]
fn script_residual_balancer_ignores_nonsevere_script_run() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Lower,
    };
    let balance = ScriptResidualBalance {
        severe_metricless_mixed_pairs: 0,
        metricless_excess_tightening_em: 0.120,
    };
    let mut config = test_config(0.170, 0.050);
    config.profile.x_height = 0.48;
    config.profile.cap_height = 0.78;

    let delta = script_residual_balance_delta(-0.038, 0.0, 0.0, 0.100, class, balance, config);

    assert_eq!(delta, 0.0);
}
