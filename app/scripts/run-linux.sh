#!/usr/bin/env bash
# Launch Intune Manager on Linux with the Wayland/WebKitGTK rendering
# workarounds pre-applied. On Hyprland/Omarchy (and some other Wayland
# compositors) WebKitGTK's DMA-BUF renderer produces a black/blank window;
# disabling it makes the app render correctly.
#
# Usage:
#   scripts/run-linux.sh dev          # tauri dev (default)
#   scripts/run-linux.sh app [path]   # run a built binary/.AppImage
#
# Override behaviour with env vars before calling if you need to:
#   WEBKIT_DISABLE_DMABUF_RENDERER   (default 1)  primary Wayland fix
#   WEBKIT_DISABLE_COMPOSITING_MODE  (unset)      fallback if still blank
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$APP_DIR"

# Only set the workaround if the caller has not already chosen a value.
export WEBKIT_DISABLE_DMABUF_RENDERER="${WEBKIT_DISABLE_DMABUF_RENDERER:-1}"

mode="${1:-dev}"

log() { printf '\033[1;35m[run]\033[0m %s\n' "$*"; }
log "WEBKIT_DISABLE_DMABUF_RENDERER=$WEBKIT_DISABLE_DMABUF_RENDERER"
if [ -n "${WEBKIT_DISABLE_COMPOSITING_MODE:-}" ]; then
  export WEBKIT_DISABLE_COMPOSITING_MODE
  log "WEBKIT_DISABLE_COMPOSITING_MODE=$WEBKIT_DISABLE_COMPOSITING_MODE"
fi

case "$mode" in
  dev)
    log "Starting tauri dev…"
    exec npm run dev
    ;;
  app)
    target="${2:-}"
    if [ -z "$target" ]; then
      # Prefer a built .AppImage, then the plain release binary.
      # Bundles land under the workspace target dir (app/target), not src-tauri/target.
      target="$(find target -type f -name '*.AppImage' 2>/dev/null | head -n1 || true)"
      if [ -z "$target" ]; then
        target="$(find target/release -maxdepth 1 -type f -name 'intune-manager' 2>/dev/null | head -n1 || true)"
      fi
    fi
    if [ -z "$target" ] || [ ! -e "$target" ]; then
      echo "No built app found. Run 'npm run build' first, or pass a path." >&2
      exit 1
    fi
    chmod +x "$target" 2>/dev/null || true
    log "Launching $target"
    exec "$target"
    ;;
  *)
    echo "Unknown mode '$mode' (expected 'dev' or 'app')." >&2
    exit 1
    ;;
esac
