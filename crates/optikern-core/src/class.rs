use crate::shape::ShapedGlyphPair;

pub(crate) const CLUSTER_CLASS_COUNT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PairClass {
    pub(crate) left: ClusterClass,
    pub(crate) right: ClusterClass,
}

impl Default for PairClass {
    fn default() -> Self {
        Self {
            left: ClusterClass::Other,
            right: ClusterClass::Other,
        }
    }
}

impl PairClass {
    pub(crate) fn from_pair(pair: &ShapedGlyphPair) -> Self {
        Self {
            left: ClusterClass::from_cluster(&pair.left_cluster),
            right: ClusterClass::from_cluster(&pair.right_cluster),
        }
    }

    pub(crate) fn from_chars(left: char, right: char) -> Self {
        Self {
            left: ClusterClass::from_char(left),
            right: ClusterClass::from_char(right),
        }
    }

    pub(crate) fn distribution_index(self) -> usize {
        self.left.index() * CLUSTER_CLASS_COUNT + self.right.index()
    }

    pub(crate) fn is_upper_upper(self) -> bool {
        self.left == ClusterClass::Upper && self.right == ClusterClass::Upper
    }

    pub(crate) fn is_upper_lower(self) -> bool {
        self.left == ClusterClass::Upper && self.right == ClusterClass::Lower
    }

    pub(crate) fn is_lower_upper(self) -> bool {
        self.left == ClusterClass::Lower && self.right == ClusterClass::Upper
    }

    pub(crate) fn is_upper_digit(self) -> bool {
        self.left == ClusterClass::Upper && self.right == ClusterClass::Digit
    }

    pub(crate) fn is_digit_digit(self) -> bool {
        self.left == ClusterClass::Digit && self.right == ClusterClass::Digit
    }

    pub(crate) fn is_upper_punctuation(self) -> bool {
        self.left == ClusterClass::Upper && self.right == ClusterClass::Punctuation
    }

    pub(crate) fn is_digit_punctuation(self) -> bool {
        self.left == ClusterClass::Digit && self.right == ClusterClass::Punctuation
    }

    pub(crate) fn is_punctuation_digit(self) -> bool {
        self.left == ClusterClass::Punctuation && self.right == ClusterClass::Digit
    }

    pub(crate) fn has_digit(self) -> bool {
        self.left == ClusterClass::Digit || self.right == ClusterClass::Digit
    }

    pub(crate) fn has_punctuation(self) -> bool {
        self.left == ClusterClass::Punctuation || self.right == ClusterClass::Punctuation
    }

    pub(crate) fn allows_safe_compaction(self) -> bool {
        !self.has_punctuation() && !self.has_digit()
    }

    pub(crate) fn allows_tight_nearest_override(self) -> bool {
        self.is_digit_digit()
            || self.is_upper_digit()
            || self.is_upper_lower()
            || self.is_upper_punctuation()
    }

    pub(crate) fn allows_collision_opening(self) -> bool {
        !self.has_punctuation() && !self.has_digit()
    }

    pub(crate) fn uses_class_gap_calibration(self) -> bool {
        self.is_upper_upper()
            || self.is_upper_digit()
            || self.is_digit_digit()
            || self.is_upper_punctuation()
            || self.is_digit_punctuation()
            || self.is_punctuation_digit()
    }

    pub(crate) fn class_gap_calibration_weight(self) -> f32 {
        if self.uses_class_gap_calibration() {
            0.22
        } else {
            0.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClusterClass {
    Upper,
    Lower,
    Digit,
    Punctuation,
    Other,
}

impl ClusterClass {
    fn from_cluster(cluster: &str) -> Self {
        let chars = cluster.chars().collect::<Vec<_>>();
        if chars.is_empty() {
            return Self::Other;
        }

        if chars.iter().all(|ch| ch.is_ascii_uppercase()) {
            Self::Upper
        } else if chars.iter().all(|ch| ch.is_ascii_lowercase()) {
            Self::Lower
        } else if chars.iter().all(|ch| ch.is_ascii_digit()) {
            Self::Digit
        } else if chars.iter().all(|ch| ch.is_ascii_punctuation()) {
            Self::Punctuation
        } else {
            Self::Other
        }
    }

    fn from_char(ch: char) -> Self {
        if ch.is_ascii_uppercase() {
            Self::Upper
        } else if ch.is_ascii_lowercase() {
            Self::Lower
        } else if ch.is_ascii_digit() {
            Self::Digit
        } else if ch.is_ascii_punctuation() {
            Self::Punctuation
        } else {
            Self::Other
        }
    }

    fn index(self) -> usize {
        match self {
            Self::Upper => 0,
            Self::Lower => 1,
            Self::Digit => 2,
            Self::Punctuation => 3,
            Self::Other => 4,
        }
    }
}
