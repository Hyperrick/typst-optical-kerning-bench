# Cloudflare Worker

The Worker in `persistence/cloudflare-worker.js` provides the small backend
that GitHub Pages cannot provide:

```text
POST   /submit        store one completed survey session
GET    /results       return public aggregate results
POST   /admin/reset   clear the KV database with RESET_TOKEN
DELETE /admin/reset   same reset operation
```

## Setup

Install or run Wrangler through `npx`, then create a KV namespace:

```sh
npx wrangler kv namespace create VOTES
```

Copy the returned namespace id into `wrangler.toml`:

```toml
kv_namespaces = [
  { binding = "VOTES", id = "..." }
]
```

Configure secrets and deploy:

```sh
npx wrangler secret put RESET_TOKEN
npx wrangler secret put FINGERPRINT_SALT
npx wrangler deploy
```

Set `ALLOWED_ORIGIN` as a normal variable in `wrangler.toml` when the GitHub
Pages URL is known:

```toml
[vars]
ALLOWED_ORIGIN = "https://hyperrick.github.io"
```

## Reset

Reset deletes the three persisted prefixes:

```text
submission:
participant:
fingerprint:
```

Run:

```sh
curl -X POST \
  -H "Authorization: Bearer $RESET_TOKEN" \
  https://typst-optical-kerning-bench.example.workers.dev/admin/reset
```

Or use the repo script:

```sh
OPTIKERN_WORKER_URL=https://typst-optical-kerning-bench.example.workers.dev \
RESET_TOKEN=... \
scripts/reset-cloudflare-db.sh
```

The token is a Cloudflare secret. It is not committed to the repository and is
not exposed to the static site.
