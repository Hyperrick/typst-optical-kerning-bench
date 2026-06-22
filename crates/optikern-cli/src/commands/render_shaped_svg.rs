use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use optikern_core::{
    Algorithm, EvaluationConfig, FontKit, ShapingOptions, SvgBounds,
    evaluate_shaped_run_with_config, shape_text, svg_glyph_by_id,
};
use serde::Serialize;
use ttf_parser::GlyphId;

use crate::corpus;

pub fn run(
    root: &Path,
    font_id: &str,
    font_path: Option<&Path>,
    text: &str,
    ligatures: bool,
    point_size: f32,
    dpi: f32,
    output_svg: &Path,
    output_json: Option<&Path>,
) -> Result<()> {
    let manifest = corpus::load_fonts(root)?;
    let entry = manifest
        .fonts
        .iter()
        .find(|entry| entry.id == font_id)
        .ok_or_else(|| anyhow!("unknown font id {font_id:?}"))?;
    let font_path = match font_path {
        Some(path) => resolved_font_path(root, path),
        None => entry.local_path(root),
    };
    let font = FontKit::load(&entry.id, &font_path)
        .with_context(|| format!("failed to load {}", font_path.display()))?;
    let options = ShapingOptions {
        kerning: false,
        ligatures,
        contextual_alternates: ligatures,
    };
    let run = shape_text(&font, text, options)?;
    let results =
        evaluate_shaped_run_with_config(&font, &run, EvaluationConfig::for_font(&font), ligatures)?;
    let deltas = guarded_deltas_by_left_index(&results)?;
    let rendered = render_svg(&font, &run.glyphs, &deltas, point_size, dpi)?;
    write_text(root, output_svg, &rendered.svg)?;

    if let Some(output_json) = output_json {
        let sidecar = ShapedSvgSidecar {
            schema_version: 1,
            renderer: "shaped-svg",
            font_id: entry.id.clone(),
            font_path: font_path.display().to_string(),
            text: text.to_owned(),
            ligatures,
            contextual_alternates: options.contextual_alternates,
            point_size,
            dpi,
            width_px: rendered.width_px,
            height_px: rendered.height_px,
            view_box_em: rendered.view_box_em,
            deltas,
        };
        write_text(root, output_json, &serde_json::to_string_pretty(&sidecar)?)?;
    }

    Ok(())
}

fn guarded_deltas_by_left_index(
    results: &[optikern_core::AlgorithmSet],
) -> Result<BTreeMap<usize, f32>> {
    let mut deltas = BTreeMap::new();
    for result in results {
        let output = result
            .outputs
            .iter()
            .find(|output| output.algorithm == Algorithm::GuardedProfileHybrid)
            .ok_or_else(|| anyhow!("missing guarded output for {}", result.display))?;
        deltas.insert(result.left_index, output.delta_em);
    }
    Ok(deltas)
}

fn render_svg(
    font: &FontKit,
    glyphs: &[optikern_core::ShapedGlyph],
    deltas: &BTreeMap<usize, f32>,
    point_size: f32,
    dpi: f32,
) -> Result<RenderedSvg> {
    let mut x = 0.0;
    let mut paths = Vec::with_capacity(glyphs.len());
    let mut bounds: Option<SvgBounds> = None;

    for (index, glyph) in glyphs.iter().enumerate() {
        let svg_glyph = svg_glyph_by_id(font, GlyphId(glyph.glyph_id))?;
        let glyph_x = x + glyph.x_offset_em;
        let glyph_y = -glyph.y_offset_em;
        if !svg_glyph.path_data.is_empty() {
            paths.push(format!(
                "<path d=\"{}\" transform=\"translate({:.5} {:.5})\"/>",
                svg_glyph.path_data, glyph_x, glyph_y
            ));
        }
        bounds = Some(include_bounds(
            bounds,
            svg_glyph.bounds.translated_xy(glyph_x, glyph_y),
        ));
        x += glyph.x_advance_em + deltas.get(&index).copied().unwrap_or(0.0);
    }

    let bounds = bounds.unwrap_or(SvgBounds {
        min_x: 0.0,
        min_y: -1.0,
        max_x: x.max(0.001),
        max_y: 0.0,
    });
    let padding_em = 0.02;
    let min_x = bounds.min_x - padding_em;
    let min_y = bounds.min_y - padding_em;
    let width_em = (bounds.max_x - bounds.min_x + padding_em * 2.0).max(0.001);
    let height_em = (bounds.max_y - bounds.min_y + padding_em * 2.0).max(0.001);
    let px_per_em = point_size * dpi / 72.0;
    let width_px = (width_em * px_per_em).ceil() as u32;
    let height_px = (height_em * px_per_em).ceil() as u32;
    let svg = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}px\" height=\"{}px\" viewBox=\"{:.5} {:.5} {:.5} {:.5}\"><rect x=\"{:.5}\" y=\"{:.5}\" width=\"{:.5}\" height=\"{:.5}\" fill=\"white\"/><g fill=\"black\">{}</g></svg>\n",
        width_px,
        height_px,
        min_x,
        min_y,
        width_em,
        height_em,
        min_x,
        min_y,
        width_em,
        height_em,
        paths.join("")
    );

    Ok(RenderedSvg {
        svg,
        width_px,
        height_px,
        view_box_em: [min_x, min_y, width_em, height_em],
    })
}

fn include_bounds(existing: Option<SvgBounds>, next: SvgBounds) -> SvgBounds {
    match existing {
        Some(bounds) => SvgBounds {
            min_x: bounds.min_x.min(next.min_x),
            min_y: bounds.min_y.min(next.min_y),
            max_x: bounds.max_x.max(next.max_x),
            max_y: bounds.max_y.max(next.max_y),
        },
        None => next,
    }
}

fn write_text(root: &Path, path: &Path, contents: &str) -> Result<()> {
    let path = resolved_font_path(root, path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, contents).with_context(|| format!("failed to write {}", path.display()))
}

fn resolved_font_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

struct RenderedSvg {
    svg: String,
    width_px: u32,
    height_px: u32,
    view_box_em: [f32; 4],
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ShapedSvgSidecar {
    schema_version: u32,
    renderer: &'static str,
    font_id: String,
    font_path: String,
    text: String,
    ligatures: bool,
    contextual_alternates: bool,
    point_size: f32,
    dpi: f32,
    width_px: u32,
    height_px: u32,
    view_box_em: [f32; 4],
    deltas: BTreeMap<usize, f32>,
}
