#!/usr/bin/env bash
# Orchestrate fleet smoke across inventory nodes (Mac side).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CFG="${ROOT}/.generated/ssh_config"
NODES="${ROOT}/.generated/nodes.json"
INV="${ROOT}/scripts/lib/inventory.py"
WITH_PACKAGES="${RIG_APPLE_WITH_PACKAGES:-0}"
FROM_GITHUB="${RIG_APPLE_FROM_GITHUB:-0}"
GIT_URL="${RIG_GIT_URL:-https://github.com/yamanori99/rig.git}"

if [[ ! -f "${CFG}" || ! -f "${NODES}" ]]; then
  echo "missing ${CFG} or ${NODES}; run scripts/fleet-up.sh first" >&2
  exit 1
fi

# Snapshot inventory so nested `ssh` cannot consume the loop stdin.
mapfile -t FLEET_LINES < <(python3 "${INV}" tsv)

seed_or_run_guest() {
  local name="$1" role="$2"
  if [[ "${FROM_GITHUB}" == "1" ]]; then
    echo "== ${name}: clone ${GIT_URL} + fleet-guest =="
    ssh -F "${CFG}" "${name}" \
      env RUSTUP_HOME=/opt/rustup \
          CARGO_HOME=/home/dev/.cargo \
          PATH="/opt/cargo/bin:/home/dev/.cargo/bin:/usr/local/bin:/usr/bin:/bin" \
          RIG_APPLE_WITH_PACKAGES="${WITH_PACKAGES}" \
          RIG_FLEET_NAME="${name}" \
          RIG_FLEET_ROLE="${role}" \
          RIG_GIT_URL="${GIT_URL}" \
          bash -s <<'EOS'
set -euo pipefail
export RUSTUP_HOME=/opt/rustup
export CARGO_HOME="${HOME}/.cargo"
export PATH="/opt/cargo/bin:${CARGO_HOME}/bin:${PATH}"
URL="${RIG_GIT_URL:?}"
rm -rf /home/dev/rig
git clone --depth 1 "${URL}" /home/dev/rig
chmod +x /home/dev/rig/testenv/apple-container/scripts/fleet-guest.sh
exec bash /home/dev/rig/testenv/apple-container/scripts/fleet-guest.sh
EOS
  else
    echo "== ${name}: fleet-guest (bind-mount) =="
    chmod +x "${ROOT}/scripts/fleet-guest.sh"
    ssh -n -F "${CFG}" "${name}" \
      env RUSTUP_HOME=/opt/rustup \
          CARGO_HOME=/home/dev/.cargo \
          PATH="/opt/cargo/bin:/home/dev/.cargo/bin:/usr/local/bin:/usr/bin:/bin" \
          RIG_APPLE_WITH_PACKAGES="${WITH_PACKAGES}" \
          RIG_FLEET_NAME="${name}" \
          RIG_FLEET_ROLE="${role}" \
          bash /home/dev/rig/testenv/apple-container/scripts/fleet-guest.sh
  fi
}

echo "== fleet-smoke: apply on every node =="
for line in "${FLEET_LINES[@]}"; do
  [[ -n "${line}" ]] || continue
  IFS=$'\t' read -r name role <<<"${line}"
  seed_or_run_guest "${name}" "${role}"
done

echo "== fleet-smoke: install peer hosts on workstations =="
PEERS_B64="$(base64 < "${NODES}" | tr -d '\n')"
for line in "${FLEET_LINES[@]}"; do
  [[ -n "${line}" ]] || continue
  IFS=$'\t' read -r name role <<<"${line}"
  [[ "${role}" == "workstation" ]] || continue
  echo "  peers → ${name}"
  ssh -F "${CFG}" "${name}" \
    env PEERS_B64="${PEERS_B64}" RIG_FLEET_NAME="${name}" \
    bash -s <<'EOS'
set -euo pipefail
export RUSTUP_HOME=/opt/rustup
export CARGO_HOME="${HOME}/.cargo"
export PATH="/opt/cargo/bin:${CARGO_HOME}/bin:${PATH}"
export RIG_ROOT=/home/dev/rig
cd "${RIG_ROOT}"
python3 - <<'PY'
import base64, json, os
from pathlib import Path

me = os.environ["RIG_FLEET_NAME"]
nodes = json.loads(base64.b64decode(os.environ["PEERS_B64"]))
hosts = Path("hosts")
hosts.mkdir(exist_ok=True)
for n in nodes:
    if n["name"] == me:
        continue
    path = hosts / f'{n["name"]}.toml'
    path.write_text(
        "\n".join(
            [
                f'name = "{n["name"]}"',
                f'role = "{n["role"]}"',
                "schema_version = 1",
                'os = "linux"',
                'shell = "bash"',
                f'user = "{n.get("user", "dev")}"',
                f'lan = "{n["ip"]}"',
                "",
            ]
        )
    )
    print("wrote", path)
PY
rig ssh-config --write
echo "--- ssh-config ---"
rig ssh-config | head -40
EOS
done

echo "== fleet-smoke: verify workstation → peers =="
for line in "${FLEET_LINES[@]}"; do
  [[ -n "${line}" ]] || continue
  IFS=$'\t' read -r name role <<<"${line}"
  [[ "${role}" == "workstation" ]] || continue
  for peer_line in "${FLEET_LINES[@]}"; do
    [[ -n "${peer_line}" ]] || continue
    IFS=$'\t' read -r peer_name peer_role <<<"${peer_line}"
    [[ "${peer_name}" == "${name}" ]] && continue
    alias="${peer_name}-lan"
    echo "  ${name} → ${alias}"
    ssh -n -F "${CFG}" "${name}" \
      ssh -o BatchMode=yes -o StrictHostKeyChecking=accept-new \
          -o ConnectTimeout=10 "${alias}" "echo ok-from-${peer_name}"
  done
done

echo "OK — apple-container fleet smoke passed"
