use super::*;

#[test]
fn serif_ligature_run_tightens_safe_lowercase_bridges() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        serif_ligature_lower_run_like: true,
        lower_pairs: 6,
        metricless_lower_pairs: 6,
        multi_char_letter_pairs: 2,
        ..RunContext::default()
    };
    let mut config = test_config(0.271, 0.052);
    config.profile.x_height = 0.50;
    config.profile.cap_height = 0.72;

    let delta =
        serif_ligature_lower_run_delta(-0.013, 0.0, 0.120, 0.260, class, 1, context, config);

    assert!((delta + 0.0136).abs() < 0.001);
}

#[test]
fn serif_ligature_run_keeps_right_ligature_entries() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        serif_ligature_lower_run_like: true,
        lower_pairs: 6,
        metricless_lower_pairs: 6,
        multi_char_letter_pairs: 2,
        ..RunContext::default()
    };
    let mut config = test_config(0.271, 0.052);
    config.profile.x_height = 0.50;
    config.profile.cap_height = 0.72;

    let delta =
        serif_ligature_lower_run_delta(-0.013, 0.0, 0.120, 0.260, class, 3, context, config);

    assert_eq!(delta, 0.0);
}

#[test]
fn serif_ligature_run_keeps_tight_local_gaps() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        serif_ligature_lower_run_like: true,
        lower_pairs: 6,
        metricless_lower_pairs: 6,
        multi_char_letter_pairs: 2,
        ..RunContext::default()
    };
    let mut config = test_config(0.271, 0.052);
    config.profile.x_height = 0.50;
    config.profile.cap_height = 0.72;

    let delta =
        serif_ligature_lower_run_delta(-0.013, 0.0, 0.060, 0.260, class, 1, context, config);

    assert_eq!(delta, 0.0);
}

#[test]
fn short_serif_ligature_run_relaxes_compaction_when_gap_is_already_compact() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        short_serif_ligature_lower_run_like: true,
        lower_pairs: 3,
        metricless_lower_pairs: 3,
        multi_char_letter_pairs: 1,
        max_cluster_chars: 2,
        ..RunContext::default()
    };
    let mut config = test_config(0.271, 0.052);
    config.profile.x_height = 0.50;
    config.profile.cap_height = 0.72;

    let delta =
        serif_ligature_lower_run_delta(-0.013, 0.0, 0.094, 0.180, class, 1, context, config);

    assert!((delta - 0.013).abs() < 0.001);
}

#[test]
fn short_serif_ligature_run_keeps_compaction_when_gap_is_not_compact() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        short_serif_ligature_lower_run_like: true,
        lower_pairs: 3,
        metricless_lower_pairs: 3,
        multi_char_letter_pairs: 1,
        max_cluster_chars: 2,
        ..RunContext::default()
    };
    let mut config = test_config(0.271, 0.052);
    config.profile.x_height = 0.50;
    config.profile.cap_height = 0.72;

    let delta =
        serif_ligature_lower_run_delta(-0.013, 0.0, 0.120, 0.225, class, 1, context, config);

    assert_eq!(delta, 0.0);
}

#[test]
fn wide_serif_lower_run_neutralizes_positive_metric_opening_near_touch() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        wide_serif_lower_run_like: true,
        lower_pairs: 4,
        ..RunContext::default()
    };
    let mut config = test_config(0.271, 0.052);
    config.profile.x_height = 0.50;
    config.profile.cap_height = 0.72;

    let delta =
        serif_ligature_lower_run_delta(0.025, 0.025, 0.005, 0.260, class, 1, context, config);

    assert!((delta + 0.025).abs() < 0.001);
}

#[test]
fn wide_serif_lower_run_keeps_positive_metric_opening_when_gap_is_comfortable() {
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };
    let context = RunContext {
        wide_serif_lower_run_like: true,
        lower_pairs: 4,
        ..RunContext::default()
    };
    let mut config = test_config(0.271, 0.052);
    config.profile.x_height = 0.50;
    config.profile.cap_height = 0.72;

    let delta =
        serif_ligature_lower_run_delta(0.025, 0.025, 0.060, 0.260, class, 1, context, config);

    assert_eq!(delta, 0.0);
}
