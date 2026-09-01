# Compute node — headless (no GUI casks)
#
# Role packages = common + compute. From packages/brew/common.Brewfile you
# already get: bash zsh git openssh tmux fzf eza bat zoxide rsync tree btop
# fastfetch tldr gitleaks thefuck watch
#
# Intentional apps from ~/dotfiles Brewfile (not ffmpeg leaf deps like aom,
# frei0r, …; not GUI casks; not toys like c2048/nsnake/vitetris).

# Build / languages
# Julia → juliaup | Python → uv+pyenv | R → r
# C/C++ → cmake/ninja/llvm/gcc/libomp (OpenMP: gcc-14 or clang -fopenmp)
# Rust → rustup (rustup default stable) | Java → openjdk@17
brew "cmake"
brew "ninja"
brew "pkgconf"
brew "llvm"
brew "libomp"
brew "gcc"
brew "uv"
brew "pyenv"
brew "openssl@3"
brew "readline"
brew "sqlite"
brew "xz"
brew "zlib"
brew "juliaup"
brew "r"
brew "openjdk@17"
brew "maven"
brew "rustup"
brew "zeromq"

# Containers / remote / VCS
brew "gh"
brew "colima"
brew "docker"
brew "docker-compose"
brew "mosh"
brew "wget"
brew "jq"
brew "ripgrep"
brew "fd"
brew "neovim"
brew "parallel"

# Workloads / media / ML-ish
brew "ffmpeg"
brew "ollama"
brew "tesseract"

# Ops / diagnostics on lab nodes
brew "htop"
brew "iperf3"
brew "nmap"
brew "socat"
brew "telnet"
brew "highlight"
