use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use optikern_core::{
    EvaluationConfig, FontKit, ShapedGlyphPair, ShapingOptions, evaluate_pair_with_config,
    evaluate_shaped_pair_with_config, shape_text,
};

use crate::corpus;
use crate::data::{BenchFailure, BenchFont, BenchReport};

pub fn run(root: &Path) -> Result<()> {
    corpus::ensure_output_dirs(root)?;
    let started = Instant::now();
    let manifest = corpus::load_fonts(root)?;
    let configured_pairs = corpus::load_pairs(root)?;
    let words = corpus::load_words(root)?;

    let mut fonts = vec![];
    let mut results = vec![];
    let mut failures = vec![];

    for entry in &manifest.fonts {
        let path = entry.local_path(root);
        fonts.push(BenchFont {
            id: entry.id.clone(),
            family: entry.family.clone(),
            category: entry.category.clone(),
            path: path.display().to_string(),
        });

        if !path.exists() {
            failures.push(BenchFailure {
                font_id: entry.id.clone(),
                pair: "*".into(),
                reason: format!("font file missing: {}", path.display()),
            });
            continue;
        }

        let font = FontKit::load(&entry.id, &path)
            .with_context(|| format!("failed to load font {}", path.display()))?;
        let config = EvaluationConfig::for_font(&font);
        let mut seen = BTreeSet::new();

        for pair in &configured_pairs {
            match evaluate_pair_with_config(&font, pair, config) {
                Ok(result) => {
                    seen.insert(result.pair.clone());
                    results.push(result);
                }
                Err(error) => failures.push(BenchFailure {
                    font_id: entry.id.clone(),
                    pair: pair.clone(),
                    reason: error.to_string(),
                }),
            }
        }

        for word in &words {
            let pairs = match shaped_word_pairs(&font, word) {
                Ok(pairs) => pairs,
                Err(error) => {
                    failures.push(BenchFailure {
                        font_id: entry.id.clone(),
                        pair: word.clone(),
                        reason: error.to_string(),
                    });
                    continue;
                }
            };
            for pair in pairs {
                if !seen.insert(pair.key.clone()) {
                    continue;
                }
                match evaluate_shaped_pair_with_config(&font, &pair, config, true) {
                    Ok(result) => results.push(result),
                    Err(error) => failures.push(BenchFailure {
                        font_id: entry.id.clone(),
                        pair: pair.display,
                        reason: error.to_string(),
                    }),
                }
            }
        }
    }

    let report = BenchReport {
        schema_version: 2,
        font_manifest_commit: manifest.commit,
        fonts,
        pair_count: results.len(),
        word_count: words.len(),
        results,
        failures,
        runtime_ms: started.elapsed().as_millis(),
    };

    let path = root.join("metrics/bench.json");
    fs::write(&path, serde_json::to_string_pretty(&report)?)
        .with_context(|| format!("failed to write {}", path.display()))?;
    println!(
        "wrote {} ({} pair results, {} failures)",
        path.display(),
        report.results.len(),
        report.failures.len()
    );
    Ok(())
}

fn shaped_word_pairs(font: &FontKit, word: &str) -> Result<Vec<ShapedGlyphPair>> {
    Ok(shape_text(font, word, ShapingOptions::typst_word())?.adjacent_pairs())
}
