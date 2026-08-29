#!/usr/bin/env bash
# Runs *inside* the Apple container (mounted from the rig checkout).
set -euo pipefail

export RUSTUP_HOME=/opt/rustup
# Shared toolchain; per-user cargo home so `cargo install` can write.
export CARGO_HOME="${HOME}/.cargo"
export PATH="/opt/cargo/bin:${CARGO_HOME}/bin:${PATH}"
export RIG_ROOT=/home/dev/rig
cd "${RIG_ROOT}"

WITH_PACKAGES="${RIG_APPLE_WITH_PACKAGES:-0}"

echo "smoke"
echo "  step    toolchain"
rustc --version
cargo --version

echo "  step    cargo install rig"
cargo install --path crates/rig --force

echo "  step    init compute host"
hn="$(hostname -s | tr '[:upper:]' '[:lower:]')"
rm -f "hosts/${hn}.toml"
rig init --role compute --name "${hn}"
python3 - <<'PY'
from pathlib import Path
import socket
short = socket.gethostname().split(".")[0].lower()
path = Path("hosts") / f"{short}.toml"
text = path.read_text().splitlines()
out, seen_os, seen_shell = [], False, False
for ln in text:
    s = ln.strip()
    if s.startswith("os") and "=" in s:
        out.append('os = "linux"')
        seen_os = True
    elif s.startswith("shell") and "=" in s:
        out.append('shell = "bash"')
        seen_shell = True
    else:
        out.append(ln)
if not seen_os:
    out.insert(3, 'os = "linux"')
if not seen_shell:
    out.insert(4, 'shell = "bash"')
path.write_text("\n".join(out) + "\n")
print("updated", path)
PY
rig host detect

echo "  step    apply preview"
rig apply

echo "  step    apply --yes --skip-packages"
rig apply --yes --skip-packages

if [[ "${WITH_PACKAGES}" == "1" ]]; then
  echo "  step    apply --yes apt"
  rig apply --yes
fi

echo "  step    ssh-config"
rig ssh-config | head -20

echo "done"
