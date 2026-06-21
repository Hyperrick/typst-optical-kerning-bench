use ttf_parser::{Face, GlyphId, OutlineBuilder};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineSegment {
    pub start: Point,
    pub end: Point,
}

#[derive(Debug, Clone)]
pub struct GlyphOutline {
    pub segments: Vec<LineSegment>,
    pub bounds: Bounds,
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub min_x: f32,
    pub min_y: f32,
    pub max_x: f32,
    pub max_y: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct FlattenOptions {
    pub curve_steps: usize,
}

impl Default for FlattenOptions {
    fn default() -> Self {
        Self { curve_steps: 16 }
    }
}

pub fn outline_glyph(
    face: &Face<'_>,
    glyph_id: GlyphId,
    units_per_em: f32,
    options: FlattenOptions,
) -> Option<GlyphOutline> {
    let mut builder = Collector::new(units_per_em, options);
    face.outline_glyph(glyph_id, &mut builder)?;
    builder.finish()
}

struct Collector {
    units_per_em: f32,
    options: FlattenOptions,
    start: Option<Point>,
    current: Option<Point>,
    segments: Vec<LineSegment>,
    bounds: Option<Bounds>,
}

impl Collector {
    fn new(units_per_em: f32, options: FlattenOptions) -> Self {
        Self {
            units_per_em,
            options,
            start: None,
            current: None,
            segments: vec![],
            bounds: None,
        }
    }

    fn point(&self, x: f32, y: f32) -> Point {
        Point {
            x: x / self.units_per_em,
            y: y / self.units_per_em,
        }
    }

    fn move_to_point(&mut self, point: Point) {
        self.start = Some(point);
        self.current = Some(point);
        self.include(point);
    }

    fn line_to_point(&mut self, point: Point) {
        if let Some(start) = self.current {
            if start != point {
                self.segments.push(LineSegment { start, end: point });
            }
        }
        self.current = Some(point);
        self.include(point);
    }

    fn include(&mut self, point: Point) {
        self.bounds = Some(match self.bounds {
            Some(bounds) => Bounds {
                min_x: bounds.min_x.min(point.x),
                min_y: bounds.min_y.min(point.y),
                max_x: bounds.max_x.max(point.x),
                max_y: bounds.max_y.max(point.y),
            },
            None => Bounds {
                min_x: point.x,
                min_y: point.y,
                max_x: point.x,
                max_y: point.y,
            },
        });
    }

    fn finish(self) -> Option<GlyphOutline> {
        Some(GlyphOutline {
            segments: self.segments,
            bounds: self.bounds?,
        })
    }
}

impl OutlineBuilder for Collector {
    fn move_to(&mut self, x: f32, y: f32) {
        let point = self.point(x, y);
        self.move_to_point(point);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let point = self.point(x, y);
        self.line_to_point(point);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let Some(p0) = self.current else {
            self.move_to(x, y);
            return;
        };
        let p1 = self.point(x1, y1);
        let p2 = self.point(x, y);
        for step in 1..=self.options.curve_steps.max(1) {
            let t = step as f32 / self.options.curve_steps.max(1) as f32;
            let mt = 1.0 - t;
            self.line_to_point(Point {
                x: mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x,
                y: mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y,
            });
        }
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let Some(p0) = self.current else {
            self.move_to(x, y);
            return;
        };
        let p1 = self.point(x1, y1);
        let p2 = self.point(x2, y2);
        let p3 = self.point(x, y);
        for step in 1..=self.options.curve_steps.max(1) {
            let t = step as f32 / self.options.curve_steps.max(1) as f32;
            let mt = 1.0 - t;
            self.line_to_point(Point {
                x: mt.powi(3) * p0.x
                    + 3.0 * mt.powi(2) * t * p1.x
                    + 3.0 * mt * t.powi(2) * p2.x
                    + t.powi(3) * p3.x,
                y: mt.powi(3) * p0.y
                    + 3.0 * mt.powi(2) * t * p1.y
                    + 3.0 * mt * t.powi(2) * p2.y
                    + t.powi(3) * p3.y,
            });
        }
    }

    fn close(&mut self) {
        if let (Some(current), Some(start)) = (self.current, self.start)
            && current != start
        {
            self.segments.push(LineSegment {
                start: current,
                end: start,
            });
        }
        self.current = self.start;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_grow_for_points() {
        let mut c = Collector::new(1000.0, FlattenOptions::default());
        c.move_to(0.0, 0.0);
        c.line_to(1000.0, 500.0);
        c.line_to(-250.0, 1200.0);
        let outline = c.finish().unwrap();
        assert_eq!(outline.bounds.min_x, -0.25);
        assert_eq!(outline.bounds.max_y, 1.2);
    }
}
