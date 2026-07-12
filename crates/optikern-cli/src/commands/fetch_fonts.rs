use std::fs;
use std::path::Path;

use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};

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

        let url = font
            .url
            .clone()
            .unwrap_or_else(|| google_fonts_url(&manifest.commit, &font.path));
        println!("fetch {} -> {}", font.id, local.display());
        let bytes = reqwest::blocking::get(&url)
            .with_context(|| format!("request failed for {url}"))?
            .error_for_status()
            .with_context(|| format!("download failed for {url}"))?
            .bytes()
            .with_context(|| format!("failed to read response for {url}"))?;
        verify_sha256(&font.id, &bytes, font.sha256.as_deref())?;
        fs::write(&local, &bytes)
            .with_context(|| format!("failed to write {}", local.display()))?;
    }

    Ok(())
}

fn google_fonts_url(commit: &str, path: &str) -> String {
    let encoded_path = path
        .replace('[', "%5B")
        .replace(']', "%5D")
        .replace(',', "%2C");
    format!("https://raw.githubusercontent.com/google/fonts/{commit}/{encoded_path}")
}

fn verify_sha256(id: &str, bytes: &[u8], expected: Option<&str>) -> Result<()> {
    let Some(expected) = expected else {
        return Ok(());
    };
    let actual = format!("{:x}", Sha256::digest(bytes));
    if actual != expected.to_ascii_lowercase() {
        bail!("SHA-256 mismatch for {id}: expected {expected}, got {actual}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{google_fonts_url, verify_sha256};

    #[test]
    fn google_url_encodes_variable_font_path() {
        assert_eq!(
            google_fonts_url("abc", "ofl/example/Example[opsz,wght].ttf"),
            "https://raw.githubusercontent.com/google/fonts/abc/ofl/example/Example%5Bopsz%2Cwght%5D.ttf"
        );
    }

    #[test]
    fn checksum_rejects_changed_bytes() {
        let expected = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(verify_sha256("example", b"hello", Some(expected)).is_ok());
        assert!(verify_sha256("example", b"changed", Some(expected)).is_err());
    }
}
