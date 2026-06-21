use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use optikern_core::{EvaluationConfig, FontKit, evaluate_pair_with_config};

use crate::corpus;
use crate::data::{BenchFailure, BenchFont, BenchReport};

pub fn run(root: &Path) -> Result<()> {
    corpus::ensure_output_dirs(root)?;
    let started = Instant::now();
    let manifest = corpus::load_fonts(root)?;
    let configured_pairs = corpus::load_pairs(root)?;
    let words = corpus::load_words(root)?;
    let pairs = collect_pairs(&configured_pairs, &words);

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

        for pair in &pairs {
            match evaluate_pair_with_config(&font, pair, config) {
                Ok(result) => results.push(result),
                Err(error) => failures.push(BenchFailure {
                    font_id: entry.id.clone(),
                    pair: pair.clone(),
                    reason: error.to_string(),
                }),
            }
        }
    }

    let report = BenchReport {
        schema_version: 1,
        font_manifest_commit: manifest.commit,
        fonts,
        pair_count: pairs.len(),
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

fn collect_pairs(configured_pairs: &[String], words: &[String]) -> Vec<String> {
    let mut pairs = BTreeSet::new();
    for pair in configured_pairs {
        pairs.insert(pair.clone());
    }
    for word in words {
        let chars = word.chars().collect::<Vec<_>>();
        for pair in chars.windows(2) {
            if pair[0].is_whitespace() || pair[1].is_whitespace() {
                continue;
            }
            pairs.insert(pair.iter().collect());
        }
    }
    pairs.into_iter().collect()
}
