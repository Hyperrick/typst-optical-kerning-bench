use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use optikern_core::{
    Algorithm, AlgorithmSet, FontKit, ShapedGlyphPair, ShapingOptions, shape_text,
};

use crate::corpus;
use crate::data::BenchReport;

const CONTACT_SAMPLES: &[&str] = &["AV", "To", "AVATAR", "Typography", "Goldfish", "office"];
const FONTS_PER_PAGE: usize = 1;
const MANUAL_VARIANTS: &[ManualVariant] = &[
    ManualVariant {
        label: "Standard",
        metric_adjust_em: 0.0,
    },
    ManualVariant {
        label: "Metric -40",
        metric_adjust_em: -0.040,
    },
    ManualVariant {
        label: "Metric -20",
        metric_adjust_em: -0.020,
    },
    ManualVariant {
        label: "Metric +20",
        metric_adjust_em: 0.020,
    },
    ManualVariant {
        label: "Metric +40",
        metric_adjust_em: 0.040,
    },
];

#[derive(Debug, Clone, Copy)]
struct ManualVariant {
    label: &'static str,
    metric_adjust_em: f32,
}

pub fn run(root: &Path, compile: bool) -> Result<()> {
    corpus::ensure_output_dirs(root)?;
    let report = read_bench(root)?;
    let by_font = index_results(&report);
    let source = build_typst(root, &report, &by_font)?;
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
    root: &Path,
    report: &BenchReport,
    by_font: &BTreeMap<String, BTreeMap<String, &AlgorithmSet>>,
) -> Result<String> {
    let mut source = String::new();
    source.push_str("#set page(width: 420mm, height: 297mm, margin: 8mm)\n");
    source.push_str("#set text(size: 6pt)\n");
    source.push_str("#let stroke = 0.25pt + rgb(\"d8d8d8\")\n");
    source.push_str("#let muted(it) = text(size: 4.8pt, fill: rgb(\"666666\"), it)\n\n");
    source.push_str("#let tiny(it) = text(size: 3.8pt, fill: rgb(\"555555\"), it)\n\n");

    for (page_index, chunk) in report.fonts.chunks(FONTS_PER_PAGE).enumerate() {
        if page_index > 0 {
            source.push_str("#pagebreak()\n");
        }
        source.push_str("#text(size: 16pt, weight: \"bold\")[Optical Kerning Contact Sheet]\n");
        source.push_str("#h(1fr)\n");
        source.push_str(&format!(
            "#text(size: 6pt)[font sheet {} / {}]\n#v(4pt)\n",
            page_index + 1,
            report.fonts.len().div_ceil(FONTS_PER_PAGE)
        ));
        source.push_str("#text(size: 7pt)[Rows compare simulated metric kerning plus optional manual pair adjustment against guarded optical kerning. Manual values use 1/1000 em units, so -20 means -0.020em per adjacent non-space glyph pair.]\n#v(5pt)\n");
        source.push_str("#table(\n");
        source.push_str("  columns: (34mm, 24mm, 82mm, 82mm),\n");
        source.push_str("  inset: (x: 2.5pt, y: 2.6pt),\n");
        source.push_str("  stroke: stroke,\n");
        source.push_str("  [#strong[Font / sample]], [#strong[Variant]], [#strong[Metric kerning + manual]], [#strong[Guarded optical]],\n");

        for font in chunk {
            let Some(results) = by_font.get(&font.id) else {
                continue;
            };
            let font_path = root.join(font.path.trim_start_matches("./"));
            let font_kit = FontKit::load(&font.id, &font_path)
                .with_context(|| format!("failed to load {}", font_path.display()))?;
            for sample in CONTACT_SAMPLES {
                for variant in MANUAL_VARIANTS {
                    source.push_str(&render_row(
                        &font.family,
                        &font.category,
                        sample,
                        &font_kit,
                        results,
                        *variant,
                    )?);
                }
            }
        }
        source.push_str(")\n");
    }

    Ok(source)
}

fn render_row(
    family: &str,
    category: &str,
    sample: &str,
    font: &FontKit,
    results: &BTreeMap<String, &AlgorithmSet>,
    variant: ManualVariant,
) -> Result<String> {
    let mut row = String::new();
    row.push_str(&format!(
        "  [{}], [{}], {}, {},\n",
        label_cell(family, category, sample),
        variant_cell(variant),
        render_cell(
            family,
            sample,
            font,
            RenderMode::Metric {
                manual_adjust_em: variant.metric_adjust_em,
            },
            results,
            variant.metric_adjust_em == 0.0,
        )?,
        render_cell(
            family,
            sample,
            font,
            RenderMode::GuardedOptical,
            results,
            variant.metric_adjust_em == 0.0,
        )?,
    ));
    Ok(row)
}

fn label_cell(family: &str, category: &str, sample: &str) -> String {
    format!(
        "#text(size: 5.2pt)[#strong[{}]#linebreak(){}#linebreak()#muted[{}]]",
        escape_content(family),
        escape_content(sample),
        escape_content(category)
    )
}

fn variant_cell(variant: ManualVariant) -> String {
    if variant.metric_adjust_em == 0.0 {
        return "#text(size: 5.2pt)[#strong[Standard]#linebreak()#muted[0]]".to_owned();
    }
    format!(
        "#text(size: 5.2pt)[#strong[{}]#linebreak()#muted[{:+.3}em]]",
        escape_content(variant.label),
        variant.metric_adjust_em
    )
}

#[derive(Debug, Clone, Copy)]
enum RenderMode {
    Metric { manual_adjust_em: f32 },
    GuardedOptical,
}

fn render_cell(
    family: &str,
    sample: &str,
    font: &FontKit,
    mode: RenderMode,
    results: &BTreeMap<String, &AlgorithmSet>,
    show_details: bool,
) -> Result<String> {
    let rendered = render_sample_body(font, sample, mode, results)?;
    let details = if show_details && !rendered.details.is_empty() {
        format!(
            "#linebreak()#tiny[{}]",
            render_delta_details(&rendered.details)
        )
    } else {
        String::new()
    };
    Ok(format!(
        "[#block(width: 78mm)[#text(font: \"{}\", size: 14pt, kerning: false)[{}]#linebreak()#muted[total {:+.3}em]{}]]",
        escape_string(family),
        rendered.body,
        rendered.total_delta,
        details
    ))
}

struct RenderedSample {
    body: String,
    total_delta: f32,
    details: Vec<PairDeltaDetail>,
}

struct PairDeltaDetail {
    label: String,
    delta: f32,
}

fn render_sample_body(
    font: &FontKit,
    sample: &str,
    mode: RenderMode,
    results: &BTreeMap<String, &AlgorithmSet>,
) -> Result<RenderedSample> {
    let sample_is_pair = sample.chars().filter(|ch| !ch.is_whitespace()).count() <= 2;
    let options = if sample_is_pair {
        ShapingOptions::typst_pair()
    } else {
        ShapingOptions::typst_word()
    };
    let run = shape_text(font, sample, options)?;
    let mut body = String::new();
    let mut total_delta = 0.0;
    let mut details = Vec::new();

    for (index, shaped) in run.glyphs.iter().enumerate() {
        body.push_str(&escape_content(&shaped.cluster_text));
        if index + 1 >= run.glyphs.len() {
            continue;
        }
        let Some(next) = run.glyphs.get(index + 1) else {
            continue;
        };
        if shaped.cluster_start == next.cluster_start
            || shaped.cluster_text.chars().all(char::is_whitespace)
            || next.cluster_text.chars().all(char::is_whitespace)
        {
            continue;
        }
        let pair = ShapedGlyphPair::new(index, shaped, next);
        let Some((label, delta)) = pair_delta(results, sample, sample_is_pair, &pair, mode) else {
            continue;
        };
        total_delta += delta;
        details.push(PairDeltaDetail { label, delta });
        body.push_str(&format!("#h({delta:.5}em)"));
    }

    Ok(RenderedSample {
        body,
        total_delta,
        details,
    })
}

fn pair_delta(
    results: &BTreeMap<String, &AlgorithmSet>,
    sample: &str,
    sample_is_pair: bool,
    pair: &ShapedGlyphPair,
    mode: RenderMode,
) -> Option<(String, f32)> {
    let keys = if sample_is_pair {
        [sample, pair.key.as_str(), pair.shaping_text.as_str()]
    } else {
        [pair.key.as_str(), pair.shaping_text.as_str(), sample]
    };
    keys.into_iter().find_map(|key| {
        let set = results.get(key)?;
        let output = set.outputs.iter().find(|output| {
            matches!(mode, RenderMode::Metric { .. })
                || output.algorithm == Algorithm::GuardedProfileHybrid
        })?;
        let delta = match mode {
            RenderMode::Metric { manual_adjust_em } => output.metric_delta_em + manual_adjust_em,
            RenderMode::GuardedOptical => output.delta_em,
        };
        Some((set.display.clone(), delta))
    })
}

fn render_delta_details(details: &[PairDeltaDetail]) -> String {
    details
        .chunks(3)
        .map(|chunk| {
            let line = chunk
                .iter()
                .map(|detail| format!("{} {:+.3}em", detail.label, detail.delta))
                .collect::<Vec<_>>()
                .join("; ");
            escape_content(&line)
        })
        .collect::<Vec<_>>()
        .join("#linebreak()")
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
