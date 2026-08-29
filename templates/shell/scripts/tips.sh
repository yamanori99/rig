#!/usr/bin/env zsh
# Random tip from templates/shell/tips.txt (copied to RIG_CONFIG/shell/).
set -euo pipefail
tips="${RIG_CONFIG:-}/shell/tips.txt"
if [[ ! -f "$tips" && -n "${RIG_ROOT:-}" ]]; then
  tips="$RIG_ROOT/templates/shell/tips.txt"
fi
[[ -f "$tips" ]] || { echo "error" >&2; echo "  tips.txt not found" >&2; exit 1; }
grep -v '^#' "$tips" | grep -v '^$' | sort -R | head -1
