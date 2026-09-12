#!/usr/bin/env bash
# Start the browser-based dev stack: axum sidecar (:8787) + Vite dev server (:5173).
# For the native desktop app use `npm run dev` (tauri dev) instead.
set -euo pipefail
APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$APP_DIR"

cargo build -p intune_server
RUST_LOG="${RUST_LOG:-info}" ./target/debug/intune-server &
SERVER_PID=$!
trap 'kill "$SERVER_PID" 2>/dev/null || true' EXIT

npm run dev:web
