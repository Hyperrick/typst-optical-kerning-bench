use anyhow::{Result, anyhow};
use rustybuzz::{Feature, UnicodeBuffer};
use serde::{Deserialize, Serialize};
use ttf_parser::{GlyphId, Tag};

use crate::font::FontKit;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShapingOptions {
    pub kerning: bool,
    pub ligatures: bool,
}

impl Default for ShapingOptions {
    fn default() -> Self {
        Self {
            kerning: true,
            ligatures: true,
        }
    }
}

impl ShapingOptions {
    pub const fn typst_pair() -> Self {
        Self {
            kerning: false,
            ligatures: false,
        }
    }

    pub const fn typst_word() -> Self {
        Self {
            kerning: false,
            ligatures: true,
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
            .filter_map(|window| {
                let left = &window[0];
                let right = &window[1];
                if left.cluster_start == right.cluster_start
                    || left.cluster_text.chars().all(char::is_whitespace)
                    || right.cluster_text.chars().all(char::is_whitespace)
                {
                    return None;
                }
                Some(ShapedGlyphPair::new(left, right))
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
    pub left_glyph_id: u16,
    pub right_glyph_id: u16,
    pub left_cluster: String,
    pub right_cluster: String,
    pub left_advance_em: f32,
}

impl ShapedGlyphPair {
    pub fn new(left: &ShapedGlyph, right: &ShapedGlyph) -> Self {
        let display = format!("{}|{}", left.cluster_text, right.cluster_text);
        Self {
            key: glyph_pair_key(left, right),
            display,
            shaping_text: format!("{}{}", left.cluster_text, right.cluster_text),
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
    };
    let without_options = ShapingOptions {
        kerning: false,
        ligatures,
    };
    let with = shape_text(font, &pair.shaping_text, with_options)?;
    let without = shape_text(font, &pair.shaping_text, without_options)?;
    if !same_pair_shape(&with, pair) || !same_pair_shape(&without, pair) {
        return Ok(0.0);
    }
    Ok(total_advance(&with) - total_advance(&without))
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

    #[test]
    fn cluster_ranges_expand_to_next_cluster() {
        let ranges = cluster_ranges("Goldfish", [0, 1, 2, 3, 4, 6, 7]);
        assert_eq!(ranges.get(&4), Some(&6));
    }
}
