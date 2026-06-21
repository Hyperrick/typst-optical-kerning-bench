#!/usr/bin/env sh
set -eu

: "${OPTIKERN_WORKER_URL:?Set OPTIKERN_WORKER_URL to the deployed Worker base URL.}"
: "${RESET_TOKEN:?Set RESET_TOKEN to the Cloudflare Worker RESET_TOKEN secret value.}"

curl -fsS -X POST \
  -H "Authorization: Bearer ${RESET_TOKEN}" \
  "${OPTIKERN_WORKER_URL%/}/admin/reset"
printf "\n"
