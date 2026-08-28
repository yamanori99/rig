#!/usr/bin/env bash
# Stop/remove all Apple-container nodes listed in inventory.toml.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"

if ! command -v container >/dev/null 2>&1; then
  echo "container CLI not found (Apple container)" >&2
  exit 1
fi

INV="${ROOT}/scripts/lib/inventory.py"
while IFS= read -r name; do
  [[ -n "${name}" ]] || continue
  container stop "${name}" >/dev/null 2>&1 || true
  container rm "${name}" >/dev/null 2>&1 || true
  echo "Removed ${name}"
done < <(python3 "${INV}" names)
