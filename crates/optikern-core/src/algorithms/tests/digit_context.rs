use super::*;

#[test]
fn digit_run_context_tightens_metricless_sans_number_runs() {
    let mut results = vec![
        guarded_run_result_with_metrics('1', '0', -0.040, 0.0, -0.040, 0.133),
        guarded_run_result_with_metrics('0', '.', -0.010, 0.0, -0.010, 0.132),
        guarded_run_result_with_metrics('.', '0', -0.010, 0.0, -0.010, 0.132),
        guarded_run_result_with_metrics('0', '0', -0.040, 0.0, -0.040, 0.133),
        guarded_run_result_with_metrics('0', '0', -0.040, 0.0, -0.040, 0.133),
    ];
    let mut config = test_config(0.218, 0.050);
    config.profile.x_height = 0.48;
    config.profile.cap_height = 0.66;

    apply_run_context_adjustments(&mut results, config);

    assert!(guarded_delta(&results[0]) < -0.056);
    assert!(guarded_delta(&results[1]) < -0.027);
    assert!(guarded_delta(&results[3]) < -0.056);
}

#[test]
fn digit_run_context_keeps_sans_numbers_with_metric_pairs() {
    let mut results = vec![
        guarded_run_result_with_metrics('1', '0', -0.040, 0.0, -0.040, 0.133),
        guarded_run_result_with_metrics('0', '.', -0.026, -0.026, -0.026, 0.132),
        guarded_run_result_with_metrics('.', '0', -0.026, -0.026, -0.026, 0.132),
        guarded_run_result_with_metrics('0', '0', -0.040, 0.0, -0.040, 0.133),
        guarded_run_result_with_metrics('0', '0', -0.040, 0.0, -0.040, 0.133),
    ];
    let mut config = test_config(0.218, 0.050);
    config.profile.x_height = 0.48;
    config.profile.cap_height = 0.66;

    apply_run_context_adjustments(&mut results, config);

    assert!((guarded_delta(&results[0]) + 0.040).abs() < 0.001);
    assert!((guarded_delta(&results[3]) + 0.040).abs() < 0.001);
}

#[test]
fn digit_run_context_tightens_wide_serif_digit_runs() {
    let mut results = vec![
        guarded_run_result_with_metrics('1', '0', -0.040, 0.0, -0.040, 0.122),
        guarded_run_result_with_metrics('0', '.', -0.021, -0.021, -0.021, 0.126),
        guarded_run_result_with_metrics('.', '0', -0.021, -0.021, -0.021, 0.126),
        guarded_run_result_with_metrics('0', '0', -0.040, 0.0, -0.040, 0.122),
        guarded_run_result_with_metrics('0', '0', -0.040, 0.0, -0.040, 0.122),
    ];
    let mut config = test_config(0.270, 0.050);
    config.profile.x_height = 0.50;
    config.profile.cap_height = 0.73;

    apply_run_context_adjustments(&mut results, config);

    assert!(guarded_delta(&results[0]) < -0.055);
    assert!((guarded_delta(&results[1]) + 0.021).abs() < 0.001);
    assert!(guarded_delta(&results[3]) < -0.055);
}

#[test]
fn digit_run_context_keeps_low_aperture_serif_digits() {
    let mut results = vec![
        guarded_run_result_with_metrics('1', '0', -0.040, 0.0, -0.040, 0.070),
        guarded_run_result_with_metrics('0', '.', -0.010, 0.0, -0.010, 0.070),
        guarded_run_result_with_metrics('.', '0', -0.010, 0.0, -0.010, 0.070),
        guarded_run_result_with_metrics('0', '0', -0.040, 0.0, -0.040, 0.070),
        guarded_run_result_with_metrics('0', '0', -0.040, 0.0, -0.040, 0.070),
    ];
    let mut config = test_config(0.244, 0.050);
    config.profile.x_height = 0.48;
    config.profile.cap_height = 0.78;

    apply_run_context_adjustments(&mut results, config);

    assert!((guarded_delta(&results[0]) + 0.040).abs() < 0.001);
    assert!((guarded_delta(&results[3]) + 0.040).abs() < 0.001);
}
