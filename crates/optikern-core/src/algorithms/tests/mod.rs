use crate::calibration::ClassGapCalibration;
use crate::class::{ClusterClass, PairClass};
use crate::outline::{Bounds, GlyphOutline, LineSegment, Point};
use crate::profile::{GapStats, ProfileConfig};

use super::basic::metric_prior_hybrid_for_class;
use super::capital_context::{CapitalRunContext, serif_cap_run_delta};
use super::constraints::DeltaPlan;
use super::geometry::{PairGeometry, Side, SideFeatures, top_left_overhang};
use super::guarded::{
    collision_opening_delta, guarded_profile_hybrid, punctuation_spacing_delta,
    sans_lowercase_compaction_delta, side_shape_delta, suppress_false_diagonal_opening,
    wide_serif_display_delta,
};
use super::run_context::{
    RunContext, ScriptResidualBalance, apply_run_context_adjustments, connected_script_delta,
    script_lower_run_delta, script_mixed_case_delta, script_residual_balance_delta,
    script_upper_run_delta,
};
use super::sans_context::sans_run_context_delta;
use super::{Algorithm, AlgorithmOutput, AlgorithmSet, EvaluationConfig};

mod basic;
mod capital_context;
mod constraints;
mod digit_context;
mod geometry;
mod guarded;
mod run_context;

fn test_config(target_gap_em: f32, gap_mad_em: f32) -> EvaluationConfig {
    EvaluationConfig {
        profile: ProfileConfig::default(),
        target_gap_em,
        gap_mad_em,
        preserve_monospace: false,
        class_gap_calibration: ClassGapCalibration::empty(),
    }
}

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

fn glyph_from_rects(rects: &[(f32, f32, f32, f32)]) -> GlyphOutline {
    let mut segments = Vec::new();
    let mut bounds = Bounds {
        min_x: f32::INFINITY,
        min_y: f32::INFINITY,
        max_x: f32::NEG_INFINITY,
        max_y: f32::NEG_INFINITY,
    };

    for &(min_x, min_y, max_x, max_y) in rects {
        let p1 = Point { x: min_x, y: min_y };
        let p2 = Point { x: max_x, y: min_y };
        let p3 = Point { x: max_x, y: max_y };
        let p4 = Point { x: min_x, y: max_y };
        segments.extend([
            LineSegment { start: p1, end: p2 },
            LineSegment { start: p2, end: p3 },
            LineSegment { start: p3, end: p4 },
            LineSegment { start: p4, end: p1 },
        ]);
        bounds.min_x = bounds.min_x.min(min_x);
        bounds.min_y = bounds.min_y.min(min_y);
        bounds.max_x = bounds.max_x.max(max_x);
        bounds.max_y = bounds.max_y.max(max_y);
    }

    GlyphOutline { segments, bounds }
}

fn glyph_from_polygon(points: &[(f32, f32)]) -> GlyphOutline {
    let mut segments = Vec::new();
    let mut bounds = Bounds {
        min_x: f32::INFINITY,
        min_y: f32::INFINITY,
        max_x: f32::NEG_INFINITY,
        max_y: f32::NEG_INFINITY,
    };
    let polygon = points
        .iter()
        .map(|&(x, y)| {
            bounds.min_x = bounds.min_x.min(x);
            bounds.min_y = bounds.min_y.min(y);
            bounds.max_x = bounds.max_x.max(x);
            bounds.max_y = bounds.max_y.max(y);
            Point { x, y }
        })
        .collect::<Vec<_>>();
    for window in polygon.windows(2) {
        segments.push(LineSegment {
            start: window[0],
            end: window[1],
        });
    }
    if let (Some(first), Some(last)) = (polygon.first(), polygon.last()) {
        segments.push(LineSegment {
            start: *last,
            end: *first,
        });
    }

    GlyphOutline { segments, bounds }
}
