# GitHub Pages Hosting

The preference suite is static and can be hosted on GitHub Pages.

Generate the bundle:

```sh
cargo run -p optikern-cli -- fetch-fonts
cargo run -p optikern-cli -- bench
cargo run -p optikern-cli -- survey \
  --submit-endpoint https://typst-optical-kerning-bench.hyperrick.workers.dev/submit \
  --repo-url https://github.com/Hyperrick/typst-optical-kerning-bench
```

The publishable directory is:

```text
site/
```

It contains:

- `index.html`
- `methods.html`
- `results.html`
- `.nojekyll`
- a short README

## Deployment Shape

This repository includes `.github/workflows/pages.yml`, which uploads `site/`
as the Pages artifact on every push to `main`.

Expected public URL:

```text
https://hyperrick.github.io/typst-optical-kerning-bench/
```

Do not point Pages directly at `reports/`; that folder is ignored and meant for
local generated output. The hosted survey embeds samples as SVG paths, so it
does not need runtime font files.

## Runtime Data Flow

GitHub Pages cannot receive or store votes by itself. The V1 collection model
is:

1. `index.html` runs entirely as a static page.
2. It autosaves progress in browser `localStorage`.
3. At the end, it posts the completed session to the configured Worker.
4. The Worker stores the session in Cloudflare KV.
5. `results.html` reads public aggregates from the configured `/results`
   endpoint.

The provided Worker stores data in Cloudflare Workers KV. The public UI does
not include a JSON export path.
