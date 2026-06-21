# Data Persistence

The preference suite has two persistence layers:

1. Local browser persistence through `localStorage`.
2. Optional central persistence through the Cloudflare Worker endpoints.

Generate the hosted site with central submission enabled:

```sh
cargo run -p optikern-cli -- survey \
  --submit-endpoint https://typst-optical-kerning-bench.example.workers.dev/submit
```

If no endpoint is configured, the generated site is a local/static preview only.
The public page does not include a JSON export path.

When the submit endpoint ends in `/submit`, the generator derives the public
results endpoint as `/results`. Use `--results-endpoint` only if the aggregate
endpoint has a different URL.

## Storage Location

With the provided Cloudflare Worker, submitted data is stored in a Cloudflare
Workers KV namespace bound as `VOTES`.

The Worker writes three key types:

```text
submission:<uuid>        append-only audit record for every submit attempt
participant:<id>         canonical latest record for one browser participant
fingerprint:<hash>       soft duplicate marker for same IP/user-agent/language
```

Public results are aggregated from `participant:<id>` records. Same-browser
updates replace the canonical participant record. A different participant with
the same coarse fingerprint is marked with
`excludedFromPublicResults: true` and is excluded from public aggregates.
Duplicate attempts are not deleted; they remain available as
`submission:<uuid>` audit records with `duplicate`, `duplicateReason`, and
`duplicateOf`.

## Worker Routes

```text
GET    /health        health check
POST   /submit        store one survey session
GET    /results       public aggregate counts, no raw participant data
POST   /admin/reset   delete all KV records, requires RESET_TOKEN
DELETE /admin/reset   same reset operation, requires RESET_TOKEN
```

The `/results` response intentionally exposes only aggregate mode counts,
pairwise margins, and basic submission totals. It does not expose participant
ids, browser fingerprints, raw user agents, or raw vote records.

## Resetting the Database

Set an admin token as a Cloudflare secret:

```sh
npx wrangler secret put RESET_TOKEN
```

Reset the KV data with:

```sh
curl -X POST \
  -H "Authorization: Bearer $RESET_TOKEN" \
  https://typst-optical-kerning-bench.example.workers.dev/admin/reset
```

The reset deletes these prefixes:

```text
submission:
participant:
fingerprint:
```

## Backend Contract

For GitHub Pages, use a small endpoint outside Pages. Cloudflare Workers plus
KV is a reasonable V1 choice because no write or reset secret is exposed in the
static page.

The Worker should:

- accept `POST` with JSON,
- validate that `votes` is an array,
- add a server-side receive timestamp and random id,
- store the payload,
- return `{ "ok": true, "id": "..." }`,
- expose public aggregates at `/results`,
- protect `/admin/reset` with `RESET_TOKEN`,
- set CORS for the GitHub Pages origin.

See `persistence/cloudflare-worker.js` for the Worker implementation.
