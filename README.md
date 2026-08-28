# rig

[English](README.md) | [日本語](README.ja.md)

Opinionated setup for **workstation** machines and **compute** nodes.

One CLI configures shell (zsh/bash), packages by role, SSH host entries,
and (later) sync/clean — without putting personal IPs or private inventory
in the product repo.

## Plan

設計・ロードマップは [docs/plan.md](docs/plan.md)。
手順の詳細は [docs/quickstart.md](docs/quickstart.md)。
Linux 検証は [testenv/apple-container/README.md](testenv/apple-container/README.md)
(Apple `container`; ホストの brew は触らない)。
公開前の秘密情報チェックは [docs/security.md](docs/security.md)。

## Build

Requires [Rust](https://rustup.rs/) (`cargo`).

```bash
cd /path/to/rig
cargo build -p rig                  # debug: target/debug/rig
cargo build -p rig --release        # release: target/release/rig
cargo install --path crates/rig --force   # ~/.cargo/bin/rig
cargo run -p rig -- --help          # run without installing
```

After editing sources, re-run `cargo install --path crates/rig --force`
(or use `cargo run -p rig -- …`) so PATH picks up the new binary.

## Status

`v0.1.0`: schema, roles, package sets, `init` / `host` / `roles`,
`apply --dry-run` / `apply --yes` (shell snippet, brew/apt, ssh-config, state).
Hostname/features/clean/self-update still pending.

## Quick start (dev)

```bash
cargo install --path crates/rig
cd /path/to/rig
rig init --role workstation    # writes hosts/<hostname>.toml (gitignored)
rig host list
rig apply --dry-run
rig ssh-config
```

## Roles

| Role | Intent |
| --- | --- |
| `workstation` | GUI-friendly laptop/desktop, zsh default |
| `compute` | Headless node, bash default, remote/tailscale features |

Package sets: `packages/brew/{common,workstation,compute}.Brewfile`
and `packages/apt/*.list`.

## Privacy

- Tracked: examples under `hosts/examples/` only
- Not tracked: `hosts/*.toml` (your real machines), `overlay/`
- Do not commit VPN/LAN/Thunderbolt addresses belonging to a lab or home network

## Commands

```text
rig init [--role workstation|compute] [--name HOST]
rig host list | detect
rig roles [NAME] [--os macos|linux]
rig apply [--dry-run] [-y]
rig clean [--dry-run] [-y] [--packages]
rig ssh-config [--write]
```

`rig roles` prints each role's packages (brew / apt), features,
and default shell.

## License

MIT (planned)
