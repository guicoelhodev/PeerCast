#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$project_dir"

if [[ ! -f .env ]]; then
  echo "Missing .env. Copy .env.example to .env and fill in the Tailscale values." >&2
  exit 1
fi

set -a
# shellcheck disable=SC1091
source ./.env
set +a

: "${TAILSCALE_AUTHKEY:?Set TAILSCALE_AUTHKEY in .env}"
: "${PUBLIC_APP_URL:?Set PUBLIC_APP_URL in .env}"

docker compose up -d tailscale

for _ in {1..30}; do
  if docker compose exec -T tailscale tailscale ip -4 >/dev/null 2>&1; then
    break
  fi
  sleep 2
done

if ! docker compose exec -T tailscale tailscale ip -4 >/dev/null 2>&1; then
  echo "Tailscale did not connect. Inspect with: docker compose logs tailscale" >&2
  exit 1
fi

docker compose exec -T tailscale \
  tailscale serve --bg --https=443 http://127.0.0.1:17777

echo "Tailscale HTTPS proxy ready for ${PUBLIC_APP_URL}"
exec pnpm tauri:dev
