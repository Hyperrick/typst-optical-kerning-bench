use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use image::GenericImageView;
use serde::Serialize;
use walkdir::WalkDir;

use crate::corpus;

#[derive(Debug, Serialize)]
struct PdfEvalReport {
    schema_version: u32,
    input: String,
    pages: Vec<PageEval>,
}

#[derive(Debug, Serialize)]
struct PageEval {
    pdf: String,
    png: String,
    width: u32,
    height: u32,
    dark_pixels: u64,
    bbox: Option<[u32; 4]>,
}

pub fn run(root: &Path, input: &Path) -> Result<()> {
    corpus::ensure_output_dirs(root)?;
    let input_dir = if input.is_absolute() {
        input.to_path_buf()
    } else {
        root.join(input)
    };
    let eval_dir = root.join("renders/eval");
    fs::create_dir_all(&eval_dir)
        .with_context(|| format!("failed to create {}", eval_dir.display()))?;

    let mut pages = vec![];
    for entry in WalkDir::new(&input_dir).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("pdf") {
            continue;
        }
        let page = eval_pdf(entry.path(), &eval_dir)?;
        pages.push(page);
    }

    let report = PdfEvalReport {
        schema_version: 1,
        input: input_dir.display().to_string(),
        pages,
    };
    let out = root.join("metrics/pdf-eval.json");
    fs::write(&out, serde_json::to_string_pretty(&report)?)
        .with_context(|| format!("failed to write {}", out.display()))?;
    println!("wrote {}", out.display());
    Ok(())
}

fn eval_pdf(pdf: &Path, eval_dir: &Path) -> Result<PageEval> {
    let stem = pdf
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("page")
        .replace(' ', "-");
    let prefix = eval_dir.join(stem);
    let status = Command::new("pdftoppm")
        .arg("-png")
        .arg("-r")
        .arg("300")
        .arg("-singlefile")
        .arg(pdf)
        .arg(&prefix)
        .status()
        .with_context(|| format!("failed to start pdftoppm for {}", pdf.display()))?;
    if !status.success() {
        return Err(anyhow!(
            "pdftoppm failed for {} with {status}",
            pdf.display()
        ));
    }

    let png = prefix.with_extension("png");
    let img = image::open(&png).with_context(|| format!("failed to open {}", png.display()))?;
    let (width, height) = img.dimensions();
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0;
    let mut max_y = 0;
    let mut dark_pixels = 0;

    for (x, y, pixel) in img.pixels() {
        let [r, g, b, a] = pixel.0;
        if a > 0 && u16::from(r) + u16::from(g) + u16::from(b) < 660 {
            dark_pixels += 1;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    let bbox = if dark_pixels == 0 {
        None
    } else {
        Some([min_x, min_y, max_x, max_y])
    };

    Ok(PageEval {
        pdf: pdf.display().to_string(),
        png: png.display().to_string(),
        width,
        height,
        dark_pixels,
        bbox,
    })
}
