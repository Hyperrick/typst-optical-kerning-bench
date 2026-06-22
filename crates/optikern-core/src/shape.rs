use anyhow::{Result, anyhow};
use rustybuzz::{Feature, UnicodeBuffer};
use serde::{Deserialize, Serialize};
use ttf_parser::{GlyphId, Tag};

use crate::font::FontKit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShapingOptions {
    pub kerning: bool,
    pub ligatures: bool,
    pub contextual_alternates: bool,
}

impl Default for ShapingOptions {
    fn default() -> Self {
        Self {
            kerning: true,
            ligatures: true,
            contextual_alternates: true,
        }
    }
}

impl ShapingOptions {
    pub const fn typst_pair() -> Self {
        Self {
            kerning: false,
            ligatures: false,
            contextual_alternates: false,
        }
    }

    pub const fn typst_word() -> Self {
        Self {
            kerning: false,
            ligatures: true,
            contextual_alternates: true,
        }
    }

    fn features(self) -> Vec<Feature> {
        let mut features = Vec::new();
        if !self.kerning {
            features.push(Feature::new(Tag::from_bytes(b"kern"), 0, ..));
        }
        if !self.ligatures {
            features.push(Feature::new(Tag::from_bytes(b"liga"), 0, ..));
            features.push(Feature::new(Tag::from_bytes(b"clig"), 0, ..));
        }
        if !self.contextual_alternates {
            features.push(Feature::new(Tag::from_bytes(b"calt"), 0, ..));
        }
        features
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapedRun {
    pub text: String,
    pub options: ShapingOptions,
    pub glyphs: Vec<ShapedGlyph>,
}

impl ShapedRun {
    pub fn adjacent_pairs(&self) -> Vec<ShapedGlyphPair> {
        self.glyphs
            .windows(2)
            .enumerate()
            .filter_map(|(left_index, window)| {
                let left = &window[0];
                let right = &window[1];
                if left.cluster_start == right.cluster_start
                    || left.cluster_text.chars().all(char::is_whitespace)
                    || right.cluster_text.chars().all(char::is_whitespace)
                {
                    return None;
                }
                Some(ShapedGlyphPair::new(left_index, left, right))
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapedGlyph {
    pub glyph_id: u16,
    pub cluster_start: usize,
    pub cluster_end: usize,
    pub cluster_text: String,
    pub x_advance_em: f32,
    pub y_advance_em: f32,
    pub x_offset_em: f32,
    pub y_offset_em: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapedGlyphPair {
    pub key: String,
    pub display: String,
    pub shaping_text: String,
    #[serde(default)]
    pub left_index: usize,
    #[serde(default)]
    pub right_index: usize,
    pub left_glyph_id: u16,
    pub right_glyph_id: u16,
    pub left_cluster: String,
    pub right_cluster: String,
    pub left_advance_em: f32,
}

impl ShapedGlyphPair {
    pub fn new(left_index: usize, left: &ShapedGlyph, right: &ShapedGlyph) -> Self {
        let display = format!("{}|{}", left.cluster_text, right.cluster_text);
        Self {
            key: glyph_pair_key(left, right),
            display,
            shaping_text: format!("{}{}", left.cluster_text, right.cluster_text),
            left_index,
            right_index: left_index + 1,
            left_glyph_id: left.glyph_id,
            right_glyph_id: right.glyph_id,
            left_cluster: left.cluster_text.clone(),
            right_cluster: right.cluster_text.clone(),
            left_advance_em: left.x_advance_em,
        }
    }

    pub fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = key.into();
        self
    }
}

pub fn shape_text(font: &FontKit, text: &str, options: ShapingOptions) -> Result<ShapedRun> {
    let face = rustybuzz_face(font)?;
    let features = options.features();
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    let output = rustybuzz::shape(&face, &features, buffer);
    let infos = output.glyph_infos();
    let positions = output.glyph_positions();
    let cluster_ranges = cluster_ranges(text, infos.iter().map(|info| info.cluster as usize));

    let mut glyphs = Vec::with_capacity(infos.len());
    for (info, position) in infos.iter().zip(positions) {
        let glyph_id = u16::try_from(info.glyph_id)
            .map_err(|_| anyhow!("font {} emitted glyph id {}", font.id(), info.glyph_id))?;
        let cluster_start = info.cluster as usize;
        let cluster_end = *cluster_ranges.get(&cluster_start).unwrap_or(&text.len());
        let cluster_text = text
            .get(cluster_start..cluster_end)
            .unwrap_or_default()
            .to_owned();
        glyphs.push(ShapedGlyph {
            glyph_id,
            cluster_start,
            cluster_end,
            cluster_text,
            x_advance_em: position.x_advance as f32 / font.units_per_em(),
            y_advance_em: position.y_advance as f32 / font.units_per_em(),
            x_offset_em: position.x_offset as f32 / font.units_per_em(),
            y_offset_em: position.y_offset as f32 / font.units_per_em(),
        });
    }

    Ok(ShapedRun {
        text: text.to_owned(),
        options,
        glyphs,
    })
}

pub fn metric_pair_delta_em(font: &FontKit, pair: &str) -> Result<f32> {
    if pair.chars().count() < 2 {
        return Ok(0.0);
    }

    let with_kern = shaped_advance_em(
        font,
        pair,
        ShapingOptions {
            kerning: true,
            ligatures: false,
            contextual_alternates: false,
        },
    )?;
    let without_kern = shaped_advance_em(font, pair, ShapingOptions::typst_pair())?;
    Ok(with_kern - without_kern)
}

pub fn metric_shaped_pair_delta_em(
    font: &FontKit,
    pair: &ShapedGlyphPair,
    ligatures: bool,
) -> Result<f32> {
    let with_options = ShapingOptions {
        kerning: true,
        ligatures,
        contextual_alternates: ligatures,
    };
    let without_options = ShapingOptions {
        kerning: false,
        ligatures,
        contextual_alternates: ligatures,
    };
    let with = shape_text(font, &pair.shaping_text, with_options)?;
    let without = shape_text(font, &pair.shaping_text, without_options)?;
    if !same_pair_shape(&with, pair) || !same_pair_shape(&without, pair) {
        return Ok(0.0);
    }
    Ok(total_advance(&with) - total_advance(&without))
}

pub fn metric_shaped_run_pair_deltas_em(
    font: &FontKit,
    run: &ShapedRun,
    ligatures: bool,
) -> Result<Vec<Option<f32>>> {
    let with = shape_text(
        font,
        &run.text,
        ShapingOptions {
            kerning: true,
            ligatures,
            contextual_alternates: run.options.contextual_alternates,
        },
    )?;
    Ok(metric_pair_deltas_from_aligned_runs(run, &with))
}

fn shaped_advance_em(font: &FontKit, text: &str, options: ShapingOptions) -> Result<f32> {
    Ok(total_advance(&shape_text(font, text, options)?))
}

fn total_advance(run: &ShapedRun) -> f32 {
    run.glyphs.iter().map(|glyph| glyph.x_advance_em).sum()
}

fn same_pair_shape(run: &ShapedRun, pair: &ShapedGlyphPair) -> bool {
    run.glyphs.len() == 2
        && run.glyphs[0].glyph_id == pair.left_glyph_id
        && run.glyphs[1].glyph_id == pair.right_glyph_id
}

fn metric_pair_deltas_from_aligned_runs(without: &ShapedRun, with: &ShapedRun) -> Vec<Option<f32>> {
    let pairs = without.adjacent_pairs();
    if !same_run_shape(without, with) {
        return vec![None; pairs.len()];
    }

    pairs
        .iter()
        .map(|pair| {
            let without_glyph = without.glyphs.get(pair.left_index)?;
            let with_glyph = with.glyphs.get(pair.left_index)?;
            Some(with_glyph.x_advance_em - without_glyph.x_advance_em)
        })
        .collect()
}

fn same_run_shape(without: &ShapedRun, with: &ShapedRun) -> bool {
    without.glyphs.len() == with.glyphs.len()
        && without
            .glyphs
            .iter()
            .zip(&with.glyphs)
            .all(|(left, right)| {
                left.glyph_id == right.glyph_id
                    && left.cluster_start == right.cluster_start
                    && left.cluster_end == right.cluster_end
                    && left.cluster_text == right.cluster_text
            })
}

fn rustybuzz_face(font: &FontKit) -> Result<rustybuzz::Face<'_>> {
    rustybuzz::Face::from_slice(font.bytes(), font.face_index()).ok_or_else(|| {
        anyhow!(
            "failed to parse font {} with rustybuzz",
            font.path().display()
        )
    })
}

fn cluster_ranges(
    text: &str,
    clusters: impl IntoIterator<Item = usize>,
) -> std::collections::BTreeMap<usize, usize> {
    let mut starts = clusters
        .into_iter()
        .filter(|cluster| *cluster <= text.len() && text.is_char_boundary(*cluster))
        .collect::<Vec<_>>();
    starts.sort_unstable();
    starts.dedup();

    let mut ranges = std::collections::BTreeMap::new();
    for (index, start) in starts.iter().copied().enumerate() {
        let end = starts.get(index + 1).copied().unwrap_or(text.len());
        ranges.insert(start, end);
    }
    ranges
}

fn glyph_pair_key(left: &ShapedGlyph, right: &ShapedGlyph) -> String {
    format!(
        "gid{}-gid{}:{}+{}",
        left.glyph_id,
        right.glyph_id,
        key_text(&left.cluster_text),
        key_text(&right.cluster_text)
    )
}

fn key_text(input: &str) -> String {
    input
        .chars()
        .flat_map(|ch| {
            if ch.is_ascii_alphanumeric() {
                vec![ch.to_ascii_lowercase()]
            } else {
                format!("u{:04x}", ch as u32).chars().collect()
            }
        })
        .collect()
}

pub fn glyph_id(id: u16) -> GlyphId {
    GlyphId(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_glyph(index: usize, text: &str, glyph_id: u16, advance: f32) -> ShapedGlyph {
        ShapedGlyph {
            glyph_id,
            cluster_start: index,
            cluster_end: index + text.len(),
            cluster_text: text.to_owned(),
            x_advance_em: advance,
            y_advance_em: 0.0,
            x_offset_em: 0.0,
            y_offset_em: 0.0,
        }
    }

    fn test_run(glyphs: Vec<ShapedGlyph>) -> ShapedRun {
        ShapedRun {
            text: glyphs
                .iter()
                .map(|glyph| glyph.cluster_text.as_str())
                .collect(),
            options: ShapingOptions {
                kerning: false,
                ligatures: false,
                contextual_alternates: false,
            },
            glyphs,
        }
    }

    #[test]
    fn cluster_ranges_expand_to_next_cluster() {
        let ranges = cluster_ranges("Goldfish", [0, 1, 2, 3, 4, 6, 7]);
        assert_eq!(ranges.get(&4), Some(&6));
    }

    #[test]
    fn no_ligature_options_disable_contextual_alternates() {
        let features = ShapingOptions {
            kerning: false,
            ligatures: false,
            contextual_alternates: false,
        }
        .features();

        assert!(
            features
                .iter()
                .any(|feature| feature.tag == Tag::from_bytes(b"calt") && feature.value == 0)
        );
    }

    #[test]
    fn run_metric_deltas_use_aligned_glyph_advances() {
        let without = test_run(vec![
            test_glyph(0, "T", 157, 0.697),
            test_glyph(1, "o", 472, 0.451),
            test_glyph(2, "T", 157, 0.697),
            test_glyph(3, "a", 365, 0.470),
            test_glyph(4, "L", 92, 0.673),
        ]);
        let with = test_run(vec![
            test_glyph(0, "T", 157, 0.607),
            test_glyph(1, "o", 472, 0.451),
            test_glyph(2, "T", 157, 0.602),
            test_glyph(3, "a", 365, 0.470),
            test_glyph(4, "L", 92, 0.673),
        ]);

        let deltas = metric_pair_deltas_from_aligned_runs(&without, &with);

        assert_eq!(deltas.len(), 4);
        assert!((deltas[0].unwrap() + 0.090).abs() < 0.001);
        assert_eq!(deltas[1], Some(0.0));
        assert!((deltas[2].unwrap() + 0.095).abs() < 0.001);
        assert_eq!(deltas[3], Some(0.0));
    }

    #[test]
    fn run_metric_deltas_fall_back_when_glyphs_change() {
        let without = test_run(vec![
            test_glyph(0, "T", 157, 0.697),
            test_glyph(1, "o", 472, 0.451),
        ]);
        let with = test_run(vec![
            test_glyph(0, "T", 157, 0.607),
            test_glyph(1, "o", 670, 0.453),
        ]);

        assert_eq!(
            metric_pair_deltas_from_aligned_runs(&without, &with),
            vec![None]
        );
    }
}
