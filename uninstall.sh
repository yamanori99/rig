#!/bin/sh
# Remove the `rig` CLI installed by install.sh (no Rust required).
# Default: delete the binary only.
# Purge product data + state: RIG_PURGE=1 on the pipe side.
set -eu

REPO="${RIG_REPO:-yamanori99/rig}"

die() {
  echo "error: $*" >&2
  exit 1
}

bin_dir="${RIG_BIN_DIR:-$HOME/.local/bin}"
bin="$bin_dir/rig"

data_dir() {
  case "$(uname -s)" in
    Darwin)
      echo "$HOME/Library/Application Support/dev.rig.rig"
      ;;
    Linux)
      echo "${XDG_DATA_HOME:-$HOME/.local/share}/rig"
      ;;
    *)
      die "unsupported OS: $(uname -s)"
      ;;
  esac
}

echo "rig uninstall"

removed=0
if [ -e "$bin" ] || [ -L "$bin" ]; then
  rm -f "$bin"
  echo "removed: $bin"
  removed=1
else
  echo "not found: $bin"
fi

if [ "${RIG_PURGE:-}" = "1" ]; then
  data=$(data_dir)
  if [ -e "$data" ]; then
    rm -rf "$data"
    echo "removed: $data"
    removed=1
  else
    echo "not found: $data"
  fi
else
  echo "note: product data kept (hosts / overlay / state)."
  echo "  purge: curl -fsSL https://raw.githubusercontent.com/${REPO}/main/uninstall.sh | RIG_PURGE=1 sh"
fi

# cargo install path (maintainer / RIG_FORCE_SOURCE) — tip only
cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin/rig"
if [ -e "$cargo_bin" ] || [ -L "$cargo_bin" ]; then
  echo "note: also found $cargo_bin (cargo). remove with: cargo uninstall rig"
fi

if [ "$removed" -eq 0 ] && [ "${RIG_PURGE:-}" != "1" ]; then
  echo "nothing removed."
  exit 1
fi

echo
echo "done."
echo "note: shell snippets / packages from \`rig apply\` are not undone."
echo "  use \`rig clean\` before uninstall if you still have the binary."
