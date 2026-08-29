# rig

Opinionated setup for **workstation** and **compute** machines.

One CLI configures shell (zsh/bash), role-based packages, and SSH host
entries — without putting personal IPs in the product repo.

**You do not need Rust.** Install the release binary; product files
(roles / packages / templates) are embedded and unpack on first run.

## Requirements

Installing `rig` needs `curl` and `tar`.

`rig apply` installs packages with Homebrew (`brew`) on macOS, and `apt`
on Debian/Ubuntu. Rig does not install those tools. Skip packages with
`rig apply --yes --skip-packages`.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/yamanori99/rig/main/install.sh | sh
```

Puts `rig` in `~/.local/bin` (override with `RIG_BIN_DIR`). That is the
**latest** GitHub Release. Install appends that dir to zsh/bash startup
files. The `curl | sh` process is not your prompt — run the printed
`export PATH=...` in that terminal, or open a new one.

Optional: `RIG_BIN_DIR=...` or pin `RIG_VERSION=vX.Y.Z` on the **pipe
side** (env must apply to `sh`, not only `curl`):

```bash
curl -fsSL https://raw.githubusercontent.com/yamanori99/rig/main/install.sh \
  | RIG_BIN_DIR=/tmp/rig-bin sh
```

Releases: <https://github.com/yamanori99/rig/releases>

## Update

```bash
rig update           # preview
rig update --yes
```

Installs the latest Release into `~/.local/bin/rig`. `hosts/` and
`overlay/` stay. Templates refresh on the next run if the version changed.

## Uninstall

```bash
curl -fsSL \
  https://raw.githubusercontent.com/yamanori99/rig/main/uninstall.sh | sh
```

Removes `~/.local/bin/rig` (override with `RIG_BIN_DIR`). Product data
(hosts / overlay / state) is kept by default. Purge it too:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/yamanori99/rig/main/uninstall.sh \
  | RIG_PURGE=1 sh
```

This does not undo shell snippets or packages from `rig apply` — run
`rig clean --yes` first while the binary is still installed if you want that.

## Use

```bash
rig init --role workstation   # or: --role compute
# init prints the paths to edit — then:
rig apply            # preview
rig apply --yes
rig status
```

Then:

```bash
rig ssh-config --yes
rig keys distribute --yes
rig check
```

### What to edit

Release installs unpack product files under the OS data dir. Run `rig root`
for the absolute path.

- macOS: `~/Library/Application Support/dev.rig.rig/product/`
- Linux: typically `~/.local/share/rig/product/`

Checkout / `--root` uses that tree instead.

| Path | You edit? | Purpose |
| --- | --- | --- |
| `hosts/<this-host>.toml` | yes | role, `[[ssh]]`, package add/remove |
| `hosts/<peer>.toml` | yes | peer reachability (`alias` / `ip` / `link`) |
| `overlay/` | yes | personal shell / tmux / Cursor overrides |
| `templates/` | no | product defaults — override via `overlay/` |

`[[ssh]]` goes on the **peer** host file. Seeds: `hosts/examples/` (also
embedded). Use `--root` / `RIG_ROOT` only if you point at a checkout.

## Roles

| Role | Intent |
| --- | --- |
| `workstation` | GUI laptop/desktop, zsh default |
| `compute` | Headless, bash default, remote/tailscale, stay-awake |

## Privacy

Do not put real VPN / LAN / Thunderbolt addresses in a shared git repo.

## Commands

```text
rig root                         # product data path (hosts / overlay)
rig status                       # apply steps; live for enabled features
rig init [--role workstation|compute] [--name HOST]
rig host list | detect
rig roles [NAME] [--os macos|linux]
rig apply [-y] [--skip-packages] # preview; -y writes
rig check
rig keys distribute [-y]         # preview; -y copies
rig clean [-y] [--packages]      # preview; -y deletes
rig ssh-config [-y|--write]      # preview; -y writes
rig update [-y] [--force]        # preview; -y installs
```

Most commands also print the data root on stderr so the path stays visible.

## For maintainers

Needs [Rust](https://rustup.rs/). Normal users should ignore this section.

```bash
git clone https://github.com/yamanori99/rig.git
cd rig
RIG_FORCE_SOURCE=1 ./install.sh
# or: cargo install --path crates/rig --force
```

Inside a checkout, `rig` uses that tree; otherwise it materializes embedded
assets. Tag `v*` publishes binaries via GitHub Actions.

Linux smoke (Apple `container`):

```bash
# local checkout + cargo
./testenv/apple-container/scripts/up.sh --smoke

# release binary (no Rust in guest)
./testenv/apple-container/scripts/up.sh --smoke --from-release
```

See [testenv/apple-container/README.md](testenv/apple-container/README.md).

Before push: `gitleaks protect --staged -c .gitleaks.toml`

## Status

`v0.2.6` — compute stay-awake uses `pmset -a`; macOS + Linux.

## License

[MIT](LICENSE)
