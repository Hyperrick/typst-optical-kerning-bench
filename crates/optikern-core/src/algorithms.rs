use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use ttf_parser::GlyphId;

use crate::calibration::{ClassGapCalibration, GapDistribution, calibrated_gap_distribution};
use crate::class::PairClass;
use crate::font::FontKit;
use crate::outline::{FlattenOptions, GlyphOutline, LineSegment};
use crate::profile::{GapStats, ProfileConfig, gap_stats};
use crate::shape::{ShapedGlyphPair, ShapingOptions, metric_shaped_pair_delta_em, shape_text};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Algorithm {
    NearestContourDistance,
    ProfileWhitespace,
    AreaBalance,
    MetricPriorHybrid,
    GuardedProfileHybrid,
    SafeFallbackOnly,
}

impl Algorithm {
    pub const fn all() -> &'static [Algorithm] {
        &[
            Algorithm::NearestContourDistance,
            Algorithm::ProfileWhitespace,
            Algorithm::AreaBalance,
            Algorithm::MetricPriorHybrid,
            Algorithm::GuardedProfileHybrid,
            Algorithm::SafeFallbackOnly,
        ]
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Algorithm::NearestContourDistance => "nearest-contour-distance",
            Algorithm::ProfileWhitespace => "profile-whitespace",
            Algorithm::AreaBalance => "area-balance",
            Algorithm::MetricPriorHybrid => "metric-prior-hybrid",
            Algorithm::GuardedProfileHybrid => "guarded-profile-hybrid",
            Algorithm::SafeFallbackOnly => "safe-fallback-only",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmOutput {
    pub algorithm: Algorithm,
    pub delta_em: f32,
    pub metric_delta_em: f32,
    pub optical_delta_em: f32,
    #[serde(default)]
    pub target_gap_em: f32,
    #[serde(default)]
    pub gap_distribution_mad_em: f32,
    pub gap_min_em: f32,
    pub gap_weighted_mean_em: f32,
    pub gap_robust_mean_em: f32,
    pub gap_mad_em: f32,
    pub samples: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmSet {
    pub font_id: String,
    pub pair: String,
    pub left: char,
    pub right: char,
    #[serde(default)]
    pub display: String,
    #[serde(default)]
    pub shaping_text: String,
    #[serde(default)]
    pub left_glyph_id: u16,
    #[serde(default)]
    pub right_glyph_id: u16,
    #[serde(default)]
    pub left_cluster: String,
    #[serde(default)]
    pub right_cluster: String,
    pub outputs: Vec<AlgorithmOutput>,
}

#[derive(Debug, Clone, Copy)]
pub struct EvaluationConfig {
    pub profile: ProfileConfig,
    pub target_gap_em: f32,
    pub gap_mad_em: f32,
    pub preserve_monospace: bool,
    class_gap_calibration: ClassGapCalibration,
}

impl EvaluationConfig {
    pub fn for_font(font: &FontKit) -> Self {
        let profile = ProfileConfig::for_latin(font.x_height_em(), font.cap_height_em());
        let (global, class_gap_calibration) = calibrated_gap_distribution(font, profile);
        Self {
            profile,
            target_gap_em: global.target_gap_em,
            gap_mad_em: global.gap_mad_em,
            preserve_monospace: font.is_monospaced(),
            class_gap_calibration,
        }
    }

    fn for_pair_class(self, pair_class: PairClass) -> Self {
        if !pair_class.uses_class_gap_calibration() {
            return self;
        }

        let fallback = GapDistribution {
            target_gap_em: self.target_gap_em,
            gap_mad_em: self.gap_mad_em,
            sample_count: 0,
        };
        let distribution = self
            .class_gap_calibration
            .distribution(pair_class, fallback);
        let weight = pair_class.class_gap_calibration_weight();
        Self {
            target_gap_em: blend(self.target_gap_em, distribution.target_gap_em, weight),
            gap_mad_em: blend(self.gap_mad_em, distribution.gap_mad_em, weight),
            ..self
        }
    }
}

fn blend(base: f32, target: f32, weight: f32) -> f32 {
    base + (target - base) * weight.clamp(0.0, 1.0)
}

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

fn nearest_distance_delta(stats: GapStats, target_gap: f32) -> f32 {
    let desired_min = (target_gap * 0.38).clamp(0.018, 0.040);
    if stats.min_gap < desired_min {
        normalized_delta(desired_min - stats.min_gap)
    } else {
        let config = EvaluationConfig {
            profile: ProfileConfig::default(),
            target_gap_em: target_gap,
            gap_mad_em: (target_gap * 0.32).clamp(0.025, 0.08),
            preserve_monospace: false,
            class_gap_calibration: ClassGapCalibration::empty(),
        };
        distribution_delta(stats.min_gap, config) * 0.5
    }
}

fn distribution_delta(gap: f32, config: EvaluationConfig) -> f32 {
    let spread = (config.gap_mad_em * 1.35).clamp(0.035, 0.14);
    let lower = (config.target_gap_em - spread * 1.15).max(0.020);
    let upper = config.target_gap_em + spread;
    if gap > upper {
        normalized_delta((upper - gap) * 0.85)
    } else if gap < lower {
        normalized_delta((lower - gap) * 0.65)
    } else {
        0.0
    }
}

fn metric_prior_hybrid_for_class(
    metric_delta: f32,
    optical_delta: f32,
    pair_class: PairClass,
) -> f32 {
    if metric_delta.abs() < dead_zone() {
        return zero_metric_delta(optical_delta, pair_class);
    }

    let disagreement = (optical_delta - metric_delta).abs();
    if disagreement <= 0.045 {
        metric_delta
    } else {
        let pull = metric_optical_pull(metric_delta, pair_class);
        normalized_delta(metric_delta + pull * (optical_delta - metric_delta))
    }
}

fn guarded_profile_hybrid(
    metric_delta: f32,
    optical_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
    pair_geometry: PairGeometry,
) -> f32 {
    if config.preserve_monospace {
        return metric_delta;
    }

    let proposed = metric_prior_hybrid_for_class(metric_delta, optical_delta, pair_class);
    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);
    let mut adjusted = if proposed > 0.0 && metric_delta.abs() < dead_zone() {
        if nearest_delta > nearest_guard {
            proposed
        } else {
            0.0
        }
    } else {
        proposed
    };

    if adjusted < 0.0
        && !pair_class.allows_tight_nearest_override()
        && (aperture_risk(stats, config) || nearest_delta > nearest_guard)
    {
        adjusted = if metric_delta.abs() >= dead_zone() {
            metric_delta
        } else {
            0.0
        };
    }

    adjusted += lower_upper_overhang_delta(
        metric_delta,
        optical_delta,
        adjusted,
        nearest_delta,
        stats,
        config,
        pair_class,
        pair_geometry,
    );
    adjusted += side_shape_delta(
        metric_delta,
        adjusted,
        nearest_delta,
        stats,
        config,
        pair_class,
        pair_geometry,
    );
    adjusted += collision_opening_delta(adjusted, nearest_delta, stats, config, pair_class);
    adjusted += punctuation_spacing_delta(
        metric_delta,
        adjusted,
        nearest_delta,
        stats,
        config,
        pair_class,
    );

    normalized_delta(
        adjusted
            + spacing_compaction_delta(
                metric_delta,
                adjusted,
                nearest_delta,
                stats,
                config,
                pair_class,
            ),
    )
}

fn side_shape_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
    pair_geometry: PairGeometry,
) -> f32 {
    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);

    if pair_class.is_upper_lower()
        && metric_delta < -dead_zone()
        && pair_geometry.right_left_side.roundness > 0.040
        && nearest_delta <= nearest_guard
    {
        let spread = (config.gap_mad_em * 1.35).clamp(0.035, 0.14);
        let upper = config.target_gap_em + spread;
        if stats.robust_mean_gap > upper + 0.012 {
            let target = (metric_delta * 0.94).clamp(metric_delta, adjusted_delta);
            return normalized_delta(target - adjusted_delta);
        }
    }

    if !pair_class.has_digit() {
        return 0.0;
    }

    let safe_min = if pair_class.is_digit_digit() {
        (config.target_gap_em * 0.24).clamp(0.045, 0.075)
    } else {
        (config.target_gap_em * 0.32).clamp(0.060, 0.105)
    };
    if nearest_delta > nearest_guard || stats.min_gap <= safe_min {
        return 0.0;
    }

    let target = if pair_class.is_digit_digit() {
        digit_digit_target(pair_geometry)
    } else if pair_class.is_digit_punctuation() || pair_class.is_punctuation_digit() {
        digit_punctuation_target(pair_geometry)
    } else {
        0.0
    };

    if target >= adjusted_delta {
        return 0.0;
    }

    normalized_delta(target - adjusted_delta)
}

fn collision_opening_delta(
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
) -> f32 {
    if !pair_class.allows_collision_opening() {
        return 0.0;
    }

    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);
    if stats.min_gap > 0.0 || nearest_delta <= nearest_guard {
        return 0.0;
    }

    let penetration = (-stats.min_gap).max(0.0);
    let target = (nearest_delta * 0.78 + penetration * 0.22).clamp(nearest_guard, 0.055);
    if target <= adjusted_delta {
        return 0.0;
    }

    normalized_delta(target - adjusted_delta)
}

fn punctuation_spacing_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
) -> f32 {
    if !pair_class.is_upper_punctuation() || metric_delta >= -dead_zone() {
        return 0.0;
    }

    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);
    let safe_min = (config.target_gap_em * 0.65).clamp(0.12, 0.18);
    if nearest_delta > nearest_guard || stats.min_gap <= safe_min {
        return 0.0;
    }

    let base = (config.gap_mad_em * 0.46).clamp(0.018, 0.035);
    let gap_excess = (stats.robust_mean_gap - config.target_gap_em).max(0.0);
    let gap_bonus = (gap_excess * 0.12).clamp(0.0, 0.014);
    let target =
        (metric_delta.min(adjusted_delta) - base - gap_bonus).clamp(-0.120, adjusted_delta);

    normalized_delta(target - adjusted_delta)
}

fn digit_digit_target(pair_geometry: PairGeometry) -> f32 {
    let left = pair_geometry.left_right_side;
    let right = pair_geometry.right_left_side;
    let stem_round = left.stemness > 0.62 && right.is_round_like();
    let round_stem = left.is_round_like() && right.stemness > 0.62;
    let round_round = left.is_round_like() && right.is_round_like();

    if stem_round || round_stem {
        -0.040
    } else if round_round {
        -0.024
    } else {
        0.0
    }
}

fn digit_punctuation_target(pair_geometry: PairGeometry) -> f32 {
    let digit_side = if pair_geometry.left_right_side.has_shape() {
        pair_geometry.left_right_side
    } else {
        pair_geometry.right_left_side
    };

    if digit_side.is_round_like() || digit_side.stemness > 0.62 {
        -0.010
    } else {
        0.0
    }
}

fn lower_upper_overhang_delta(
    metric_delta: f32,
    optical_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
    pair_geometry: PairGeometry,
) -> f32 {
    if !pair_class.is_lower_upper()
        || metric_delta.abs() >= dead_zone()
        || optical_delta >= -dead_zone()
        || adjusted_delta <= -0.090
    {
        return 0.0;
    }

    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);
    let safe_min = (config.target_gap_em * 0.58).clamp(0.10, 0.18);
    if nearest_delta > nearest_guard || stats.min_gap <= safe_min || aperture_risk(stats, config) {
        return 0.0;
    }

    let overhang = pair_geometry.right_top_left_overhang;
    if overhang <= 0.10 {
        return 0.0;
    }

    let spread = (config.gap_mad_em * 1.35).clamp(0.035, 0.14);
    let upper = config.target_gap_em + spread;
    let gap_excess = (stats.robust_mean_gap - upper).max(0.0);
    let shape_bonus = ((overhang - 0.10) * 0.24).clamp(0.0, 0.040);
    let gap_bonus = (gap_excess * 0.40).clamp(0.0, 0.030);
    let round_bonus = if pair_geometry.left_right_side.is_round_like() && overhang > 0.18 {
        let curvature_bonus =
            ((pair_geometry.left_right_side.roundness - 0.030) * 0.70).clamp(0.0, 0.024);
        let overhang_bonus = ((overhang - 0.18) * 0.16).clamp(0.0, 0.020);
        (curvature_bonus + overhang_bonus).clamp(0.0, 0.034)
    } else {
        0.0
    };
    let lower_bound = if pair_geometry.left_right_side.is_round_like() && overhang > 0.18 {
        -0.120
    } else {
        -0.095
    };
    let target =
        (adjusted_delta - shape_bonus - gap_bonus - round_bonus).clamp(lower_bound, adjusted_delta);
    normalized_delta(target - adjusted_delta)
}

fn metric_optical_pull(metric_delta: f32, pair_class: PairClass) -> f32 {
    if metric_delta < -dead_zone() {
        if pair_class.is_upper_lower()
            || pair_class.is_upper_punctuation()
            || pair_class.is_upper_upper()
        {
            return 0.25;
        }
    }
    0.80
}

fn zero_metric_delta(optical_delta: f32, pair_class: PairClass) -> f32 {
    if pair_class.is_upper_digit() {
        return normalized_delta(optical_delta.clamp(-0.055, 0.030));
    }

    if pair_class.is_upper_upper() {
        return normalized_delta(optical_delta.clamp(-0.070, 0.030));
    }

    if pair_class.is_digit_punctuation() || pair_class.is_punctuation_digit() {
        return normalized_delta(optical_delta.clamp(-0.035, 0.035));
    }

    if pair_class.is_digit_digit() {
        return normalized_delta(optical_delta.min(0.0).max(-0.055));
    }

    if pair_class.is_lower_upper() {
        return normalized_delta(optical_delta.clamp(-0.060, 0.030));
    }

    optical_delta
}

fn aperture_risk(stats: GapStats, config: EvaluationConfig) -> bool {
    if stats.min_gap <= 0.0 {
        return false;
    }

    let close_min = (config.target_gap_em * 0.42).clamp(0.040, 0.120);
    let spread = (config.gap_mad_em * 1.35).clamp(0.035, 0.14);
    let upper = config.target_gap_em + spread;
    let mean_to_min_ratio = stats.robust_mean_gap / stats.min_gap;

    stats.min_gap <= close_min && stats.robust_mean_gap > upper && mean_to_min_ratio >= 3.2
}

fn spacing_compaction_delta(
    metric_delta: f32,
    adjusted_delta: f32,
    nearest_delta: f32,
    stats: GapStats,
    config: EvaluationConfig,
    pair_class: PairClass,
) -> f32 {
    if !pair_class.allows_safe_compaction() {
        return 0.0;
    }

    if adjusted_delta.abs() >= 0.045 {
        return 0.0;
    }

    if adjusted_delta < -dead_zone() && metric_delta.abs() < dead_zone() {
        return 0.0;
    }

    let nearest_guard = (config.target_gap_em * 0.08).clamp(0.012, 0.020);
    if nearest_delta > nearest_guard || aperture_risk(stats, config) {
        return 0.0;
    }

    let safe_min = (config.target_gap_em * 0.22).clamp(0.045, 0.065);
    if stats.min_gap <= safe_min {
        return 0.0;
    }

    let amount = (config.gap_mad_em * 0.25).clamp(0.008, 0.016);
    -amount
}

#[derive(Debug, Clone, Copy, Default)]
struct PairGeometry {
    right_top_left_overhang: f32,
    left_right_side: SideFeatures,
    right_left_side: SideFeatures,
}

impl PairGeometry {
    fn from_outlines(left: &GlyphOutline, right: &GlyphOutline, config: ProfileConfig) -> Self {
        Self {
            right_top_left_overhang: top_left_overhang(right, config),
            left_right_side: SideFeatures::from_outline(left, Side::Right),
            right_left_side: SideFeatures::from_outline(right, Side::Left),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct SideFeatures {
    roundness: f32,
    stemness: f32,
}

impl SideFeatures {
    fn from_outline(outline: &GlyphOutline, side: Side) -> Self {
        let min_y = outline.bounds.min_y.max(0.0);
        let max_y = outline.bounds.max_y.min(1.05);
        let height = max_y - min_y;
        if height <= 0.08 {
            return Self::default();
        }

        let lower_end = min_y + height * 0.28;
        let middle_start = min_y + height * 0.34;
        let middle_end = min_y + height * 0.66;
        let upper_start = min_y + height * 0.72;

        let Some(lower) = sampled_edge(&outline.segments, side, min_y, lower_end, 16, 0.50) else {
            return Self::default();
        };
        let Some(middle) =
            sampled_edge(&outline.segments, side, middle_start, middle_end, 16, 0.50)
        else {
            return Self::default();
        };
        let Some(upper) = sampled_edge(&outline.segments, side, upper_start, max_y, 16, 0.50)
        else {
            return Self::default();
        };
        let Some(p10) = sampled_edge(&outline.segments, side, min_y, max_y, 32, 0.10) else {
            return Self::default();
        };
        let Some(p90) = sampled_edge(&outline.segments, side, min_y, max_y, 32, 0.90) else {
            return Self::default();
        };

        let end_average = (lower + upper) * 0.5;
        let roundness = match side {
            Side::Left => (end_average - middle).max(0.0),
            Side::Right => (middle - end_average).max(0.0),
        };
        let spread = (p90 - p10).abs();
        let stemness = (1.0 - (spread / 0.085)).clamp(0.0, 1.0);

        Self {
            roundness,
            stemness,
        }
    }

    fn has_shape(self) -> bool {
        self.roundness > 0.0 || self.stemness > 0.0
    }

    fn is_round_like(self) -> bool {
        self.roundness > 0.035 || self.stemness < 0.45
    }
}

#[derive(Debug, Clone, Copy)]
enum Side {
    Left,
    Right,
}

fn top_left_overhang(outline: &GlyphOutline, config: ProfileConfig) -> f32 {
    let lower_start = 0.04;
    let lower_end = (config.x_height * 0.78).clamp(0.20, config.cap_height * 0.82);
    let upper_start = (config.x_height * 0.95).clamp(0.34, config.cap_height * 0.92);
    let upper_end = config.cap_height.clamp(upper_start + 0.02, 1.05);
    let Some(lower_left) = sampled_left_edge(&outline.segments, lower_start, lower_end, 24, 0.50)
    else {
        return 0.0;
    };
    let Some(upper_left) = sampled_left_edge(&outline.segments, upper_start, upper_end, 24, 0.15)
    else {
        return 0.0;
    };
    (lower_left - upper_left).max(0.0)
}

fn sampled_left_edge(
    segments: &[LineSegment],
    min_y: f32,
    max_y: f32,
    slices: usize,
    percentile_index: f32,
) -> Option<f32> {
    if max_y <= min_y {
        return None;
    }

    let mut xs = Vec::new();
    for index in 0..slices.max(1) {
        let t = (index as f32 + 0.5) / slices.max(1) as f32;
        let y = min_y + t * (max_y - min_y);
        if let Some(x) = leftmost_intersection(segments, y) {
            xs.push(x);
        }
    }

    if xs.is_empty() {
        return None;
    }
    xs.sort_by(|a, b| a.total_cmp(b));
    Some(percentile(&xs, percentile_index))
}

fn sampled_edge(
    segments: &[LineSegment],
    side: Side,
    min_y: f32,
    max_y: f32,
    slices: usize,
    percentile_index: f32,
) -> Option<f32> {
    if max_y <= min_y {
        return None;
    }

    let mut xs = Vec::new();
    for index in 0..slices.max(1) {
        let t = (index as f32 + 0.5) / slices.max(1) as f32;
        let y = min_y + t * (max_y - min_y);
        let x = match side {
            Side::Left => leftmost_intersection(segments, y),
            Side::Right => rightmost_intersection(segments, y),
        };
        if let Some(x) = x {
            xs.push(x);
        }
    }

    if xs.is_empty() {
        return None;
    }
    xs.sort_by(|a, b| a.total_cmp(b));
    Some(percentile(&xs, percentile_index))
}

fn leftmost_intersection(segments: &[LineSegment], y: f32) -> Option<f32> {
    let mut min_x = None;
    for segment in segments {
        let y1 = segment.start.y;
        let y2 = segment.end.y;
        if (y2 - y1).abs() < f32::EPSILON {
            continue;
        }
        let crosses = (y1 <= y && y < y2) || (y2 <= y && y < y1);
        if !crosses {
            continue;
        }
        let t = (y - y1) / (y2 - y1);
        let x = segment.start.x + t * (segment.end.x - segment.start.x);
        min_x = Some(match min_x {
            Some(current) if current < x => current,
            _ => x,
        });
    }
    min_x
}

fn rightmost_intersection(segments: &[LineSegment], y: f32) -> Option<f32> {
    let mut max_x = None;
    for segment in segments {
        let y1 = segment.start.y;
        let y2 = segment.end.y;
        if (y2 - y1).abs() < f32::EPSILON {
            continue;
        }
        let crosses = (y1 <= y && y < y2) || (y2 <= y && y < y1);
        if !crosses {
            continue;
        }
        let t = (y - y1) / (y2 - y1);
        let x = segment.start.x + t * (segment.end.x - segment.start.x);
        max_x = Some(match max_x {
            Some(current) if current > x => current,
            _ => x,
        });
    }
    max_x
}

fn normalized_delta(value: f32) -> f32 {
    let clamped = clamp_delta(value);
    if clamped.abs() < dead_zone() {
        0.0
    } else {
        clamped
    }
}

fn dead_zone() -> f32 {
    0.006
}

fn clamp_delta(value: f32) -> f32 {
    value.clamp(-0.16, 0.16)
}

fn percentile(values: &[f32], p: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let idx = ((values.len() - 1) as f32 * p.clamp(0.0, 1.0)).round() as usize;
    values[idx]
}

#[cfg(test)]
mod tests {
    use crate::class::ClusterClass;
    use crate::outline::{Bounds, Point};

    use super::*;

    fn test_config(target_gap_em: f32, gap_mad_em: f32) -> EvaluationConfig {
        EvaluationConfig {
            profile: ProfileConfig::default(),
            target_gap_em,
            gap_mad_em,
            preserve_monospace: false,
            class_gap_calibration: ClassGapCalibration::empty(),
        }
    }

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
            (metric_prior_hybrid_for_class(-0.04, -0.05, PairClass::default()) + 0.04).abs()
                < 0.001
        );
    }

    #[test]
    fn hybrid_uses_optical_when_metric_missing() {
        assert!(
            (metric_prior_hybrid_for_class(0.0, -0.06, PairClass::default()) + 0.06).abs() < 0.001
        );
    }

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

    #[test]
    fn detects_top_left_overhang() {
        let outline = glyph_from_rects(&[(0.00, 0.60, 0.62, 0.72), (0.26, 0.00, 0.36, 0.60)]);
        let config = ProfileConfig::default();
        assert!(top_left_overhang(&outline, config) > 0.20);
    }

    #[test]
    fn ignores_plain_left_edge_as_overhang() {
        let outline = glyph_from_rects(&[(0.00, 0.00, 0.42, 0.72)]);
        let config = ProfileConfig::default();
        assert_eq!(top_left_overhang(&outline, config), 0.0);
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
    fn detects_round_side_features() {
        let outline = glyph_from_polygon(&[
            (0.22, 0.00),
            (0.04, 0.18),
            (0.00, 0.36),
            (0.04, 0.54),
            (0.22, 0.72),
            (0.48, 0.72),
            (0.66, 0.54),
            (0.70, 0.36),
            (0.66, 0.18),
            (0.48, 0.00),
        ]);
        let left = SideFeatures::from_outline(&outline, Side::Left);
        let right = SideFeatures::from_outline(&outline, Side::Right);

        assert!(left.roundness > 0.10);
        assert!(right.roundness > 0.10);
        assert!(left.stemness < 0.45);
        assert!(right.stemness < 0.45);
    }

    #[test]
    fn detects_stem_side_features() {
        let outline = glyph_from_rects(&[(0.20, 0.00, 0.32, 0.72)]);
        let left = SideFeatures::from_outline(&outline, Side::Left);
        let right = SideFeatures::from_outline(&outline, Side::Right);

        assert!(left.stemness > 0.90);
        assert!(right.stemness > 0.90);
        assert!(left.roundness < 0.01);
        assert!(right.roundness < 0.01);
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
}
