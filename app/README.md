# Intune Manager — Cross-Platform Desktop

A beautiful, simple, genuinely cross-platform desktop GUI that reimagines
[IntuneManagement](../README.md) (the Windows-only PowerShell + WPF tool) as a
modern app that runs on **Linux (including Omarchy / Arch + Hyprland/Wayland),
macOS and Windows**.

All Microsoft Intune / Azure AD functionality goes through the Microsoft Graph
`beta` API, so it is inherently cross-platform once decoupled from WPF.

![Architecture](docs/architecture.svg)

---

## Architecture decision & rationale

**Stack: Tauri v2 (Rust core + web frontend) + React + TypeScript + Vite +
Tailwind CSS + shadcn/ui.**

| Concern | Decision | Why |
| --- | --- | --- |
| Desktop shell | **Tauri v2** | Tiny native binaries, first-class Linux/Wayland (WebKitGTK) support that runs on Omarchy, secure by default. Ships `deb` + `AppImage`. |
| Graph client | **Rust core (`reqwest`/`tokio`)** | Tokens and secrets stay in the native process, never the renderer. Handles pagination (`@odata.nextLink`), 429/5xx retry, and `beta` endpoints. |
| Frontend | **React + TS + Tailwind + shadcn/ui (Radix)** | Elegant, minimal, keyboard-friendly UI with light/dark themes. |
| Auth | **Device-code** (primary, best for Linux/Wayland — no embedded browser) and **app-only client-credentials** (headless/DevOps + automated tests) | Matches how the original tool authenticates; device-code is the real-user path on Linux. Device-code defaults to Microsoft's first-party **Graph Command Line Tools** public client (`14d82eec-…`) so sign-in works in any tenant with no app registration; overridable in the UI. |

### One backend, two hosts

The core logic lives in a **UI-agnostic Rust library** (`crates/core`) and is
exposed **two ways**:

- **Tauri commands** (`src-tauri`) for the native desktop app.
- A tiny **axum HTTP sidecar** (`crates/server`, `127.0.0.1` only) for
  browser-based development and headless/DevOps automation.

The frontend's transport layer (`src/api/client.ts`) auto-detects its host: it
uses native `invoke()` inside Tauri and `fetch('/api/...')` in the browser. In
**both** cases tokens live only in the Rust process — never in the renderer.

```
crates/core     Rust library: auth, Graph client, catalog, export/import/compare
crates/server   axum sidecar exposing core over HTTP (dev + headless)
src-tauri       Tauri v2 desktop shell (#[tauri::command] wrappers)
src             React + Tailwind + shadcn/ui frontend
```

> **Why not Electron?** Tauri builds, runs and packages cleanly on this Linux
> stack (verified: `cargo build`/`tauri build` produce a `deb` + `AppImage`),
> so the smaller, more secure option was kept. Electron remains a drop-in
> fallback since the frontend and the HTTP sidecar are host-agnostic.

---

## Features

- **Auth** — two modes:
  - **Device code (interactive, default).** You sign in as yourself and approve
    the request in your browser — ideal for Linux/Wayland/Omarchy since there is
    no embedded browser. It uses Microsoft's built-in **Graph Command Line
    Tools** public client by default, so it works in every tenant without
    registering an app. You can override the client id and tenant from the
    sign-in screen's *Advanced* section; the override persists on the device.
  - **App credentials (unattended).** Your own Entra ID app registration +
    client secret, for automation/CI and the headless smoke test.

  Shows tenant/org name after sign in; sign out clears the session.
- **Browse** — sidebar grouped by object type (48 types from the original tool);
  fast, searchable, sortable tables; Graph pagination handled server-side.
- **Detail** — friendly rendered overview + full JSON + assignments.
- **Export** — selected/all objects to JSON on disk, mirroring the original
  tool's `<Object Type>/<name>.json` folder layout (assignments embedded).
- **Import** — recreate objects from exported JSON. **Safe by default**: a dry
  run returns the cleaned payload (read-only fields stripped) without writing
  to the tenant. Pass `dryRun: false` to actually create.
- **Compare** — property-level diff between a live Intune object and an exported
  file, ignoring volatile fields (ids, timestamps).

---

## Build & run

### Prerequisites (install once)

```bash
bash app/scripts/setup.sh   # idempotent: system deps + Rust + npm install
```

- **Rust** stable ≥ 1.85 (via rustup)
- **Node.js** ≥ 20
- **Linux system deps** (Ubuntu/Debian): `libwebkit2gtk-4.1-dev build-essential
  curl wget file libxdo-dev libssl-dev libgtk-3-dev
  libayatana-appindicator3-dev librsvg2-dev patchelf`
- **Arch / Omarchy**: `webkit2gtk-4.1 base-devel curl wget file openssl gtk3
  libappindicator-gtk3 librsvg patchelf nodejs npm` (installed by
  `scripts/setup.sh`; package names follow the official Tauri v2 Linux
  prerequisites for Arch). Rust is installed via `rustup`.

### Native desktop app (Linux / Omarchy / macOS / Windows)

```bash
cd app
npm run dev      # tauri dev — hot-reloading desktop window
npm run build    # tauri build — produces target/release/bundle (deb + AppImage)
```

### Running on Omarchy / Hyprland (Wayland)

On Hyprland/Omarchy (and some other Wayland compositors) WebKitGTK's DMA-BUF
renderer can produce a **blank/black window**. The fix is to disable it. The
bundled Linux launcher sets this for you:

```bash
cd app
npm run dev:linux     # tauri dev with the Wayland workaround pre-applied
npm run start:linux   # launch a built binary/.AppImage with the workaround
```

Both wrap `scripts/run-linux.sh`, which exports
`WEBKIT_DISABLE_DMABUF_RENDERER=1` before starting the app. If the window is
still blank, add the compositing fallback:

```bash
WEBKIT_DISABLE_COMPOSITING_MODE=1 npm run dev:linux
```

Or run any command manually with the same variables:

```bash
WEBKIT_DISABLE_DMABUF_RENDERER=1 WEBKIT_DISABLE_COMPOSITING_MODE=1 npm run dev
```

**Portable artifact:** for Arch/Omarchy the recommended distributable is the
**`.AppImage`** produced by `npm run build` (self-contained, no system package
install, just `chmod +x` and run — launch it via `npm run start:linux` or with
`WEBKIT_DISABLE_DMABUF_RENDERER=1` set to be safe on Wayland). The generated
`.deb` targets Debian/Ubuntu; on Arch install the runtime libraries listed
above with `pacman` and run the built binary directly instead.

### Browser dev / headless (no desktop shell)

```bash
cd app
npm run dev:server     # axum sidecar on http://127.0.0.1:8787
npm run dev:web        # Vite dev server on http://localhost:5173 (proxies /api)
# …or both at once:
bash scripts/dev.sh
```

App-only sign in reads `tenant_id`, `app_id`, `app_secret` from the environment,
which makes the HTTP sidecar usable for CI/automation:

```bash
curl -s -XPOST localhost:8787/api/auth/app-only -d '{}'
curl -s localhost:8787/api/objects/SettingsCatalog | jq '.count'
```

---

## Quality gates

```bash
npm run typecheck    # tsc --noEmit
npm run lint         # eslint
npm run build:web    # production frontend build
cargo build          # workspace (core + server + tauri)
```

## Safety

Testing against a live tenant is **read-only** by default. Import is a dry run
unless `dryRun: false` is explicitly passed, and no destructive operations are
exposed. The original PowerShell tool under [`../`](../) is left untouched.
