#!/usr/bin/env bash
# Idempotent setup for the cross-platform Intune Manager desktop app.
# Installs: Tauri v2 Linux system deps, Rust (stable), Node dependencies.
# Safe to re-run. Works on Ubuntu/Debian; see README for Arch/Omarchy notes.
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$APP_DIR"

log() { printf '\033[1;35m[setup]\033[0m %s\n' "$*"; }

# ---- 1. System dependencies (Tauri v2 / WebKitGTK) -------------------------
if command -v apt-get >/dev/null 2>&1; then
  if ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
    log "Installing Tauri system dependencies via apt…"
    sudo apt-get update -y
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y \
      -o Dpkg::Options::=--force-confold -o Dpkg::Options::=--force-confdef \
      libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev \
      libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev patchelf
  else
    log "System dependencies already present."
  fi
elif command -v pacman >/dev/null 2>&1; then
  # Arch Linux / Omarchy. Package names verified against the official
  # Tauri v2 "Linux prerequisites" docs for Arch.
  log "Installing Tauri system dependencies via pacman…"
  sudo pacman -Sy --needed --noconfirm \
    webkit2gtk-4.1 base-devel curl wget file openssl gtk3 \
    libappindicator-gtk3 librsvg patchelf \
    nodejs npm
fi

# ---- 2. Rust toolchain -----------------------------------------------------
if ! command -v cargo >/dev/null 2>&1; then
  log "Installing Rust via rustup…"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1091
  source "$HOME/.cargo/env"
fi
# Tauri v2 dependencies require a recent stable (edition2024). Ensure >= 1.85.
RUST_MINOR="$(rustc --version | sed -E 's/rustc 1\.([0-9]+).*/\1/')"
if [ "${RUST_MINOR:-0}" -lt 85 ]; then
  log "Updating Rust stable toolchain…"
  rustup default stable
  rustup update stable
fi
log "Using $(rustc --version)"

# ---- 3. Node dependencies --------------------------------------------------
if command -v npm >/dev/null 2>&1; then
  log "Installing Node dependencies…"
  npm install --no-fund --no-audit
else
  log "WARNING: npm not found. Install Node.js 20+ to build the frontend."
fi

log "Setup complete. Run 'npm run dev' (desktop) or 'npm run dev:server' + 'npm run dev:web' (browser)."
