use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use optikern_core::{
    Algorithm, AlgorithmOutput, EvaluationConfig, FontKit, ShapedRun, ShapingOptions,
    evaluate_shaped_run_with_config, shape_text,
};
use serde::Serialize;

use crate::corpus;

pub fn run(
    root: &Path,
    font_id: &str,
    font_path: Option<&Path>,
    text: &str,
    ligatures: bool,
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
    let config = EvaluationConfig::for_font(&font);
    let run = shape_text(
        &font,
        text,
        ShapingOptions {
            kerning: false,
            ligatures,
            contextual_alternates: ligatures,
        },
    )?;
    let fragmented_render_safe = fragmented_shape_matches(&font, &run)?;

    let mut pairs = Vec::new();
    for result in evaluate_shaped_run_with_config(&font, &run, config, ligatures)? {
        let guarded = result
            .outputs
            .iter()
            .find(|output| output.algorithm == Algorithm::GuardedProfileHybrid)
            .ok_or_else(|| anyhow!("missing guarded output for {}", result.display))?;
        pairs.push(SamplePairDelta {
            display: result.display,
            shaping_text: result.shaping_text,
            left_cluster: result.left_cluster,
            right_cluster: result.right_cluster,
            left_glyph_id: result.left_glyph_id,
            right_glyph_id: result.right_glyph_id,
            delta_em: guarded.delta_em,
            metric_delta_em: guarded.metric_delta_em,
            optical_delta_em: guarded.optical_delta_em,
            outputs: result.outputs,
        });
    }

    let output = SampleDeltaReport {
        schema_version: 1,
        font_id: entry.id.clone(),
        family: entry.family.clone(),
        font_path: font_path.display().to_string(),
        text: text.to_owned(),
        ligatures,
        contextual_alternates: run.options.contextual_alternates,
        fragmented_render_safe,
        pairs,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn resolved_font_path(root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    }
}

fn fragmented_shape_matches(font: &FontKit, run: &ShapedRun) -> Result<bool> {
    let mut fragmented = Vec::new();
    for glyph in &run.glyphs {
        let shaped = shape_text(font, &glyph.cluster_text, run.options)?;
        fragmented.extend(shaped.glyphs);
    }

    if fragmented.len() != run.glyphs.len() {
        return Ok(false);
    }

    Ok(run.glyphs.iter().zip(fragmented.iter()).all(|(full, part)| {
        full.glyph_id == part.glyph_id
            && (full.x_advance_em - part.x_advance_em).abs() < 0.0005
            && (full.x_offset_em - part.x_offset_em).abs() < 0.0005
            && (full.y_offset_em - part.y_offset_em).abs() < 0.0005
    }))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SampleDeltaReport {
    schema_version: u32,
    font_id: String,
    family: String,
    font_path: String,
    text: String,
    ligatures: bool,
    contextual_alternates: bool,
    fragmented_render_safe: bool,
    pairs: Vec<SamplePairDelta>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SamplePairDelta {
    display: String,
    shaping_text: String,
    left_cluster: String,
    right_cluster: String,
    left_glyph_id: u16,
    right_glyph_id: u16,
    delta_em: f32,
    metric_delta_em: f32,
    optical_delta_em: f32,
    outputs: Vec<AlgorithmOutput>,
}
