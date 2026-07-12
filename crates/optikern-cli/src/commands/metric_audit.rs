use std::collections::HashSet;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use optikern_core::{
    Algorithm, EvaluationConfig, FontKit, ShapingOptions, evaluate_shaped_pair_with_config,
    metric_shaped_pair_delta_em, shape_text,
};
use serde::Serialize;
use ttf_parser::GlyphId;

use crate::corpus::{self, FontEntry};

const DEFAULT_CHARACTERS: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789.,:;!?-–—()[]{}'\"/&@";
const METRIC_EPSILON_EM: f32 = 0.0001;

pub fn run(
    root: &Path,
    font_ids: &str,
    characters: &str,
    algorithm: Algorithm,
    output: &Path,
    top: usize,
) -> Result<()> {
    let manifest = corpus::load_fonts(root)?;
    let fonts = select_fonts(&manifest.fonts, font_ids)?;
    let characters = unique_characters(if characters.is_empty() {
        DEFAULT_CHARACTERS
    } else {
        characters
    });
    let mut rows = Vec::new();
    let mut font_summaries = Vec::new();

    for entry in fonts {
        let font = FontKit::load(&entry.id, entry.local_path(root)).with_context(|| {
            format!(
                "failed to load {}; run `optikern fetch-fonts` first",
                entry.local_path(root).display()
            )
        })?;
        let config = EvaluationConfig::for_font(&font);
        let start = rows.len();
        let mut evaluated_pairs = 0usize;
        let mut skipped_pairs = 0usize;

        for &left in &characters {
            for &right in &characters {
                evaluated_pairs += 1;
                match audit_pair(&font, entry, left, right, config, algorithm) {
                    Ok(Some(row)) => rows.push(row),
                    Ok(None) => {}
                    Err(_) => skipped_pairs += 1,
                }
            }
        }

        font_summaries.push(FontSummary {
            font_id: entry.id.clone(),
            family: entry.family.clone(),
            evaluated_pairs,
            metric_pair_count: rows.len() - start,
            skipped_pairs,
        });
        println!(
            "{}: {} effective metric pairs from {} candidates",
            entry.id,
            rows.len() - start,
            evaluated_pairs
        );
    }

    rows.sort_by(|left, right| {
        left.font_id
            .cmp(&right.font_id)
            .then_with(|| left.left_glyph_id.cmp(&right.left_glyph_id))
            .then_with(|| left.right_glyph_id.cmp(&right.right_glyph_id))
    });
    let mut largest = rows.clone();
    largest.sort_by(|left, right| {
        right
            .absolute_difference_em
            .total_cmp(&left.absolute_difference_em)
            .then_with(|| left.font_id.cmp(&right.font_id))
    });
    largest.truncate(top.min(largest.len()));

    let report = AuditReport {
        schema_version: 1,
        algorithm,
        metric_source: "rustybuzz-effective-pair-positioning",
        metric_epsilon_em: METRIC_EPSILON_EM,
        characters: characters.iter().collect(),
        font_count: font_summaries.len(),
        metric_pair_count: rows.len(),
        summary: DifferenceSummary::from_rows(&rows),
        fonts: font_summaries,
        top_differences: largest.clone(),
    };

    let output = root.join(output);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        output.with_extension("json"),
        serde_json::to_string_pretty(&report)? + "\n",
    )?;
    fs::write(output.with_extension("tsv"), write_tsv(&rows))?;
    fs::write(
        output.with_file_name(format!(
            "{}-top{}.tsv",
            output
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("metric-agreement-audit"),
            largest.len()
        )),
        write_tsv(&largest),
    )?;
    println!("Wrote {} metric pairs", rows.len());
    Ok(())
}

fn select_fonts<'a>(fonts: &'a [FontEntry], requested: &str) -> Result<Vec<&'a FontEntry>> {
    let requested = requested
        .split(',')
        .map(str::trim)
        .filter(|id| !id.is_empty())
        .collect::<HashSet<_>>();
    let selected = fonts
        .iter()
        .filter(|font| {
            if requested.is_empty() {
                font.url.is_none()
            } else {
                requested.contains(font.id.as_str())
            }
        })
        .collect::<Vec<_>>();
    if !requested.is_empty() {
        let found = selected
            .iter()
            .map(|font| font.id.as_str())
            .collect::<HashSet<_>>();
        let mut missing = requested.difference(&found).copied().collect::<Vec<_>>();
        missing.sort_unstable();
        if !missing.is_empty() {
            return Err(anyhow!("unknown font id(s): {}", missing.join(", ")));
        }
    }
    Ok(selected)
}

fn unique_characters(value: &str) -> Vec<char> {
    let mut seen = HashSet::new();
    value
        .chars()
        .filter(|character| !character.is_whitespace() && seen.insert(*character))
        .collect()
}

fn audit_pair(
    font: &FontKit,
    entry: &FontEntry,
    left: char,
    right: char,
    config: EvaluationConfig,
    algorithm: Algorithm,
) -> Result<Option<AuditRow>> {
    let text = format!("{left}{right}");
    let run = shape_text(font, &text, ShapingOptions::typst_pair())?;
    let Some(pair) = run.adjacent_pairs().into_iter().next() else {
        return Ok(None);
    };
    let metric_delta = metric_shaped_pair_delta_em(font, &pair, false)?;
    if metric_delta.abs() < METRIC_EPSILON_EM {
        return Ok(None);
    }
    let result = evaluate_shaped_pair_with_config(font, &pair, config, false)?;
    let calculated = result
        .outputs
        .iter()
        .find(|output| output.algorithm == algorithm)
        .ok_or_else(|| anyhow!("missing {} result for {text:?}", algorithm.as_str()))?
        .delta_em;
    let difference = calculated - metric_delta;
    let face = font.face()?;

    Ok(Some(AuditRow {
        font_id: entry.id.clone(),
        font_family: entry.family.clone(),
        font_path: entry.path.clone(),
        left_character: left.to_string(),
        right_character: right.to_string(),
        left_glyph_id: result.left_glyph_id,
        right_glyph_id: result.right_glyph_id,
        left_glyph_name: glyph_name(&face, result.left_glyph_id),
        right_glyph_name: glyph_name(&face, result.right_glyph_id),
        available_kern_em: metric_delta,
        calculated_kern_em: calculated,
        difference_em: difference,
        absolute_difference_em: difference.abs(),
        sign_changed: metric_delta.signum() != calculated.signum() && calculated.abs() > 0.0001,
    }))
}

fn glyph_name(face: &ttf_parser::Face<'_>, glyph_id: u16) -> String {
    face.glyph_name(GlyphId(glyph_id))
        .map(str::to_owned)
        .unwrap_or_else(|| format!("gid{glyph_id}"))
}

fn write_tsv(rows: &[AuditRow]) -> String {
    let mut output = String::from(
        "fontFileId\tfontFamily\tfontPath\tleftCharacter\trightCharacter\tleftGlyphId\trightGlyphId\tleftGlyphName\trightGlyphName\tavailableKernEm\tcalculatedKernEm\tdifferenceEm\tabsDifferenceEm\tsignChanged\n",
    );
    for row in rows {
        writeln!(
            output,
            "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{:.6}\t{:.6}\t{:.6}\t{:.6}\t{}",
            tsv_cell(&row.font_id),
            tsv_cell(&row.font_family),
            tsv_cell(&row.font_path),
            tsv_cell(&row.left_character),
            tsv_cell(&row.right_character),
            row.left_glyph_id,
            row.right_glyph_id,
            tsv_cell(&row.left_glyph_name),
            tsv_cell(&row.right_glyph_name),
            row.available_kern_em,
            row.calculated_kern_em,
            row.difference_em,
            row.absolute_difference_em,
            row.sign_changed,
        )
        .expect("writing to a String cannot fail");
    }
    output
}

fn tsv_cell(value: &str) -> String {
    value.replace(['\t', '\r', '\n'], " ")
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditRow {
    font_id: String,
    font_family: String,
    font_path: String,
    left_character: String,
    right_character: String,
    left_glyph_id: u16,
    right_glyph_id: u16,
    left_glyph_name: String,
    right_glyph_name: String,
    available_kern_em: f32,
    calculated_kern_em: f32,
    difference_em: f32,
    absolute_difference_em: f32,
    sign_changed: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FontSummary {
    font_id: String,
    family: String,
    evaluated_pairs: usize,
    metric_pair_count: usize,
    skipped_pairs: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DifferenceSummary {
    mean_absolute_difference_em: f32,
    median_absolute_difference_em: f32,
    p95_absolute_difference_em: f32,
    p99_absolute_difference_em: f32,
    maximum_absolute_difference_em: f32,
    within_005_em: usize,
    within_010_em: usize,
    sign_changed: usize,
}

impl DifferenceSummary {
    fn from_rows(rows: &[AuditRow]) -> Self {
        let mut differences = rows
            .iter()
            .map(|row| row.absolute_difference_em)
            .collect::<Vec<_>>();
        differences.sort_by(f32::total_cmp);
        let mean = if differences.is_empty() {
            0.0
        } else {
            differences.iter().sum::<f32>() / differences.len() as f32
        };
        Self {
            mean_absolute_difference_em: mean,
            median_absolute_difference_em: percentile(&differences, 0.5),
            p95_absolute_difference_em: percentile(&differences, 0.95),
            p99_absolute_difference_em: percentile(&differences, 0.99),
            maximum_absolute_difference_em: differences.last().copied().unwrap_or(0.0),
            within_005_em: differences.iter().filter(|value| **value <= 0.005).count(),
            within_010_em: differences.iter().filter(|value| **value <= 0.010).count(),
            sign_changed: rows.iter().filter(|row| row.sign_changed).count(),
        }
    }
}

fn percentile(sorted: &[f32], percentile: f32) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() - 1) as f32 * percentile).round() as usize;
    sorted[index]
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AuditReport {
    schema_version: u32,
    algorithm: Algorithm,
    metric_source: &'static str,
    metric_epsilon_em: f32,
    characters: String,
    font_count: usize,
    metric_pair_count: usize,
    summary: DifferenceSummary,
    fonts: Vec<FontSummary>,
    top_differences: Vec<AuditRow>,
}

#[cfg(test)]
mod tests {
    use super::{percentile, tsv_cell, unique_characters};

    #[test]
    fn character_set_is_ordered_unique_and_excludes_spaces() {
        assert_eq!(unique_characters("AB A!B"), vec!['A', 'B', '!']);
    }

    #[test]
    fn tsv_cells_cannot_create_extra_rows_or_columns() {
        assert_eq!(tsv_cell("a\tb\nc"), "a b c");
    }

    #[test]
    fn percentile_uses_nearest_rank_index() {
        assert_eq!(percentile(&[0.0, 1.0, 2.0, 3.0, 4.0], 0.5), 2.0);
        assert_eq!(percentile(&[], 0.95), 0.0);
    }
}
