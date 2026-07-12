use crate::DEAD_ZONE;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphClass {
    Upper,
    Lower,
    Digit,
    Punctuation,
    Other,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SideShape {
    pub roundness: f32,
    pub stemness: f32,
}

impl SideShape {
    pub fn is_round(self) -> bool {
        self.roundness > 0.035 || self.stemness < 0.45
    }

    pub(crate) fn is_stem(self) -> bool {
        self.stemness > 0.62
    }

    pub(crate) fn is_diagonal(self) -> bool {
        self.stemness < 0.55 && self.roundness <= 0.035
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PairEvidence {
    pub left: GlyphClass,
    pub right: GlyphClass,
    pub metric_delta: f32,
    pub optical_delta: f32,
    pub nearest_delta: f32,
    pub target_gap: f32,
    pub gap_mad: f32,
    pub min_gap: f32,
    pub robust_gap: f32,
    pub x_height: f32,
    pub cap_height: f32,
    pub left_side: SideShape,
    pub right_side: SideShape,
    pub right_top_left_overhang: f32,
    pub monospaced: bool,
}

impl PairEvidence {
    pub(crate) fn is(self, left: GlyphClass, right: GlyphClass) -> bool {
        self.left == left && self.right == right
    }

    pub(crate) fn has_digit(self) -> bool {
        self.left == GlyphClass::Digit || self.right == GlyphClass::Digit
    }

    pub(crate) fn has_punctuation(self) -> bool {
        self.left == GlyphClass::Punctuation || self.right == GlyphClass::Punctuation
    }

    pub(crate) fn nearest_guard(self) -> f32 {
        (self.target_gap * 0.08).clamp(0.012, 0.020)
    }

    pub(crate) fn spread_upper(self) -> f32 {
        self.target_gap + (self.gap_mad * 1.35).clamp(0.035, 0.14)
    }

    pub(crate) fn x_to_cap(self) -> f32 {
        if self.cap_height > 0.0 {
            self.x_height / self.cap_height
        } else {
            1.0
        }
    }

    pub(crate) fn aperture_risk(self) -> bool {
        if self.min_gap <= 0.0 {
            return false;
        }
        let close_min = (self.target_gap * 0.42).clamp(0.040, 0.120);
        self.min_gap <= close_min
            && self.robust_gap > self.spread_upper()
            && self.robust_gap / self.min_gap >= 3.2
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RunPair {
    pub left: GlyphClass,
    pub right: GlyphClass,
    pub left_cluster_chars: u8,
    pub right_cluster_chars: u8,
    pub metric_delta: f32,
    pub optical_delta: f32,
    pub min_gap: f32,
    pub delta: f32,
}

impl RunPair {
    pub(crate) fn is_upper_upper(self) -> bool {
        self.left == GlyphClass::Upper && self.right == GlyphClass::Upper
    }

    pub(crate) fn is_upper_lower(self) -> bool {
        self.left == GlyphClass::Upper && self.right == GlyphClass::Lower
    }

    pub(crate) fn is_lower_upper(self) -> bool {
        self.left == GlyphClass::Lower && self.right == GlyphClass::Upper
    }

    pub(crate) fn is_lower_lower(self) -> bool {
        self.left == GlyphClass::Lower && self.right == GlyphClass::Lower
    }

    pub(crate) fn is_mixed_case(self) -> bool {
        matches!(
            (self.left, self.right),
            (GlyphClass::Upper, GlyphClass::Lower) | (GlyphClass::Lower, GlyphClass::Upper)
        )
    }

    pub(crate) fn is_digit_digit(self) -> bool {
        self.left == GlyphClass::Digit && self.right == GlyphClass::Digit
    }

    pub(crate) fn is_digit_run(self) -> bool {
        self.is_digit_digit()
            || matches!(
                (self.left, self.right),
                (GlyphClass::Digit, GlyphClass::Punctuation)
                    | (GlyphClass::Punctuation, GlyphClass::Digit)
            )
    }

    pub(crate) fn is_metricless(self) -> bool {
        self.metric_delta.abs() < DEAD_ZONE
    }

    pub(crate) fn is_lower_involved(self) -> bool {
        self.is_upper_lower() || self.is_lower_upper() || self.is_lower_lower()
    }

    pub(crate) fn has_multi_char_cluster(self) -> bool {
        self.left_cluster_chars > 1 || self.right_cluster_chars > 1
    }

    pub(crate) fn max_cluster_chars(self) -> u8 {
        self.left_cluster_chars.max(self.right_cluster_chars)
    }
}
