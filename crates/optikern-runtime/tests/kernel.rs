use optikern_runtime::{
    GlyphClass, PairEvidence, RunPair, SideShape, compact_guarded, compact_guarded_run,
};

fn evidence() -> PairEvidence {
    PairEvidence {
        left: GlyphClass::Lower,
        right: GlyphClass::Lower,
        metric_delta: 0.0,
        optical_delta: -0.04,
        nearest_delta: 0.0,
        target_gap: 0.23,
        gap_mad: 0.05,
        min_gap: 0.14,
        robust_gap: 0.29,
        x_height: 0.52,
        cap_height: 0.72,
        left_side: SideShape::default(),
        right_side: SideShape::default(),
        right_top_left_overhang: 0.0,
        monospaced: false,
    }
}

fn run_pair(left: GlyphClass, right: GlyphClass, metric_delta: f32) -> RunPair {
    RunPair {
        left,
        right,
        left_cluster_chars: 1,
        right_cluster_chars: 1,
        metric_delta,
        optical_delta: metric_delta,
        min_gap: 0.12,
        delta: metric_delta,
    }
}

#[test]
fn monospaced_pairs_preserve_metric_positions() {
    let pair = PairEvidence {
        monospaced: true,
        metric_delta: -0.03,
        ..evidence()
    };
    assert_eq!(compact_guarded(pair), -0.03);
}

#[test]
fn collisions_open_instead_of_tightening() {
    let pair = PairEvidence {
        min_gap: -0.02,
        nearest_delta: 0.04,
        optical_delta: -0.08,
        ..evidence()
    };
    assert!(compact_guarded(pair) > 0.02);
}

#[test]
fn aperture_guard_rejects_false_whitespace() {
    let pair = PairEvidence {
        min_gap: 0.04,
        robust_gap: 0.36,
        optical_delta: -0.09,
        ..evidence()
    };
    assert_eq!(compact_guarded(pair), 0.0);
}

#[test]
fn metric_pair_stays_close_to_metric_prior() {
    let pair = PairEvidence {
        left: GlyphClass::Upper,
        right: GlyphClass::Lower,
        metric_delta: -0.10,
        optical_delta: -0.04,
        ..evidence()
    };
    assert!(compact_guarded(pair) <= -0.08);
}

#[test]
fn metric_pair_cannot_change_sign() {
    let pair = PairEvidence {
        left: GlyphClass::Upper,
        right: GlyphClass::Punctuation,
        metric_delta: 0.18,
        optical_delta: -0.09,
        ..evidence()
    };
    assert_eq!(compact_guarded(pair), 0.15);
}

#[test]
fn metric_pair_correction_is_bounded_dynamically() {
    let pair = PairEvidence {
        metric_delta: -0.08,
        optical_delta: 0.08,
        ..evidence()
    };
    assert!((compact_guarded(pair) + 0.05).abs() < 0.0001);
}

#[test]
fn sub_dead_zone_metric_is_preserved_exactly() {
    let pair = PairEvidence {
        metric_delta: 0.002,
        optical_delta: -0.08,
        ..evidence()
    };
    assert_eq!(compact_guarded(pair), 0.002);
}

#[test]
fn sans_cap_run_tightens_repeated_strong_pairs() {
    let mut pairs = vec![
        run_pair(GlyphClass::Upper, GlyphClass::Upper, -0.07),
        run_pair(GlyphClass::Upper, GlyphClass::Upper, -0.06),
        run_pair(GlyphClass::Upper, GlyphClass::Upper, -0.08),
        run_pair(GlyphClass::Upper, GlyphClass::Upper, -0.07),
    ];
    compact_guarded_run(&mut pairs, 0.22, 0.05, 0.74);
    assert_eq!(pairs[0].delta, -0.10);
}

#[test]
fn run_adjustments_cannot_escape_metric_preservation_bound() {
    let mut pairs = vec![
        RunPair {
            delta: -0.14,
            ..run_pair(GlyphClass::Upper, GlyphClass::Upper, -0.07)
        };
        4
    ];
    compact_guarded_run(&mut pairs, 0.22, 0.05, 0.74);
    assert!(pairs.iter().all(|pair| pair.delta == -0.10));
}

#[test]
fn metricless_sans_digit_run_gets_consistent_rhythm() {
    let mut pairs = vec![run_pair(GlyphClass::Digit, GlyphClass::Digit, 0.0); 6];
    compact_guarded_run(&mut pairs, 0.22, 0.05, 0.74);
    assert!(pairs.iter().all(|pair| pair.delta <= -0.014));
}

#[test]
fn serif_cap_run_limits_accumulated_tightening() {
    let mut pairs = vec![
        run_pair(GlyphClass::Upper, GlyphClass::Upper, -0.12),
        run_pair(GlyphClass::Upper, GlyphClass::Upper, -0.11),
        run_pair(GlyphClass::Upper, GlyphClass::Upper, -0.08),
        run_pair(GlyphClass::Upper, GlyphClass::Upper, -0.07),
    ];
    compact_guarded_run(&mut pairs, 0.28, 0.05, 0.68);
    assert!(pairs[0].delta >= -0.105);
    assert!(pairs[1].delta >= -0.105);
}

#[test]
fn connected_script_run_does_not_treat_joins_as_collisions() {
    let mut pairs = vec![
        RunPair {
            min_gap: -0.08,
            delta: 0.055,
            ..run_pair(GlyphClass::Lower, GlyphClass::Lower, 0.0)
        };
        5
    ];
    compact_guarded_run(&mut pairs, 0.17, 0.054, 0.68);
    assert!(
        pairs
            .iter()
            .all(|pair| (-0.012..=0.0).contains(&pair.delta))
    );
}

#[test]
fn connected_script_ligature_run_opens_replacement_glyph_boundaries() {
    let mut pairs = vec![
        RunPair {
            min_gap: -0.08,
            ..run_pair(GlyphClass::Lower, GlyphClass::Lower, 0.0)
        };
        6
    ];
    pairs[3].right_cluster_chars = 2;
    pairs[4].left_cluster_chars = 2;
    compact_guarded_run(&mut pairs, 0.17, 0.054, 0.68);
    assert!(
        pairs
            .iter()
            .all(|pair| (pair.delta - 0.01955).abs() < 0.0001)
    );
}

#[test]
fn long_script_ligature_run_softens_multiple_metric_tightenings() {
    let mut pairs = vec![
        RunPair {
            min_gap: -0.08,
            ..run_pair(GlyphClass::Lower, GlyphClass::Lower, 0.0)
        };
        6
    ];
    pairs[0].metric_delta = -0.01;
    pairs[1].metric_delta = -0.01;
    pairs[3].right_cluster_chars = 2;
    pairs[4].left_cluster_chars = 2;
    compact_guarded_run(&mut pairs, 0.17, 0.054, 0.68);
    assert!(pairs[..2].iter().all(|pair| pair.delta == -0.006));
    assert!(
        pairs[2..]
            .iter()
            .all(|pair| (pair.delta - 0.0153).abs() < 0.0001)
    );
}

#[test]
fn sans_lowercase_run_relaxes_accumulated_pair_tightening() {
    let mut pairs = vec![run_pair(GlyphClass::Lower, GlyphClass::Lower, 0.0); 5];
    for pair in &mut pairs {
        pair.delta = -0.018;
    }
    pairs[1].metric_delta = -0.039;
    pairs[1].delta = -0.039;
    pairs[4].optical_delta = -0.033;
    pairs[4].delta = -0.052;
    compact_guarded_run(&mut pairs, 0.217, 0.049, 0.74);
    assert_eq!(pairs[0].delta, -0.010);
    assert_eq!(pairs[1].delta, -0.039);
    assert_eq!(pairs[4].delta, -0.010);
}

#[test]
fn wide_serif_ligature_run_respects_cluster_entries_and_tight_gaps() {
    let mut pairs = vec![run_pair(GlyphClass::Lower, GlyphClass::Lower, 0.0); 6];
    pairs[0].right_cluster_chars = 3;
    pairs[1].left_cluster_chars = 3;
    pairs[2].min_gap = 0.08;
    for pair in &mut pairs {
        pair.delta = -0.013;
    }
    compact_guarded_run(&mut pairs, 0.271, 0.052, 0.68);
    assert_eq!(pairs[0].delta, -0.013);
    assert_eq!(pairs[2].delta, 0.0);
    assert!((pairs[3].delta + 0.026558).abs() < 0.0001);
}

#[test]
fn short_connected_script_lowercase_run_compacts_metricless_joins() {
    let mut pairs = vec![
        RunPair {
            min_gap: -0.08,
            ..run_pair(GlyphClass::Lower, GlyphClass::Lower, 0.0)
        };
        4
    ];
    compact_guarded_run(&mut pairs, 0.158, 0.050, 0.68);
    assert!(
        pairs
            .iter()
            .all(|pair| (pair.delta + 0.01027).abs() < 0.0001)
    );
}

#[test]
fn short_script_compaction_ignores_mixed_case_runs() {
    let mut pairs = vec![
        run_pair(GlyphClass::Upper, GlyphClass::Lower, 0.0),
        RunPair {
            min_gap: -0.08,
            ..run_pair(GlyphClass::Lower, GlyphClass::Lower, 0.0)
        },
        RunPair {
            min_gap: -0.08,
            ..run_pair(GlyphClass::Lower, GlyphClass::Lower, 0.0)
        },
        RunPair {
            min_gap: -0.08,
            ..run_pair(GlyphClass::Lower, GlyphClass::Lower, 0.0)
        },
        RunPair {
            min_gap: -0.08,
            ..run_pair(GlyphClass::Lower, GlyphClass::Lower, 0.0)
        },
    ];
    compact_guarded_run(&mut pairs, 0.158, 0.050, 0.68);
    assert!(pairs.iter().all(|pair| pair.delta == 0.0));
}

#[test]
fn wide_serif_run_neutralizes_metric_opening_at_near_touch() {
    let mut pairs = vec![run_pair(GlyphClass::Lower, GlyphClass::Lower, 0.0); 4];
    pairs[0].metric_delta = 0.025;
    pairs[0].delta = 0.025;
    pairs[0].min_gap = 0.01;
    compact_guarded_run(&mut pairs, 0.271, 0.052, 0.68);
    assert_eq!(pairs[0].delta, 0.0125);
}

#[test]
fn wide_serif_lowercase_run_uses_font_gap_distribution() {
    let mut pairs = vec![
        RunPair {
            min_gap: 0.10,
            delta: 0.0,
            ..run_pair(GlyphClass::Lower, GlyphClass::Lower, 0.0)
        };
        4
    ];
    compact_guarded_run(&mut pairs, 0.27, 0.052, 0.68);
    assert!(pairs.iter().all(|pair| (pair.delta + 0.013).abs() < 0.0001));
}

#[test]
fn moderate_wide_serif_run_keeps_lowercase_rhythm() {
    let mut pairs = vec![
        RunPair {
            min_gap: 0.10,
            delta: 0.0,
            ..run_pair(GlyphClass::Lower, GlyphClass::Lower, 0.0)
        };
        4
    ];
    compact_guarded_run(&mut pairs, 0.23, 0.052, 0.68);
    assert!(pairs.iter().all(|pair| (pair.delta + 0.013).abs() < 0.0001));
}

#[test]
fn overlapping_script_mixed_pair_keeps_its_join() {
    let mut pairs = vec![
        RunPair {
            delta: -0.09,
            ..run_pair(GlyphClass::Upper, GlyphClass::Lower, -0.09)
        },
        RunPair {
            optical_delta: -0.08,
            delta: -0.12,
            ..run_pair(GlyphClass::Lower, GlyphClass::Upper, 0.0)
        },
        RunPair {
            delta: -0.095,
            ..run_pair(GlyphClass::Upper, GlyphClass::Lower, -0.095)
        },
        RunPair {
            optical_delta: 0.01,
            min_gap: -0.15,
            delta: 0.055,
            ..run_pair(GlyphClass::Lower, GlyphClass::Upper, 0.0)
        },
    ];
    compact_guarded_run(&mut pairs, 0.17, 0.054, 0.68);
    assert_eq!(pairs[3].delta, 0.0);
}
