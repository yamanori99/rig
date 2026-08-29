#!/usr/bin/env bash
# Runs inside Apple container: install release binary (no Rust), then smoke apply.
set -euo pipefail

export PATH="${HOME}/.local/bin:/usr/local/bin:/usr/bin:/bin:${PATH:-}"

echo "smoke  release"
echo "  step    install.sh"
curl -fsSL https://raw.githubusercontent.com/yamanori99/rig/main/install.sh | sh
hash -r
command -v rig
rig --version

# Ensure we are not accidentally pointed at a checkout.
unset RIG_ROOT || true

echo "  step    init compute"
hn="$(hostname -s | tr '[:upper:]' '[:lower:]')"
# product hosts dir is under embedded data root
rig init --role compute --name "${hn}" || true
# If init said already exists, continue
rig host detect

echo "  step    apply preview"
rig apply

echo "  step    apply --yes --skip-packages"
rig apply --yes --skip-packages

echo "  step    roles"
rig roles compute >/tmp/rig-roles.out
head -25 /tmp/rig-roles.out

echo "done"
