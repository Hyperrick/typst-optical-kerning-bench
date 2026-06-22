use super::*;

#[test]
fn serif_cap_run_opens_long_strong_metric_cap_runs() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Upper,
    };
    let mut context = CapitalRunContext::default();
    for _ in 0..4 {
        context.record(class, -0.074);
    }
    context.record(class, 0.0);
    let mut config = test_config(0.271, 0.052);
    config.profile.x_height = 0.50;
    config.profile.cap_height = 0.73;

    let delta = serif_cap_run_delta(-0.110, -0.074, class, context, false, config);

    assert!(delta > 0.010);
}

#[test]
fn serif_cap_run_keeps_short_cap_controls() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Upper,
    };
    let mut context = CapitalRunContext::default();
    for _ in 0..3 {
        context.record(class, -0.074);
    }
    let mut config = test_config(0.271, 0.052);
    config.profile.x_height = 0.50;
    config.profile.cap_height = 0.73;

    let delta = serif_cap_run_delta(-0.110, -0.074, class, context, false, config);

    assert_eq!(delta, 0.0);
}

#[test]
fn serif_cap_run_ignores_sans_cap_runs() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Upper,
    };
    let mut context = CapitalRunContext::default();
    for _ in 0..5 {
        context.record(class, -0.074);
    }
    let mut config = test_config(0.220, 0.052);
    config.profile.x_height = 0.52;
    config.profile.cap_height = 0.70;

    let delta = serif_cap_run_delta(-0.110, -0.074, class, context, true, config);

    assert_eq!(delta, 0.0);
}
