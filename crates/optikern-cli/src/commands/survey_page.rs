use anyhow::Result;

use crate::data::BenchReport;

pub fn build_html(
    _report: &BenchReport,
    trials_json: &str,
    _font_url_prefix: &str,
    submit_endpoint: Option<&str>,
    methods_href: &str,
    results_href: &str,
    repo_url: &str,
) -> Result<String> {
    let submit_endpoint_json = serde_json::to_string(&submit_endpoint.unwrap_or(""))?;
    let methods_href = escape_attr(methods_href);
    let results_href = escape_attr(results_href);
    let repo_url = escape_attr(repo_url);

    Ok(format!(
        r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Optical Kerning Preference Study</title>
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
  --accent-ink: #ffffff;
  --accent-2: #334f7a;
  --choice-hover: #eef8f6;
  --choice-active: #e1f1ee;
  --choice-shadow: rgba(31, 111, 104, 0.18);
  --none-hover: #f3f1eb;
  --none-border: #9f9b8f;
  --complete-bg: #f2faf8;
  --complete-border: #b8d8d2;
  --progress-bg: #e7e6df;
  --sample-scale: 1;
}}
:root[data-theme="dark"] {{
  color-scheme: dark;
  --bg: #151514;
  --ink: #f1f0ea;
  --muted: #aaa79e;
  --line: #3f3d38;
  --panel: #201f1d;
  --accent: #69c7ba;
  --accent-ink: #10211f;
  --accent-2: #8fafdc;
  --choice-hover: #1b3835;
  --choice-active: #214743;
  --choice-shadow: rgba(105, 199, 186, 0.22);
  --none-hover: #2b2925;
  --none-border: #6d685f;
  --complete-bg: #162b28;
  --complete-border: #366e66;
}}
@media (prefers-color-scheme: dark) {{
  :root:not([data-theme="light"]) {{
    color-scheme: dark;
    --bg: #151514;
    --ink: #f1f0ea;
    --muted: #aaa79e;
    --line: #3f3d38;
    --panel: #201f1d;
    --accent: #69c7ba;
    --accent-ink: #10211f;
    --accent-2: #8fafdc;
    --choice-hover: #1b3835;
    --choice-active: #214743;
    --choice-shadow: rgba(105, 199, 186, 0.22);
    --none-hover: #2b2925;
    --none-border: #6d685f;
    --complete-bg: #162b28;
    --complete-border: #366e66;
  }}
}}
* {{ box-sizing: border-box; }}
[hidden] {{ display: none !important; }}
body {{
  margin: 0;
  background: var(--bg);
  color: var(--ink);
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
  line-height: 1.45;
}}
main {{
  max-width: 980px;
  margin: 0 auto;
  padding: 28px;
}}
header {{
  border-bottom: 1px solid var(--line);
  padding-bottom: 18px;
}}
.header-row {{
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 18px;
}}
h1 {{
  margin: 0;
  font-size: 24px;
  font-weight: 700;
}}
.theme-control {{
  display: inline-flex;
  align-items: center;
  gap: 2px;
  padding: 3px;
  border: 1px solid var(--line);
  border-radius: 999px;
  background: var(--panel);
}}
.theme-control button {{
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border: 0;
  border-radius: 999px;
  padding: 0;
  color: var(--muted);
  background: transparent;
}}
.theme-control button:hover,
.theme-control button:focus-visible {{
  color: var(--ink);
  background: var(--none-hover);
}}
.theme-control button.is-active {{
  color: var(--accent-ink);
  background: var(--accent);
}}
.theme-icon {{
  width: 16px;
  height: 16px;
}}
.meta {{
  margin-top: 6px;
  color: var(--muted);
  font-size: 13px;
}}
button, select, input[type="range"] {{
  font: inherit;
}}
button {{
  border: 1px solid var(--line);
  background: var(--panel);
  color: var(--ink);
  padding: 8px 11px;
  border-radius: 6px;
  cursor: pointer;
}}
a {{
  color: var(--accent);
  text-underline-offset: 3px;
}}
a:hover, a:focus-visible {{
  color: var(--ink);
}}
button.primary {{
  background: var(--accent);
  border-color: var(--accent);
  color: var(--accent-ink);
}}
button.secondary {{
  background: var(--accent-2);
  border-color: var(--accent-2);
  color: white;
}}
button:disabled {{
  cursor: not-allowed;
  opacity: 0.45;
}}
button.ghost {{
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: transparent;
}}
.button-icon {{
  width: 16px;
  height: 16px;
}}
.study {{
  margin-top: 22px;
}}
.intro-screen {{
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 30px;
}}
.intro-panel {{
  max-width: 720px;
}}
.intro-panel h2 {{
  margin: 0;
  max-width: 680px;
  font-size: 34px;
  line-height: 1.08;
}}
.intro-panel p {{
  margin: 14px 0 0;
  color: var(--muted);
  font-size: 16px;
}}
.intro-points {{
  display: grid;
  gap: 8px;
  margin: 22px 0;
  padding: 0;
  list-style: none;
  color: var(--ink);
}}
.context-section {{
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 18px;
  margin: 24px 0 22px;
  padding: 18px 0;
  border-top: 1px solid var(--line);
  border-bottom: 1px solid var(--line);
}}
.context-section strong {{
  display: block;
  margin-bottom: 5px;
  color: var(--ink);
  font-size: 16px;
}}
.context-section p {{
  margin: 0;
  color: var(--muted);
  font-size: 16px;
}}
.intro-points li {{
  padding-left: 18px;
  position: relative;
}}
.intro-points li::before {{
  content: "";
  position: absolute;
  left: 0;
  top: 0.72em;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
}}
.intro-action {{
  margin-top: 24px;
}}
.site-footer {{
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  margin-top: 22px;
  padding-top: 16px;
  border-top: 1px solid var(--line);
  color: var(--muted);
  font-size: 16px;
}}
.footer-links {{
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  white-space: nowrap;
}}
.intro-action button {{
  min-width: 150px;
  padding: 10px 14px;
}}
.trial {{
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 18px;
}}
.trial-head {{
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  color: var(--muted);
  font-size: 13px;
  margin-bottom: 14px;
}}
.trial-tools {{
  display: flex;
  align-items: center;
  gap: 16px;
}}
.scale-control {{
  display: flex;
  align-items: center;
  gap: 8px;
  white-space: nowrap;
}}
.scale-control input {{
  width: 112px;
  accent-color: var(--accent);
}}
.scale-value {{
  width: 42px;
  text-align: right;
  font-variant-numeric: tabular-nums;
}}
.complete {{
  display: grid;
  grid-template-columns: 1fr auto;
  gap: 18px;
  align-items: center;
  padding: 26px;
  border: 1px solid var(--complete-border);
  border-radius: 8px;
  background: var(--complete-bg);
}}
.complete strong {{
  display: block;
  color: var(--ink);
  font-size: 18px;
  margin-bottom: 4px;
}}
.complete-copy {{
  color: var(--muted);
  font-size: 16px;
}}
.complete-actions {{
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: 10px;
}}
.survey-nav {{
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 16px;
}}
.choices {{
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(100%, calc(140px * var(--sample-scale))), 1fr));
  gap: 14px;
  align-items: stretch;
}}
.choice {{
  min-height: 190px;
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 14px;
  display: grid;
  grid-template-rows: auto 1fr;
  background: var(--panel);
  text-align: left;
  width: 100%;
  transition: background-color 140ms ease, border-color 140ms ease, box-shadow 140ms ease, transform 140ms ease;
}}
button.choice {{
  cursor: pointer;
}}
.choice:hover, .choice:focus-visible {{
  background: var(--choice-hover);
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--choice-shadow);
  transform: translateY(-1px);
}}
.choice:active {{
  background: var(--choice-active);
  transform: translateY(0);
}}
.choice.is-selected {{
  background: var(--choice-active);
  border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--choice-shadow);
}}
.choice-label {{
  color: var(--ink);
  font-size: 16px;
  font-weight: 700;
}}
.sample-wrap {{
  display: flex;
  align-items: center;
  justify-content: center;
  min-height: calc(118px * var(--sample-scale));
  overflow: hidden;
  white-space: nowrap;
}}
.sample-text {{
  display: inline-block;
  line-height: 1.05;
}}
.sample-svg {{
  display: block;
  width: min(calc(var(--sample-width) * var(--sample-scale)), 100%);
  height: auto;
  overflow: visible;
}}
.intro {{
  max-width: 820px;
  margin-top: 18px;
  color: var(--muted);
  font-size: 16px;
}}
.intro strong {{
  color: var(--ink);
}}
.progress {{
  height: 8px;
  background: var(--progress-bg);
  border-radius: 999px;
  overflow: hidden;
  margin: 10px 0 14px;
}}
.progress > div {{
  height: 100%;
  width: 0;
  background: var(--accent);
}}
@media (max-width: 860px) {{
  main {{ padding: 16px; }}
  .header-row {{ flex-direction: column; }}
  .site-footer {{ align-items: flex-start; flex-direction: column; }}
  .intro-screen {{ padding: 22px; }}
  .intro-panel h2 {{ font-size: 28px; }}
  .context-section {{ grid-template-columns: 1fr; }}
  .trial-head {{ align-items: flex-start; }}
  .trial-tools {{ flex-direction: column; align-items: flex-end; gap: 8px; }}
  .choices {{ grid-template-columns: 1fr; }}
  .survey-nav {{ align-items: stretch; flex-direction: column-reverse; }}
  .complete {{ grid-template-columns: 1fr; }}
  .complete-actions {{ justify-content: flex-start; }}
}}
</style>
</head>
<body>
<main>
  <header>
    <div class="header-row">
      <div>
        <h1>Optical Kerning Preference Study</h1>
        <div class="intro">
          A community preference study for optical-kerning methods in Typst.
        </div>
      </div>
      <div class="theme-control" aria-label="Theme">
        <button type="button" data-theme-choice="light" aria-label="Light theme">
          <svg class="theme-icon" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="12" cy="12" r="4"></circle>
            <path d="M12 2v2"></path>
            <path d="M12 20v2"></path>
            <path d="m4.93 4.93 1.41 1.41"></path>
            <path d="m17.66 17.66 1.41 1.41"></path>
            <path d="M2 12h2"></path>
            <path d="M20 12h2"></path>
            <path d="m6.34 17.66-1.41 1.41"></path>
            <path d="m19.07 4.93-1.41 1.41"></path>
          </svg>
        </button>
        <button type="button" data-theme-choice="dark" aria-label="Dark theme">
          <svg class="theme-icon" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M20.99 14.86A9 9 0 0 1 9.14 3.01 7 7 0 1 0 20.99 14.86Z"></path>
          </svg>
        </button>
      </div>
    </div>
  </header>

  <section class="study">
    <div id="introScreen" class="intro-screen">
      <div class="intro-panel">
        <h2>Help us choose an optical kerning direction for Typst.</h2>
        <p>
          In professional publishing and brand typography, optical spacing is expected. This survey compares Typst-friendly methods to find out which one produces the most convincing results.
        </p>
        <p>
          You will see the same letter pair or word in five anonymous spacing versions. Choose the version that looks best to you.
        </p>
        <div class="context-section" aria-label="Context">
          <div>
            <strong>What is Typst?</strong>
            <p><a href="https://typst.app/" target="_blank" rel="noopener">Typst</a> is an open-source typesetting system for making PDFs from markup. It is used for technical writing, publishing, templates, and documents that need precise layout.</p>
          </div>
          <div>
            <strong>Current kerning</strong>
            <p>Typst currently supports metric kerning: it can use spacing pairs built into the font. It does not yet offer an optical mode based on the visible letter shapes.</p>
          </div>
          <div>
            <strong>What we test</strong>
            <p>These comparisons show candidate optical methods only. Your vote helps decide which approach is visually strongest before performance and implementation tradeoffs are evaluated.</p>
          </div>
        </div>
        <ul class="intro-points" aria-label="Survey details">
          <li>30 short comparisons, randomly selected from the test set.</li>
          <li>Your choices help identify which deterministic, fast, and maintainable approach is worth developing further.</li>
        </ul>
        <div class="intro-action">
          <button id="startSurvey" class="primary">Start survey</button>
        </div>
      </div>
    </div>

    <div id="surveyFrame" class="trial" hidden>
      <div id="trialContent">
        <div class="trial-head">
          <div>Choose the best spacing</div>
          <div class="trial-tools">
            <label class="scale-control">
              <span>Scale</span>
              <input id="scaleSlider" type="range" min="0.75" max="1.6" step="0.05" value="1" aria-label="Scale examples">
              <span id="scaleValue" class="scale-value">100%</span>
            </label>
            <div id="trialCount"></div>
          </div>
        </div>
        <div class="progress"><div id="bar"></div></div>
        <div class="choices">
          <button class="choice" data-vote="0" aria-label="Choose option A">
            <span class="choice-label">A</span>
            <span class="sample-wrap" id="choiceA"></span>
          </button>
          <button class="choice" data-vote="1" aria-label="Choose option B">
            <span class="choice-label">B</span>
            <span class="sample-wrap" id="choiceB"></span>
          </button>
          <button class="choice" data-vote="2" aria-label="Choose option C">
            <span class="choice-label">C</span>
            <span class="sample-wrap" id="choiceC"></span>
          </button>
          <button class="choice" data-vote="3" aria-label="Choose option D">
            <span class="choice-label">D</span>
            <span class="sample-wrap" id="choiceD"></span>
          </button>
          <button class="choice" data-vote="4" aria-label="Choose option E">
            <span class="choice-label">E</span>
            <span class="sample-wrap" id="choiceE"></span>
          </button>
        </div>
        <div class="survey-nav">
          <button id="backButton" class="ghost" type="button" disabled>
            <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="m12 19-7-7 7-7"></path>
              <path d="M19 12H5"></path>
            </svg>
            <span>Back</span>
          </button>
          <button id="nextButton" class="primary" type="button" disabled>Next</button>
        </div>
      </div>
      <div id="complete" class="complete" hidden>
        <div class="complete-copy">
          <strong>Thank you.</strong>
          <div>Your choices help us understand which optical spacing direction feels most balanced in real reading samples.</div>
          <div id="submitStatus" class="meta"></div>
        </div>
        <div class="complete-actions">
          <button id="backButtonComplete" class="ghost" type="button">
            <svg class="button-icon" viewBox="0 0 24 24" aria-hidden="true" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="m12 19-7-7 7-7"></path>
              <path d="M19 12H5"></path>
            </svg>
            <span>Back</span>
          </button>
          <button id="submit" class="primary">Submit Results</button>
        </div>
      </div>
    </div>
  </section>
  <footer class="site-footer">
    <span>This is an independent community study. It is not an official Typst survey, and it is not affiliated with Typst.</span>
    <span class="footer-links">
      <a href="{methods_href}">Methods</a>
      <a href="{results_href}">Results</a>
      <a href="{repo_url}" target="_blank" rel="noopener">Repository</a>
    </span>
  </footer>
</main>

<script>
const TRIALS = {trials_json};
const SUBMIT_ENDPOINT = {submit_endpoint_json};
const SESSION_TRIAL_LIMIT = 30;
const PARTICIPANT_KEY = "optikern.participant.v1";
const STORAGE_KEY = "optikern.preference.v7";
const CHOICE_IDS = ["choiceA", "choiceB", "choiceC", "choiceD", "choiceE"];

function xmur3(str) {{
  let h = 1779033703 ^ str.length;
  for (let i = 0; i < str.length; i++) {{
    h = Math.imul(h ^ str.charCodeAt(i), 3432918353);
    h = h << 13 | h >>> 19;
  }}
  return function() {{
    h = Math.imul(h ^ h >>> 16, 2246822507);
    h = Math.imul(h ^ h >>> 13, 3266489909);
    return (h ^= h >>> 16) >>> 0;
  }};
}}
function mulberry32(a) {{
  return function() {{
    let t = a += 0x6D2B79F5;
    t = Math.imul(t ^ t >>> 15, t | 1);
    t ^= t + Math.imul(t ^ t >>> 7, t | 61);
    return ((t ^ t >>> 14) >>> 0) / 4294967296;
  }};
}}
function shuffle(items, rand) {{
  const out = items.slice();
  for (let i = out.length - 1; i > 0; i--) {{
    const j = Math.floor(rand() * (i + 1));
    [out[i], out[j]] = [out[j], out[i]];
  }}
  return out;
}}
function uuid() {{
  if (crypto.randomUUID) return crypto.randomUUID();
  return "id-" + Date.now().toString(36) + "-" + Math.random().toString(36).slice(2);
}}
function getParticipantId() {{
  const existing = localStorage.getItem(PARTICIPANT_KEY);
  if (existing) return existing;
  const id = uuid();
  localStorage.setItem(PARTICIPANT_KEY, id);
  return id;
}}
function selectSessionOrder(rand) {{
  const groups = new Map();
  for (let i = 0; i < TRIALS.length; i++) {{
    const trial = TRIALS[i];
    const key = `${{trial.kind}}:${{trial.comparison_id}}`;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(i);
  }}
  const keys = shuffle([...groups.keys()], rand);
  for (const key of keys) groups.set(key, shuffle(groups.get(key), rand));
  const order = [];
  while (order.length < SESSION_TRIAL_LIMIT) {{
    let added = false;
    for (const key of keys) {{
      const group = groups.get(key);
      if (!group.length) continue;
      order.push(group.pop());
      added = true;
      if (order.length >= SESSION_TRIAL_LIMIT) break;
    }}
    if (!added) break;
  }}
  return shuffle(order, rand);
}}
function loadState() {{
  const existing = localStorage.getItem(STORAGE_KEY);
  if (existing) {{
    const parsed = JSON.parse(existing);
    if (parsed.schemaVersion === 7) {{
      parsed.scale = normalizeScale(parsed.scale || 1);
      parsed.theme = normalizeTheme(parsed.theme);
      parsed.cursor = normalizeCursor(parsed);
      return parsed;
    }}
  }}
  const seed = String(Date.now());
  const seedFn = xmur3(seed);
  const rand = mulberry32(seedFn());
  const order = selectSessionOrder(rand);
  const sides = TRIALS.map(trial => shuffle(trial.choices.map((_, index) => index), rand));
  return {{
    schemaVersion: 7,
    createdAt: new Date().toISOString(),
    startedAt: null,
    participantId: getParticipantId(),
    sessionId: uuid(),
    seed,
    order,
    sides,
    scale: 1,
    theme: getInitialTheme(),
    cursor: 0,
    trialPoolCount: TRIALS.length,
    votes: []
  }};
}}
function saveState() {{
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
}}

let state = loadState();

function getInitialTheme() {{
  return window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches
    ? "dark"
    : "light";
}}
function normalizeTheme(value) {{
  return value === "light" || value === "dark" ? value : getInitialTheme();
}}
function applyTheme() {{
  const theme = normalizeTheme(state.theme);
  state.theme = theme;
  document.documentElement.dataset.theme = theme;
  document.querySelectorAll("[data-theme-choice]").forEach(button => {{
    const active = button.dataset.themeChoice === theme;
    button.classList.toggle("is-active", active);
    button.setAttribute("aria-pressed", active ? "true" : "false");
  }});
}}
function normalizeScale(value) {{
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return 1;
  return Math.min(1.6, Math.max(0.75, parsed));
}}
function applyScale() {{
  const scale = normalizeScale(state.scale);
  state.scale = scale;
  document.documentElement.style.setProperty("--sample-scale", scale.toFixed(2));
  const slider = document.getElementById("scaleSlider");
  const value = document.getElementById("scaleValue");
  if (slider) slider.value = String(scale);
  if (value) value.textContent = `${{Math.round(scale * 100)}}%`;
}}

function currentTrialIndex() {{
  return Math.min(normalizeCursor(state), state.order.length - 1);
}}
function currentTrial() {{
  return TRIALS[state.order[currentTrialIndex()]];
}}
function currentSideOrder() {{
  return state.sides[state.order[currentTrialIndex()]];
}}
function hasStarted() {{
  return Boolean(state.startedAt) || state.votes.length > 0;
}}
function normalizeCursor(value) {{
  const voteCount = Array.isArray(value.votes) ? value.votes.length : 0;
  const max = Array.isArray(value.order) ? value.order.length : 0;
  const fallback = voteCount >= max && max > 0 ? max : voteCount;
  const cursor = Number.isInteger(value.cursor) ? value.cursor : fallback;
  return Math.min(Math.max(cursor, 0), max);
}}
function updateNavigation() {{
  state.cursor = normalizeCursor(state);
  const canGoBack = state.cursor > 0;
  const canGoNext = state.cursor < state.votes.length;
  ["backButton", "backButtonComplete"].forEach(id => {{
    const button = document.getElementById(id);
    if (button) button.disabled = !canGoBack;
  }});
  const next = document.getElementById("nextButton");
  if (next) next.disabled = !canGoNext;
}}
function render() {{
  applyTheme();
  applyScale();
  updateNavigation();
  if (!hasStarted()) {{
    document.getElementById("introScreen").hidden = false;
    document.getElementById("surveyFrame").hidden = true;
    return;
  }}

  document.getElementById("introScreen").hidden = true;
  document.getElementById("surveyFrame").hidden = false;
  const done = state.votes.length;
  document.getElementById("bar").style.width = `${{100 * done / state.order.length}}%`;
  if (done >= state.order.length && state.cursor >= state.order.length) {{
    document.getElementById("trialContent").hidden = true;
    document.getElementById("complete").hidden = false;
    document.getElementById("submit").hidden = !SUBMIT_ENDPOINT;
    document.getElementById("submitStatus").textContent = SUBMIT_ENDPOINT
      ? "Ready to submit."
      : "Local preview only. Submission is not connected in this build.";
    return;
  }}

  document.getElementById("trialContent").hidden = false;
  document.getElementById("complete").hidden = true;
  document.getElementById("submit").hidden = false;
  const trial = currentTrial();
  const sideOrder = currentSideOrder();
  const choices = sideOrder.map(i => trial.choices[i]);
  const cursor = currentTrialIndex();
  document.getElementById("trialCount").textContent = `${{cursor + 1}} / ${{state.order.length}}`;
  CHOICE_IDS.forEach((id, slot) => {{
    document.getElementById(id).innerHTML = choices[slot] ? choices[slot].html : "";
  }});
  document.querySelectorAll("[data-vote]").forEach(button => {{
    const selected = state.votes[cursor]?.vote === button.dataset.vote;
    button.classList.toggle("is-selected", selected);
    button.setAttribute("aria-pressed", selected ? "true" : "false");
  }});
  applyScale();
}}
function recordVote(value) {{
  state.cursor = normalizeCursor(state);
  if (state.cursor >= state.order.length) return;
  const trial = currentTrial();
  const sideOrder = currentSideOrder();
  let winner = null;
  let loser = null;
  const pickedSlot = Number(value);
  if (Number.isInteger(pickedSlot) && pickedSlot >= 0 && pickedSlot < sideOrder.length) {{
    winner = trial.choices[sideOrder[pickedSlot]].mode;
    loser = sideOrder
      .filter((_, slot) => slot !== pickedSlot)
      .map(choiceIndex => trial.choices[choiceIndex].mode)
      .join(",");
  }}
  const vote = {{
    trialId: trial.id,
    comparisonId: trial.comparison_id,
    fontId: trial.font_id,
    family: trial.family,
    category: trial.category,
    kind: trial.kind,
    sample: trial.sample,
    shownModes: sideOrder.map(i => trial.choices[i].mode),
    vote: value,
    winner,
    loser,
    losers: loser ? loser.split(",") : [],
    confidence: null,
    recordedAt: new Date().toISOString()
  }};
  if (state.cursor < state.votes.length) {{
    state.votes[state.cursor] = vote;
  }} else {{
    state.votes.push(vote);
  }}
  state.cursor = Math.min(state.cursor + 1, state.order.length);
  delete state.lastSubmittedAt;
  delete state.lastSubmittedVoteCount;
  delete state.lastSubmitResponse;
  saveState();
  render();
}}
function goBack() {{
  state.cursor = normalizeCursor(state);
  if (state.cursor === 0) return;
  state.cursor -= 1;
  saveState();
  render();
}}
function goNext() {{
  state.cursor = normalizeCursor(state);
  if (state.cursor >= state.votes.length) return;
  state.cursor = Math.min(state.cursor + 1, state.order.length);
  saveState();
  render();
}}
function sessionPayload() {{
  return {{
    ...state,
    exportedAt: new Date().toISOString(),
    selectedTrialCount: state.order.length,
    trialCount: state.order.length,
    completed: state.votes.length,
    userAgent: navigator.userAgent,
    pageUrl: location.href
  }};
}}
async function submitResults() {{
  const status = document.getElementById("submitStatus");
  if (state.lastSubmittedVoteCount === state.votes.length && state.votes.length > 0) {{
    status.textContent = "This result set was already submitted from this browser.";
    return;
  }}
  if (!SUBMIT_ENDPOINT) {{
    status.textContent = "Submission is not connected in this build.";
    return;
  }}
  status.textContent = "Submitting...";
  try {{
    const response = await fetch(SUBMIT_ENDPOINT, {{
      method: "POST",
      headers: {{ "content-type": "application/json" }},
      body: JSON.stringify(sessionPayload())
    }});
    if (!response.ok) throw new Error(`HTTP ${{response.status}}`);
    const data = await response.json().catch(() => ({{}}));
    state.lastSubmittedAt = new Date().toISOString();
    state.lastSubmittedVoteCount = state.votes.length;
    state.lastSubmitResponse = data;
    saveState();
    status.textContent = data.duplicate
      ? `Submitted update; server flagged it as duplicate (${{data.duplicateReason || "same participant"}}).`
      : `Submitted ${{state.votes.length}} votes.`;
  }} catch (error) {{
    status.textContent = `Submit failed: ${{error.message}}. Please try again later.`;
  }}
}}

document.querySelectorAll("[data-vote]").forEach(button => {{
  button.addEventListener("click", () => recordVote(button.dataset.vote));
}});
document.getElementById("backButton").addEventListener("click", goBack);
document.getElementById("backButtonComplete").addEventListener("click", goBack);
document.getElementById("nextButton").addEventListener("click", goNext);
document.querySelectorAll("[data-theme-choice]").forEach(button => {{
  button.addEventListener("click", () => {{
    state.theme = normalizeTheme(button.dataset.themeChoice);
    saveState();
    applyTheme();
  }});
}});
document.getElementById("startSurvey").addEventListener("click", () => {{
  if (!state.startedAt) state.startedAt = new Date().toISOString();
  saveState();
  render();
}});
document.getElementById("scaleSlider").addEventListener("input", event => {{
  state.scale = normalizeScale(event.target.value);
  saveState();
  applyScale();
}});
document.getElementById("submit").addEventListener("click", submitResults);
render();
</script>
</body>
</html>"#
    ))
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
