use super::*;

#[test]
fn algorithm_names_are_stable() {
    let names = Algorithm::all()
        .iter()
        .map(|algorithm| algorithm.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        vec![
            "nearest-contour-distance",
            "profile-whitespace",
            "area-balance",
            "metric-prior-hybrid",
            "guarded-profile-hybrid",
            "safe-fallback-only",
        ]
    );
}

#[test]
fn hybrid_preserves_close_metric_delta() {
    assert!(
        (metric_prior_hybrid_for_class(-0.04, -0.05, PairClass::default()) + 0.04).abs() < 0.001
    );
}

#[test]
fn hybrid_uses_optical_when_metric_missing() {
    assert!((metric_prior_hybrid_for_class(0.0, -0.06, PairClass::default()) + 0.06).abs() < 0.001);
}

#[test]
fn class_aware_hybrid_trusts_upper_lower_metric_pairs() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Lower,
    };
    let delta = metric_prior_hybrid_for_class(-0.105, -0.032, class);
    assert!(delta < -0.08);
}

#[test]
fn class_aware_hybrid_dampens_metricless_upper_pairs() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Upper,
    };
    let delta = metric_prior_hybrid_for_class(0.0, -0.138, class);
    assert!((delta + 0.070).abs() < 0.001);
}

#[test]
fn class_aware_hybrid_clamps_upper_digit_pairs() {
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Digit,
    };
    let delta = metric_prior_hybrid_for_class(0.0, -0.131, class);
    assert!((delta + 0.055).abs() < 0.001);
}
