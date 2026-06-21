use crate::data::BenchReport;

pub fn build_html(report: &BenchReport, survey_modes: &[&str], repo_url: &str) -> String {
    let repo_url = escape_attr(repo_url);
    let mode_list = survey_modes
        .iter()
        .map(|mode| format!("<code>{}</code>", escape_html(mode)))
        .collect::<Vec<_>>()
        .join(", ");
    let font_count = report.fonts.len();
    let pair_count = report.results.len();

    format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Methods | Optical Kerning Preference Study</title>
<link rel="icon" href='data:image/svg+xml,<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect width="32" height="32" rx="6" fill="%231f6f68"/><path d="M7 23 14 8h3l8 15h-4l-2-4h-7l-2 4H7Zm6-7h5l-2.5-5L13 16Z" fill="white"/></svg>'>
<style>
:root {{
  color-scheme: light;
  --bg: #f7f7f4;
  --ink: #171717;
  --muted: #686868;
  --line: #d9d8d2;
  --panel: #ffffff;
  --accent: #1f6f68;
  --code: #eef8f6;
}}
@media (prefers-color-scheme: dark) {{
  :root {{
    color-scheme: dark;
    --bg: #151514;
    --ink: #f1f0ea;
    --muted: #aaa79e;
    --line: #3f3d38;
    --panel: #201f1d;
    --accent: #69c7ba;
    --code: #1b3835;
  }}
}}
* {{ box-sizing: border-box; }}
body {{
  margin: 0;
  background: var(--bg);
  color: var(--ink);
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  line-height: 1.5;
}}
main {{
  max-width: 900px;
  margin: 0 auto;
  padding: 28px;
}}
header {{
  border-bottom: 1px solid var(--line);
  padding-bottom: 18px;
}}
h1 {{
  margin: 0;
  font-size: 28px;
  line-height: 1.12;
}}
h2 {{
  margin: 30px 0 10px;
  font-size: 21px;
}}
p {{
  color: var(--muted);
  margin: 10px 0;
  font-size: 16px;
}}
a {{
  color: var(--accent);
  text-underline-offset: 3px;
}}
.actions {{
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 16px;
}}
.actions a {{
  border: 1px solid var(--line);
  border-radius: 6px;
  padding: 8px 11px;
  background: var(--panel);
  text-decoration: none;
}}
.panel {{
  margin-top: 22px;
  padding: 20px;
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel);
}}
.grid {{
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 14px;
}}
.card {{
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 16px;
}}
.card h3 {{
  margin: 0 0 8px;
  font-size: 17px;
}}
code {{
  padding: 1px 5px;
  border-radius: 4px;
  background: var(--code);
  color: var(--ink);
  font-size: 0.92em;
}}
ul {{
  margin: 10px 0 0;
  padding-left: 20px;
  color: var(--muted);
}}
li + li {{ margin-top: 6px; }}
footer {{
  margin-top: 24px;
  padding-top: 16px;
  border-top: 1px solid var(--line);
  color: var(--muted);
  font-size: 16px;
}}
@media (max-width: 760px) {{
  main {{ padding: 16px; }}
  .grid {{ grid-template-columns: 1fr; }}
}}
</style>
</head>
<body>
<main>
  <header>
    <h1>Methods and Repository Notes</h1>
    <p>This page documents what the preference study is comparing. The voting screen keeps A-E anonymous, but the project itself should be easy to inspect and reproduce.</p>
    <div class="actions">
      <a href="index.html">Back to survey</a>
      <a href="results.html">Results</a>
      <a href="{repo_url}" target="_blank" rel="noopener">GitHub repository</a>
    </div>
  </header>

  <section class="panel">
    <h2>What the survey compares</h2>
    <p>The public study currently compares five optical-kerning candidates: {mode_list}. Each screen shows the same text rendered five ways as SVG paths generated from pinned font outlines.</p>
    <p>The survey does not compare against Typst's current metric kerning in the voting UI. Typst and InDesign baselines remain part of the benchmark reports, while the public survey focuses on choosing between candidate optical methods.</p>
  </section>

  <section>
    <h2>Algorithms used in the examples</h2>
    <div class="grid">
      <div class="card">
        <h3><code>nearest-contour-distance</code></h3>
        <p>Uses glyph outlines directly. The implementation flattens contours, samples left and right profiles over vertical slices, and adjusts the pair toward a target visible gap.</p>
      </div>
      <div class="card">
        <h3><code>profile-whitespace</code></h3>
        <p>Builds weighted side profiles for both glyphs and estimates the whitespace between them. Slices near x-height and cap-height matter more than ascenders or descenders.</p>
      </div>
      <div class="card">
        <h3><code>area-balance</code></h3>
        <p>Scores the area of visible whitespace between glyphs and normalizes it against robust median/MAD statistics, so unusual contour spikes do not dominate the result.</p>
      </div>
      <div class="card">
        <h3><code>metric-prior-hybrid</code></h3>
        <p>Starts from the font's own kerning as the prior. Optical correction is only allowed when the font metric result appears missing or strongly inconsistent.</p>
      </div>
      <div class="card">
        <h3><code>safe-fallback-only</code></h3>
        <p>The conservative candidate. It applies optical spacing only when metric kerning effectively offers no useful correction for the pair.</p>
      </div>
    </div>
  </section>

  <section class="panel">
    <h2>What is in the repository</h2>
    <ul>
      <li>Rust code for outline loading, profile extraction, algorithm scoring, and report generation.</li>
      <li>Pinned font corpus metadata and critical pair/word samples.</li>
      <li>Typst-rendered comparison sheets using generated <code>#h(...)</code> spacing deltas.</li>
      <li>Optional InDesign Metrics and Optical exports for visual comparison.</li>
      <li>A static GitHub Pages survey bundle and optional Cloudflare Worker persistence.</li>
    </ul>
  </section>

  <section class="panel">
    <h2>Reproducibility snapshot</h2>
    <p>This build was generated from {font_count} configured fonts and {pair_count} font-pair benchmark rows. Generated samples are embedded as SVG paths, so the public page does not depend on browser font rendering or external font downloads.</p>
    <p>The five-way survey exists because people are more likely to complete 30 focused comparisons than hundreds of pairwise trials, while still ranking every implemented V1 candidate.</p>
  </section>

  <footer>
    This is an independent community study. It is not an official Typst survey, and it is not affiliated with Typst.
  </footer>
</main>
</body>
</html>"#
    )
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_attr(input: &str) -> String {
    escape_html(input).replace('\'', "&#39;")
}
