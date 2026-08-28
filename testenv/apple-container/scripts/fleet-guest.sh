#!/usr/bin/env bash
# Runs *inside* one fleet guest: install, init, apply for this node's role.
set -euo pipefail

export RUSTUP_HOME=/opt/rustup
export CARGO_HOME="${HOME}/.cargo"
export PATH="/opt/cargo/bin:${CARGO_HOME}/bin:${PATH}"
export RIG_ROOT=/home/dev/rig
cd "${RIG_ROOT}"

NAME="${RIG_FLEET_NAME:?RIG_FLEET_NAME required}"
ROLE="${RIG_FLEET_ROLE:?RIG_FLEET_ROLE required}"
WITH_PACKAGES="${RIG_APPLE_WITH_PACKAGES:-0}"

echo "== ${NAME}: toolchain =="
rustc --version
cargo --version

echo "== ${NAME}: cargo install rig =="
cargo install --path crates/rig --force

echo "== ${NAME}: write hosts/${NAME}.toml (role=${ROLE}) =="
mkdir -p hosts
cat > "hosts/${NAME}.toml" <<EOF
name = "${NAME}"
role = "${ROLE}"
schema_version = 1
os = "linux"
shell = "bash"
user = "dev"
EOF
rig host detect

echo "== ${NAME}: apply --dry-run =="
rig apply --dry-run

echo "== ${NAME}: apply --yes --skip-packages =="
rig apply --yes --skip-packages

if [[ "${WITH_PACKAGES}" == "1" ]]; then
  echo "== ${NAME}: apply --yes (apt) =="
  rig apply --yes
fi

echo "== ${NAME}: guest apply done =="
