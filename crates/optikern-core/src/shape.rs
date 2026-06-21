use anyhow::{Result, anyhow};
use rustybuzz::{Feature, UnicodeBuffer};
use ttf_parser::Tag;

use crate::font::FontKit;

pub fn metric_pair_delta_em(font: &FontKit, pair: &str) -> Result<f32> {
    if pair.chars().count() < 2 {
        return Ok(0.0);
    }

    let with_kern = shaped_advance_em(font, pair, &[])?;
    let without_kern =
        shaped_advance_em(font, pair, &[Feature::new(Tag::from_bytes(b"kern"), 0, ..)])?;
    Ok(with_kern - without_kern)
}

fn shaped_advance_em(font: &FontKit, text: &str, features: &[Feature]) -> Result<f32> {
    let face = rustybuzz::Face::from_slice(font.bytes(), font.face_index()).ok_or_else(|| {
        anyhow!(
            "failed to parse font {} with rustybuzz",
            font.path().display()
        )
    })?;
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(text);
    let output = rustybuzz::shape(&face, features, buffer);
    let advance: i32 = output
        .glyph_positions()
        .iter()
        .map(|pos| pos.x_advance)
        .sum();
    Ok(advance as f32 / font.units_per_em())
}
