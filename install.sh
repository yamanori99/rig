#!/bin/sh
# Install the `rig` CLI from this clone, or clone the public repo first.
set -eu

REPO_URL="${RIG_REPO_URL:-https://github.com/yamanori99/rig.git}"
ROOT=""

die() {
  echo "error: $*" >&2
  exit 1
}

have_cargo() {
  command -v cargo >/dev/null 2>&1
}

is_rig_root() {
  [ -f "$1/crates/rig/Cargo.toml" ] && [ -f "$1/Cargo.toml" ]
}

find_root() {
  if [ -n "${RIG_ROOT:-}" ] && is_rig_root "$RIG_ROOT"; then
    ROOT=$RIG_ROOT
    return
  fi
  here=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
  if is_rig_root "$here"; then
    ROOT=$here
    return
  fi
  if is_rig_root "$PWD"; then
    ROOT=$PWD
    return
  fi
  ROOT=""
}

install_from() {
  root=$1
  echo "rig install"
  echo "  root=$root"
  have_cargo || die "cargo not found — install Rust from https://rustup.rs/"
  cargo install --path "$root/crates/rig" --force
  echo
  echo "next:"
  echo "  cd $root"
  echo "  rig init --role workstation   # or compute"
  echo "  rig apply --dry-run"
  echo "  rig apply --yes"
}

clone_then_install() {
  dest=${RIG_CLONE_DIR:-$HOME/rig}
  if is_rig_root "$dest"; then
    echo "using existing clone: $dest"
  else
    command -v git >/dev/null 2>&1 || die "git not found"
    if [ -e "$dest" ]; then
      die "refusing to overwrite: $dest (set RIG_CLONE_DIR)"
    fi
    echo "cloning $REPO_URL → $dest"
    git clone --depth 1 "$REPO_URL" "$dest"
  fi
  install_from "$dest"
}

find_root
if [ -n "$ROOT" ]; then
  install_from "$ROOT"
else
  clone_then_install
fi
