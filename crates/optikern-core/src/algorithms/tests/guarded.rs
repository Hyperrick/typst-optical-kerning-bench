use super::*;

#[test]
fn guarded_hybrid_blocks_aperture_bias() {
    let stats = GapStats {
        min_gap: 0.0755,
        weighted_mean_gap: 0.4507,
        robust_mean_gap: 0.4544,
        mad: 0.05,
        samples: 80,
    };
    let config = test_config(0.2846, 0.0567);
    let delta = guarded_profile_hybrid(
        0.0,
        -0.079,
        0.027,
        stats,
        config,
        PairClass::default(),
        PairGeometry::default(),
    );
    assert_eq!(delta, 0.0);
}

#[test]
fn guarded_hybrid_keeps_clear_wide_gap_adjustment() {
    let stats = GapStats {
        min_gap: 0.3406,
        weighted_mean_gap: 0.4147,
        robust_mean_gap: 0.4233,
        mad: 0.03,
        samples: 80,
    };
    let config = test_config(0.2846, 0.0567);
    let delta = guarded_profile_hybrid(
        0.0,
        -0.053,
        0.0,
        stats,
        config,
        PairClass::default(),
        PairGeometry::default(),
    );
    assert!((delta + 0.053).abs() < 0.001);
}

#[test]
fn guarded_hybrid_clamps_metricless_upper_lower_aperture_bias() {
    let stats = GapStats {
        min_gap: 0.0755,
        weighted_mean_gap: 0.4507,
        robust_mean_gap: 0.4544,
        mad: 0.263,
        samples: 39,
    };
    let config = test_config(0.271, 0.052);
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Lower,
    };
    let geometry = PairGeometry {
        right_left_side: SideFeatures {
            roundness: 0.08,
            stemness: 0.10,
        },
        ..PairGeometry::default()
    };

    let delta = guarded_profile_hybrid(0.0, -0.093, 0.023, stats, config, class, geometry);
    assert!(delta > -0.060);
    assert!(delta < -0.045);
}

#[test]
fn guarded_hybrid_tightens_safe_lower_upper_overhang() {
    let stats = GapStats {
        min_gap: 0.331,
        weighted_mean_gap: 0.360,
        robust_mean_gap: 0.353,
        mad: 0.04,
        samples: 80,
    };
    let config = test_config(0.231, 0.056);
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Upper,
    };
    let geometry = PairGeometry {
        right_top_left_overhang: 0.26,
        ..PairGeometry::default()
    };
    let delta = guarded_profile_hybrid(0.0, -0.038, 0.0, stats, config, class, geometry);
    assert!(delta < -0.070);
}

#[test]
fn side_shape_tightens_stem_to_round_digits() {
    let stats = GapStats {
        min_gap: 0.20,
        weighted_mean_gap: 0.25,
        robust_mean_gap: 0.25,
        mad: 0.02,
        samples: 80,
    };
    let config = test_config(0.231, 0.056);
    let class = PairClass {
        left: ClusterClass::Digit,
        right: ClusterClass::Digit,
    };
    let geometry = PairGeometry {
        right_top_left_overhang: 0.0,
        left_right_side: SideFeatures {
            roundness: 0.0,
            stemness: 0.90,
        },
        right_left_side: SideFeatures {
            roundness: 0.08,
            stemness: 0.10,
        },
    };

    let delta = side_shape_delta(0.0, 0.0, 0.0, stats, config, class, geometry);
    assert!((delta + 0.040).abs() < 0.001);
}

#[test]
fn side_shape_tightens_upper_to_round_lower_when_gap_is_wide() {
    let stats = GapStats {
        min_gap: 0.32,
        weighted_mean_gap: 0.35,
        robust_mean_gap: 0.35,
        mad: 0.03,
        samples: 80,
    };
    let config = test_config(0.231, 0.056);
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Lower,
    };
    let geometry = PairGeometry {
        right_top_left_overhang: 0.0,
        left_right_side: SideFeatures::default(),
        right_left_side: SideFeatures {
            roundness: 0.08,
            stemness: 0.10,
        },
    };

    let delta = side_shape_delta(-0.105, -0.087, 0.0, stats, config, class, geometry);
    assert!(delta < -0.010);
}

#[test]
fn guarded_hybrid_opens_local_letter_collisions() {
    let stats = GapStats {
        min_gap: -0.006,
        weighted_mean_gap: 0.300,
        robust_mean_gap: 0.326,
        mad: 0.043,
        samples: 80,
    };
    let config = test_config(0.231, 0.056);
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Upper,
    };

    let delta = collision_opening_delta(0.0, 0.046, stats, config, class);
    assert!(delta > 0.030);
}

#[test]
fn guarded_hybrid_tightens_clear_upper_punctuation_gap() {
    let stats = GapStats {
        min_gap: 0.254,
        weighted_mean_gap: 0.319,
        robust_mean_gap: 0.315,
        mad: 0.043,
        samples: 80,
    };
    let config = test_config(0.231, 0.056);
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Punctuation,
    };

    let delta = punctuation_spacing_delta(-0.075, -0.056, 0.0, stats, config, class);
    assert!(delta < -0.045);
}

#[test]
fn guarded_hybrid_tightens_round_to_upper_overhang() {
    let stats = GapStats {
        min_gap: 0.331,
        weighted_mean_gap: 0.363,
        robust_mean_gap: 0.353,
        mad: 0.019,
        samples: 80,
    };
    let config = test_config(0.231, 0.056);
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Upper,
    };
    let geometry = PairGeometry {
        right_top_left_overhang: 0.26,
        left_right_side: SideFeatures {
            roundness: 0.052,
            stemness: 0.20,
        },
        right_left_side: SideFeatures::default(),
    };

    let delta = guarded_profile_hybrid(0.0, -0.047, 0.0, stats, config, class, geometry);
    assert!(delta < -0.110);
}

#[test]
fn guarded_hybrid_closes_metricless_serif_round_to_upper_gap() {
    let stats = GapStats {
        min_gap: 0.331,
        weighted_mean_gap: 0.363,
        robust_mean_gap: 0.353,
        mad: 0.019,
        samples: 80,
    };
    let mut config = test_config(0.231, 0.056);
    config.profile.x_height = 0.48;
    config.profile.cap_height = 0.78;
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Upper,
    };
    let geometry = PairGeometry {
        right_top_left_overhang: 0.26,
        left_right_side: SideFeatures {
            roundness: 0.052,
            stemness: 0.20,
        },
        right_left_side: SideFeatures::default(),
    };

    let delta = guarded_profile_hybrid(0.0, -0.047, 0.0, stats, config, class, geometry);
    assert!(delta < -0.125);
    assert!(delta > -0.145);
}

#[test]
fn guarded_hybrid_preserves_strong_serif_upper_to_round_lower_metric() {
    let stats = GapStats {
        min_gap: 0.320,
        weighted_mean_gap: 0.362,
        robust_mean_gap: 0.346,
        mad: 0.018,
        samples: 80,
    };
    let mut config = test_config(0.231, 0.056);
    config.profile.x_height = 0.48;
    config.profile.cap_height = 0.78;
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Lower,
    };
    let geometry = PairGeometry {
        right_left_side: SideFeatures {
            roundness: 0.052,
            stemness: 0.20,
        },
        ..PairGeometry::default()
    };

    let delta = guarded_profile_hybrid(-0.105, -0.045, 0.0, stats, config, class, geometry);
    assert!((delta + 0.105).abs() < 0.001);
}

#[test]
fn suppresses_false_diagonal_opening_when_collision_is_only_local() {
    let stats = GapStats {
        min_gap: -0.013,
        weighted_mean_gap: 0.470,
        robust_mean_gap: 0.463,
        mad: 0.13,
        samples: 55,
    };
    let mut config = test_config(0.285, 0.052);
    config.profile.x_height = 0.69;
    config.profile.cap_height = 1.0;
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Upper,
    };
    let geometry = PairGeometry {
        left_right_side: SideFeatures {
            roundness: 0.0,
            stemness: 0.35,
        },
        right_left_side: SideFeatures {
            roundness: 0.0,
            stemness: 0.35,
        },
        ..PairGeometry::default()
    };

    let delta = suppress_false_diagonal_opening(0.044, 0.0, stats, config, class, geometry);
    assert_eq!(delta, 0.0);
}

#[test]
fn wide_serif_display_tightens_weak_diagonal_metric_pairs() {
    let stats = GapStats {
        min_gap: 0.330,
        weighted_mean_gap: 0.420,
        robust_mean_gap: 0.410,
        mad: 0.004,
        samples: 55,
    };
    let mut config = test_config(0.285, 0.052);
    config.profile.x_height = 0.69;
    config.profile.cap_height = 1.0;
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Upper,
    };
    let geometry = PairGeometry {
        left_right_side: SideFeatures {
            roundness: 0.0,
            stemness: 0.35,
        },
        right_left_side: SideFeatures {
            roundness: 0.0,
            stemness: 0.35,
        },
        ..PairGeometry::default()
    };

    let delta = wide_serif_display_delta(-0.076, -0.076, 0.0, stats, config, class, geometry);
    assert!(delta < -0.020);
}

#[test]
fn wide_serif_display_leaves_strong_metric_pairs_alone() {
    let stats = GapStats {
        min_gap: 0.298,
        weighted_mean_gap: 0.386,
        robust_mean_gap: 0.381,
        mad: 0.006,
        samples: 53,
    };
    let config = test_config(0.230, 0.056);
    let class = PairClass {
        left: ClusterClass::Upper,
        right: ClusterClass::Upper,
    };
    let geometry = PairGeometry {
        left_right_side: SideFeatures {
            roundness: 0.0,
            stemness: 0.35,
        },
        right_left_side: SideFeatures {
            roundness: 0.0,
            stemness: 0.35,
        },
        ..PairGeometry::default()
    };

    let delta = wide_serif_display_delta(-0.140, -0.117, 0.0, stats, config, class, geometry);
    assert_eq!(delta, 0.0);
}

#[test]
fn sans_lowercase_compaction_tightens_weak_lowercase_pairs() {
    let stats = GapStats {
        min_gap: 0.102,
        weighted_mean_gap: 0.150,
        robust_mean_gap: 0.150,
        mad: 0.046,
        samples: 41,
    };
    let mut config = test_config(0.217, 0.050);
    config.profile.x_height = 0.75;
    config.profile.cap_height = 1.0;
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };

    let delta = sans_lowercase_compaction_delta(0.0, -0.012, 0.0, stats, config, class);
    assert!(delta < -0.010);
}

#[test]
fn sans_lowercase_compaction_does_not_apply_to_serif_profile() {
    let stats = GapStats {
        min_gap: 0.102,
        weighted_mean_gap: 0.150,
        robust_mean_gap: 0.150,
        mad: 0.046,
        samples: 41,
    };
    let mut config = test_config(0.230, 0.050);
    config.profile.x_height = 0.61;
    config.profile.cap_height = 1.0;
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };

    let delta = sans_lowercase_compaction_delta(0.0, -0.012, 0.0, stats, config, class);
    assert_eq!(delta, 0.0);
}

#[test]
fn sans_lowercase_compaction_keeps_already_tight_pairs() {
    let stats = GapStats {
        min_gap: 0.200,
        weighted_mean_gap: 0.260,
        robust_mean_gap: 0.250,
        mad: 0.020,
        samples: 80,
    };
    let mut config = test_config(0.210, 0.050);
    config.profile.x_height = 0.78;
    config.profile.cap_height = 1.0;
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };

    let delta = sans_lowercase_compaction_delta(0.0, -0.123, 0.0, stats, config, class);

    assert_eq!(delta, 0.0);
}

#[test]
fn guarded_hybrid_stacks_safe_lowercase_tightening() {
    let stats = GapStats {
        min_gap: 0.148,
        weighted_mean_gap: 0.282,
        robust_mean_gap: 0.203,
        mad: 0.058,
        samples: 41,
    };
    let mut config = test_config(0.217, 0.049);
    config.profile.x_height = 0.75;
    config.profile.cap_height = 1.0;
    let class = PairClass {
        left: ClusterClass::Lower,
        right: ClusterClass::Lower,
    };

    let delta = guarded_profile_hybrid(
        -0.0195,
        0.0,
        0.0,
        stats,
        config,
        class,
        PairGeometry::default(),
    );
    assert!(delta < -0.045);
}
