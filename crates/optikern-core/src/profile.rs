use crate::outline::{GlyphOutline, LineSegment};

#[derive(Debug, Clone, Copy)]
pub struct ProfileConfig {
    pub slices: usize,
    pub min_y: f32,
    pub max_y: f32,
    pub x_height: f32,
    pub cap_height: f32,
}

impl Default for ProfileConfig {
    fn default() -> Self {
        Self {
            slices: 80,
            min_y: -0.20,
            max_y: 0.88,
            x_height: 0.52,
            cap_height: 0.72,
        }
    }
}

impl ProfileConfig {
    pub fn for_latin(x_height: Option<f32>, cap_height: Option<f32>) -> Self {
        let mut config = Self::default();
        config.x_height = x_height.unwrap_or(config.x_height).clamp(0.35, 0.72);
        config.cap_height = cap_height
            .unwrap_or(config.cap_height)
            .clamp(config.x_height, 0.95);
        config.max_y = (config.cap_height + 0.16).clamp(0.70, 1.05);
        config
    }
}

#[derive(Debug, Clone, Copy)]
pub struct GapStats {
    pub min_gap: f32,
    pub weighted_mean_gap: f32,
    pub robust_mean_gap: f32,
    pub mad: f32,
    pub samples: usize,
}

pub fn gap_stats(
    left: &GlyphOutline,
    left_advance: f32,
    right: &GlyphOutline,
    config: ProfileConfig,
) -> Option<GapStats> {
    let mut gaps = Vec::with_capacity(config.slices);
    let mut weighted_sum = 0.0;
    let mut weight_sum = 0.0;

    for index in 0..config.slices.max(1) {
        let t = (index as f32 + 0.5) / config.slices.max(1) as f32;
        let y = config.min_y + t * (config.max_y - config.min_y);
        let Some(left_x) = rightmost_intersection(&left.segments, y) else {
            continue;
        };
        let Some(right_x) = leftmost_intersection(&right.segments, y) else {
            continue;
        };
        let gap = left_advance + right_x - left_x;
        let weight = optical_weight(y, config);
        gaps.push(gap);
        weighted_sum += gap * weight;
        weight_sum += weight;
    }

    if gaps.is_empty() || weight_sum == 0.0 {
        return None;
    }

    gaps.sort_by(|a, b| a.total_cmp(b));
    let median = percentile(&gaps, 0.5);
    let mut deviations = gaps
        .iter()
        .map(|gap| (gap - median).abs())
        .collect::<Vec<_>>();
    deviations.sort_by(|a, b| a.total_cmp(b));
    let mad = percentile(&deviations, 0.5);
    let cutoff = (mad * 2.5).max(0.015);
    let filtered = gaps
        .iter()
        .copied()
        .filter(|gap| (*gap - median).abs() <= cutoff)
        .collect::<Vec<_>>();
    let robust_mean_gap = filtered.iter().sum::<f32>() / filtered.len().max(1) as f32;

    Some(GapStats {
        min_gap: *gaps.first().unwrap(),
        weighted_mean_gap: weighted_sum / weight_sum,
        robust_mean_gap,
        mad,
        samples: gaps.len(),
    })
}

fn leftmost_intersection(segments: &[LineSegment], y: f32) -> Option<f32> {
    intersections(segments, y)
        .into_iter()
        .min_by(|a, b| a.total_cmp(b))
}

fn rightmost_intersection(segments: &[LineSegment], y: f32) -> Option<f32> {
    intersections(segments, y)
        .into_iter()
        .max_by(|a, b| a.total_cmp(b))
}

fn intersections(segments: &[LineSegment], y: f32) -> Vec<f32> {
    let mut xs = vec![];
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
        xs.push(segment.start.x + t * (segment.end.x - segment.start.x));
    }
    xs
}

fn optical_weight(y: f32, config: ProfileConfig) -> f32 {
    if (0.02..=config.x_height).contains(&y) {
        3.0
    } else if (-0.04..=config.cap_height).contains(&y) {
        2.0
    } else {
        0.75
    }
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
    use crate::outline::{Bounds, GlyphOutline, LineSegment, Point};

    use super::*;

    fn rect(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> GlyphOutline {
        let p1 = Point { x: min_x, y: min_y };
        let p2 = Point { x: max_x, y: min_y };
        let p3 = Point { x: max_x, y: max_y };
        let p4 = Point { x: min_x, y: max_y };
        GlyphOutline {
            segments: vec![
                LineSegment { start: p1, end: p2 },
                LineSegment { start: p2, end: p3 },
                LineSegment { start: p3, end: p4 },
                LineSegment { start: p4, end: p1 },
            ],
            bounds: Bounds {
                min_x,
                min_y,
                max_x,
                max_y,
            },
        }
    }

    #[test]
    fn computes_simple_rect_gap() {
        let left = rect(0.0, 0.0, 0.5, 0.7);
        let right = rect(0.0, 0.0, 0.4, 0.7);
        let stats = gap_stats(&left, 0.7, &right, ProfileConfig::default()).unwrap();
        assert!((stats.weighted_mean_gap - 0.2).abs() < 0.001);
        assert!(stats.samples > 20);
    }
}
