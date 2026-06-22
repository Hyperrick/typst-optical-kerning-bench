mod algorithms;
mod calibration;
mod class;
mod font;
mod outline;
mod profile;
mod shape;
mod svg;

pub use algorithms::{
    Algorithm, AlgorithmOutput, AlgorithmSet, EvaluationConfig, evaluate_pair,
    evaluate_pair_with_config, evaluate_shaped_pair_with_config, evaluate_shaped_run_with_config,
};
pub use font::{FontKit, GlyphMetrics};
pub use outline::{FlattenOptions, GlyphOutline, LineSegment, Point};
pub use profile::{GapStats, ProfileConfig};
pub use shape::{
    ShapedGlyph, ShapedGlyphPair, ShapedRun, ShapingOptions, glyph_id, metric_pair_delta_em,
    metric_shaped_pair_delta_em, shape_text,
};
pub use svg::{SvgBounds, SvgGlyph, svg_glyph, svg_glyph_by_id};
