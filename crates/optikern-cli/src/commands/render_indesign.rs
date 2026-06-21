use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use serde::Serialize;

use crate::corpus;

const PAIRS_PER_FONT: usize = 4;
const WORDS_PER_FONT: usize = 6;

pub fn run(root: &Path, execute: bool) -> Result<()> {
    let root = root.canonicalize().context("failed to resolve repo root")?;
    let root = root.as_path();
    corpus::ensure_output_dirs(root)?;
    let manifest = corpus::load_fonts(root)?;
    let pairs = corpus::load_pairs(root)?;
    let words = corpus::load_words(root)?;
    let samples = corpus::load_samples(root)?;
    prepare_document_fonts(root, &manifest.fonts)?;
    let cases_by_font = build_cases_by_font(&manifest.fonts, &pairs, &words);

    let fonts_json = serde_json::to_string_pretty(&manifest.fonts)?;
    let pairs_json = serde_json::to_string_pretty(&pairs)?;
    let words_json = serde_json::to_string_pretty(&words)?;
    let samples_json = serde_json::to_string_pretty(&samples)?;
    let cases_json = serde_json::to_string_pretty(&cases_by_font)?;
    let out_dir = root.join("renders/indesign");
    let out_dir_js = escape_js_string(&out_dir.display().to_string());

    let jsx = format!(
        r#"#target indesign
app.scriptPreferences.userInteractionLevel = UserInteractionLevels.NEVER_INTERACT;
app.scriptPreferences.measurementUnit = MeasurementUnits.POINTS;

var OUT_DIR = "{out_dir_js}";
var FONTS = {fonts_json};
var PAIRS = {pairs_json};
var WORDS = {words_json};
var SAMPLES = {samples_json};
var CASES_BY_FONT = {cases_json};

function writeTextFile(path, text) {{
  var file = File(path);
  file.encoding = "UTF-8";
  file.open("w");
  file.write(text);
  file.close();
}}

function jsonQuote(value) {{
  return "\"" + String(value)
    .replace(/\\/g, "\\\\")
    .replace(/"/g, "\\\"")
    .replace(/\r/g, "\\r")
    .replace(/\n/g, "\\n") + "\"";
}}

function toJson(value) {{
  if (value === null) return "null";
  if (value instanceof Array) {{
    var parts = [];
    for (var i = 0; i < value.length; i++) parts.push(toJson(value[i]));
    return "[" + parts.join(",") + "]";
  }}
  if (typeof value === "object") {{
    var props = [];
    for (var key in value) {{
      if (value.hasOwnProperty(key)) props.push(jsonQuote(key) + ":" + toJson(value[key]));
    }}
    return "{{" + props.join(",") + "}}";
  }}
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  return jsonQuote(value);
}}

function configureText(text, fontFamily, size, mode, isPair) {{
  try {{
    text.appliedFont = fontFamily;
  }} catch (error) {{
    text.appliedFont = fallbackFont(fontFamily);
  }}
  text.pointSize = size;
  text.kerningMethod = mode;
  text.tracking = 0;
  text.horizontalScale = 100;
  text.verticalScale = 100;
  text.hyphenation = false;
  text.ligatures = !isPair;
  text.justification = Justification.LEFT_ALIGN;
}}

function fallbackFont(fontFamily) {{
  if (fontFamily.indexOf("Mono") >= 0 || fontFamily.indexOf("Code") >= 0) {{
    return "Courier New";
  }}
  if (
    fontFamily.indexOf("Garamond") >= 0 ||
    fontFamily.indexOf("Baskerville") >= 0 ||
    fontFamily.indexOf("Merriweather") >= 0 ||
    fontFamily.indexOf("Playfair") >= 0
  ) {{
    return "Times New Roman";
  }}
  return "Arial";
}}

function addFrame(page, bounds, contents, fontFamily, size, mode, isPair) {{
  var frame = page.textFrames.add();
  frame.geometricBounds = pageBounds(page, bounds);
  frame.contents = contents;
  configureText(frame.parentStory, fontFamily, size, mode, isPair);
  return frame;
}}

function addPageAtEnd(doc) {{
  return doc.pages.add(LocationOptions.AT_END);
}}

function setupDocument(doc) {{
  doc.documentPreferences.facingPages = false;
  doc.documentPreferences.pageWidth = "210mm";
  doc.documentPreferences.pageHeight = "297mm";
  doc.marginPreferences.top = "10mm";
  doc.marginPreferences.bottom = "10mm";
  doc.marginPreferences.left = "10mm";
  doc.marginPreferences.right = "10mm";
  doc.viewPreferences.rulerOrigin = RulerOrigin.PAGE_ORIGIN;
  doc.viewPreferences.horizontalMeasurementUnits = MeasurementUnits.POINTS;
  doc.viewPreferences.verticalMeasurementUnits = MeasurementUnits.POINTS;
  doc.zeroPoint = [0, 0];
}}

function pageBounds(page, bounds) {{
  var pageBox = page.bounds;
  var pageTop = Number(pageBox[0]);
  var pageLeft = Number(pageBox[1]);
  return [
    (pageTop + bounds[0]) + "pt",
    (pageLeft + bounds[1]) + "pt",
    (pageTop + bounds[2]) + "pt",
    (pageLeft + bounds[3]) + "pt"
  ];
}}

function build(modeName, modeValue) {{
  var doc = app.documents.add();
  setupDocument(doc);

  var sidecar = {{
    schemaVersion: 1,
    mode: modeName,
    kerningMethod: modeValue,
    cases: []
  }};

  var page = doc.pages[0];
  var y = 36;
  var pageNo = 1;
  for (var f = 0; f < FONTS.length; f++) {{
    var font = FONTS[f];
    addFrame(page, [y, 36, y + 18, 560], font.family + " / " + modeName, "Helvetica", 10, "$ID/Metrics", false);
    y += 28;

    var cases = CASES_BY_FONT[font.id] || [];
    for (var i = 0; i < cases.length; i++) {{
      if (y > 760) {{
        page = addPageAtEnd(doc);
        pageNo += 1;
        y = 36;
      }}
      var selected = cases[i];
      var frameHeight = selected.kind == "pair" ? 54 : 48;
      addFrame(page, [y, 36, y + frameHeight, 560], selected.sample, font.family, selected.pointSize, modeValue, selected.kind == "pair");
      sidecar.cases.push({{
        kind: selected.kind,
        fontId: font.id,
        family: font.family,
        sample: selected.sample,
        pointSize: selected.pointSize,
        page: pageNo,
        roiPt: [y, 36, y + frameHeight, 560],
        source: "review-selection"
      }});
      y += frameHeight + 8;
    }}

    for (var s = 0; s < Math.min(SAMPLES.length, 2); s++) {{
      if (y > 700) {{
        page = addPageAtEnd(doc);
        pageNo += 1;
        y = 36;
      }}
      var sample = SAMPLES[s];
      addFrame(page, [y, 36, y + 96, 560], sample, font.family, 12, modeValue, false);
      sidecar.cases.push({{
        kind: "paragraph",
        fontId: font.id,
        family: font.family,
        sample: sample,
        pointSize: 12,
        page: pageNo,
        roiPt: [y, 36, y + 96, 560]
      }});
      y += 108;
    }}

    if (f < FONTS.length - 1) {{
      page = addPageAtEnd(doc);
      pageNo += 1;
      y = 36;
    }}
  }}

  var pdfPath = OUT_DIR + "/indesign-" + modeName + ".pdf";
  var jsonPath = OUT_DIR + "/indesign-" + modeName + ".json";
  doc.exportFile(ExportFormat.PDF_TYPE, File(pdfPath), false);
  writeTextFile(jsonPath, toJson(sidecar));
  doc.close(SaveOptions.NO);
}}

build("metrics", "$ID/Metrics");
build("optical", "$ID/Optical");

function addComparisonHeader(page, y) {{
  addFrame(page, [y, 36, y + 16, 116], "Sample", "Helvetica", 8, "$ID/Metrics", false);
  addFrame(page, [y, 130, y + 16, 330], "InDesign Metrics", "Helvetica", 8, "$ID/Metrics", false);
  addFrame(page, [y, 350, y + 16, 550], "InDesign Optical", "Helvetica", 8, "$ID/Metrics", false);
}}

function addComparisonRow(page, y, sample, fontFamily, size, kind, sidecar, fontId, pageNo) {{
  var frameHeight = 34;
  addFrame(page, [y, 36, y + frameHeight, 116], sample, "Helvetica", 7, "$ID/Metrics", false);
  addFrame(page, [y, 130, y + frameHeight, 330], sample, fontFamily, size, "$ID/Metrics", kind == "pair");
  addFrame(page, [y, 350, y + frameHeight, 550], sample, fontFamily, size, "$ID/Optical", kind == "pair");
  sidecar.cases.push({{
    kind: kind,
    fontId: fontId,
    family: fontFamily,
    sample: sample,
    pointSize: size,
    page: pageNo,
    metricsRoiPt: [y, 130, y + frameHeight, 330],
    opticalRoiPt: [y, 350, y + frameHeight, 550]
  }});
}}

function buildComparison() {{
  var doc = app.documents.add();
  setupDocument(doc);

  var sidecar = {{
    schemaVersion: 1,
    mode: "comparison",
    columns: ["sample", "metrics", "optical"],
    cases: []
  }};

  var page = doc.pages[0];
  var pageNo = 1;
  for (var f = 0; f < FONTS.length; f++) {{
    var font = FONTS[f];
    if (f > 0) {{
      page = addPageAtEnd(doc);
      pageNo += 1;
    }}
    var y = 36;
    addFrame(page, [y, 36, y + 22, 550], font.family, "Helvetica", 11, "$ID/Metrics", false);
    y += 28;
    addComparisonHeader(page, y);
    y += 20;

    var cases = CASES_BY_FONT[font.id] || [];
    for (var i = 0; i < cases.length; i++) {{
      var selected = cases[i];
      var rowSize = selected.kind == "pair" ? 28 : 23;
      addComparisonRow(page, y, selected.sample, font.family, rowSize, selected.kind, sidecar, font.id, pageNo);
      y += 39;
    }}
  }}

  var pdfPath = OUT_DIR + "/indesign-comparison.pdf";
  var jsonPath = OUT_DIR + "/indesign-comparison.json";
  doc.exportFile(ExportFormat.PDF_TYPE, File(pdfPath), false);
  writeTextFile(jsonPath, toJson(sidecar));
  doc.close(SaveOptions.NO);
}}

buildComparison();
"#
    );

    let script_path = out_dir.join("export-baselines.jsx");
    fs::write(&script_path, jsx)
        .with_context(|| format!("failed to write {}", script_path.display()))?;
    println!("wrote {}", script_path.display());

    if execute {
        let status = Command::new("osascript")
            .arg(root.join("scripts/run-indesign-export.scpt"))
            .arg(&script_path)
            .status()
            .context("failed to start osascript")?;
        if !status.success() {
            return Err(anyhow!("InDesign script failed with {status}"));
        }
    }

    Ok(())
}

fn escape_js_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

fn build_cases_by_font(
    fonts: &[crate::corpus::FontEntry],
    pairs: &[String],
    words: &[String],
) -> std::collections::BTreeMap<String, Vec<IndesignCase>> {
    let mut cases_by_font = std::collections::BTreeMap::new();
    for (font_index, font) in fonts.iter().enumerate() {
        let mut cases = Vec::new();
        for index in rotated_indices(pairs.len(), font_index * PAIRS_PER_FONT)
            .into_iter()
            .take(PAIRS_PER_FONT)
        {
            cases.push(IndesignCase {
                kind: "pair",
                sample: pairs[index].clone(),
                point_size: 48,
            });
        }
        for index in rotated_indices(words.len(), font_index * WORDS_PER_FONT)
            .into_iter()
            .take(WORDS_PER_FONT)
        {
            cases.push(IndesignCase {
                kind: "word",
                sample: words[index].clone(),
                point_size: 42,
            });
        }
        cases_by_font.insert(font.id.clone(), cases);
    }
    cases_by_font
}

fn rotated_indices(total: usize, start: usize) -> Vec<usize> {
    if total == 0 {
        return Vec::new();
    }
    (0..total).map(|offset| (start + offset) % total).collect()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndesignCase {
    kind: &'static str,
    sample: String,
    point_size: u32,
}

fn prepare_document_fonts(root: &Path, fonts: &[crate::corpus::FontEntry]) -> Result<()> {
    let dir = root.join("renders/indesign/Document fonts");
    fs::create_dir_all(&dir)?;
    for font in fonts {
        let source = font.local_path(root);
        if !source.exists() {
            continue;
        }
        let Some(file_name) = source.file_name() else {
            continue;
        };
        fs::copy(&source, dir.join(file_name))?;
    }
    Ok(())
}
