#!/usr/bin/env bash
# Runs inside Apple container: install release binary (no Rust), then smoke apply.
set -euo pipefail

export PATH="${HOME}/.local/bin:/usr/local/bin:/usr/bin:/bin:${PATH:-}"

echo "== release-smoke: install.sh (GitHub Release binary) =="
curl -fsSL https://raw.githubusercontent.com/yamanori99/rig/main/install.sh | sh
hash -r
command -v rig
rig --version

# Ensure we are not accidentally pointed at a checkout.
unset RIG_ROOT || true

echo "== release-smoke: init compute =="
hn="$(hostname -s | tr '[:upper:]' '[:lower:]')"
# product hosts dir is under embedded data root
rig init --role compute --name "${hn}" || true
# If init said already exists, continue
rig host detect

echo "== release-smoke: apply --dry-run =="
rig apply --dry-run

echo "== release-smoke: apply --yes --skip-packages =="
rig apply --yes --skip-packages

echo "== release-smoke: roles (embedded) =="
rig roles compute | head -25

echo "OK — apple-container release binary smoke passed"
