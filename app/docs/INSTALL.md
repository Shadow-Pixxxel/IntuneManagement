# Installation & Launch Guide

End-to-end instructions for installing and launching **Intune Manager** on every
supported OS. Two paths are covered for each platform:

- **(A) Prebuilt bundle** — for end users who just want to run the app.
- **(B) From source** — for developers who want to build and hack on it.

> The app is a [Tauri v2](https://v2.tauri.app) desktop app (Rust core + web UI).
> `npm run build` runs `tauri build`, which produces the **native installer(s)
> for the OS you build on** (Linux: `.deb` + `.rpm` + `.AppImage`; macOS: `.app`
> + `.dmg`; Windows: `.msi` + `.exe`). Tauri automatically skips bundle targets
> that don't apply to the current host, so you build each platform's artifacts
> on that platform (or via CI).

**Contents**

- [Prerequisites & from-source quickstart](#prerequisites--from-source-quickstart)
- [First launch / sign-in](#first-launch--sign-in)
- [Where the build artifacts land](#where-the-build-artifacts-land)
- [Linux — Debian / Ubuntu (`.deb`)](#linux--debian--ubuntu-deb)
- [Linux — universal `.AppImage`](#linux--universal-appimage)
- [Linux — Arch / Omarchy (Hyprland/Wayland)](#linux--arch--omarchy-hyprlandwayland)
- [Linux — Fedora / RHEL (`.rpm`, dnf)](#linux--fedora--rhel-rpm-dnf)
- [macOS](#macos)
- [Windows](#windows)

---

## Prerequisites & from-source quickstart

You only need these for the **from-source** path (B). End users installing a
prebuilt bundle can skip to their OS section below.

**Toolchain (all platforms):**

- **Node.js ≥ 20** — <https://nodejs.org>
- **Rust (stable ≥ 1.85) via rustup** — <https://rustup.rs>
  ```bash
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```
- **Platform system libraries** (WebKitGTK etc. on Linux; see per-OS sections).

**One-shot setup (Linux + toolchain):**

```bash
git clone <this-repo> && cd <repo>/app
bash scripts/setup.sh   # idempotent: installs Tauri system deps (apt or pacman),
                        # Rust via rustup, and runs `npm install`
```

`scripts/setup.sh` auto-detects your package manager: it uses **apt** on
Debian/Ubuntu and **pacman** on Arch/Omarchy. On other distros (Fedora, etc.)
install the system libraries manually from that OS's section below, then run
`npm install` yourself.

**Build & run commands** (run from the `app/` directory):

| Command | What it does |
| --- | --- |
| `npm run dev` | `tauri dev` — hot-reloading native desktop window. |
| `npm run dev:linux` | Same as `dev`, but with the Wayland/WebKitGTK workaround pre-applied (use on Hyprland/Omarchy). |
| `npm run build` | `tauri build` — release binary + native installers for the current OS. |
| `npm run start:linux` | Launch an already-built binary/`.AppImage` with the Wayland workaround. |
| `npm run dev:server` + `npm run dev:web` | Browser dev stack (axum sidecar on `:8787` + Vite on `:5173`); or `bash scripts/dev.sh` for both. |

---

## First launch / sign-in

On first launch you'll see a sign-in screen with **two authentication modes**:

**1. Device code (interactive — default, works out of the box).**
You sign in as yourself and approve the request in your browser, so there's no
embedded browser — ideal for Linux/Wayland/Omarchy. It uses Microsoft's
first-party **Graph Command Line Tools** public client
(`14d82eec-204b-4c2f-b7e8-296a70dab67e`) by default, which exists in every
tenant, so **no app registration is required**.

- Need your own Entra ID app or a specific tenant? Expand **Advanced: custom app
  registration** on the sign-in screen and enter your **client id** and
  **tenant** — the override is persisted on the device.

**2. App credentials (unattended).**
Your own Entra ID app registration + client secret, for automation/CI/headless
use. Leave the fields blank to read them from environment variables:

```bash
export tenant_id="<your-tenant-guid-or-domain>"
export app_id="<your-app-client-id>"
export app_secret="<your-client-secret>"
```

These same variables let the headless HTTP sidecar authenticate (see
`scripts/smoke-live.sh` and the browser-dev section of the README).

> Tokens and secrets are handled only by the Rust core process — they never live
> in the web UI. Testing against a live tenant is read-only by default.

---

## Where the build artifacts land

After `npm run build`, artifacts are written under `app/target/release/`:

```
app/target/release/intune-manager                              # raw executable (Linux)
app/target/release/bundle/deb/Intune Manager_0.1.0_amd64.deb
app/target/release/bundle/rpm/Intune Manager-0.1.0-1.x86_64.rpm
app/target/release/bundle/appimage/Intune Manager_0.1.0_amd64.AppImage
app/target/release/bundle/macos/Intune Manager.app            # macOS host
app/target/release/bundle/dmg/Intune Manager_0.1.0_aarch64.dmg # macOS host
app/target/release/bundle/msi/Intune Manager_0.1.0_x64_en-US.msi   # Windows host
app/target/release/bundle/nsis/Intune Manager_0.1.0_x64-setup.exe  # Windows host
```

Exact filenames encode the version (`0.1.0`) and architecture (`amd64`/`x86_64`/
`aarch64`/`x64`). Commands below use shell globs so they keep working as the
version bumps.

---

## Linux — Debian / Ubuntu (`.deb`)

**Runtime dependencies:** `libwebkit2gtk-4.1-0`, `libgtk-3-0` (the `.deb`
declares these; `apt` resolves them automatically).

### (A) Install the prebuilt `.deb`

```bash
cd app/target/release/bundle/deb
sudo apt install ./Intune\ Manager_0.1.0_amd64.deb
# or, equivalently:
sudo dpkg -i ./Intune\ Manager_*.deb || sudo apt -f install
```

- **Installed binary:** `/usr/bin/intune-manager`
- **Desktop entry:** `/usr/share/applications/Intune Manager.desktop`
- **Launch:** search **“Intune Manager”** in your app menu, or run
  `intune-manager` from a terminal.
- **Uninstall:** `sudo apt remove intune-manager`

### (B) Build from source

```bash
cd app
bash scripts/setup.sh   # installs libwebkit2gtk-4.1-dev, build tools, Rust, npm deps
npm run build
sudo apt install ./target/release/bundle/deb/Intune\ Manager_*.deb
```

---

## Linux — universal `.AppImage`

A single self-contained file that bundles the WebKitGTK runtime, so it runs on
most distributions (including those that don't ship WebKitGTK 4.1) with no
system packages to install.

### (A) Run the prebuilt `.AppImage`

```bash
cd app/target/release/bundle/appimage
chmod +x "Intune Manager_0.1.0_amd64.AppImage"
./"Intune Manager_0.1.0_amd64.AppImage"
```

- **FUSE requirement:** AppImages need **FUSE 2** at runtime. If you see
  `dlopen(): error loading libfuse.so.2`, install it
  (`sudo apt install libfuse2` / `sudo pacman -S fuse2` / `sudo dnf install fuse`),
  **or** run without FUSE by extracting first:
  ```bash
  ./"Intune Manager_0.1.0_amd64.AppImage" --appimage-extract
  ./squashfs-root/AppRun
  ```
- **Desktop integration** (optional menu entry/icon): install
  [`appimaged`](https://github.com/probonopd/go-appimage) or
  [AppImageLauncher](https://github.com/TheAssassin/AppImageLauncher), or create
  a `.desktop` file by hand pointing `Exec=` at the AppImage.
- **Wayland/Hyprland:** prefix with `WEBKIT_DISABLE_DMABUF_RENDERER=1` (see the
  Arch/Omarchy section).

### (B) Build from source

```bash
cd app && npm run build
# → target/release/bundle/appimage/Intune Manager_0.1.0_amd64.AppImage
```

---

## Linux — Arch / Omarchy (Hyprland/Wayland)

**Runtime dependencies (pacman):**

```bash
sudo pacman -S --needed webkit2gtk-4.1 gtk3 librsvg libappindicator-gtk3
```

> ### ⚠️ Wayland / Hyprland rendering fix (required)
> On Hyprland/Omarchy (and some other Wayland compositors) WebKitGTK's DMA-BUF
> renderer produces a **blank/black window**. Disable it:
> ```bash
> export WEBKIT_DISABLE_DMABUF_RENDERER=1
> ```
> If the window is still blank, add the compositing fallback:
> ```bash
> export WEBKIT_DISABLE_COMPOSITING_MODE=1
> ```

### (A) Run the prebuilt bundle (recommended: `.AppImage`)

The **`.AppImage`** is the recommended portable artifact for Arch/Omarchy — no
system package install beyond the Wayland fix:

```bash
cd app/target/release/bundle/appimage
chmod +x "Intune Manager_0.1.0_amd64.AppImage"
WEBKIT_DISABLE_DMABUF_RENDERER=1 ./"Intune Manager_0.1.0_amd64.AppImage"
```

Or use the bundled launcher, which sets the Wayland variable for you and
auto-finds the built AppImage/binary:

```bash
cd app && npm run start:linux
```

> The `.deb`/`.rpm` bundles target Debian/Fedora and aren't meant for pacman.
> On Arch, prefer the `.AppImage`, or install the runtime libraries above and run
> the raw binary directly (`app/target/release/intune-manager`).

### (B) Build from source

```bash
cd app
bash scripts/setup.sh    # detects pacman: installs webkit2gtk-4.1, base-devel, nodejs/npm, …
npm run dev:linux        # dev window with the Wayland workaround pre-applied
# or a release build:
npm run build            # → deb + rpm + appimage
npm run start:linux      # launch the built AppImage with the Wayland fix
```

---

## Linux — Fedora / RHEL (`.rpm`, dnf)

`tauri build` **does emit an `.rpm`** (Tauri v2's RPM bundler is pure-Rust and
builds fine on any Linux host). The `.rpm` declares its runtime deps
(`webkit2gtk4.1`, `gtk3`), so `dnf` resolves them.

**Runtime dependencies (dnf):** `webkit2gtk4.1`, `gtk3` (plus
`librsvg2`, `libappindicator-gtk3` for icons/tray).

### (A) Install the prebuilt `.rpm`

```bash
cd app/target/release/bundle/rpm
sudo dnf install ./Intune\ Manager-0.1.0-1.x86_64.rpm
# or: sudo rpm -i ./Intune\ Manager-*.rpm   (dnf is preferred — it pulls deps)
```

- **Launch:** search **“Intune Manager”** in your app menu, or run
  `intune-manager`.
- **Uninstall:** `sudo dnf remove intune-manager`

> Prefer the universal **`.AppImage`** if you're not on the same major Fedora
> release the bundle was built on — it carries its own WebKitGTK and avoids
> glibc/WebKit version skew.

### (B) Build from source

Install the Fedora dev libraries (the official Tauri v2 Fedora prerequisites),
then build:

```bash
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel
sudo dnf group install "C Development Tools and Libraries"

cd app
npm install
npm run build   # → target/release/bundle/rpm/Intune Manager-0.1.0-1.x86_64.rpm
```

---

## macOS

Tauri produces a `.app` and a `.dmg` **when built on macOS** (there is no
prebuilt macOS bundle in this repo yet — build it from source as below).

### Prerequisites

- **Xcode Command Line Tools:** `xcode-select --install`
- **Rust** (rustup) and **Node ≥ 20** (see quickstart).

### (B) Build from source

```bash
cd app
npm install
npm run build
```

Outputs:

```
app/target/release/bundle/macos/Intune Manager.app
app/target/release/bundle/dmg/Intune Manager_0.1.0_<arch>.dmg
```

- **Apple Silicon vs Intel:** by default it builds for your Mac's own
  architecture. To target the other one explicitly, add the Rust target and pass
  `--target`:
  ```bash
  rustup target add aarch64-apple-darwin x86_64-apple-darwin
  npm run tauri build -- --target aarch64-apple-darwin   # Apple Silicon
  npm run tauri build -- --target x86_64-apple-darwin    # Intel
  ```
  (A `universal-apple-darwin` target can produce a fat binary.)

### (A) Install / launch the built app

Open the `.dmg` and drag **Intune Manager** into `/Applications`, or run the
`.app` directly.

- **Gatekeeper (unsigned app):** because the app isn't code-signed/notarized,
  macOS will warn "…cannot be opened because the developer cannot be verified."
  Either:
  - **Right-click** the app → **Open** → **Open** (only needed the first time), or
  - clear the quarantine attribute:
    ```bash
    xattr -dr com.apple.quarantine "/Applications/Intune Manager.app"
    ```

---

## Windows

Tauri produces an `.msi` (WiX) and an `.exe` (NSIS) installer **when built on
Windows** (build from source as below).

### Prerequisites

- **Rust with the MSVC toolchain** — install rustup and the default
  `x86_64-pc-windows-msvc` toolchain.
- **Microsoft C++ Build Tools** — "Desktop development with C++" workload from
  the [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/).
- **WebView2 Runtime** — preinstalled on Windows 11 and most Windows 10; if
  missing, install the
  [Evergreen WebView2 Runtime](https://developer.microsoft.com/microsoft-edge/webview2/).
- **Node ≥ 20**.

### (B) Build from source

```powershell
cd app
npm install
npm run build
```

Outputs:

```
app\target\release\bundle\msi\Intune Manager_0.1.0_x64_en-US.msi
app\target\release\bundle\nsis\Intune Manager_0.1.0_x64-setup.exe
```

### (A) Install / launch

Run either installer (double-click the `.msi`, or the `..._x64-setup.exe` NSIS
installer). After install, launch **Intune Manager** from the Start menu.

- **SmartScreen (unsigned installer):** Windows may show
  "Windows protected your PC". Click **More info → Run anyway** to proceed (this
  appears because the build isn't code-signed).
- **Uninstall:** via *Settings → Apps* (or the MSI's *Add/Remove Programs* entry).

---

## Troubleshooting quick reference

| Symptom | Fix |
| --- | --- |
| Blank/black window on Hyprland/Wayland | `export WEBKIT_DISABLE_DMABUF_RENDERER=1` (then `WEBKIT_DISABLE_COMPOSITING_MODE=1`). |
| `libfuse.so.2` error running AppImage | Install FUSE 2, or run with `--appimage-extract` + `./squashfs-root/AppRun`. |
| `webkit2gtk not found` when building | Install the dev libs for your distro (`libwebkit2gtk-4.1-dev` / `webkit2gtk4.1-devel` / `webkit2gtk-4.1`). |
| macOS "developer cannot be verified" | Right-click → Open, or `xattr -dr com.apple.quarantine <app>`. |
| Windows SmartScreen warning | **More info → Run anyway** (unsigned build). |
| Sign-in fails / `AADSTS700016` | Update to the current build (default client is the Graph CLI public client), or set a custom client id/tenant under **Advanced** on the sign-in screen. |
