#!/usr/bin/env zsh
# List aliases defined in the product workstation zshrc.
set -euo pipefail
zshrc="${RIG_CONFIG:-}/shell/zshrc"
if [[ ! -f "$zshrc" && -n "${RIG_ROOT:-}" ]]; then
  zshrc="$RIG_ROOT/templates/shell/zsh/zshrc"
fi
[[ -f "$zshrc" ]] || { echo "error" >&2; echo "  zshrc not found" >&2; exit 1; }

G=$'\033[32m'
N=$'\033[0m'

if [[ $# -gt 0 ]]; then
  grep -E "^alias .*${1}" "$zshrc" | sed 's/^alias //' | while IFS='=' read -r name cmd; do
    printf "  ${G}%-14s${N} %s\n" "$name" "$cmd"
  done
  exit 0
fi

echo "aliases"
grep '^alias ' "$zshrc" | sed 's/^alias //' | sort -u | while IFS='=' read -r name cmd; do
  cmd="${cmd#\'}"; cmd="${cmd%\'}"; cmd="${cmd#\"}"; cmd="${cmd%\"}"
  printf "  ${G}%-14s${N} %s\n" "$name" "$cmd"
done
