use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use optikern_core::Algorithm;

use crate::corpus;
use crate::data::BenchReport;

pub fn run(root: &Path, compile: bool) -> Result<()> {
    corpus::ensure_output_dirs(root)?;
    let report = read_bench(root)?;

    let html = build_html(&report);
    let html_path = root.join("reports/index.html");
    fs::write(&html_path, html)
        .with_context(|| format!("failed to write {}", html_path.display()))?;
    println!("wrote {}", html_path.display());

    let typ = build_typst(&report);
    let typ_path = root.join("reports/summary.typ");
    fs::write(&typ_path, typ).with_context(|| format!("failed to write {}", typ_path.display()))?;
    println!("wrote {}", typ_path.display());

    if compile {
        let pdf_path = root.join("reports/summary.pdf");
        let status = Command::new("typst")
            .arg("compile")
            .arg(&typ_path)
            .arg(&pdf_path)
            .status()
            .context("failed to start typst")?;
        if !status.success() {
            return Err(anyhow!("typst report compile failed with {status}"));
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

fn build_html(report: &BenchReport) -> String {
    let summaries = summarize(report);
    let mut html = String::new();
    html.push_str(
        "<!doctype html><meta charset=\"utf-8\"><title>Typst Optical Kerning Bench</title>",
    );
    html.push_str("<style>body{font-family:system-ui,sans-serif;max-width:1100px;margin:40px auto;line-height:1.45}table{border-collapse:collapse;width:100%;margin:20px 0}td,th{border:1px solid #ddd;padding:6px 8px;text-align:left}th{background:#f5f5f5}.num{text-align:right;font-variant-numeric:tabular-nums}code{background:#f6f6f6;padding:1px 4px}</style>");
    html.push_str("<h1>Typst Optical Kerning Bench</h1>");
    html.push_str(&format!(
        "<p>Fonts: <b>{}</b>. Pair cases: <b>{}</b>. Successful results: <b>{}</b>. Failures: <b>{}</b>. Runtime: <b>{}ms</b>.</p>",
        report.fonts.len(),
        report.pair_count,
        report.results.len(),
        report.failures.len(),
        report.runtime_ms
    ));
    html.push_str("<h2>Algorithm Summary</h2><table><tr><th>Algorithm</th><th>Mean |delta| em</th><th>Mean gap MAD em</th><th>Mean collision score</th><th>Samples</th></tr>");
    for summary in summaries {
        html.push_str(&format!(
            "<tr><td><code>{}</code></td><td class=\"num\">{:.5}</td><td class=\"num\">{:.5}</td><td class=\"num\">{:.5}</td><td class=\"num\">{}</td></tr>",
            summary.algorithm,
            summary.mean_abs_delta,
            summary.mean_gap_mad,
            summary.mean_collision_score,
            summary.samples
        ));
    }
    html.push_str("</table>");

    html.push_str("<h2>Largest Optical Adjustments</h2><table><tr><th>Font</th><th>Pair</th><th>Algorithm</th><th>Delta em</th><th>Metric delta em</th><th>Gap mean em</th></tr>");
    let mut rows = report
        .results
        .iter()
        .flat_map(|set| set.outputs.iter().map(move |out| (set, out)))
        .collect::<Vec<_>>();
    rows.sort_by(|(_, a), (_, b)| b.delta_em.abs().total_cmp(&a.delta_em.abs()));
    for (set, out) in rows.into_iter().take(40) {
        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td><code>{}</code></td><td class=\"num\">{:.5}</td><td class=\"num\">{:.5}</td><td class=\"num\">{:.5}</td></tr>",
            escape_html(&set.font_id),
            escape_html(&set.pair),
            out.algorithm.as_str(),
            out.delta_em,
            out.metric_delta_em,
            out.gap_weighted_mean_em
        ));
    }
    html.push_str("</table>");

    if !report.failures.is_empty() {
        html.push_str("<h2>Failures</h2><table><tr><th>Font</th><th>Pair</th><th>Reason</th></tr>");
        for failure in report.failures.iter().take(80) {
            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape_html(&failure.font_id),
                escape_html(&failure.pair),
                escape_html(&failure.reason)
            ));
        }
        html.push_str("</table>");
    }
    html
}

fn build_typst(report: &BenchReport) -> String {
    let summaries = summarize(report);
    let mut typ = String::new();
    typ.push_str("#set page(paper: \"a4\", margin: 18mm)\n#set text(size: 10pt)\n");
    typ.push_str("= Typst Optical Kerning Bench\n\n");
    typ.push_str(&format!(
        "Fonts: #strong[{}] \\\nPair cases: #strong[{}] \\\nSuccessful results: #strong[{}] \\\nFailures: #strong[{}] \\\nRuntime: #strong[{}ms]\n\n",
        report.fonts.len(),
        report.pair_count,
        report.results.len(),
        report.failures.len(),
        report.runtime_ms
    ));
    typ.push_str("== Algorithm Summary\n\n");
    typ.push_str("#table(columns: 5, inset: 5pt, [Algorithm], [Mean abs delta], [Mean gap MAD], [Collision], [Samples],\n");
    for summary in summaries {
        typ.push_str(&format!(
            "[{}], [{:.5}], [{:.5}], [{:.5}], [{}],\n",
            escape_typ(&summary.algorithm),
            summary.mean_abs_delta,
            summary.mean_gap_mad,
            summary.mean_collision_score,
            summary.samples
        ));
    }
    typ.push_str(")\n\n");
    typ.push_str("== Notes\n\n");
    typ.push_str("- InDesign Optical is used as an external baseline, not as ground truth.\n");
    typ.push_str("- Algorithm sheets use Typst `kerning: false` and explicit `h()` deltas.\n");
    typ.push_str("- Lower collision scores and lower gap MAD are better, but visual inspection remains required.\n");
    typ
}

#[derive(Debug)]
struct Summary {
    algorithm: String,
    mean_abs_delta: f32,
    mean_gap_mad: f32,
    mean_collision_score: f32,
    samples: usize,
}

fn summarize(report: &BenchReport) -> Vec<Summary> {
    let mut by_algorithm: BTreeMap<Algorithm, Vec<_>> = BTreeMap::new();
    for set in &report.results {
        for output in &set.outputs {
            by_algorithm
                .entry(output.algorithm)
                .or_default()
                .push(output);
        }
    }

    by_algorithm
        .into_iter()
        .map(|(algorithm, outputs)| {
            let samples = outputs.len();
            let mean_abs_delta =
                outputs.iter().map(|out| out.delta_em.abs()).sum::<f32>() / samples.max(1) as f32;
            let mean_gap_mad =
                outputs.iter().map(|out| out.gap_mad_em).sum::<f32>() / samples.max(1) as f32;
            let mean_collision_score = outputs
                .iter()
                .map(|out| {
                    if out.gap_min_em < 0.0 {
                        -out.gap_min_em
                    } else {
                        0.0
                    }
                })
                .sum::<f32>()
                / samples.max(1) as f32;
            Summary {
                algorithm: algorithm.as_str().to_owned(),
                mean_abs_delta,
                mean_gap_mad,
                mean_collision_score,
                samples,
            }
        })
        .collect()
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_typ(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('#', "\\#")
        .replace('[', "\\[")
        .replace(']', "\\]")
}
