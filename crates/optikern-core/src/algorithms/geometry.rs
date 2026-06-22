use crate::outline::{GlyphOutline, LineSegment};
use crate::profile::ProfileConfig;

use super::math::percentile;

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct PairGeometry {
    pub(super) right_top_left_overhang: f32,
    pub(super) left_right_side: SideFeatures,
    pub(super) right_left_side: SideFeatures,
}

impl PairGeometry {
    pub(super) fn from_outlines(
        left: &GlyphOutline,
        right: &GlyphOutline,
        config: ProfileConfig,
    ) -> Self {
        Self {
            right_top_left_overhang: top_left_overhang(right, config),
            left_right_side: SideFeatures::from_outline(left, Side::Right),
            right_left_side: SideFeatures::from_outline(right, Side::Left),
        }
    }

    pub(super) fn has_diagonal_pair(self) -> bool {
        self.left_right_side.is_diagonal_like() || self.right_left_side.is_diagonal_like()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(super) struct SideFeatures {
    pub(super) roundness: f32,
    pub(super) stemness: f32,
}

impl SideFeatures {
    pub(super) fn from_outline(outline: &GlyphOutline, side: Side) -> Self {
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

    pub(super) fn has_shape(self) -> bool {
        self.roundness > 0.0 || self.stemness > 0.0
    }

    pub(super) fn is_round_like(self) -> bool {
        self.roundness > 0.035 || self.stemness < 0.45
    }

    pub(super) fn is_diagonal_like(self) -> bool {
        self.stemness < 0.55 && self.roundness <= 0.035
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum Side {
    Left,
    Right,
}

pub(super) fn top_left_overhang(outline: &GlyphOutline, config: ProfileConfig) -> f32 {
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
