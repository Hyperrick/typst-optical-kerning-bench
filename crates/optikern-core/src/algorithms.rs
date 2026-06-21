use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use crate::font::FontKit;
use crate::outline::FlattenOptions;
use crate::profile::{GapStats, ProfileConfig, gap_stats};
use crate::shape::metric_pair_delta_em;

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

pub fn evaluate_pair(font: &FontKit, pair: &str) -> Result<AlgorithmSet> {
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
        ProfileConfig::default(),
    )
    .ok_or_else(|| anyhow!("not enough outline overlap for pair {pair:?}"))?;
    let metric_delta = metric_pair_delta_em(font, pair).unwrap_or(0.0);
    let optical_profile_delta = clamp_delta(target_gap() - stats.weighted_mean_gap);

    let outputs = Algorithm::all()
        .iter()
        .copied()
        .map(|algorithm| {
            let delta = match algorithm {
                Algorithm::NearestContourDistance => nearest_distance_delta(stats),
                Algorithm::ProfileWhitespace => optical_profile_delta,
                Algorithm::AreaBalance => clamp_delta(target_gap() - stats.robust_mean_gap),
                Algorithm::MetricPriorHybrid => {
                    metric_prior_hybrid(metric_delta, optical_profile_delta)
                }
                Algorithm::SafeFallbackOnly => {
                    if metric_delta.abs() >= 0.01 {
                        metric_delta
                    } else {
                        optical_profile_delta
                    }
                }
            };
            AlgorithmOutput {
                algorithm,
                delta_em: delta,
                metric_delta_em: metric_delta,
                optical_delta_em: optical_profile_delta,
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

fn nearest_distance_delta(stats: GapStats) -> f32 {
    let desired_min = 0.025;
    if stats.min_gap < desired_min {
        clamp_delta(desired_min - stats.min_gap)
    } else {
        clamp_delta((target_gap() - stats.min_gap) * 0.55)
    }
}

fn metric_prior_hybrid(metric_delta: f32, optical_delta: f32) -> f32 {
    if metric_delta.abs() < 0.01 {
        return optical_delta;
    }

    let disagreement = (optical_delta - metric_delta).abs();
    if disagreement <= 0.08 {
        metric_delta
    } else {
        clamp_delta(metric_delta + 0.35 * (optical_delta - metric_delta))
    }
}

fn target_gap() -> f32 {
    0.065
}

fn clamp_delta(value: f32) -> f32 {
    value.clamp(-0.18, 0.18)
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
