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
    algorithm: Algorithm,
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
    let mut selected_deltas = Vec::new();
    for result in evaluate_shaped_run_with_config(&font, &run, config, ligatures)? {
        let selected = result
            .outputs
            .iter()
            .find(|output| output.algorithm == algorithm)
            .ok_or_else(|| {
                anyhow!(
                    "missing {} output for {}",
                    algorithm.as_str(),
                    result.display
                )
            })?;
        selected_deltas.push((result.left_index, selected.delta_em));
        pairs.push(SamplePairDelta {
            display: result.display,
            shaping_text: result.shaping_text,
            left_index: result.left_index,
            right_index: result.right_index,
            left_cluster: result.left_cluster,
            right_cluster: result.right_cluster,
            left_glyph_id: result.left_glyph_id,
            right_glyph_id: result.right_glyph_id,
            delta_em: selected.delta_em,
            metric_delta_em: selected.metric_delta_em,
            optical_delta_em: selected.optical_delta_em,
            outputs: result.outputs,
        });
    }
    let fragments = build_fragments(&run, &selected_deltas);

    let output = SampleDeltaReport {
        schema_version: 2,
        font_id: entry.id.clone(),
        family: entry.family.clone(),
        font_path: font_path.display().to_string(),
        text: text.to_owned(),
        ligatures,
        contextual_alternates: run.options.contextual_alternates,
        algorithm,
        fragmented_render_safe,
        fragments,
        pairs,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn build_fragments(run: &ShapedRun, selected_deltas: &[(usize, f32)]) -> Vec<SampleFragment> {
    let mut delta_after = vec![0.0; run.glyphs.len()];
    for &(left_index, delta) in selected_deltas {
        if let Some(slot) = delta_after.get_mut(left_index) {
            *slot = delta;
        }
    }
    run.glyphs
        .iter()
        .zip(delta_after)
        .map(|(glyph, delta_after_em)| SampleFragment {
            text: glyph.cluster_text.clone(),
            delta_after_em,
        })
        .collect()
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

    Ok(run
        .glyphs
        .iter()
        .zip(fragmented.iter())
        .all(|(full, part)| {
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
    algorithm: Algorithm,
    fragmented_render_safe: bool,
    fragments: Vec<SampleFragment>,
    pairs: Vec<SamplePairDelta>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SampleFragment {
    text: String,
    delta_after_em: f32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SamplePairDelta {
    display: String,
    shaping_text: String,
    left_index: usize,
    right_index: usize,
    left_cluster: String,
    right_cluster: String,
    left_glyph_id: u16,
    right_glyph_id: u16,
    delta_em: f32,
    metric_delta_em: f32,
    optical_delta_em: f32,
    outputs: Vec<AlgorithmOutput>,
}

#[cfg(test)]
mod tests {
    use optikern_core::{ShapedGlyph, ShapedRun, ShapingOptions};

    use super::build_fragments;

    #[test]
    fn fragments_preserve_spaces_between_independent_pair_runs() {
        let run = ShapedRun {
            text: "A0 POSTER".into(),
            options: ShapingOptions::typst_pair(),
            glyphs: "A0 POSTER"
                .char_indices()
                .map(|(index, ch)| ShapedGlyph {
                    glyph_id: index as u16,
                    cluster_start: index,
                    cluster_end: index + ch.len_utf8(),
                    cluster_text: ch.to_string(),
                    x_advance_em: 0.5,
                    y_advance_em: 0.0,
                    x_offset_em: 0.0,
                    y_offset_em: 0.0,
                })
                .collect(),
        };

        let fragments = build_fragments(&run, &[(0, -0.01), (3, -0.02)]);
        assert_eq!(
            fragments
                .iter()
                .map(|item| item.text.as_str())
                .collect::<String>(),
            "A0 POSTER"
        );
        assert_eq!(fragments[0].delta_after_em, -0.01);
        assert_eq!(fragments[2].text, " ");
        assert_eq!(fragments[3].delta_after_em, -0.02);
    }
}
