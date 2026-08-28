# rig

Opinionated setup for **workstation** and **compute** machines.

One CLI configures shell (zsh/bash), role-based packages, and SSH host
entries — without putting personal IPs in the product repo.

**You do not need Rust.** Install the release binary; product files
(roles / packages / templates) are embedded and unpack on first run.

## Install

```bash
curl -fsSL https://raw.githubusercontent.com/yamanori99/rig/main/install.sh | sh
```

Puts `rig` in `~/.local/bin` (override with `RIG_BIN_DIR`). Add that dir to
`PATH` if the installer says so.

Pin a release: `RIG_VERSION=v0.2.0` on the **pipe side** (env must apply to `sh`, not only `curl`):

```bash
curl -fsSL https://raw.githubusercontent.com/yamanori99/rig/main/install.sh | RIG_VERSION=v0.2.0 sh
# optional install dir:
curl -fsSL https://raw.githubusercontent.com/yamanori99/rig/main/install.sh | RIG_BIN_DIR=/tmp/rig-bin sh
```


Releases: https://github.com/yamanori99/rig/releases

## Use

```bash
rig init --role workstation   # or: --role compute
# edit the host file — add peer hosts/*.toml with [[ssh]] for machines you reach
rig apply --dry-run
rig apply --yes
```

Then:

```bash
rig ssh-config --write
rig keys distribute --yes
rig check
```

`[[ssh]]` goes on the **peer** host file (`alias` / `ip` / `link`). Seeds:
`hosts/examples/` (also embedded).

Host files and `overlay/` live in the product data directory when using a
release binary (created on first run). Use `--root` / `RIG_ROOT` only if you
point at a checkout.

## Roles

| Role | Intent |
| --- | --- |
| `workstation` | GUI laptop/desktop, zsh default |
| `compute` | Headless, bash default, remote/tailscale |

## Privacy

Do not put real VPN / LAN / Thunderbolt addresses in a shared git repo.

## Commands

```text
rig init [--role workstation|compute] [--name HOST]
rig host list | detect
rig roles [NAME] [--os macos|linux]
rig apply [--dry-run] [-y] [--skip-packages]
rig check
rig keys distribute [--dry-run] [-y]
rig clean [--dry-run] [-y] [--packages]
rig ssh-config [--write]
```

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

`v0.2.0` — release binary first; embedded product tree; Rust only for maintainers.

## License

[MIT](LICENSE)
