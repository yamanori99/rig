#!/usr/bin/env bash
# Stop/remove rig Apple-container smoke host.
set -euo pipefail

if ! command -v container >/dev/null 2>&1; then
  echo "container CLI not found (Apple container)" >&2
  exit 1
fi

NAME="${RIG_APPLE_NAME:-rig-smoke}"
container stop "${NAME}" >/dev/null 2>&1 || true
container rm "${NAME}" >/dev/null 2>&1 || true
echo "Removed ${NAME}"
