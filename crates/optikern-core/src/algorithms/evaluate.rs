use anyhow::{Result, anyhow};
use optikern_runtime::{
    GlyphClass, PairEvidence, RunPair, SideShape, compact_guarded, compact_guarded_run,
};
use ttf_parser::GlyphId;

use crate::class::{ClusterClass, PairClass};
use crate::font::FontKit;
use crate::outline::FlattenOptions;
use crate::profile::gap_stats;
use crate::shape::{
    ShapedGlyphPair, ShapingOptions, metric_shaped_pair_delta_em, metric_shaped_run_pair_deltas_em,
    shape_text,
};

use super::basic::{distribution_delta, metric_prior_hybrid_for_class, nearest_distance_delta};
use super::geometry::PairGeometry;
use super::guarded::guarded_profile_hybrid;
use super::math::dead_zone;
use super::run_context::apply_run_context_adjustments;
use super::types::{Algorithm, AlgorithmOutput, AlgorithmSet, EvaluationConfig};

pub fn evaluate_pair(font: &FontKit, pair: &str) -> Result<AlgorithmSet> {
    evaluate_pair_with_config(font, pair, EvaluationConfig::for_font(font))
}

pub fn evaluate_pair_with_config(
    font: &FontKit,
    pair: &str,
    config: EvaluationConfig,
) -> Result<AlgorithmSet> {
    let run = shape_text(font, pair, ShapingOptions::typst_pair())?;
    let shaped_pair = run
        .adjacent_pairs()
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("pair {pair:?} did not shape to an adjacent glyph pair"))?
        .with_key(pair);
    evaluate_shaped_pair_with_config(font, &shaped_pair, config, false)
}

pub fn evaluate_shaped_pair_with_config(
    font: &FontKit,
    pair: &ShapedGlyphPair,
    config: EvaluationConfig,
    ligatures: bool,
) -> Result<AlgorithmSet> {
    let metric_delta = metric_shaped_pair_delta_em(font, pair, ligatures).unwrap_or(0.0);
    evaluate_shaped_pair_with_metric_delta(font, pair, config, metric_delta)
}

fn evaluate_shaped_pair_with_metric_delta(
    font: &FontKit,
    pair: &ShapedGlyphPair,
    config: EvaluationConfig,
    metric_delta: f32,
) -> Result<AlgorithmSet> {
    let flatten = FlattenOptions::default();
    let (_left_metrics, left_outline) = font.outline_by_id(GlyphId(pair.left_glyph_id), flatten)?;
    let (_right_metrics, right_outline) =
        font.outline_by_id(GlyphId(pair.right_glyph_id), flatten)?;
    let stats = gap_stats(
        &left_outline,
        pair.left_advance_em,
        &right_outline,
        config.profile,
    )
    .ok_or_else(|| anyhow!("not enough outline overlap for pair {:?}", pair.display))?;
    let pair_class = PairClass::from_pair(pair);
    let pair_config = config.for_pair_class(pair_class);
    let optical_profile_delta = distribution_delta(stats.weighted_mean_gap, pair_config);
    let optical_robust_delta = distribution_delta(stats.robust_mean_gap, pair_config);
    let nearest_delta = nearest_distance_delta(stats, pair_config.target_gap_em);
    let pair_geometry =
        PairGeometry::from_outlines(&left_outline, &right_outline, pair_config.profile);

    let outputs = Algorithm::all()
        .iter()
        .copied()
        .map(|algorithm| {
            let delta = match algorithm {
                Algorithm::NearestContourDistance => nearest_delta,
                Algorithm::ProfileWhitespace => optical_profile_delta,
                Algorithm::AreaBalance => optical_robust_delta,
                Algorithm::MetricPriorHybrid => {
                    if pair_config.preserve_monospace {
                        metric_delta
                    } else {
                        metric_prior_hybrid_for_class(
                            metric_delta,
                            optical_robust_delta,
                            pair_class,
                        )
                    }
                }
                Algorithm::CompactGuarded => compact_guarded(runtime_evidence(
                    metric_delta,
                    optical_robust_delta,
                    nearest_delta,
                    stats,
                    pair_config,
                    pair_class,
                    pair_geometry,
                )),
                Algorithm::GuardedProfileHybrid => guarded_profile_hybrid(
                    metric_delta,
                    optical_robust_delta,
                    nearest_delta,
                    stats,
                    pair_config,
                    pair_class,
                    pair_geometry,
                ),
                Algorithm::SafeFallbackOnly => {
                    if pair_config.preserve_monospace || metric_delta.abs() >= dead_zone() {
                        metric_delta
                    } else {
                        optical_robust_delta
                    }
                }
            };
            AlgorithmOutput {
                algorithm,
                delta_em: delta,
                metric_delta_em: metric_delta,
                optical_delta_em: optical_profile_delta,
                target_gap_em: pair_config.target_gap_em,
                gap_distribution_mad_em: pair_config.gap_mad_em,
                gap_min_em: stats.min_gap,
                gap_weighted_mean_em: stats.weighted_mean_gap,
                gap_robust_mean_em: stats.robust_mean_gap,
                gap_mad_em: stats.mad,
                samples: stats.samples,
            }
        })
        .collect();

    Ok(AlgorithmSet {
        font_id: font.id().to_owned(),
        pair: pair.key.clone(),
        left: pair.left_cluster.chars().next().unwrap_or(' '),
        right: pair.right_cluster.chars().next().unwrap_or(' '),
        display: pair.display.clone(),
        shaping_text: pair.shaping_text.clone(),
        left_index: pair.left_index,
        right_index: pair.right_index,
        left_glyph_id: pair.left_glyph_id,
        right_glyph_id: pair.right_glyph_id,
        left_cluster: pair.left_cluster.clone(),
        right_cluster: pair.right_cluster.clone(),
        outputs,
    })
}

fn runtime_evidence(
    metric_delta: f32,
    optical_delta: f32,
    nearest_delta: f32,
    stats: crate::profile::GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
    geometry: PairGeometry,
) -> PairEvidence {
    PairEvidence {
        left: runtime_class(pair_class.left),
        right: runtime_class(pair_class.right),
        metric_delta,
        optical_delta,
        nearest_delta,
        target_gap: config.target_gap_em,
        gap_mad: config.gap_mad_em,
        min_gap: stats.min_gap,
        robust_gap: stats.robust_mean_gap,
        x_height: config.profile.x_height,
        cap_height: config.profile.cap_height,
        left_side: SideShape {
            roundness: geometry.left_right_side.roundness,
            stemness: geometry.left_right_side.stemness,
        },
        right_side: SideShape {
            roundness: geometry.right_left_side.roundness,
            stemness: geometry.right_left_side.stemness,
        },
        right_top_left_overhang: geometry.right_top_left_overhang,
        monospaced: config.preserve_monospace,
    }
}

fn runtime_class(class: ClusterClass) -> GlyphClass {
    match class {
        ClusterClass::Upper => GlyphClass::Upper,
        ClusterClass::Lower => GlyphClass::Lower,
        ClusterClass::Digit => GlyphClass::Digit,
        ClusterClass::Punctuation => GlyphClass::Punctuation,
        ClusterClass::Other => GlyphClass::Other,
    }
}

pub fn evaluate_shaped_run_with_config(
    font: &FontKit,
    run: &crate::shape::ShapedRun,
    config: EvaluationConfig,
    ligatures: bool,
) -> Result<Vec<AlgorithmSet>> {
    let run_metric_deltas = metric_shaped_run_pair_deltas_em(font, run, ligatures).ok();
    let pairs = run.adjacent_pairs();
    let mut results = pairs
        .into_iter()
        .enumerate()
        .map(|(index, pair)| {
            let metric_delta = run_metric_deltas
                .as_ref()
                .and_then(|deltas| deltas.get(index).copied().flatten())
                .unwrap_or_else(|| {
                    metric_shaped_pair_delta_em(font, &pair, ligatures).unwrap_or(0.0)
                });
            evaluate_shaped_pair_with_metric_delta(font, &pair, config, metric_delta)
        })
        .collect::<Result<Vec<_>>>()?;
    apply_run_context_adjustments(&mut results, config);
    apply_compact_run_adjustments(&mut results, config);
    Ok(results)
}

fn apply_compact_run_adjustments(results: &mut [AlgorithmSet], config: EvaluationConfig) {
    let mut run = results
        .iter()
        .map(|result| {
            let output = result
                .outputs
                .iter()
                .find(|output| output.algorithm == Algorithm::CompactGuarded)
                .expect("algorithm evaluation always emits compact-guarded");
            let class = result.pair_class();
            RunPair {
                left: runtime_class(class.left),
                right: runtime_class(class.right),
                left_cluster_chars: result.left_cluster.chars().count().min(u8::MAX as usize) as u8,
                right_cluster_chars: result.right_cluster.chars().count().min(u8::MAX as usize)
                    as u8,
                metric_delta: output.metric_delta_em,
                optical_delta: output.optical_delta_em,
                min_gap: output.gap_min_em,
                delta: output.delta_em,
            }
        })
        .collect::<Vec<_>>();
    let x_to_cap = if config.profile.cap_height > 0.0 {
        config.profile.x_height / config.profile.cap_height
    } else {
        1.0
    };
    compact_guarded_run(&mut run, config.target_gap_em, config.gap_mad_em, x_to_cap);

    for (result, pair) in results.iter_mut().zip(run) {
        if let Some(output) = result
            .outputs
            .iter_mut()
            .find(|output| output.algorithm == Algorithm::CompactGuarded)
        {
            output.delta_em = pair.delta;
        }
    }
}
