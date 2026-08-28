#!/usr/bin/env bash
# Run rig smoke inside the Apple container (via SSH).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CFG="${ROOT}/.generated/ssh_config"
NAME="${RIG_APPLE_NAME:-rig-smoke}"
WITH_PACKAGES="${RIG_APPLE_WITH_PACKAGES:-0}"
FROM_GITHUB="${RIG_APPLE_FROM_GITHUB:-0}"
FROM_RELEASE="${RIG_APPLE_FROM_RELEASE:-0}"
GIT_URL="${RIG_GIT_URL:-https://github.com/yamanori99/rig.git}"

if [[ ! -f "${CFG}" ]]; then
  echo "missing ${CFG}; run scripts/up.sh first" >&2
  exit 1
fi

if [[ "${FROM_RELEASE}" == "1" ]]; then
  echo "Seeding release-binary smoke..."
  chmod +x "${ROOT}/scripts/smoke-release-guest.sh"
  ssh -F "${CFG}" "${NAME}" \
    env PATH="/home/dev/.local/bin:/usr/local/bin:/usr/bin:/bin" \
        bash -s <"${ROOT}/scripts/smoke-release-guest.sh"
  exit 0
fi

# When cloning from GitHub, seed only the guest smoke script via stdin,
# then let it clone the public tree and continue.
if [[ "${FROM_GITHUB}" == "1" ]]; then
  echo "Seeding guest smoke (clone ${GIT_URL})..."
  ssh -F "${CFG}" "${NAME}" \
    env RUSTUP_HOME=/opt/rustup \
        CARGO_HOME=/home/dev/.cargo \
        PATH="/opt/cargo/bin:/home/dev/.cargo/bin:/usr/local/bin:/usr/bin:/bin" \
        RIG_APPLE_WITH_PACKAGES="${WITH_PACKAGES}" \
        RIG_GIT_URL="${GIT_URL}" \
        bash -s <<'EOS'
set -euo pipefail
export RUSTUP_HOME=/opt/rustup
export CARGO_HOME="${HOME}/.cargo"
export PATH="/opt/cargo/bin:${CARGO_HOME}/bin:${PATH}"
URL="${RIG_GIT_URL:?}"
rm -rf /home/dev/rig
echo "== smoke: git clone ${URL} =="
git clone --depth 1 "${URL}" /home/dev/rig
chmod +x /home/dev/rig/testenv/apple-container/scripts/smoke-guest.sh
exec bash /home/dev/rig/testenv/apple-container/scripts/smoke-guest.sh
EOS
  exit 0
fi

chmod +x "${ROOT}/scripts/smoke-guest.sh"

ssh -F "${CFG}" "${NAME}" \
  env RUSTUP_HOME=/opt/rustup \
      CARGO_HOME=/home/dev/.cargo \
      PATH="/opt/cargo/bin:/home/dev/.cargo/bin:/usr/local/bin:/usr/bin:/bin" \
      RIG_APPLE_WITH_PACKAGES="${WITH_PACKAGES}" \
      bash /home/dev/rig/testenv/apple-container/scripts/smoke-guest.sh
