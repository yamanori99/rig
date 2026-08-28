# rig common shell helpers (POSIX-ish)
# Copied to ~/.config/rig/shell/common.sh on apply.
# Personal extras: overlay/shell.sh or overlay/shell/*.sh (gitignored).

export EDITOR="${EDITOR:-vim}"
export VISUAL="${VISUAL:-$EDITOR}"

# Homebrew (Apple Silicon / Intel)
if [ -x /opt/homebrew/bin/brew ]; then
  eval "$(/opt/homebrew/bin/brew shellenv)"
elif [ -x /usr/local/bin/brew ]; then
  eval "$(/usr/local/bin/brew shellenv)"
fi

export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$HOME/bin:$PATH"

# Safer defaults
set -o noclobber 2>/dev/null || true

# ls / navigation (prefer modern tools when installed)
if command -v eza >/dev/null 2>&1; then
  alias ls='eza --group-directories-first'
  alias ll='eza -la --group-directories-first --git'
  alias la='eza -la --group-directories-first'
  alias lt='eza -T --level=2'
else
  alias ll='ls -la'
  alias la='ls -la'
fi

alias ..='cd ..'
alias ...='cd ../..'
alias g='git'
alias gs='git status -sb'
alias gd='git diff'
alias gl='git log --oneline -20'

# Editors / quick tools
alias v='${EDITOR:-vim}'
command -v bat >/dev/null 2>&1 && alias cat='bat --paging=never'
command -v rg >/dev/null 2>&1 && alias grep='rg'
command -v fzf >/dev/null 2>&1 && export FZF_DEFAULT_OPTS='--height 40% --reverse --border'

# zoxide (smart cd) when available
if command -v zoxide >/dev/null 2>&1; then
  case "${SHELL##*/}" in
    zsh) eval "$(zoxide init zsh)" ;;
    bash) eval "$(zoxide init bash)" ;;
  esac
fi

# Personal overlay (single file and/or directory)
if [ -n "${RIG_ROOT:-}" ]; then
  if [ -f "$RIG_ROOT/overlay/shell.sh" ]; then
    # shellcheck disable=SC1091
    . "$RIG_ROOT/overlay/shell.sh"
  fi
  if [ -d "$RIG_ROOT/overlay/shell" ]; then
    for _rig_ov in "$RIG_ROOT/overlay/shell"/*.sh; do
      [ -f "$_rig_ov" ] || continue
      # shellcheck disable=SC1090
      . "$_rig_ov"
    done
    unset _rig_ov
  fi
fi
