use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::font::FontKit;
use crate::outline::FlattenOptions;
use crate::profile::{GapStats, ProfileConfig, gap_stats};
use crate::shape::metric_pair_delta_em;

const CALIBRATION_ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Algorithm {
    NearestContourDistance,
    ProfileWhitespace,
    AreaBalance,
    MetricPriorHybrid,
    SafeFallbackOnly,
}

impl Algorithm {
    pub const fn all() -> &'static [Algorithm] {
        &[
            Algorithm::NearestContourDistance,
            Algorithm::ProfileWhitespace,
            Algorithm::AreaBalance,
            Algorithm::MetricPriorHybrid,
            Algorithm::SafeFallbackOnly,
        ]
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Algorithm::NearestContourDistance => "nearest-contour-distance",
            Algorithm::ProfileWhitespace => "profile-whitespace",
            Algorithm::AreaBalance => "area-balance",
            Algorithm::MetricPriorHybrid => "metric-prior-hybrid",
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
    pub outputs: Vec<AlgorithmOutput>,
}

#[derive(Debug, Clone, Copy)]
pub struct EvaluationConfig {
    pub profile: ProfileConfig,
    pub target_gap_em: f32,
    pub gap_mad_em: f32,
    pub preserve_monospace: bool,
}

impl EvaluationConfig {
    pub fn for_font(font: &FontKit) -> Self {
        let profile = ProfileConfig::for_latin(font.x_height_em(), font.cap_height_em());
        let (target_gap_em, gap_mad_em) = calibrated_gap_distribution(font, profile);
        Self {
            profile,
            target_gap_em,
            gap_mad_em,
            preserve_monospace: font.is_monospaced(),
        }
    }
}

pub fn evaluate_pair(font: &FontKit, pair: &str) -> Result<AlgorithmSet> {
    evaluate_pair_with_config(font, pair, EvaluationConfig::for_font(font))
}

pub fn evaluate_pair_with_config(
    font: &FontKit,
    pair: &str,
    config: EvaluationConfig,
) -> Result<AlgorithmSet> {
    let mut chars = pair.chars();
    let left = chars
        .next()
        .ok_or_else(|| anyhow!("pair must contain at least two chars"))?;
    let right = chars
        .next()
        .ok_or_else(|| anyhow!("pair must contain at least two chars"))?;

    let flatten = FlattenOptions::default();
    let (left_metrics, left_outline) = font.outline(left, flatten)?;
    let (_right_metrics, right_outline) = font.outline(right, flatten)?;
    let stats = gap_stats(
        &left_outline,
        left_metrics.advance_em,
        &right_outline,
        config.profile,
    )
    .ok_or_else(|| anyhow!("not enough outline overlap for pair {pair:?}"))?;
    let metric_delta = metric_pair_delta_em(font, pair).unwrap_or(0.0);
    let optical_profile_delta = distribution_delta(stats.weighted_mean_gap, config);
    let optical_robust_delta = distribution_delta(stats.robust_mean_gap, config);

    let outputs = Algorithm::all()
        .iter()
        .copied()
        .map(|algorithm| {
            let delta = match algorithm {
                Algorithm::NearestContourDistance => {
                    nearest_distance_delta(stats, config.target_gap_em)
                }
                Algorithm::ProfileWhitespace => optical_profile_delta,
                Algorithm::AreaBalance => optical_robust_delta,
                Algorithm::MetricPriorHybrid => {
                    if config.preserve_monospace {
                        metric_delta
                    } else {
                        metric_prior_hybrid(metric_delta, optical_robust_delta)
                    }
                }
                Algorithm::SafeFallbackOnly => {
                    if config.preserve_monospace || metric_delta.abs() >= dead_zone() {
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
                target_gap_em: config.target_gap_em,
                gap_distribution_mad_em: config.gap_mad_em,
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
        pair: pair.to_owned(),
        left,
        right,
        outputs,
    })
}

fn calibrated_gap_distribution(font: &FontKit, profile: ProfileConfig) -> (f32, f32) {
    let flatten = FlattenOptions::default();
    let mut gaps = Vec::new();
    let chars = CALIBRATION_ALPHABET.chars().collect::<Vec<_>>();
    for left in &chars {
        for right in &chars {
            let Ok((left_metrics, left_outline)) = font.outline(*left, flatten) else {
                continue;
            };
            let Ok((_right_metrics, right_outline)) = font.outline(*right, flatten) else {
                continue;
            };
            if let Some(stats) = gap_stats(
                &left_outline,
                left_metrics.advance_em,
                &right_outline,
                profile,
            ) {
                gaps.push(stats.robust_mean_gap);
            }
        }
    }

    if gaps.is_empty() {
        return (default_target_gap(profile), 0.055);
    }
    gaps.sort_by(|a, b| a.total_cmp(b));
    let median = percentile(&gaps, 0.5).clamp(0.045, 0.42);
    let mut deviations = gaps
        .iter()
        .map(|gap| (gap - median).abs())
        .collect::<Vec<_>>();
    deviations.sort_by(|a, b| a.total_cmp(b));
    let mad = percentile(&deviations, 0.5).clamp(0.025, 0.16);
    (median, mad)
}

fn default_target_gap(profile: ProfileConfig) -> f32 {
    (profile.x_height * 0.13).clamp(0.055, 0.10)
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

fn metric_prior_hybrid(metric_delta: f32, optical_delta: f32) -> f32 {
    if metric_delta.abs() < dead_zone() {
        return optical_delta;
    }

    let disagreement = (optical_delta - metric_delta).abs();
    if disagreement <= 0.045 {
        metric_delta
    } else {
        normalized_delta(metric_delta + 0.35 * (optical_delta - metric_delta))
    }
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
                "safe-fallback-only",
            ]
        );
    }

    #[test]
    fn hybrid_preserves_close_metric_delta() {
        assert!((metric_prior_hybrid(-0.04, -0.05) + 0.04).abs() < 0.001);
    }

    #[test]
    fn hybrid_uses_optical_when_metric_missing() {
        assert!((metric_prior_hybrid(0.0, -0.06) + 0.06).abs() < 0.001);
    }
}
