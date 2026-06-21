use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use optikern_core::{Algorithm, AlgorithmSet};

use crate::corpus;
use crate::data::BenchReport;

const CONTACT_SAMPLES: &[&str] = &["AV", "To", "AVATAR", "Typography", "Negative space"];
const FONTS_PER_PAGE: usize = 5;

pub fn run(root: &Path, compile: bool) -> Result<()> {
    corpus::ensure_output_dirs(root)?;
    let report = read_bench(root)?;
    let by_font = index_results(&report);
    let source = build_typst(&report, &by_font);
    let typ_path = root.join("reports/contact-sheet.typ");
    fs::write(&typ_path, source)
        .with_context(|| format!("failed to write {}", typ_path.display()))?;
    println!("wrote {}", typ_path.display());

    if compile {
        let pdf_path = root.join("reports/contact-sheet.pdf");
        let status = Command::new("typst")
            .arg("compile")
            .arg("--font-path")
            .arg(root.join("corpus/fonts"))
            .arg(&typ_path)
            .arg(&pdf_path)
            .status()
            .context("failed to start typst")?;
        if !status.success() {
            return Err(anyhow!("typst contact sheet compile failed with {status}"));
        }
        println!("wrote {}", pdf_path.display());
    }

    Ok(())
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

fn build_typst(
    report: &BenchReport,
    by_font: &BTreeMap<String, BTreeMap<String, &AlgorithmSet>>,
) -> String {
    let mut source = String::new();
    source.push_str("#set page(width: 420mm, height: 297mm, margin: 8mm)\n");
    source.push_str("#set text(size: 6pt)\n");
    source.push_str("#let stroke = 0.25pt + rgb(\"d8d8d8\")\n");
    source.push_str("#let muted(it) = text(size: 4.8pt, fill: rgb(\"666666\"), it)\n\n");

    for (page_index, chunk) in report.fonts.chunks(FONTS_PER_PAGE).enumerate() {
        if page_index > 0 {
            source.push_str("#pagebreak()\n");
        }
        source.push_str("#text(size: 16pt, weight: \"bold\")[Optical Kerning Contact Sheet]\n");
        source.push_str("#h(1fr)\n");
        source.push_str(&format!(
            "#text(size: 6pt)[page {} / {}]\n#v(4pt)\n",
            page_index + 1,
            report.fonts.len().div_ceil(FONTS_PER_PAGE)
        ));
        source.push_str("#text(size: 7pt)[All algorithm columns use `kerning: false` plus explicit `h()` deltas between every adjacent non-space glyph pair. Metric keeps font kerning enabled.]\n#v(5pt)\n");
        source.push_str("#table(\n");
        source.push_str("  columns: (33mm, 48mm, 48mm, 48mm, 48mm, 48mm, 48mm, 48mm),\n");
        source.push_str("  inset: (x: 2.3pt, y: 2.5pt),\n");
        source.push_str("  stroke: stroke,\n");
        source.push_str("  [#strong[Font / sample]], [#strong[None]], [#strong[Metric]], [#strong[Nearest]], [#strong[Profile]], [#strong[Area]], [#strong[Hybrid]], [#strong[Fallback]],\n");

        for font in chunk {
            let Some(results) = by_font.get(&font.id) else {
                continue;
            };
            for sample in CONTACT_SAMPLES {
                source.push_str(&render_row(&font.family, &font.category, sample, results));
            }
        }
        source.push_str(")\n");
    }

    source
}

fn render_row(
    family: &str,
    category: &str,
    sample: &str,
    results: &BTreeMap<String, &AlgorithmSet>,
) -> String {
    let mut row = String::new();
    row.push_str(&format!(
        "  [{}], {}, {},",
        label_cell(family, category, sample),
        render_cell(family, sample, None, results, false),
        render_cell(family, sample, None, results, true),
    ));
    for algorithm in Algorithm::all() {
        row.push(' ');
        row.push_str(&render_cell(
            family,
            sample,
            Some(*algorithm),
            results,
            false,
        ));
        row.push(',');
    }
    row.push('\n');
    row
}

fn label_cell(family: &str, category: &str, sample: &str) -> String {
    format!(
        "#text(size: 5.2pt)[#strong[{}]#linebreak(){}#linebreak()#muted[{}]]",
        escape_content(family),
        escape_content(sample),
        escape_content(category)
    )
}

fn render_cell(
    family: &str,
    sample: &str,
    algorithm: Option<Algorithm>,
    results: &BTreeMap<String, &AlgorithmSet>,
    metric_kerning: bool,
) -> String {
    let (body, total_delta) = render_sample_body(sample, algorithm, results);
    let delta = algorithm
        .map(|_| format!("{total_delta:+.3}em"))
        .unwrap_or_else(|| "baseline".to_owned());
    format!(
        "[#block(width: 45mm)[#text(font: \"{}\", size: 12pt, kerning: {})[{}]#linebreak()#muted[{}]]]",
        escape_string(family),
        if metric_kerning { "true" } else { "false" },
        body,
        escape_content(&delta)
    )
}

fn render_sample_body(
    sample: &str,
    algorithm: Option<Algorithm>,
    results: &BTreeMap<String, &AlgorithmSet>,
) -> (String, f32) {
    let chars = sample.chars().collect::<Vec<_>>();
    let mut body = String::new();
    let mut total_delta = 0.0;

    for (index, ch) in chars.iter().enumerate() {
        body.push_str(&escape_content(&ch.to_string()));
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
        total_delta += output.delta_em;
        body.push_str(&format!("#h({:.5}em)", output.delta_em));
    }

    (body, total_delta)
}

fn escape_content(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('#', "\\#")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn escape_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}
