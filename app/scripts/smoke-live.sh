#!/usr/bin/env bash
# Read-only live smoke test that exercises the app's OWN backend (the axum
# sidecar from crates/server) end-to-end against a real tenant.
#
# It signs in app-only using the tenant_id / app_id / app_secret environment
# variables, then hits the real Graph pipeline to:
#   (a) resolve the organisation (tenant) name,
#   (b) list a couple of object types,
#   (c) export one object type to a temp directory.
#
# READ-ONLY: this script only performs GET requests, an app-only login, and an
# export to a LOCAL temp dir. It never calls import/copy/create endpoints and
# never sets INTUNE_ALLOW_WRITES, so it cannot mutate the tenant.
#
# A redacted request/response transcript is written to the path given by
# SMOKE_LOG (default: /opt/cursor/artifacts/increment3-live-smoke.log).
set -euo pipefail

APP_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$APP_DIR"

PORT="${INTUNE_SERVER_PORT:-8799}"
BASE="http://127.0.0.1:${PORT}/api"
SMOKE_LOG="${SMOKE_LOG:-/opt/cursor/artifacts/increment3-live-smoke.log}"
EXPORT_DIR="$(mktemp -d /tmp/intune-smoke-export.XXXXXX)"

# Refuse to run with writes enabled — this test must stay read-only.
if [ -n "${INTUNE_ALLOW_WRITES:-}" ]; then
  echo "Refusing to run: INTUNE_ALLOW_WRITES is set (this smoke test is read-only)." >&2
  exit 1
fi

for v in tenant_id app_id app_secret; do
  if [ -z "${!v:-}" ]; then
    echo "Missing required env var: $v (need tenant_id/app_id/app_secret for app-only login)." >&2
    exit 1
  fi
done

mkdir -p "$(dirname "$SMOKE_LOG")"
: > "$SMOKE_LOG"

# Redact secrets/ids from any text before it is written to the transcript.
# NOTE: the redactor lives in its own file so that piped stdin is the DATA to
# redact (not the Python program itself).
REDACTOR="$(mktemp /tmp/intune-smoke-redact.XXXXXX.py)"
cat >"$REDACTOR" <<'PY'
import re, sys
s = sys.stdin.read()
# Full GUIDs -> keep first block, mask the rest (tenant/app/object ids).
s = re.sub(r'(?<![0-9a-fA-F])([0-9a-fA-F]{8})-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}(?![0-9a-fA-F])',
           r'\1-****-****-****-************', s)
# Any token-ish key/value pairs.
s = re.sub(r'("?(?:access_token|refresh_token|id_token|token|secret|authorization)"?\s*[:=]\s*)"?[^",\s}]+"?',
           r'\1"***REDACTED***"', s, flags=re.I)
sys.stdout.write(s)
PY
redact() { python3 "$REDACTOR"; }
SERVER_PID=""
cleanup() {
  [ -n "$SERVER_PID" ] && kill "$SERVER_PID" 2>/dev/null || true
  rm -f "$REDACTOR" 2>/dev/null || true
}
trap cleanup EXIT

log()  { printf '%s\n' "$*" | tee -a "$SMOKE_LOG" >/dev/null; }
logr() { redact | tee -a "$SMOKE_LOG" >/dev/null; }

log "=== Intune Manager — live read-only smoke test (through the app backend) ==="
log "date        : $(date -u +%Y-%m-%dT%H:%M:%SZ)"
log "backend     : crates/server (axum sidecar) on $BASE"
log "export dir  : $EXPORT_DIR"
log "mode        : app-only (env tenant_id/app_id/app_secret); GETs + export only"
log ""

# ---- Boot the sidecar ------------------------------------------------------
log "[build] cargo build -p intune_server"
cargo build -p intune_server >>"$SMOKE_LOG" 2>&1

log "[boot ] starting intune-server on port $PORT"
INTUNE_SERVER_PORT="$PORT" RUST_LOG="${RUST_LOG:-warn}" ./target/debug/intune-server \
  >/tmp/intune-smoke-server.log 2>&1 &
SERVER_PID=$!

for _ in $(seq 1 30); do
  if curl -fsS "$BASE/health" >/dev/null 2>&1; then break; fi
  sleep 0.5
done
if ! curl -fsS "$BASE/health" >/dev/null 2>&1; then
  log "ERROR: server did not become healthy"; cat /tmp/intune-smoke-server.log | logr; exit 1
fi
log "[boot ] health OK"
log ""

# Small helper: run a call, echo a titled, redacted transcript entry.
call() { # METHOD PATH [JSON_BODY]
  local method="$1" path="$2" body="${3:-}"
  [ -z "$body" ] && body='{}'
  log "----------------------------------------------------------------------"
  log "> $method $path  body=$(printf '%s' "$body" | redact)"
  local out
  if [ "$method" = "GET" ]; then
    out="$(curl -fsS "$BASE$path")"
  else
    out="$(curl -fsS -XPOST "$BASE$path" -H 'Content-Type: application/json' -d "$body")"
  fi
  printf '%s' "$out"
}

# ---- (a) app-only login + resolve org name --------------------------------
log "### Step 1 — app-only sign in (env credentials)"
call POST /auth/app-only '{}' | logr
log ""

log "### Step 2 — resolve organisation / tenant name"
STATUS_JSON="$(curl -fsS "$BASE/status")"
printf '%s' "$STATUS_JSON" | redact | tee -a "$SMOKE_LOG" >/dev/null
ORG="$(printf '%s' "$STATUS_JSON" | python3 -c 'import sys,json;print(json.load(sys.stdin).get("orgName") or "")')"
AUTHED="$(printf '%s' "$STATUS_JSON" | python3 -c 'import sys,json;print(json.load(sys.stdin).get("authenticated"))')"
log ""
log "resolved authenticated=$AUTHED orgName=\"$ORG\""
log ""
if [ "$AUTHED" != "True" ]; then
  log "ERROR: not authenticated after app-only login"; exit 1
fi

# ---- (b) list a couple of object types ------------------------------------
log "### Step 3 — list a couple of object types"
CATALOG_JSON="$(curl -fsS "$BASE/catalog")"
read -r T1 T2 <<<"$(printf '%s' "$CATALOG_JSON" | python3 -c '
import sys,json
c=json.load(sys.stdin)
ids=[t["id"] for t in c]
# Prefer typically-small, universally-present types for a quick, safe listing.
pref=[x for x in ["EnrollmentRestrictions","NamedLocations","CompliancePolicies","SettingsCatalog","DeviceConfiguration"] if x in ids]
pick=(pref+ids)[:2]
print(" ".join(pick))
')"
log "chosen types: $T1, $T2"
for T in "$T1" "$T2"; do
  CNT="$(call GET "/objects/$T" | python3 -c 'import sys,json;d=json.load(sys.stdin);print(d.get("count"))')"
  log "  $T -> count=$CNT"
done
log ""

# ---- (c) export one object type to a temp dir -----------------------------
log "### Step 4 — export one object type to a temp dir (local disk only)"
EXPORT_JSON="$(call POST /export "{\"typeId\":\"$T1\",\"ids\":null,\"outDir\":\"$EXPORT_DIR\"}")"
printf '%s' "$EXPORT_JSON" | redact | tee -a "$SMOKE_LOG" >/dev/null
NFILES="$(printf '%s' "$EXPORT_JSON" | python3 -c 'import sys,json;print(len(json.load(sys.stdin).get("files",[])))')"
log ""
log "exported $NFILES file(s) for type $T1 into $EXPORT_DIR"
log "on-disk listing:"
( cd "$EXPORT_DIR" && find . -maxdepth 3 -type f | sort | head -20 ) | sed 's/^/    /' | tee -a "$SMOKE_LOG" >/dev/null
log ""
log "=== SMOKE TEST PASSED (read-only) ==="
log "org=\"$ORG\"  listed=[$T1,$T2]  exported=${NFILES} file(s)"

echo "Transcript written to: $SMOKE_LOG"
