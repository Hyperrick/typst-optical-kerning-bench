use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::corpus;

pub fn run(root: &Path, force: bool) -> Result<()> {
    corpus::ensure_output_dirs(root)?;
    fs::create_dir_all(root.join("corpus/fonts"))?;

    let manifest = corpus::load_fonts(root)?;
    for font in &manifest.fonts {
        let local = font.local_path(root);
        if local.exists() && !force {
            println!("exists {}", local.display());
            continue;
        }

        let encoded_path = font
            .path
            .replace('[', "%5B")
            .replace(']', "%5D")
            .replace(',', "%2C");
        let url = format!(
            "https://raw.githubusercontent.com/google/fonts/{}/{}",
            manifest.commit, encoded_path
        );
        println!("fetch {} -> {}", font.id, local.display());
        let bytes = reqwest::blocking::get(&url)
            .with_context(|| format!("request failed for {url}"))?
            .error_for_status()
            .with_context(|| format!("download failed for {url}"))?
            .bytes()
            .with_context(|| format!("failed to read response for {url}"))?;
        fs::write(&local, &bytes)
            .with_context(|| format!("failed to write {}", local.display()))?;
    }

    Ok(())
}
