use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FontManifest {
    pub source: String,
    pub commit: String,
    pub fonts: Vec<FontEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FontEntry {
    pub id: String,
    pub family: String,
    pub category: String,
    pub path: String,
}

impl FontEntry {
    pub fn local_path(&self, root: &Path) -> PathBuf {
        let extension = Path::new(&self.path)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("ttf");
        root.join("corpus")
            .join("fonts")
            .join(format!("{}.{}", self.id, extension))
    }
}

pub fn load_fonts(root: &Path) -> Result<FontManifest> {
    let path = root.join("corpus/fonts.toml");
    let input =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str(&input).with_context(|| format!("failed to parse {}", path.display()))
}

pub fn load_pairs(root: &Path) -> Result<Vec<String>> {
    load_lines(root.join("corpus/pairs/latin-critical.txt"))
}

pub fn load_words(root: &Path) -> Result<Vec<String>> {
    load_lines(root.join("corpus/words/headlines.txt"))
}

pub fn load_samples(root: &Path) -> Result<Vec<String>> {
    load_lines(root.join("corpus/samples/print.txt"))
}

pub fn ensure_output_dirs(root: &Path) -> Result<()> {
    for dir in ["metrics", "renders/typst", "renders/indesign", "reports"] {
        fs::create_dir_all(root.join(dir))
            .with_context(|| format!("failed to create {}", root.join(dir).display()))?;
    }
    Ok(())
}

fn load_lines(path: PathBuf) -> Result<Vec<String>> {
    let input =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut lines = vec![];
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        lines.push(line.to_owned());
    }
    Ok(lines)
}
