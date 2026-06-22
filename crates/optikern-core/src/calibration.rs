use crate::class::{CLUSTER_CLASS_COUNT, PairClass};
use crate::font::FontKit;
use crate::outline::FlattenOptions;
use crate::profile::{ProfileConfig, gap_stats};

const CALIBRATION_CHARS: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.,:;!?";
const CLASS_DISTRIBUTION_COUNT: usize = CLUSTER_CLASS_COUNT * CLUSTER_CLASS_COUNT;
const MIN_CLASS_SAMPLES: usize = 24;

#[derive(Debug, Clone, Copy)]
pub(crate) struct GapDistribution {
    pub(crate) target_gap_em: f32,
    pub(crate) gap_mad_em: f32,
    pub(crate) sample_count: usize,
}

impl GapDistribution {
    fn new(target_gap_em: f32, gap_mad_em: f32, sample_count: usize) -> Self {
        Self {
            target_gap_em,
            gap_mad_em,
            sample_count,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ClassGapCalibration {
    distributions: [Option<GapDistribution>; CLASS_DISTRIBUTION_COUNT],
}

impl ClassGapCalibration {
    pub(crate) fn empty() -> Self {
        Self {
            distributions: [None; CLASS_DISTRIBUTION_COUNT],
        }
    }

    pub(crate) fn distribution(
        self,
        pair_class: PairClass,
        fallback: GapDistribution,
    ) -> GapDistribution {
        self.distributions[pair_class.distribution_index()]
            .filter(|distribution| distribution.sample_count >= MIN_CLASS_SAMPLES)
            .unwrap_or(fallback)
    }
}

pub(crate) fn calibrated_gap_distribution(
    font: &FontKit,
    profile: ProfileConfig,
) -> (GapDistribution, ClassGapCalibration) {
    let flatten = FlattenOptions::default();
    let chars = CALIBRATION_CHARS.chars().collect::<Vec<_>>();
    let mut all_gaps = Vec::new();
    let mut class_gaps = vec![Vec::new(); CLASS_DISTRIBUTION_COUNT];

    for left in &chars {
        let Ok((left_metrics, left_outline)) = font.outline(*left, flatten) else {
            continue;
        };
        for right in &chars {
            let Ok((_right_metrics, right_outline)) = font.outline(*right, flatten) else {
                continue;
            };
            let Some(stats) = gap_stats(
                &left_outline,
                left_metrics.advance_em,
                &right_outline,
                profile,
            ) else {
                continue;
            };
            let gap = stats.robust_mean_gap;
            all_gaps.push(gap);
            class_gaps[PairClass::from_chars(*left, *right).distribution_index()].push(gap);
        }
    }

    let global = distribution_from_gaps(all_gaps, default_target_gap(profile), 0.055);
    let mut class_calibration = ClassGapCalibration::empty();
    for (index, gaps) in class_gaps.into_iter().enumerate() {
        if gaps.len() < MIN_CLASS_SAMPLES {
            continue;
        }
        class_calibration.distributions[index] = Some(distribution_from_gaps(
            gaps,
            global.target_gap_em,
            global.gap_mad_em,
        ));
    }

    (global, class_calibration)
}

fn distribution_from_gaps(
    mut gaps: Vec<f32>,
    fallback_target_gap_em: f32,
    fallback_gap_mad_em: f32,
) -> GapDistribution {
    let sample_count = gaps.len();
    if gaps.is_empty() {
        return GapDistribution::new(fallback_target_gap_em, fallback_gap_mad_em, 0);
    }

    gaps.sort_by(|a, b| a.total_cmp(b));
    let median = percentile(&gaps, 0.5).clamp(0.045, 0.42);
    let mut deviations = gaps
        .iter()
        .map(|gap| (gap - median).abs())
        .collect::<Vec<_>>();
    deviations.sort_by(|a, b| a.total_cmp(b));
    let mad = percentile(&deviations, 0.5).clamp(0.025, 0.16);
    GapDistribution::new(median, mad, sample_count)
}

fn default_target_gap(profile: ProfileConfig) -> f32 {
    (profile.x_height * 0.13).clamp(0.055, 0.10)
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
    fn class_distribution_falls_back_when_empty() {
        let fallback = GapDistribution::new(0.2, 0.05, 100);
        let class = PairClass::from_chars('A', 'V');
        let distribution = ClassGapCalibration::empty().distribution(class, fallback);
        assert_eq!(distribution.target_gap_em, fallback.target_gap_em);
        assert_eq!(distribution.gap_mad_em, fallback.gap_mad_em);
    }
}
