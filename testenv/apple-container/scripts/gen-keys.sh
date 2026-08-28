#!/usr/bin/env bash
# Generate controller key + placeholder ssh_config for Apple container smoke.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
KEYS="${ROOT}/mounted-keys"
GEN="${ROOT}/.generated"
CONTROLLER_KEY="${GEN}/controller"

mkdir -p "${KEYS}" "${GEN}"

if [[ ! -f "${CONTROLLER_KEY}" ]]; then
  ssh-keygen -t ed25519 -f "${CONTROLLER_KEY}" -N "" -C "rig-apple-container-controller"
fi
cp "${CONTROLLER_KEY}.pub" "${KEYS}/controller.pub"

echo "Controller pub → ${KEYS}/controller.pub"
