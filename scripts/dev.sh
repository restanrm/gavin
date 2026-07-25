#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

for cmd in cargo npm node; do
  if ! command -v "$cmd" >/dev/null 2>&1; then
    echo "Missing required command: $cmd" >&2
    exit 1
  fi
done

export AUTH_MODE="${AUTH_MODE:-dev}"
export HOST="${HOST:-127.0.0.1}"
export PORT="${PORT:-3000}"
export DATABASE_URL="${DATABASE_URL:-sqlite://data/gavin.db}"
export UPLOAD_DIR="${UPLOAD_DIR:-data/uploads}"
export FRONTEND_DIR="${FRONTEND_DIR:-dist}"
export PUBLIC_DOMAIN="${PUBLIC_DOMAIN:-localhost:${PORT}}"
export COOKIE_SECURE="${COOKIE_SECURE:-false}"
export RUST_LOG="${RUST_LOG:-info,gavin=debug,tower_http=debug}"

if [[ "$AUTH_MODE" != "dev" ]]; then
  echo "This helper is intended for AUTH_MODE=dev; got AUTH_MODE=$AUTH_MODE" >&2
  exit 1
fi

if [[ "$PORT" != "3000" ]]; then
  echo "Warning: frontend dev proxy currently targets localhost:3000; PORT=$PORT may not work." >&2
fi

mkdir -p "$UPLOAD_DIR" data

if [[ ! -d node_modules ]]; then
  echo "node_modules not found; running npm ci..."
  npm ci
fi

cleanup() {
  local code=$?
  trap - EXIT INT TERM
  echo
  echo "Stopping Gavin dev processes..."
  for pid in "${PIDS[@]:-}"; do
    if kill -0 "$pid" >/dev/null 2>&1; then
      kill "$pid" >/dev/null 2>&1 || true
    fi
  done
  wait >/dev/null 2>&1 || true
  exit "$code"
}
trap cleanup EXIT INT TERM

PIDS=()

echo "Starting Gavin backend in dev auth mode on http://${HOST}:${PORT}"
cargo run &
PIDS+=("$!")

echo "Starting Gavin frontend on http://127.0.0.1:5173"
npm run dev -- --host 127.0.0.1 &
PIDS+=("$!")

cat <<EOF

Gavin dev stack is starting:
  Frontend: http://127.0.0.1:5173
  Backend:  http://${HOST}:${PORT}
  Auth:     AUTH_MODE=dev (click Login to enable local dev-admin)

Press Ctrl-C to stop both processes.
EOF

wait -n "${PIDS[@]}"
