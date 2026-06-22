use serde::{Deserialize, Serialize};

use crate::calibration::{ClassGapCalibration, GapDistribution, calibrated_gap_distribution};
use crate::class::PairClass;
use crate::font::FontKit;
use crate::profile::ProfileConfig;

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
    pub(super) class_gap_calibration: ClassGapCalibration,
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

    pub(super) fn for_pair_class(self, pair_class: PairClass) -> Self {
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
