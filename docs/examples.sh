#!/usr/bin/env bash
# ecobook smoke test — run against a live ecobook-backend.
set -euo pipefail
BASE="${1:-http://127.0.0.1:9015/api}"
echo "== health =="
curl -fsS "$BASE/health"
echo
echo "== catalog =="
curl -fsS "$BASE/book/catalog" | head -c 200
echo
echo "== create a deck (needs auth token; unset to skip) =="
if [[ -n "${TOKEN:-}" ]]; then
  curl -fsS -X POST "$BASE/book" -H "Authorization: Bearer $TOKEN" \
    -H "Content-Type: application/json" \
    -d '{"name":"Smoke deck","status":"draft","slides":[{"id":"s1","elements":[]}]}' | head -c 200
  echo
else
  echo "TOKEN unset — skipping authenticated create"
fi
echo "OK"
