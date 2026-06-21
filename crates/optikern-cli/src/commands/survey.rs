use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use optikern_core::{Algorithm, AlgorithmSet, FontKit, SvgBounds, svg_glyph};
use serde::Serialize;

use super::{survey_methods_page, survey_page, survey_results_page};

use crate::corpus;
use crate::data::{BenchFont, BenchReport};

const PAIRS_PER_FONT: usize = 4;
const WORDS_PER_FONT: usize = 6;

const MODES: &[&str] = &[
    "nearest-contour-distance",
    "profile-whitespace",
    "area-balance",
    "metric-prior-hybrid",
    "safe-fallback-only",
];

pub fn run(
    root: &Path,
    submit_endpoint: Option<&str>,
    results_endpoint: Option<&str>,
    repo_url: &str,
) -> Result<()> {
    corpus::ensure_output_dirs(root)?;
    let report = read_bench(root)?;
    let configured_pairs = corpus::load_pairs(root)?;
    let words = corpus::load_words(root)?;
    let by_font = index_results(&report);
    let results_endpoint = derived_results_endpoint(submit_endpoint, results_endpoint);

    let trials = build_trials(root, &report.fonts, &configured_pairs, &words, &by_font)?;
    let trials_json = serde_json::to_string(&trials)?;
    let html = survey_page::build_html(
        &report,
        &trials_json,
        "../corpus/fonts",
        submit_endpoint,
        "methods.html",
        "results.html",
        repo_url,
    )?;
    let path = root.join("reports/survey.html");
    fs::write(&path, html).with_context(|| format!("failed to write {}", path.display()))?;
    println!("wrote {}", path.display());
    let methods = survey_methods_page::build_html(&report, MODES, repo_url);
    let methods_path = root.join("reports/methods.html");
    fs::write(&methods_path, &methods)
        .with_context(|| format!("failed to write {}", methods_path.display()))?;
    println!("wrote {}", methods_path.display());
    let results =
        survey_results_page::build_html(results_endpoint.as_deref(), "survey.html", repo_url);
    let results_path = root.join("reports/results.html");
    fs::write(&results_path, &results)
        .with_context(|| format!("failed to write {}", results_path.display()))?;
    println!("wrote {}", results_path.display());
    write_site_bundle(
        root,
        &report,
        &trials,
        submit_endpoint,
        results_endpoint.as_deref(),
        repo_url,
    )?;
    println!("trials {}", trials.len());
    Ok(())
}

fn write_site_bundle(
    root: &Path,
    report: &BenchReport,
    trials: &[Trial],
    submit_endpoint: Option<&str>,
    results_endpoint: Option<&str>,
    repo_url: &str,
) -> Result<()> {
    let site_dir = root.join("site");
    let stale_font_dir = site_dir.join("assets/fonts");
    if stale_font_dir.exists() {
        fs::remove_dir_all(&stale_font_dir)
            .with_context(|| format!("failed to remove {}", stale_font_dir.display()))?;
    }
    fs::create_dir_all(&site_dir)
        .with_context(|| format!("failed to create {}", site_dir.display()))?;

    let trials_json = serde_json::to_string(trials)?;
    let html = survey_page::build_html(
        report,
        &trials_json,
        "assets/fonts",
        submit_endpoint,
        "methods.html",
        "results.html",
        repo_url,
    )?;
    let index_path = site_dir.join("index.html");
    fs::write(&index_path, html)
        .with_context(|| format!("failed to write {}", index_path.display()))?;
    let methods = survey_methods_page::build_html(report, MODES, repo_url);
    let methods_path = site_dir.join("methods.html");
    fs::write(&methods_path, methods)
        .with_context(|| format!("failed to write {}", methods_path.display()))?;
    let results = survey_results_page::build_html(results_endpoint, "index.html", repo_url);
    let results_path = site_dir.join("results.html");
    fs::write(&results_path, results)
        .with_context(|| format!("failed to write {}", results_path.display()))?;
    fs::write(site_dir.join(".nojekyll"), "")
        .with_context(|| format!("failed to write {}", site_dir.join(".nojekyll").display()))?;
    fs::write(site_dir.join("README.md"), site_readme())
        .with_context(|| format!("failed to write {}", site_dir.join("README.md").display()))?;
    println!("wrote {}", index_path.display());
    println!("wrote {}", methods_path.display());
    println!("wrote {}", results_path.display());
    Ok(())
}

fn derived_results_endpoint(
    submit_endpoint: Option<&str>,
    results_endpoint: Option<&str>,
) -> Option<String> {
    if let Some(endpoint) = results_endpoint.and_then(non_empty) {
        return Some(endpoint.to_owned());
    }
    let endpoint = submit_endpoint.and_then(non_empty)?;
    if let Some(base) = endpoint.strip_suffix("/submit") {
        return Some(format!("{base}/results"));
    }
    Some(format!("{}/results", endpoint.trim_end_matches('/')))
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn read_bench(root: &Path) -> Result<BenchReport> {
    let path = root.join("metrics/bench.json");
    let input =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&input).with_context(|| format!("failed to parse {}", path.display()))
}

fn index_results(report: &BenchReport) -> BTreeMap<String, BTreeMap<String, &AlgorithmSet>> {
    let mut by_font: BTreeMap<String, BTreeMap<String, &AlgorithmSet>> = BTreeMap::new();
    for result in &report.results {
        by_font
            .entry(result.font_id.clone())
            .or_default()
            .insert(result.pair.clone(), result);
    }
    by_font
}

fn build_trials(
    root: &Path,
    fonts: &[BenchFont],
    pairs: &[String],
    words: &[String],
    by_font: &BTreeMap<String, BTreeMap<String, &AlgorithmSet>>,
) -> Result<Vec<Trial>> {
    let mut trials = Vec::new();
    for (font_index, font) in fonts.iter().enumerate() {
        let Some(results) = by_font.get(&font.id) else {
            continue;
        };
        let font_path = root.join(font.path.trim_start_matches("./"));
        let font_kit = FontKit::load(&font.id, &font_path)?;

        let mut added_pairs = 0;
        for index in rotated_indices(pairs.len(), font_index * PAIRS_PER_FONT) {
            if added_pairs >= PAIRS_PER_FONT {
                break;
            }
            let sample = &pairs[index];
            if results.contains_key(sample) && !contains_ligature_sequence(sample) {
                if add_sample_trial(&mut trials, font, &font_kit, "pair", sample, 44.0, results)? {
                    added_pairs += 1;
                }
            }
        }

        let mut added_words = 0;
        for index in rotated_indices(words.len(), font_index * WORDS_PER_FONT) {
            if added_words >= WORDS_PER_FONT {
                break;
            }
            if contains_ligature_sequence(&words[index]) {
                continue;
            }
            if add_sample_trial(
                &mut trials,
                font,
                &font_kit,
                "word",
                &words[index],
                32.0,
                results,
            )? {
                added_words += 1;
            }
        }
    }
    Ok(trials)
}

fn contains_ligature_sequence(sample: &str) -> bool {
    let sample = sample.to_ascii_lowercase();
    sample.contains("ff") || sample.contains("fi") || sample.contains("fl")
}

fn rotated_indices(total: usize, start: usize) -> Vec<usize> {
    if total == 0 {
        return Vec::new();
    }
    (0..total).map(|offset| (start + offset) % total).collect()
}

fn add_sample_trial(
    trials: &mut Vec<Trial>,
    font: &BenchFont,
    font_kit: &FontKit,
    kind: &str,
    sample: &str,
    size_pt: f32,
    results: &BTreeMap<String, &AlgorithmSet>,
) -> Result<bool> {
    let choices = MODES
        .iter()
        .map(|mode| render_choice(font_kit, sample, mode, results, size_pt))
        .collect::<Result<Vec<_>>>()?;
    if !is_informative(&choices) {
        return Ok(false);
    }
    let id = format!("{}:{}:{}:five-way", font.id, kind, sample_id(sample));
    trials.push(Trial {
        id,
        font_id: font.id.clone(),
        family: font.family.clone(),
        category: font.category.clone(),
        kind: kind.to_owned(),
        sample: sample.to_owned(),
        comparison_id: "five-way-candidates".to_owned(),
        size_pt,
        choices,
    });
    Ok(true)
}

fn is_informative(choices: &[Choice]) -> bool {
    choices.iter().enumerate().any(|(index, left)| {
        choices.iter().skip(index + 1).any(|right| {
            left.html != right.html || (left.total_delta_em - right.total_delta_em).abs() > 0.001
        })
    })
}

fn render_choice(
    font: &FontKit,
    sample: &str,
    mode: &str,
    results: &BTreeMap<String, &AlgorithmSet>,
    size_pt: f32,
) -> Result<Choice> {
    let algorithm = parse_algorithm(mode);
    let (html, total_delta_em) = render_svg_sample(font, sample, algorithm, results, size_pt)?;
    Ok(Choice {
        mode: mode.to_owned(),
        label: mode_label(mode),
        total_delta_em,
        html,
    })
}

fn parse_algorithm(mode: &str) -> Option<Algorithm> {
    Algorithm::all()
        .iter()
        .copied()
        .find(|algorithm| algorithm.as_str() == mode)
}

fn render_svg_sample(
    font: &FontKit,
    sample: &str,
    algorithm: Option<Algorithm>,
    results: &BTreeMap<String, &AlgorithmSet>,
    size_pt: f32,
) -> Result<(String, f32)> {
    let chars = sample.chars().collect::<Vec<_>>();
    let mut paths = String::new();
    let mut total_delta_em = 0.0;
    let mut cursor = 0.0;
    let mut bounds: Option<SvgBounds> = None;

    for (index, ch) in chars.iter().enumerate() {
        let glyph = svg_glyph(font, *ch)?;
        if !glyph.path_data.is_empty() {
            let glyph_bounds = glyph.bounds.translated(cursor);
            bounds = Some(merge_bounds(bounds, glyph_bounds));
            paths.push_str(&format!(
                "<path d=\"{}\" transform=\"translate({:.5} 0)\"/>",
                glyph.path_data, cursor
            ));
        }
        cursor += glyph.advance_em;
        if index + 1 >= chars.len() {
            continue;
        }
        let Some(algorithm) = algorithm else {
            continue;
        };
        if chars[index].is_whitespace() || chars[index + 1].is_whitespace() {
            continue;
        }
        let pair = [chars[index], chars[index + 1]].iter().collect::<String>();
        let Some(output) = results.get(&pair).and_then(|set| {
            set.outputs
                .iter()
                .find(|output| output.algorithm == algorithm)
        }) else {
            continue;
        };
        total_delta_em += output.delta_em;
        cursor += output.delta_em;
    }

    let bounds = bounds.unwrap_or(SvgBounds {
        min_x: 0.0,
        min_y: -0.8,
        max_x: cursor.max(0.1),
        max_y: 0.2,
    });
    let pad = 0.05;
    let min_x = bounds.min_x - pad;
    let min_y = bounds.min_y - pad;
    let width_em = (bounds.max_x - bounds.min_x + 2.0 * pad).max(0.1);
    let height_em = (bounds.max_y - bounds.min_y + 2.0 * pad).max(0.1);
    Ok((
        format!(
            "<svg class=\"sample-svg\" viewBox=\"{:.5} {:.5} {:.5} {:.5}\" style=\"--sample-width:{:.2}pt;--sample-height:{:.2}pt\" aria-hidden=\"true\"><g fill=\"currentColor\" fill-rule=\"nonzero\" stroke=\"none\">{}</g></svg>",
            min_x,
            min_y,
            width_em,
            height_em,
            width_em * size_pt,
            height_em * size_pt,
            paths
        ),
        total_delta_em,
    ))
}

fn merge_bounds(existing: Option<SvgBounds>, next: SvgBounds) -> SvgBounds {
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

fn site_readme() -> &'static str {
    "# OptiKern Preference Study Site\n\nThis directory is generated by `cargo run -p optikern-cli -- survey` and can be published with GitHub Pages. `index.html` contains the five-way A-E preference study, `methods.html` explains the algorithms, and `results.html` reads public aggregate results from the configured endpoint. Samples are embedded as SVG paths, so the hosted page does not need runtime font files. The page stores progress locally in the browser and submits results to the configured endpoint.\n"
}

fn mode_label(mode: &str) -> String {
    match mode {
        "none" => "Typst none".to_owned(),
        "metric" => "Typst metric".to_owned(),
        other => other.to_owned(),
    }
}

fn sample_id(sample: &str) -> String {
    sample
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

#[derive(Debug, Serialize)]
struct Trial {
    id: String,
    font_id: String,
    family: String,
    category: String,
    kind: String,
    sample: String,
    comparison_id: String,
    size_pt: f32,
    choices: Vec<Choice>,
}

#[derive(Debug, Serialize)]
struct Choice {
    mode: String,
    label: String,
    total_delta_em: f32,
    html: String,
}
