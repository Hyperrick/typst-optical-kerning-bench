use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result, anyhow};
use image::{GenericImageView, Rgba, RgbaImage};
use optikern_core::{
    Algorithm, AlgorithmSet, FontKit, ShapedGlyphPair, ShapingOptions, shape_text,
};
use serde::{Deserialize, Serialize};

use crate::corpus::{self, FontEntry};
use crate::data::BenchReport;

const TRIAD_SAMPLES: &[&str] = &["AV", "To", "AVATAR", "Typography", "Goldfish", "office"];
const DPI: u32 = 300;
const TOP_PT: f32 = 40.0;
const LEFT_LABEL_PT: f32 = 36.0;
const LEFT_A_PT: f32 = 128.0;
const LEFT_B_PT: f32 = 334.0;
const FRAME_WIDTH_PT: f32 = 190.0;
const PAIR_FRAME_HEIGHT_PT: f32 = 76.0;
const WORD_FRAME_HEIGHT_PT: f32 = 76.0;
const ROW_GAP_PT: f32 = 16.0;
const HEADER_HEIGHT_PT: f32 = 24.0;

pub fn run(root: &Path, run_indesign: bool, compile_typst: bool) -> Result<()> {
    let root = root.canonicalize().context("failed to resolve repo root")?;
    let root = root.as_path();
    corpus::ensure_output_dirs(root)?;
    let report = read_bench(root)?;
    let manifest = corpus::load_fonts(root)?;
    let by_font = index_results(&report);
    let cases = build_cases(&manifest.fonts);
    let triad_dir = root.join("renders/triad");
    fs::create_dir_all(&triad_dir)?;

    write_indesign_script(root, &triad_dir, &manifest.fonts, &cases)?;
    if run_indesign {
        run_indesign_script(root, &triad_dir)?;
    }

    write_typst(root, &triad_dir, &by_font, &cases, compile_typst)?;
    if compile_typst && triad_dir.join("indesign-optical.pdf").exists() {
        compare(root, &triad_dir)?;
    } else if compile_typst {
        println!(
            "skipped triad metrics; missing {}",
            triad_dir.join("indesign-optical.pdf").display()
        );
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

fn build_cases(fonts: &[FontEntry]) -> Vec<TriadCase> {
    let mut cases = Vec::new();
    for font in fonts {
        for sample in TRIAD_SAMPLES {
            let kind = if sample.chars().filter(|ch| !ch.is_whitespace()).count() <= 2 {
                SampleKind::Pair
            } else {
                SampleKind::Word
            };
            cases.push(TriadCase {
                id: format!("{}-{}", font.id, slug(sample)),
                font_id: font.id.clone(),
                family: font.family.clone(),
                category: font.category.clone(),
                sample: (*sample).to_owned(),
                kind,
                point_size: match kind {
                    SampleKind::Pair => 48.0,
                    SampleKind::Word => 42.0,
                },
            });
        }
    }
    cases
}

fn write_indesign_script(
    root: &Path,
    triad_dir: &Path,
    fonts: &[FontEntry],
    cases: &[TriadCase],
) -> Result<()> {
    prepare_document_fonts(root, fonts)?;
    let fonts_json = serde_json::to_string_pretty(fonts)?;
    let cases_json = serde_json::to_string_pretty(cases)?;
    let out_dir_js = escape_js_string(&triad_dir.display().to_string());
    let jsx = format!(
        r#"#target indesign
app.scriptPreferences.userInteractionLevel = UserInteractionLevels.NEVER_INTERACT;
app.scriptPreferences.measurementUnit = MeasurementUnits.POINTS;

var OUT_DIR = "{out_dir_js}";
var FONTS = {fonts_json};
var CASES = {cases_json};

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

function fallbackFont(fontFamily) {{
  if (fontFamily.indexOf("Mono") >= 0 || fontFamily.indexOf("Code") >= 0) return "Courier New";
  if (
    fontFamily.indexOf("Garamond") >= 0 ||
    fontFamily.indexOf("Baskerville") >= 0 ||
    fontFamily.indexOf("Merriweather") >= 0 ||
    fontFamily.indexOf("Playfair") >= 0
  ) return "Times New Roman";
  return "Arial";
}}

function fontFor(fontFamily) {{
  var candidates = [
    fontFamily + "\tRegular",
    fontFamily + "\tRoman",
    fontFamily + "\tLight",
    fontFamily + "\tExtraLight",
    fontFamily
  ];
  for (var i = 0; i < candidates.length; i++) {{
    try {{
      var font = app.fonts.itemByName(candidates[i]);
      if (font.isValid) return font;
    }} catch (error) {{}}
  }}
  return fallbackFont(fontFamily);
}}

function appliedFontName(text) {{
  try {{
    return String(text.appliedFont.fontFamily) + " " + String(text.appliedFont.fontStyleName);
  }} catch (error) {{
    return String(text.appliedFont);
  }}
}}

function configureText(text, fontFamily, size, isPair) {{
  try {{
    text.appliedFont = fontFor(fontFamily);
  }} catch (error) {{
    text.appliedFont = fallbackFont(fontFamily);
  }}
  text.pointSize = size;
  text.kerningMethod = "$ID/Optical";
  text.tracking = 0;
  text.horizontalScale = 100;
  text.verticalScale = 100;
  text.hyphenation = false;
  text.ligatures = !isPair;
  text.justification = Justification.LEFT_ALIGN;
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

function addFrame(page, bounds, contents, fontFamily, size, isPair) {{
  var frame = page.textFrames.add();
  frame.geometricBounds = pageBounds(page, bounds);
  frame.contents = contents;
  configureText(frame.parentStory, fontFamily, size, isPair);
  return frame;
}}

function addLabel(page, bounds, contents) {{
  var frame = page.textFrames.add();
  frame.geometricBounds = pageBounds(page, bounds);
  frame.contents = contents;
  frame.parentStory.appliedFont = "Helvetica";
  frame.parentStory.pointSize = 7;
  frame.parentStory.kerningMethod = "$ID/Metrics";
  frame.parentStory.justification = Justification.LEFT_ALIGN;
  return frame;
}}

function addPageAtEnd(doc) {{
  return doc.pages.add(LocationOptions.AT_END);
}}

function unionBounds(a, b) {{
  if (a === null) return [Number(b[0]), Number(b[1]), Number(b[2]), Number(b[3])];
  return [
    Math.min(Number(a[0]), Number(b[0])),
    Math.min(Number(a[1]), Number(b[1])),
    Math.max(Number(a[2]), Number(b[2])),
    Math.max(Number(a[3]), Number(b[3]))
  ];
}}

function paddedBounds(bounds, pad) {{
  return [
    Number(bounds[0]) - pad,
    Number(bounds[1]) - pad,
    Number(bounds[2]) + pad,
    Number(bounds[3]) + pad
  ];
}}

function outlineFrame(frame) {{
  var outlined = frame.createOutlines(false);
  var items = outlined instanceof Array ? outlined : [outlined];
  var bounds = null;
  for (var i = 0; i < items.length; i++) {{
    try {{
      bounds = unionBounds(bounds, items[i].visibleBounds);
    }} catch (error) {{}}
  }}
  if (bounds === null) bounds = frame.visibleBounds;
  return paddedBounds(bounds, 3);
}}

function build() {{
  var doc = app.documents.add();
  setupDocument(doc);
  var docPath = File(OUT_DIR + "/triad-fontload.indd");
  doc.save(docPath);
  doc.close(SaveOptions.YES);
  doc = app.open(docPath);
  setupDocument(doc);

  var sidecar = {{
    schemaVersion: 1,
    renderer: "indesign",
    mode: "optical",
    cases: []
  }};

  var page = doc.pages[0];
  var pageNo = 1;
  var currentFont = "";
  var y = {TOP_PT};
  for (var i = 0; i < CASES.length; i++) {{
    var selected = CASES[i];
    if (selected.fontId !== currentFont) {{
      if (i > 0) {{
        page = addPageAtEnd(doc);
        pageNo += 1;
      }}
      currentFont = selected.fontId;
      y = {TOP_PT};
      addLabel(page, [y, {LEFT_LABEL_PT}, y + 18, 550], selected.family + " / InDesign Optical");
      y += {HEADER_HEIGHT_PT};
    }}

    var frameHeight = selected.kind == "pair" ? {PAIR_FRAME_HEIGHT_PT} : {WORD_FRAME_HEIGHT_PT};
    var roi = [y, {LEFT_A_PT}, y + frameHeight, {LEFT_A_PT} + {FRAME_WIDTH_PT}];
    addLabel(page, [y, {LEFT_LABEL_PT}, y + frameHeight, {LEFT_LABEL_PT} + 82], selected.sample);
    var frame = addFrame(page, roi, selected.sample, selected.family, selected.pointSize, selected.kind == "pair");
    var actualFont = appliedFontName(frame.parentStory);
    var visibleRoi = outlineFrame(frame);
    sidecar.cases.push({{
      id: selected.id,
      kind: selected.kind,
      fontId: selected.fontId,
      family: selected.family,
      sample: selected.sample,
      pointSize: selected.pointSize,
      appliedFont: actualFont,
      page: pageNo,
      roiPt: visibleRoi
    }});
    y += frameHeight + {ROW_GAP_PT};
  }}

  doc.exportFile(ExportFormat.PDF_TYPE, File(OUT_DIR + "/indesign-optical.pdf"), false);
  writeTextFile(OUT_DIR + "/indesign-optical.json", toJson(sidecar));
  doc.close(SaveOptions.NO);
}}

build();
"#
    );

    let script_path = triad_dir.join("export-indesign-optical.jsx");
    fs::write(&script_path, jsx)
        .with_context(|| format!("failed to write {}", script_path.display()))?;
    println!("wrote {}", script_path.display());
    Ok(())
}

fn run_indesign_script(root: &Path, triad_dir: &Path) -> Result<()> {
    let script_path = triad_dir.join("export-indesign-optical.jsx");
    let status = Command::new("osascript")
        .arg(root.join("scripts/run-indesign-export.scpt"))
        .arg(&script_path)
        .status()
        .context("failed to start osascript")?;
    if !status.success() {
        return Err(anyhow!("InDesign script failed with {status}"));
    }
    Ok(())
}

fn write_typst(
    root: &Path,
    triad_dir: &Path,
    by_font: &BTreeMap<String, BTreeMap<String, &AlgorithmSet>>,
    cases: &[TriadCase],
    compile: bool,
) -> Result<()> {
    let mut source = String::new();
    source.push_str("#set page(width: 210mm, height: 297mm, margin: 0pt)\n");
    source.push_str("#set text(size: 7pt)\n");
    source.push_str("#let label(it) = text(font: \"Helvetica\", size: 7pt, it)\n\n");

    let mut sidecar = TypstSidecar {
        schema_version: 1,
        renderer: "typst".to_owned(),
        cases: Vec::new(),
    };
    let mut current_font = String::new();
    let mut page = 0u32;
    let mut y = TOP_PT;

    for case in cases {
        if case.font_id != current_font {
            if !current_font.is_empty() {
                source.push_str("#pagebreak()\n");
            }
            page += 1;
            current_font = case.font_id.clone();
            y = TOP_PT;
            source.push_str(&place_text(
                LEFT_LABEL_PT,
                y,
                520.0,
                18.0,
                &format!(
                    "#label[{} / Typst Metric vs Guarded Optical]",
                    escape_typ_content(&case.family)
                ),
            ));
            y += HEADER_HEIGHT_PT;
        }

        let frame_height = case.frame_height();
        source.push_str(&place_text(
            LEFT_LABEL_PT,
            y,
            82.0,
            frame_height,
            &format!("#label[{}]", escape_typ_content(&case.sample)),
        ));
        source.push_str(&place_text(
            LEFT_A_PT,
            y,
            FRAME_WIDTH_PT,
            frame_height,
            &render_typst_metric(case),
        ));
        let results = by_font
            .get(&case.font_id)
            .ok_or_else(|| anyhow!("missing bench results for {}", case.font_id))?;
        let font_path = root
            .join("corpus/fonts")
            .join(format!("{}.ttf", case.font_id));
        let font = FontKit::load(&case.font_id, &font_path)
            .with_context(|| format!("failed to load {}", font_path.display()))?;
        source.push_str(&place_text(
            LEFT_B_PT,
            y,
            FRAME_WIDTH_PT,
            frame_height,
            &render_typst_guarded(&font, case, results)?,
        ));

        sidecar.cases.push(TypstCaseSidecar {
            id: case.id.clone(),
            kind: case.kind.as_str().to_owned(),
            font_id: case.font_id.clone(),
            family: case.family.clone(),
            sample: case.sample.clone(),
            point_size: case.point_size,
            page,
            metric_roi_pt: [y, LEFT_A_PT, y + frame_height, LEFT_A_PT + FRAME_WIDTH_PT],
            guarded_roi_pt: [y, LEFT_B_PT, y + frame_height, LEFT_B_PT + FRAME_WIDTH_PT],
        });
        y += frame_height + ROW_GAP_PT;
    }

    let typ_path = triad_dir.join("typst-triad.typ");
    fs::write(&typ_path, source)
        .with_context(|| format!("failed to write {}", typ_path.display()))?;
    let json_path = triad_dir.join("typst-triad.json");
    fs::write(&json_path, serde_json::to_string_pretty(&sidecar)?)
        .with_context(|| format!("failed to write {}", json_path.display()))?;
    println!("wrote {}", typ_path.display());
    println!("wrote {}", json_path.display());

    if compile {
        let pdf_path = triad_dir.join("typst-triad.pdf");
        let status = Command::new("typst")
            .arg("compile")
            .arg("--font-path")
            .arg(root.join("corpus/fonts"))
            .arg(&typ_path)
            .arg(&pdf_path)
            .status()
            .context("failed to start typst")?;
        if !status.success() {
            return Err(anyhow!("typst triad compile failed with {status}"));
        }
        println!("wrote {}", pdf_path.display());
    }

    Ok(())
}

fn place_text(x: f32, y: f32, width: f32, height: f32, body: &str) -> String {
    format!(
        "#place(dx: {x:.2}pt, dy: {y:.2}pt)[#block(width: {width:.2}pt, height: {height:.2}pt)[{body}]]\n"
    )
}

fn render_typst_metric(case: &TriadCase) -> String {
    format!(
        "#text(font: \"{}\", size: {:.1}pt, kerning: true)[{}]",
        escape_typ_string(&case.family),
        case.point_size,
        escape_typ_content(&case.sample)
    )
}

fn render_typst_guarded(
    font: &FontKit,
    case: &TriadCase,
    results: &BTreeMap<String, &AlgorithmSet>,
) -> Result<String> {
    let rendered = render_sample_body(font, case, results)?;
    Ok(format!(
        "#text(font: \"{}\", size: {:.1}pt, kerning: false)[{}]",
        escape_typ_string(&case.family),
        case.point_size,
        rendered
    ))
}

fn render_sample_body(
    font: &FontKit,
    case: &TriadCase,
    results: &BTreeMap<String, &AlgorithmSet>,
) -> Result<String> {
    let run = shape_text(font, &case.sample, case.shaping_options())?;
    let mut body = String::new();
    for (index, shaped) in run.glyphs.iter().enumerate() {
        body.push_str(&escape_typ_content(&shaped.cluster_text));
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
        if let Some(delta) =
            output_delta(results, &case.sample, case.kind == SampleKind::Pair, &pair)
        {
            body.push_str(&format!("#h({delta:.5}em)"));
        }
    }
    Ok(body)
}

fn output_delta(
    results: &BTreeMap<String, &AlgorithmSet>,
    sample: &str,
    sample_is_pair: bool,
    pair: &ShapedGlyphPair,
) -> Option<f32> {
    let keys = if sample_is_pair {
        [sample, pair.key.as_str(), pair.shaping_text.as_str()]
    } else {
        [pair.key.as_str(), pair.shaping_text.as_str(), sample]
    };
    keys.into_iter().find_map(|key| {
        let set = results.get(key)?;
        let output = set
            .outputs
            .iter()
            .find(|output| output.algorithm == Algorithm::GuardedProfileHybrid)?;
        Some(output.delta_em)
    })
}

fn compare(root: &Path, triad_dir: &Path) -> Result<()> {
    let render_dir = triad_dir.join("rendered");
    let crop_dir = triad_dir.join("crops");
    fs::create_dir_all(&render_dir)?;
    fs::create_dir_all(&crop_dir)?;

    let indesign_sidecar: IndesignSidecar = read_json(&triad_dir.join("indesign-optical.json"))?;
    let typst_sidecar: TypstSidecar = read_json(&triad_dir.join("typst-triad.json"))?;
    let indesign_pages = render_pdf(
        &triad_dir.join("indesign-optical.pdf"),
        &render_dir.join("indesign-optical"),
    )?;
    let typst_pages = render_pdf(
        &triad_dir.join("typst-triad.pdf"),
        &render_dir.join("typst-triad"),
    )?;
    let typst_by_id = typst_sidecar
        .cases
        .iter()
        .map(|case| (case.id.clone(), case))
        .collect::<BTreeMap<_, _>>();

    let mut rows = Vec::new();
    for indesign_case in &indesign_sidecar.cases {
        let Some(typst_case) = typst_by_id.get(&indesign_case.id) else {
            continue;
        };
        let id_crop = crop_roi(
            &indesign_pages,
            indesign_case.page,
            indesign_case.roi_pt,
            &crop_dir.join(format!("{}-indesign-optical.png", indesign_case.id)),
        )?;
        let metric_crop = crop_roi(
            &typst_pages,
            typst_case.page,
            typst_case.metric_roi_pt,
            &crop_dir.join(format!("{}-typst-metric.png", indesign_case.id)),
        )?;
        let guarded_crop = crop_roi(
            &typst_pages,
            typst_case.page,
            typst_case.guarded_roi_pt,
            &crop_dir.join(format!("{}-typst-guarded.png", indesign_case.id)),
        )?;
        let id_stats = analyze_crop(&id_crop)?;
        let metric_stats = analyze_crop(&metric_crop)?;
        let guarded_stats = analyze_crop(&guarded_crop)?;
        let metric_height_scale = height_scale(id_stats.bbox, metric_stats.bbox);
        let guarded_height_scale = height_scale(id_stats.bbox, guarded_stats.bbox);
        let metric_overlay = crop_dir.join(format!(
            "{}-overlay-metric-vs-indesign.png",
            indesign_case.id
        ));
        let guarded_overlay = crop_dir.join(format!(
            "{}-overlay-guarded-vs-indesign.png",
            indesign_case.id
        ));
        write_overlay(
            &id_crop,
            id_stats.bbox,
            &metric_crop,
            metric_stats.bbox,
            OverlayColor::Magenta,
            &metric_overlay,
        )?;
        write_overlay(
            &id_crop,
            id_stats.bbox,
            &guarded_crop,
            guarded_stats.bbox,
            OverlayColor::Green,
            &guarded_overlay,
        )?;
        let px_per_em = indesign_case.point_size * DPI as f32 / 72.0;
        let metric_width_error = width_error_em(
            id_stats.bbox,
            metric_stats.bbox,
            metric_height_scale,
            px_per_em,
        );
        let guarded_width_error = width_error_em(
            id_stats.bbox,
            guarded_stats.bbox,
            guarded_height_scale,
            px_per_em,
        );
        rows.push(TriadMetric {
            id: indesign_case.id.clone(),
            kind: indesign_case.kind.clone(),
            font_id: indesign_case.font_id.clone(),
            family: indesign_case.family.clone(),
            sample: indesign_case.sample.clone(),
            point_size: indesign_case.point_size,
            indesign_optical_crop: relative(root, &id_crop),
            typst_metric_crop: relative(root, &metric_crop),
            typst_guarded_crop: relative(root, &guarded_crop),
            metric_overlay_crop: relative(root, &metric_overlay),
            guarded_overlay_crop: relative(root, &guarded_overlay),
            indesign_width_px: bbox_width(id_stats.bbox),
            typst_metric_width_px: bbox_width(metric_stats.bbox),
            typst_guarded_width_px: bbox_width(guarded_stats.bbox),
            typst_metric_height_scale: metric_height_scale,
            typst_guarded_height_scale: guarded_height_scale,
            metric_width_error_em: metric_width_error,
            guarded_width_error_em: guarded_width_error,
            guarded_closer: guarded_width_error < metric_width_error,
        });
    }

    let report = TriadReport {
        schema_version: 1,
        dpi: DPI,
        rows,
    };
    let metrics_path = root.join("metrics/triad-comparison.json");
    fs::write(&metrics_path, serde_json::to_string_pretty(&report)?)
        .with_context(|| format!("failed to write {}", metrics_path.display()))?;
    let html_path = root.join("reports/triad-comparison.html");
    fs::write(&html_path, build_html(&report))
        .with_context(|| format!("failed to write {}", html_path.display()))?;
    println!("wrote {}", metrics_path.display());
    println!("wrote {}", html_path.display());
    Ok(())
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let input =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&input).with_context(|| format!("failed to parse {}", path.display()))
}

fn render_pdf(pdf: &Path, prefix: &Path) -> Result<BTreeMap<u32, PathBuf>> {
    let status = Command::new("pdftoppm")
        .arg("-png")
        .arg("-r")
        .arg(DPI.to_string())
        .arg(pdf)
        .arg(prefix)
        .status()
        .with_context(|| format!("failed to start pdftoppm for {}", pdf.display()))?;
    if !status.success() {
        return Err(anyhow!(
            "pdftoppm failed for {} with {status}",
            pdf.display()
        ));
    }

    let dir = prefix.parent().unwrap_or_else(|| Path::new("."));
    let stem = prefix
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let mut pages = BTreeMap::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.starts_with(stem)
            || path.extension().and_then(|ext| ext.to_str()) != Some("png")
        {
            continue;
        }
        let page = file_name
            .trim_start_matches(stem)
            .trim_start_matches('-')
            .trim_end_matches(".png")
            .parse::<u32>()
            .unwrap_or(1);
        pages.insert(page, path);
    }
    Ok(pages)
}

fn crop_roi(
    pages: &BTreeMap<u32, PathBuf>,
    page: u32,
    roi: [f32; 4],
    output: &Path,
) -> Result<PathBuf> {
    let page_path = pages
        .get(&page)
        .ok_or_else(|| anyhow!("missing rendered page {page}"))?;
    let img = image::open(page_path)
        .with_context(|| format!("failed to open {}", page_path.display()))?;
    let scale = DPI as f32 / 72.0;
    let x = (roi[1] * scale).round().max(0.0) as u32;
    let y = (roi[0] * scale).round().max(0.0) as u32;
    let width = ((roi[3] - roi[1]) * scale).round().max(1.0) as u32;
    let height = ((roi[2] - roi[0]) * scale).round().max(1.0) as u32;
    let crop = img.crop_imm(
        x,
        y,
        width.min(img.width() - x),
        height.min(img.height() - y),
    );
    crop.save(output)
        .with_context(|| format!("failed to write {}", output.display()))?;
    Ok(output.to_path_buf())
}

fn analyze_crop(path: &Path) -> Result<CropStats> {
    let img = image::open(path).with_context(|| format!("failed to open {}", path.display()))?;
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
    Ok(CropStats { bbox })
}

fn write_overlay(
    reference_path: &Path,
    reference_bbox: Option<[u32; 4]>,
    candidate_path: &Path,
    candidate_bbox: Option<[u32; 4]>,
    candidate_color: OverlayColor,
    output: &Path,
) -> Result<()> {
    let reference = image::open(reference_path)
        .with_context(|| format!("failed to open {}", reference_path.display()))?;
    let candidate = image::open(candidate_path)
        .with_context(|| format!("failed to open {}", candidate_path.display()))?;
    let reference_bbox = reference_bbox.unwrap_or([
        0,
        0,
        reference.width().saturating_sub(1),
        reference.height().saturating_sub(1),
    ]);
    let candidate_bbox = candidate_bbox.unwrap_or([
        0,
        0,
        candidate.width().saturating_sub(1),
        candidate.height().saturating_sub(1),
    ]);
    let padding = 8;
    let candidate_scale = height_scale(Some(reference_bbox), Some(candidate_bbox));
    let width = scaled_bbox_width(Some(reference_bbox), 1.0)
        .max(scaled_bbox_width(Some(candidate_bbox), candidate_scale))
        + padding * 2;
    let height = scaled_bbox_height(Some(reference_bbox), 1.0)
        .max(scaled_bbox_height(Some(candidate_bbox), candidate_scale))
        + padding * 2;
    let mut canvas = RgbaImage::from_pixel(width, height, Rgba([255, 255, 255, 255]));

    paint_dark_pixels(
        &mut canvas,
        &reference,
        reference_bbox,
        Rgba([0, 170, 220, 255]),
        1.0,
        padding,
    );
    paint_dark_pixels(
        &mut canvas,
        &candidate,
        candidate_bbox,
        candidate_color.rgba(),
        candidate_scale,
        padding,
    );
    canvas
        .save(output)
        .with_context(|| format!("failed to write {}", output.display()))?;
    Ok(())
}

fn paint_dark_pixels(
    canvas: &mut RgbaImage,
    source: &image::DynamicImage,
    bbox: [u32; 4],
    color: Rgba<u8>,
    scale: f32,
    padding: u32,
) {
    for (x, y, pixel) in source.pixels() {
        if !is_dark(pixel.0) {
            continue;
        }
        if x < bbox[0] || x > bbox[2] || y < bbox[1] || y > bbox[3] {
            continue;
        }
        let target_x = ((x - bbox[0]) as f32 * scale).round() as u32 + padding;
        let target_y = ((y - bbox[1]) as f32 * scale).round() as u32 + padding;
        if target_x >= canvas.width() || target_y >= canvas.height() {
            continue;
        }
        let existing = canvas.get_pixel(target_x, target_y);
        let next = if existing.0 == [255, 255, 255, 255] {
            color
        } else if existing.0 == color.0 {
            color
        } else {
            Rgba([10, 10, 10, 255])
        };
        canvas.put_pixel(target_x, target_y, next);
    }
}

fn is_dark([r, g, b, a]: [u8; 4]) -> bool {
    a > 0 && u16::from(r) + u16::from(g) + u16::from(b) < 660
}

fn width_error_em(
    reference: Option<[u32; 4]>,
    candidate: Option<[u32; 4]>,
    candidate_scale: f32,
    px_per_em: f32,
) -> f32 {
    ((bbox_width(candidate) as f32 * candidate_scale - bbox_width(reference) as f32) / px_per_em)
        .abs()
}

fn bbox_width(bbox: Option<[u32; 4]>) -> u32 {
    bbox.map(|bbox| bbox[2].saturating_sub(bbox[0]) + 1)
        .unwrap_or_default()
}

fn bbox_height(bbox: Option<[u32; 4]>) -> u32 {
    bbox.map(|bbox| bbox[3].saturating_sub(bbox[1]) + 1)
        .unwrap_or_default()
}

fn height_scale(reference: Option<[u32; 4]>, candidate: Option<[u32; 4]>) -> f32 {
    let reference_height = bbox_height(reference);
    let candidate_height = bbox_height(candidate);
    if reference_height == 0 || candidate_height == 0 {
        return 1.0;
    }
    reference_height as f32 / candidate_height as f32
}

fn scaled_bbox_width(bbox: Option<[u32; 4]>, scale: f32) -> u32 {
    (bbox_width(bbox) as f32 * scale).ceil() as u32
}

fn scaled_bbox_height(bbox: Option<[u32; 4]>, scale: f32) -> u32 {
    (bbox_height(bbox) as f32 * scale).ceil() as u32
}

fn build_html(report: &TriadReport) -> String {
    let mut html = String::new();
    html.push_str("<!doctype html><meta charset=\"utf-8\"><title>Triad Kerning Comparison</title>");
    html.push_str("<style>body{font-family:system-ui,sans-serif;margin:28px;line-height:1.4}table{border-collapse:collapse;width:100%}td,th{border:1px solid #ddd;padding:7px;vertical-align:top}th{background:#f4f4f4;text-align:left}.num{text-align:right;font-variant-numeric:tabular-nums}img{display:block;max-width:260px;background:white}.win{background:#ecf8ef}</style>");
    html.push_str("<h1>Triad Kerning Comparison</h1>");
    html.push_str("<p>Reference is InDesign Optical. Width error is measured from rendered black-pixel bounding boxes, in em. Typst candidates are scaled to the InDesign ink height before overlay and width comparison. Overlays align ink bounds: cyan = InDesign Optical, magenta = Typst Metric, green = Typst Guarded, dark = overlap.</p>");
    html.push_str("<table><tr><th>Font / sample</th><th>InDesign Optical</th><th>Metric overlay</th><th>Guarded overlay</th><th>Metric error</th><th>Guarded error</th></tr>");
    for row in &report.rows {
        let metric_class = if !row.guarded_closer {
            " class=\"win\""
        } else {
            ""
        };
        let guarded_class = if row.guarded_closer {
            " class=\"win\""
        } else {
            ""
        };
        html.push_str(&format!(
            "<tr><td><strong>{}</strong><br>{}</td><td><img src=\"../{}\"></td><td{metric_class}><img src=\"../{}\"></td><td{guarded_class}><img src=\"../{}\"></td><td class=\"num\">{:.4}<br><small>scale {:.3}</small></td><td class=\"num\">{:.4}<br><small>scale {:.3}</small></td></tr>",
            escape_html(&row.family),
            escape_html(&row.sample),
            escape_html(&row.indesign_optical_crop),
            escape_html(&row.metric_overlay_crop),
            escape_html(&row.guarded_overlay_crop),
            row.metric_width_error_em,
            row.typst_metric_height_scale,
            row.guarded_width_error_em,
            row.typst_guarded_height_scale,
        ));
    }
    html.push_str("</table>");
    html
}

fn prepare_document_fonts(root: &Path, fonts: &[FontEntry]) -> Result<()> {
    let dir = root.join("renders/triad/Document fonts");
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

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn slug(input: &str) -> String {
    input
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect()
}

fn escape_js_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_typ_content(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('#', "\\#")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn escape_typ_string(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum SampleKind {
    Pair,
    Word,
}

impl SampleKind {
    fn as_str(self) -> &'static str {
        match self {
            SampleKind::Pair => "pair",
            SampleKind::Word => "word",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TriadCase {
    id: String,
    font_id: String,
    family: String,
    category: String,
    sample: String,
    kind: SampleKind,
    point_size: f32,
}

impl TriadCase {
    fn frame_height(&self) -> f32 {
        match self.kind {
            SampleKind::Pair => PAIR_FRAME_HEIGHT_PT,
            SampleKind::Word => WORD_FRAME_HEIGHT_PT,
        }
    }

    fn shaping_options(&self) -> ShapingOptions {
        match self.kind {
            SampleKind::Pair => ShapingOptions::typst_pair(),
            SampleKind::Word => ShapingOptions::typst_word(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndesignSidecar {
    cases: Vec<IndesignCaseSidecar>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IndesignCaseSidecar {
    id: String,
    kind: String,
    font_id: String,
    family: String,
    sample: String,
    point_size: f32,
    page: u32,
    roi_pt: [f32; 4],
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TypstSidecar {
    schema_version: u32,
    renderer: String,
    cases: Vec<TypstCaseSidecar>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TypstCaseSidecar {
    id: String,
    kind: String,
    font_id: String,
    family: String,
    sample: String,
    point_size: f32,
    page: u32,
    metric_roi_pt: [f32; 4],
    guarded_roi_pt: [f32; 4],
}

#[derive(Debug)]
struct CropStats {
    bbox: Option<[u32; 4]>,
}

#[derive(Debug, Clone, Copy)]
enum OverlayColor {
    Magenta,
    Green,
}

impl OverlayColor {
    fn rgba(self) -> Rgba<u8> {
        match self {
            OverlayColor::Magenta => Rgba([220, 0, 170, 255]),
            OverlayColor::Green => Rgba([0, 170, 80, 255]),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TriadReport {
    schema_version: u32,
    dpi: u32,
    rows: Vec<TriadMetric>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TriadMetric {
    id: String,
    kind: String,
    font_id: String,
    family: String,
    sample: String,
    point_size: f32,
    indesign_optical_crop: String,
    typst_metric_crop: String,
    typst_guarded_crop: String,
    metric_overlay_crop: String,
    guarded_overlay_crop: String,
    indesign_width_px: u32,
    typst_metric_width_px: u32,
    typst_guarded_width_px: u32,
    typst_metric_height_scale: f32,
    typst_guarded_height_scale: f32,
    metric_width_error_em: f32,
    guarded_width_error_em: f32,
    guarded_closer: bool,
}
