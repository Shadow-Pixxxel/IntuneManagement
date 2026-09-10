#!/usr/bin/env bash
#
# Cloud Agent bootstrap for IntuneManagement.
#
# IntuneManagement is a Windows-only PowerShell + WPF GUI application. The GUI
# itself cannot run on the Linux Cloud Agent VM (WPF/WinForms, Win32 P/Invoke,
# Unblock-File and the HKCU registry settings are all Windows-only). What this
# script does provide is the cross-platform PowerShell development toolchain so
# the modules can be edited, parsed, and linted on Linux:
#   - PowerShell 7 (pwsh)
#   - PSScriptAnalyzer (static analysis / linting)
#
# The script is idempotent: it is safe to run repeatedly and skips work that is
# already done.
set -euo pipefail

PWSH_VERSION="7.4.6"

if ! command -v pwsh >/dev/null 2>&1; then
  echo "Installing PowerShell ${PWSH_VERSION}..."
  tmp_deb="$(mktemp --suffix=.deb)"
  curl -fsSL -o "$tmp_deb" \
    "https://github.com/PowerShell/PowerShell/releases/download/v${PWSH_VERSION}/powershell_${PWSH_VERSION}-1.deb_amd64.deb"
  sudo dpkg -i "$tmp_deb" || sudo apt-get install -f -y
  rm -f "$tmp_deb"
else
  echo "PowerShell already installed: $(pwsh --version)"
fi

echo "Ensuring PSScriptAnalyzer is available..."
pwsh -NoProfile -Command '
  if (-not (Get-Module -ListAvailable -Name PSScriptAnalyzer)) {
    Set-PSRepository -Name PSGallery -InstallationPolicy Trusted
    Install-Module -Name PSScriptAnalyzer -Scope CurrentUser -Force -AllowClobber
  }
  $v = (Get-Module -ListAvailable -Name PSScriptAnalyzer | Select-Object -First 1).Version
  Write-Host "PSScriptAnalyzer $v ready."
'

echo "Cloud Agent environment ready. PowerShell version:"
pwsh --version
