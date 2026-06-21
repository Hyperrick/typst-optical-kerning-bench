use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use ttf_parser::{Face, GlyphId};

use crate::outline::{FlattenOptions, GlyphOutline, outline_glyph};

#[derive(Debug, Clone)]
pub struct FontKit {
    id: String,
    path: PathBuf,
    bytes: Vec<u8>,
    face_index: u32,
    units_per_em: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct GlyphMetrics {
    pub glyph_id: GlyphId,
    pub advance_em: f32,
}

impl FontKit {
    pub fn load(id: impl Into<String>, path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let bytes = std::fs::read(&path)
            .with_context(|| format!("failed to read font {}", path.display()))?;
        let face_index = 0;
        let face = Face::parse(&bytes, face_index)
            .map_err(|err| anyhow!("failed to parse font {}: {err:?}", path.display()))?;
        let units_per_em = f32::from(face.units_per_em());

        Ok(Self {
            id: id.into(),
            path,
            bytes,
            face_index,
            units_per_em,
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn face_index(&self) -> u32 {
        self.face_index
    }

    pub fn units_per_em(&self) -> f32 {
        self.units_per_em
    }

    pub fn x_height_em(&self) -> Option<f32> {
        self.face()
            .ok()
            .and_then(|face| face.x_height())
            .map(|height| f32::from(height) / self.units_per_em)
    }

    pub fn cap_height_em(&self) -> Option<f32> {
        self.face()
            .ok()
            .and_then(|face| face.capital_height())
            .map(|height| f32::from(height) / self.units_per_em)
    }

    pub fn is_monospaced(&self) -> bool {
        if self
            .face()
            .map(|face| face.is_monospaced())
            .unwrap_or(false)
        {
            return true;
        }

        self.has_uniform_representative_advances()
    }

    fn has_uniform_representative_advances(&self) -> bool {
        let advances = "Hn0iMW"
            .chars()
            .filter_map(|ch| self.glyph_metrics(ch).ok())
            .map(|metrics| metrics.advance_em)
            .collect::<Vec<_>>();
        if advances.len() < 4 {
            return false;
        }
        let min = advances.iter().copied().fold(f32::INFINITY, f32::min);
        let max = advances.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        max - min <= 0.01
    }

    pub fn face(&self) -> Result<Face<'_>> {
        Face::parse(&self.bytes, self.face_index)
            .map_err(|err| anyhow!("failed to parse font {}: {err:?}", self.path.display()))
    }

    pub fn glyph_metrics(&self, ch: char) -> Result<GlyphMetrics> {
        let face = self.face()?;
        let glyph_id = face
            .glyph_index(ch)
            .ok_or_else(|| anyhow!("font {} has no glyph for {ch:?}", self.id))?;
        self.glyph_metrics_by_id(glyph_id)
    }

    pub fn glyph_metrics_by_id(&self, glyph_id: GlyphId) -> Result<GlyphMetrics> {
        let face = self.face()?;
        let advance = face.glyph_hor_advance(glyph_id).ok_or_else(|| {
            anyhow!(
                "font {} has no horizontal advance for glyph {:?}",
                self.id,
                glyph_id
            )
        })?;

        Ok(GlyphMetrics {
            glyph_id,
            advance_em: f32::from(advance) / self.units_per_em,
        })
    }

    pub fn outline(
        &self,
        ch: char,
        options: FlattenOptions,
    ) -> Result<(GlyphMetrics, GlyphOutline)> {
        let metrics = self.glyph_metrics(ch)?;
        let face = self.face()?;
        let outline = outline_glyph(&face, metrics.glyph_id, self.units_per_em, options)
            .ok_or_else(|| anyhow!("font {} has no outline for {ch:?}", self.id))?;
        Ok((metrics, outline))
    }

    pub fn outline_by_id(
        &self,
        glyph_id: GlyphId,
        options: FlattenOptions,
    ) -> Result<(GlyphMetrics, GlyphOutline)> {
        let metrics = self.glyph_metrics_by_id(glyph_id)?;
        let face = self.face()?;
        let outline = outline_glyph(&face, metrics.glyph_id, self.units_per_em, options)
            .ok_or_else(|| anyhow!("font {} has no outline for glyph {:?}", self.id, glyph_id))?;
        Ok((metrics, outline))
    }
}
