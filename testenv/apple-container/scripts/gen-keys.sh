#!/usr/bin/env bash
# Generate controller + client keys for Apple container fleet / smoke.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
KEYS="${ROOT}/mounted-keys"
GEN="${ROOT}/.generated"
CONTROLLER_KEY="${GEN}/controller"
CLIENT_KEY="${GEN}/client"

mkdir -p "${KEYS}" "${GEN}"

if [[ ! -f "${CONTROLLER_KEY}" ]]; then
  ssh-keygen -t ed25519 -f "${CONTROLLER_KEY}" -N "" -C "rig-apple-container-controller"
fi
cp "${CONTROLLER_KEY}.pub" "${KEYS}/controller.pub"

if [[ ! -f "${CLIENT_KEY}" ]]; then
  ssh-keygen -t ed25519 -f "${CLIENT_KEY}" -N "" -C "rig-apple-container-client"
fi
cp "${CLIENT_KEY}.pub" "${KEYS}/client.pub"

echo "Controller pub → ${KEYS}/controller.pub"
echo "Client pub     → ${KEYS}/client.pub"
