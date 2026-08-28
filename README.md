# rig

Opinionated setup for **workstation** and **compute** machines.

One CLI configures shell (zsh/bash), role-based packages, and SSH host
entries — without putting personal IPs in the product repo.

## Install

Needs [Rust](https://rustup.rs/) (`cargo`) for now (CLI is installed from source).

```bash
curl -fsSL https://raw.githubusercontent.com/yamanori99/rig/main/install.sh | sh
```

This clones to `~/rig` (override with `RIG_CLONE_DIR`) and runs
`cargo install --path crates/rig`.

Already have a clone:

```bash
cd /path/to/rig && ./install.sh
```

Releases (notes / tags): https://github.com/yamanori99/rig/releases

## Use

```bash
cd ~/rig   # or your clone
rig init --role workstation   # or: --role compute
# edit hosts/<name>.toml — add peer files with [[ssh]] for machines you reach
rig apply --dry-run
rig apply --yes
```

Typical follow-ups:

```bash
rig ssh-config --write          # ~/.ssh/config.d/rig.conf from hosts/*.toml
rig keys distribute --yes       # copy your pubkey to peers
rig check                       # TCP/22 + BatchMode SSH per [[ssh]] path
```

`[[ssh]]` lives on the **peer** host file (alias / ip / link). Examples:
`hosts/examples/`. Real addresses stay in gitignored `hosts/*.toml`.

## Roles

| Role | Intent |
| --- | --- |
| `workstation` | GUI laptop/desktop, zsh default |
| `compute` | Headless, bash default, remote/tailscale |

Packages: `packages/brew/{common,workstation,compute}.Brewfile`,
`packages/apt/*.list`.

## Privacy

- Tracked: `hosts/examples/` only
- Not tracked: `hosts/*.toml`, `overlay/`
- Do not commit real VPN/LAN/Thunderbolt addresses

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

## Development

```bash
cargo install --path crates/rig --force
# or: cargo run -p rig -- …
```

Linux smoke (Apple `container`): [testenv/apple-container/README.md](testenv/apple-container/README.md).

Before push: `gitleaks protect --staged -c .gitleaks.toml`

## Status

`v0.1.1` — user-chosen `[[ssh]]` aliases; optional workstation Thunderbolt;
OS hostname left to the user.

## License

[MIT](LICENSE)
