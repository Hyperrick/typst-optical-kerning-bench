use anyhow::{Result, anyhow};
use ttf_parser::GlyphId;

use crate::class::PairClass;
use crate::font::FontKit;
use crate::outline::FlattenOptions;
use crate::profile::gap_stats;
use crate::shape::{ShapedGlyphPair, ShapingOptions, metric_shaped_pair_delta_em, shape_text};

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
    let metric_delta = metric_shaped_pair_delta_em(font, pair, ligatures).unwrap_or(0.0);
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
        left_glyph_id: pair.left_glyph_id,
        right_glyph_id: pair.right_glyph_id,
        left_cluster: pair.left_cluster.clone(),
        right_cluster: pair.right_cluster.clone(),
        outputs,
    })
}

pub fn evaluate_shaped_run_with_config(
    font: &FontKit,
    run: &crate::shape::ShapedRun,
    config: EvaluationConfig,
    ligatures: bool,
) -> Result<Vec<AlgorithmSet>> {
    let mut results = run
        .adjacent_pairs()
        .into_iter()
        .map(|pair| evaluate_shaped_pair_with_config(font, &pair, config, ligatures))
        .collect::<Result<Vec<_>>>()?;
    apply_run_context_adjustments(&mut results, config);
    Ok(results)
}
