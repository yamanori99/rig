#!/bin/sh
# Install the `rig` CLI from a GitHub Release binary (no Rust required).
# Maintainers: RIG_FORCE_SOURCE=1 ./install.sh  → clone + cargo
set -eu

REPO="${RIG_REPO:-yamanori99/rig}"
REPO_URL="${RIG_REPO_URL:-https://github.com/${REPO}.git}"
DEST="${RIG_CLONE_DIR:-$HOME/rig}"

die() {
  echo "error: $*" >&2
  exit 1
}

have() {
  command -v "$1" >/dev/null 2>&1
}

is_rig_root() {
  [ -f "$1/crates/rig/Cargo.toml" ] && [ -f "$1/Cargo.toml" ]
}

detect_target() {
  os=$(uname -s | tr '[:upper:]' '[:lower:]')
  arch=$(uname -m)
  case "$os" in
    darwin) os=apple-darwin ;;
    linux) os=unknown-linux-gnu ;;
    *) die "unsupported OS: $(uname -s)" ;;
  esac
  case "$arch" in
    x86_64|amd64) arch=x86_64 ;;
    arm64|aarch64) arch=aarch64 ;;
    *) die "unsupported arch: $(uname -m)" ;;
  esac
  echo "${arch}-${os}"
}

install_bin_to_path() {
  bin=$1
  dir="${RIG_BIN_DIR:-$HOME/.local/bin}"
  mkdir -p "$dir"
  install -m 755 "$bin" "$dir/rig"
  echo "installed: $dir/rig"
  export PATH="$dir:$PATH"
  persist_path "$dir"
}

# Login profile on macOS, interactive rc on Linux — one file per shell.
path_rc_file() {
  name=$(basename "${SHELL:-}")
  os=$(uname -s)
  case "$name" in
    zsh)
      if [ "$os" = Darwin ]; then
        echo "$HOME/.zprofile"
      else
        echo "$HOME/.zshrc"
      fi
      ;;
    bash)
      if [ "$os" = Darwin ]; then
        echo "$HOME/.bash_profile"
      else
        echo "$HOME/.bashrc"
      fi
      ;;
    *)
      if [ "$os" = Darwin ]; then
        echo "$HOME/.zprofile"
      else
        echo "$HOME/.bashrc"
      fi
      ;;
  esac
}

persist_path() {
  dir=$1
  rc=$(path_rc_file)
  line="export PATH=\"$dir:\$PATH\"  # rig PATH"
  if grep -Fqs "$dir" "$rc" 2>/dev/null; then
    echo "PATH already in $rc"
    return 0
  fi
  touch "$rc"
  printf '\n%s\n' "$line" >> "$rc"
  echo "wrote PATH into $rc"
  echo "new terminal, or: exec \$SHELL"
}

install_from_release() {
  have curl || die "curl is required"
  have tar || die "tar is required"
  target=$(detect_target)
  tag=${RIG_VERSION:-latest}
  if [ "$tag" = "latest" ]; then
    api="https://api.github.com/repos/${REPO}/releases/latest"
    url=$(curl -fsSL "$api" | sed -n "s/.*\"browser_download_url\": \"\\([^\"]*rig-${target}\\.tar\\.gz\\)\".*/\\1/p" | head -1)
  else
    url="https://github.com/${REPO}/releases/download/${tag}/rig-${target}.tar.gz"
  fi
  [ -n "${url:-}" ] || return 1
  echo "downloading $url"
  tmp=$(mktemp -d)
  trap 'rm -rf "$tmp"' EXIT
  curl -fsSL "$url" -o "$tmp/rig.tar.gz" || return 1
  tar -xzf "$tmp/rig.tar.gz" -C "$tmp"
  if [ -f "$tmp/rig" ]; then
    install_bin_to_path "$tmp/rig"
  elif [ -f "$tmp/rig-${target}/rig" ]; then
    install_bin_to_path "$tmp/rig-${target}/rig"
  else
    found=$(find "$tmp" -type f -name rig | head -1)
    [ -n "$found" ] || return 1
    install_bin_to_path "$found"
  fi
  echo
  echo "done. Rust is not required."
  echo "packages: macOS needs Homebrew (brew); Linux needs apt."
  echo "  skip packages: rig apply --yes --skip-packages"
  echo "next:"
  echo "  rig init --role workstation   # or compute"
  echo "  rig apply            # preview"
  echo "  rig apply --yes"
  return 0
}

install_from_source() {
  root=$1
  have cargo || die "cargo not found — install Rust from https://rustup.rs/"
  echo "rig install (maintainer / source)"
  echo "  root=$root"
  cargo install --path "$root/crates/rig" --force
  echo
  echo "next:"
  echo "  cd $root && rig apply"
}

clone_repo() {
  have git || die "git not found"
  if is_rig_root "$DEST"; then
    echo "using existing clone: $DEST"
  else
    if [ -e "$DEST" ]; then
      die "refusing to overwrite: $DEST (set RIG_CLONE_DIR)"
    fi
    echo "cloning $REPO_URL → $DEST"
    git clone --depth 1 "$REPO_URL" "$DEST"
  fi
}

echo "rig install"
if [ "${RIG_FORCE_SOURCE:-}" = "1" ]; then
  echo "RIG_FORCE_SOURCE=1 — building from source"
  if [ -n "${RIG_ROOT:-}" ] && is_rig_root "$RIG_ROOT"; then
    install_from_source "$RIG_ROOT"
    exit 0
  fi
  here=$(CDPATH= cd -- "$(dirname "$0")" && pwd)
  if is_rig_root "$here"; then
    install_from_source "$here"
    exit 0
  fi
  if is_rig_root "$PWD"; then
    install_from_source "$PWD"
    exit 0
  fi
  clone_repo
  install_from_source "$DEST"
  exit 0
fi

if install_from_release; then
  exit 0
fi

echo "error: no release binary for this platform (or no release published yet)." >&2
echo "  Users: check https://github.com/${REPO}/releases" >&2
echo "  Maintainers: RIG_FORCE_SOURCE=1 $0" >&2
exit 1
