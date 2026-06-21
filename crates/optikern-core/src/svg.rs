use anyhow::Result;
use ttf_parser::OutlineBuilder;

use crate::font::FontKit;

#[derive(Debug, Clone)]
pub struct SvgGlyph {
    pub advance_em: f32,
    pub path_data: String,
    pub bounds: SvgBounds,
}

#[derive(Debug, Clone, Copy)]
pub struct SvgBounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

impl SvgBounds {
    pub fn translated(self, x: f32) -> Self {
        Self {
            min_x: self.min_x + x,
            min_y: self.min_y,
            max_x: self.max_x + x,
            max_y: self.max_y,
        }
    }
}

pub fn svg_glyph(font: &FontKit, ch: char) -> Result<SvgGlyph> {
    let metrics = font.glyph_metrics(ch)?;
    let face = font.face()?;
    let mut builder = SvgBuilder::new(font.units_per_em());
    let rect = face.outline_glyph(metrics.glyph_id, &mut builder);
    let bounds = rect
        .map(|rect| SvgBounds {
            min_x: f32::from(rect.x_min) / font.units_per_em(),
            min_y: -f32::from(rect.y_max) / font.units_per_em(),
            max_x: f32::from(rect.x_max) / font.units_per_em(),
            max_y: -f32::from(rect.y_min) / font.units_per_em(),
        })
        .unwrap_or(SvgBounds {
            min_x: 0.0,
            min_y: 0.0,
            max_x: metrics.advance_em,
            max_y: 0.0,
        });

    Ok(SvgGlyph {
        advance_em: metrics.advance_em,
        path_data: builder.path,
        bounds,
    })
}

struct SvgBuilder {
    units_per_em: f32,
    path: String,
}

impl SvgBuilder {
    fn new(units_per_em: f32) -> Self {
        Self {
            units_per_em,
            path: String::new(),
        }
    }

    fn x(&self, value: f32) -> f32 {
        value / self.units_per_em
    }

    fn y(&self, value: f32) -> f32 {
        -value / self.units_per_em
    }
}

impl OutlineBuilder for SvgBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.path
            .push_str(&format!("M{:.5},{:.5}", self.x(x), self.y(y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.path
            .push_str(&format!("L{:.5},{:.5}", self.x(x), self.y(y)));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.path.push_str(&format!(
            "Q{:.5},{:.5} {:.5},{:.5}",
            self.x(x1),
            self.y(y1),
            self.x(x),
            self.y(y)
        ));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.path.push_str(&format!(
            "C{:.5},{:.5} {:.5},{:.5} {:.5},{:.5}",
            self.x(x1),
            self.y(y1),
            self.x(x2),
            self.y(y2),
            self.x(x),
            self.y(y)
        ));
    }

    fn close(&mut self) {
        self.path.push('Z');
    }
}
