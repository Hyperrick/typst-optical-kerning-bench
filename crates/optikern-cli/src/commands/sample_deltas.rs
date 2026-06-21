use std::path::Path;

use anyhow::{Context, Result, anyhow};
use optikern_core::{
    Algorithm, AlgorithmOutput, EvaluationConfig, FontKit, ShapingOptions,
    evaluate_shaped_pair_with_config, shape_text,
};
use serde::Serialize;

use crate::corpus;

pub fn run(root: &Path, font_id: &str, text: &str, ligatures: bool) -> Result<()> {
    let manifest = corpus::load_fonts(root)?;
    let entry = manifest
        .fonts
        .iter()
        .find(|entry| entry.id == font_id)
        .ok_or_else(|| anyhow!("unknown font id {font_id:?}"))?;
    let font_path = entry.local_path(root);
    let font = FontKit::load(&entry.id, &font_path)
        .with_context(|| format!("failed to load {}", font_path.display()))?;
    let config = EvaluationConfig::for_font(&font);
    let run = shape_text(
        &font,
        text,
        ShapingOptions {
            kerning: false,
            ligatures,
        },
    )?;

    let mut pairs = Vec::new();
    for pair in run.adjacent_pairs() {
        let result = evaluate_shaped_pair_with_config(&font, &pair, config, ligatures)?;
        let guarded = result
            .outputs
            .iter()
            .find(|output| output.algorithm == Algorithm::GuardedProfileHybrid)
            .ok_or_else(|| anyhow!("missing guarded output for {}", pair.display))?;
        pairs.push(SamplePairDelta {
            display: pair.display,
            shaping_text: pair.shaping_text,
            left_cluster: pair.left_cluster,
            right_cluster: pair.right_cluster,
            left_glyph_id: pair.left_glyph_id,
            right_glyph_id: pair.right_glyph_id,
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
        text: text.to_owned(),
        ligatures,
        pairs,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SampleDeltaReport {
    schema_version: u32,
    font_id: String,
    family: String,
    text: String,
    ligatures: bool,
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
