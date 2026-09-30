#!/usr/bin/env bash
# End-to-end check of the docker-compose stack, as a browser would use it:
# sign up, create a folder, upload straight to storage, download it back.
# Requires curl and jq. Usage: ./deploy/compose/smoke.sh [base-url]
set -euo pipefail

BASE="${1:-http://localhost:8080}"
API="$BASE/api/v1"
EMAIL="smoke-$(date +%s)-$RANDOM@example.com"
PASSWORD="correct horse battery staple"
CONTENT="hello from the smoke test $RANDOM"

step() { printf '• %s\n' "$*"; }
fail() { printf '✗ %s\n' "$*" >&2; exit 1; }
json() { curl -fsS -H 'Content-Type: application/json' "$@"; }

step "web serves the SPA and its client routes"
curl -fsS "$BASE/" | grep -q '<div id="root">' || fail "no SPA at $BASE/"
curl -fsS "$BASE/folders/anything" | grep -q '<div id="root">' || fail "no SPA fallback"

step "register and sign in ($EMAIL)"
json -X POST "$API/auth/register" \
  -d "$(jq -n --arg e "$EMAIL" --arg p "$PASSWORD" '{email: $e, password: $p, display_name: "Smoke"}')" >/dev/null
TOKEN=$(json -X POST "$API/auth/login" \
  -d "$(jq -n --arg e "$EMAIL" --arg p "$PASSWORD" '{email: $e, password: $p}')" | jq -r .access_token)
AUTH=(-H "Authorization: Bearer $TOKEN")

step "create a folder"
FOLDER=$(json "${AUTH[@]}" -X POST "$API/folders" -d '{"name": "smoke"}' | jq -r .id)

step "presign an upload"
UPLOAD=$(json "${AUTH[@]}" -X POST "$API/uploads" \
  -d "$(jq -n --argjson s "${#CONTENT}" '{size_bytes: $s, mime_type: "text/plain"}')")
KEY=$(jq -r .storage_key <<<"$UPLOAD")
URL=$(jq -r .upload.url <<<"$UPLOAD")

step "storage accepts the browser's CORS preflight"
ALLOW=$(curl -fsS -o /dev/null -D - -X OPTIONS "$URL" \
  -H "Origin: $BASE" -H 'Access-Control-Request-Method: PUT' \
  -H 'Access-Control-Request-Headers: content-type' |
  tr -d '\r' | awk -F': ' 'tolower($1) == "access-control-allow-origin" {print $2}')
[[ -n "$ALLOW" ]] || fail "no Access-Control-Allow-Origin for $BASE on $URL"

step "PUT the bytes straight to storage"
HEADERS=()
while IFS=$'\t' read -r name value; do
  case "${name,,}" in host | content-length) ;; *) HEADERS+=(-H "$name: $value") ;; esac
done < <(jq -r '.upload.headers | to_entries[] | "\(.key)\t\(.value)"' <<<"$UPLOAD")
curl -fsS -X PUT "${HEADERS[@]}" --data-binary "$CONTENT" "$URL" >/dev/null

step "confirm the upload"
FILE=$(json "${AUTH[@]}" -X POST "$API/files" \
  -d "$(jq -n --arg k "$KEY" --arg p "$FOLDER" '{storage_key: $k, parent_id: $p, name: "hello.txt"}')" | jq -r .id)

step "download it back through a presigned GET"
DOWNLOAD=$(curl -fsS "${AUTH[@]}" "$API/nodes/$FILE/download" | jq -r .url)
[[ "$(curl -fsS "$DOWNLOAD")" == "$CONTENT" ]] || fail "downloaded bytes differ"

printf '✓ smoke test passed against %s\n' "$BASE"
