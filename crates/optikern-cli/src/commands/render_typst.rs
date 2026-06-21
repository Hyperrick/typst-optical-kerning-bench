use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use optikern_core::{Algorithm, AlgorithmOutput, AlgorithmSet};

use crate::corpus;
use crate::data::BenchReport;

pub fn run(root: &Path, compile: bool) -> Result<()> {
    corpus::ensure_output_dirs(root)?;
    let report = read_bench(root)?;
    let manifest = corpus::load_fonts(root)?;
    let configured_pairs = corpus::load_pairs(root)?;
    let words = corpus::load_words(root)?;
    let by_font = index_results(&report);

    let mut source = String::new();
    source.push_str("#set page(paper: \"a4\", margin: 10mm)\n");
    source.push_str("#set text(size: 8pt)\n");
    source.push_str("#let muted(it) = text(size: 6.5pt, fill: rgb(\"555555\"), it)\n");
    source.push_str("#let head(it) = text(size: 13pt, weight: \"bold\", it)\n");
    source.push_str("#let table-stroke = 0.25pt + rgb(\"d8d8d8\")\n\n");
    source.push_str("#text(size: 18pt, weight: \"bold\")[Optical Kerning Bench - Typst Tables]\n");
    source.push_str("#v(8pt)\n");
    source.push_str("#text(size: 9pt)[Generated from metrics/bench.json. Algorithm rows use kerning: false plus explicit h() deltas.]\n");
    source.push_str("#pagebreak()\n");

    for font in &manifest.fonts {
        let Some(results) = by_font.get(&font.id) else {
            continue;
        };

        source.push_str(&format!(
            "#head[{}]\n#v(5pt)\n",
            escape_content(&font.family)
        ));
        source
            .push_str("#text(size: 9pt, weight: \"bold\")[Real Word / Headline Tables]\n#v(4pt)\n");
        source.push_str("#muted[These rows apply pairwise deltas across full words. Over-tight rows are failure cases, not recommendations.]\n#v(4pt)\n");
        for word in words.iter().take(8) {
            source.push_str(&render_word_table(&font.family, word, results));
            source.push_str("#v(6pt)\n");
        }

        source.push_str("#pagebreak()\n");
        source.push_str(&format!(
            "#head[{}]\n#v(5pt)\n",
            escape_content(&font.family)
        ));
        source.push_str("#text(size: 9pt, weight: \"bold\")[Critical Pair Tables]\n#v(4pt)\n");
        for pair in configured_pairs
            .iter()
            .filter(|pair| results.contains_key(*pair))
            .take(5)
        {
            source.push_str(&render_pair_table(&font.family, pair, results));
            source.push_str("#v(6pt)\n");
        }
        source.push_str("#pagebreak()\n");
    }

    let typ_path = root.join("renders/typst/typst-comparison.typ");
    fs::write(&typ_path, source)
        .with_context(|| format!("failed to write {}", typ_path.display()))?;
    println!("wrote {}", typ_path.display());

    if compile {
        let pdf_path = root.join("renders/typst/typst-comparison.pdf");
        let status = Command::new("typst")
            .arg("compile")
            .arg("--font-path")
            .arg(root.join("corpus/fonts"))
            .arg(&typ_path)
            .arg(&pdf_path)
            .status()
            .context("failed to start typst")?;
        if !status.success() {
            return Err(anyhow!("typst compile failed with {status}"));
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

fn render_pair_table(
    family: &str,
    pair: &str,
    results: &BTreeMap<String, &AlgorithmSet>,
) -> String {
    let mut source = String::new();
    let metric_delta = results
        .get(pair)
        .and_then(|set| set.outputs.first())
        .map(|out| out.metric_delta_em)
        .unwrap_or_default();

    source.push_str(&format!(
        "#muted[Pair {} / metric delta {:.4}em]\n",
        escape_content(pair),
        metric_delta
    ));
    source.push_str(
        "#table(columns: (31mm, 62mm, 25mm, 22mm), inset: 3.5pt, stroke: table-stroke,\n",
    );
    source.push_str("[#strong[Mode]], [#strong[Rendered]], [#strong[Delta]], [#strong[MAD]],\n");
    source.push_str(&render_pair_row(family, "none", pair, None, false));
    source.push_str(&render_pair_row(family, "metric", pair, None, true));

    if let Some(set) = results.get(pair) {
        for algorithm in Algorithm::all() {
            if let Some(output) = set
                .outputs
                .iter()
                .find(|output| output.algorithm == *algorithm)
            {
                source.push_str(&render_pair_row(
                    family,
                    algorithm.as_str(),
                    pair,
                    Some(output),
                    false,
                ));
            }
        }
    }
    source.push_str(")\n");
    source
}

fn render_word_table(
    family: &str,
    word: &str,
    results: &BTreeMap<String, &AlgorithmSet>,
) -> String {
    let mut source = String::new();
    source.push_str(&format!("#muted[Word {}]\n", escape_content(word)));
    source.push_str("#table(columns: (31mm, 102mm, 25mm), inset: 3.5pt, stroke: table-stroke,\n");
    source.push_str("[#strong[Mode]], [#strong[Rendered]], [#strong[Total delta]],\n");
    source.push_str(&render_word_row(family, "none", word, None, results, false));
    source.push_str(&render_word_row(
        family, "metric", word, None, results, true,
    ));
    for algorithm in Algorithm::all() {
        source.push_str(&render_word_row(
            family,
            algorithm.as_str(),
            word,
            Some(*algorithm),
            results,
            false,
        ));
    }
    source.push_str(")\n");
    source
}

fn render_pair_row(
    family: &str,
    label: &str,
    pair: &str,
    output: Option<&AlgorithmOutput>,
    metric_kerning: bool,
) -> String {
    let body = render_pair_body(pair, output.map(|output| output.delta_em));
    let delta = output
        .map(|output| format!("{:.4}em", output.delta_em))
        .unwrap_or_else(|| "--".to_owned());
    let mad = output
        .map(|output| format!("{:.4}", output.gap_mad_em))
        .unwrap_or_else(|| "--".to_owned());
    format!(
        "[{}], [{}], [{}], [{}],\n",
        escape_content(label),
        render_text_cell(family, 23.0, metric_kerning, &body),
        delta,
        mad
    )
}

fn render_word_row(
    family: &str,
    label: &str,
    word: &str,
    algorithm: Option<Algorithm>,
    results: &BTreeMap<String, &AlgorithmSet>,
    metric_kerning: bool,
) -> String {
    let (body, total_delta) = render_word_body(word, algorithm, results);
    let delta = algorithm
        .map(|_| format!("{:.4}em", total_delta))
        .unwrap_or_else(|| "--".to_owned());
    format!(
        "[{}], [{}], [{}],\n",
        escape_content(label),
        render_text_cell(family, 18.0, metric_kerning, &body),
        delta
    )
}

fn render_text_cell(family: &str, size: f32, metric_kerning: bool, body: &str) -> String {
    format!(
        "#text(font: \"{}\", size: {:.1}pt, kerning: {})[{}]",
        escape_string(family),
        size,
        if metric_kerning { "true" } else { "false" },
        body
    )
}

fn render_pair_body(pair: &str, delta: Option<f32>) -> String {
    let mut chars = pair.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let Some(second) = chars.next() else {
        return escape_content(&first.to_string());
    };
    match delta {
        Some(delta) => format!(
            "{}#h({:.5}em){}",
            escape_content(&first.to_string()),
            delta,
            escape_content(&second.to_string())
        ),
        None => format!(
            "{}{}",
            escape_content(&first.to_string()),
            escape_content(&second.to_string())
        ),
    }
}

fn render_word_body(
    word: &str,
    algorithm: Option<Algorithm>,
    results: &BTreeMap<String, &AlgorithmSet>,
) -> (String, f32) {
    let chars = word.chars().collect::<Vec<_>>();
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
