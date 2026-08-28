# rig common shell helpers (POSIX-ish) — sourced from zsh/bash rc
# Prefer: managed copy under ~/.config/rig/shell/

export EDITOR="${EDITOR:-vim}"

# Homebrew (Apple Silicon / Intel)
if [ -x /opt/homebrew/bin/brew ]; then
  eval "$(/opt/homebrew/bin/brew shellenv)"
elif [ -x /usr/local/bin/brew ]; then
  eval "$(/usr/local/bin/brew shellenv)"
fi

export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/bin:$PATH"

alias ll='ls -la'
alias g='git'

# Optional personal overlay (gitignored in the rig checkout)
if [ -n "${RIG_ROOT:-}" ] && [ -f "$RIG_ROOT/overlay/shell.sh" ]; then
  # shellcheck disable=SC1091
  . "$RIG_ROOT/overlay/shell.sh"
fi
