#!/usr/bin/env bash
# Start one Linux smoke host with Apple `container` for rig.
# Does NOT mutate the Mac host — verify apply/apt/ssh inside the VM.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
RIG_ROOT="$(cd "${ROOT}/../.." && pwd)"
LOCAL_IMAGE="${RIG_APPLE_IMAGE:-local/rig-linux-smoke:latest}"
NAME="${RIG_APPLE_NAME:-rig-smoke}"
RUN_SMOKE=0
WITH_PACKAGES=0
FROM_GITHUB=0
FROM_RELEASE=0
GIT_URL="${RIG_GIT_URL:-https://github.com/yamanori99/rig.git}"

for arg in "$@"; do
  case "$arg" in
    --smoke) RUN_SMOKE=1 ;;
    --with-packages) WITH_PACKAGES=1 ;;
    --from-github)
      FROM_GITHUB=1
      ;;
    --from-github=*)
      FROM_GITHUB=1
      GIT_URL="${arg#--from-github=}"
      ;;
    --from-release)
      FROM_RELEASE=1
      ;;
    -h|--help)
      echo "usage: $0 [--smoke] [--with-packages] [--from-github[=URL]] [--from-release]"
      echo "  macOS 26+ Apple silicon + Apple container CLI"
      echo "  default: bind-mount ${RIG_ROOT} → /home/dev/rig (fast local loop)"
      echo "  --from-github     clone from GitHub inside the guest (release gate)"
      echo "                    default URL: ${GIT_URL}"
      echo "  --from-release    curl install.sh | sh (prebuilt binary; no Rust)"
      echo "  --smoke           run smoke inside guest"
      echo "  --with-packages   also run apply --yes (apt; slower; bind-mount/github modes)"
      exit 0
      ;;
    *)
      echo "unknown option: $arg" >&2
      exit 1
      ;;
  esac
done

if [[ "${FROM_GITHUB}" -eq 1 && "${FROM_RELEASE}" -eq 1 ]]; then
  echo "use only one of --from-github / --from-release" >&2
  exit 1
fi

if ! command -v container >/dev/null 2>&1; then
  echo "container CLI not found. Install https://github.com/apple/container" >&2
  exit 1
fi
if [[ "$(uname -s)" != "Darwin" ]] || [[ "$(uname -m)" != "arm64" ]]; then
  echo "testenv/apple-container is macOS Apple silicon only" >&2
  exit 1
fi
if ! command -v python3 >/dev/null 2>&1; then
  echo "python3 is required to read container inspect JSON" >&2
  exit 1
fi

container_ipv4() {
  local name="$1"
  container inspect "${name}" | python3 -c '
import json, sys
data = json.load(sys.stdin)
nets = data[0].get("status", {}).get("networks") or []
if not nets:
    sys.exit(1)
addr = nets[0].get("ipv4Address") or ""
print(addr.split("/")[0])
'
}

write_ssh_config() {
  local ip="$1"
  local gen="${ROOT}/.generated"
  local ctrl="${gen}/controller"
  local kh="${gen}/known_hosts"
  mkdir -p "${gen}"
  umask 077
  cat > "${gen}/ssh_config" <<EOF
Host ${NAME}
  HostName ${ip}
  User dev
  Port 22
  IdentityFile ${ctrl}
  IdentitiesOnly yes
  BatchMode yes
  ConnectTimeout 10
  StrictHostKeyChecking accept-new
  UserKnownHostsFile ${kh}
  ServerAliveInterval 60
  ServerAliveCountMax 10
  TCPKeepAlive yes
EOF
}

echo "Starting container system (no-op if already up)..."
container system start

"${ROOT}/scripts/gen-keys.sh"

echo "Building ${LOCAL_IMAGE}..."
(cd "${ROOT}" && container build -t "${LOCAL_IMAGE}" .)

"${ROOT}/scripts/down.sh"

MOUNT_KEYS="type=bind,source=${ROOT}/mounted-keys,target=/mounted-keys,readonly"
WORKER_CPUS="${RIG_APPLE_CPUS:-2}"
WORKER_MEMORY="${RIG_APPLE_MEMORY:-4096M}"

CREATE_ARGS=(
  create -d --name "${NAME}" --network default
  -c "${WORKER_CPUS}" -m "${WORKER_MEMORY}"
  -u root
  --mount "${MOUNT_KEYS}"
)

if [[ "${FROM_GITHUB}" -eq 0 && "${FROM_RELEASE}" -eq 0 ]]; then
  echo "Mode: bind-mount ${RIG_ROOT} → /home/dev/rig"
  CREATE_ARGS+=(--mount "type=bind,source=${RIG_ROOT},target=/home/dev/rig")
elif [[ "${FROM_RELEASE}" -eq 1 ]]; then
  echo "Mode: guest installs release binary via install.sh (no checkout mount)"
else
  echo "Mode: guest will git clone ${GIT_URL}"
fi

container "${CREATE_ARGS[@]}" "${LOCAL_IMAGE}"
container start "${NAME}"

echo "Waiting for ${NAME} IPv4..."
IP=""
for ((i = 1; i <= 30; i++)); do
  IP="$(container_ipv4 "${NAME}" 2>/dev/null || true)"
  if [[ -n "${IP}" ]]; then
    break
  fi
  sleep 1
done
if [[ -z "${IP}" ]]; then
  echo "no IPv4 yet; container ls:" >&2
  container ls >&2
  exit 1
fi

write_ssh_config "${IP}"
CFG="${ROOT}/.generated/ssh_config"
echo "${NAME} → ${IP}:22"
echo "SSH: ssh -F ${CFG} ${NAME}"

echo "Waiting for sshd..."
for ((i = 1; i <= 30; i++)); do
  if ssh -F "${CFG}" "${NAME}" "echo ok" >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
ssh -F "${CFG}" "${NAME}" "echo ok" >/dev/null

if [[ "${RUN_SMOKE}" -eq 1 ]]; then
  export RIG_APPLE_WITH_PACKAGES="${WITH_PACKAGES}"
  export RIG_APPLE_FROM_GITHUB="${FROM_GITHUB}"
  export RIG_APPLE_FROM_RELEASE="${FROM_RELEASE}"
  export RIG_GIT_URL="${GIT_URL}"
  "${ROOT}/scripts/smoke.sh"
fi
