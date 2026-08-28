# rig

Opinionated setup for **workstation** and **compute** machines.

One CLI configures shell (zsh/bash), role-based packages, and SSH host
entries — without putting personal IPs in the product repo.

## Build

Needs [Rust](https://rustup.rs/) (`cargo`).

```bash
cargo install --path crates/rig --force
```

Re-run after editing sources (or use `cargo run -p rig -- …`).

## Quick start

```bash
cd /path/to/rig
rig init --role workstation   # hosts/<hostname>.toml (gitignored)
rig roles
rig apply --dry-run
rig apply --yes               # shell snippet, brew/apt, ssh-config, state
```

Linux smoke (Apple `container`, not the Mac host):
see [testenv/apple-container/README.md](testenv/apple-container/README.md).

Before push: `gitleaks protect --staged -c .gitleaks.toml`

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
rig clean [--dry-run] [-y] [--packages]
rig ssh-config [--write]
```

## Status

`v0.1.0` — apply core + hostname + remote-login; clean / remaining features pending.

## License

MIT (planned)
