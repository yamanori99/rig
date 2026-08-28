# Quickstart

## Build

Requires [Rust](https://rustup.rs/) (`cargo` on PATH).

```bash
cd /path/to/rig
cargo build -p rig                # debug binary
cargo build -p rig --release      # optimized binary
cargo install --path crates/rig --force
```

- Debug: `target/debug/rig`
- Release: `target/release/rig`
- Install: `~/.cargo/bin/rig` (needs `~/.cargo/bin` on PATH)

Without installing:

```bash
cargo run -p rig -- roles
cargo run -p rig -- init --role workstation
```

Re-install (or use `cargo run`) after pulling or editing sources;
an old `~/.cargo/bin/rig` will not pick up changes by itself.

## Use

1. Install the `rig` binary (see above).
2. `rig roles` — packages / features / shell per role
   (optional: `rig roles workstation --os macos`).
3. `rig init --role workstation` or `--role compute`.
4. Edit the generated `hosts/<name>.toml` (local only).
5. `rig apply --dry-run` then `rig apply --yes`
   (shell snippet + packages + ssh-config + state;
   hostname/features still pending).
6. Optional: `rig ssh-config --write` alone.

### Linux smoke (Apple container)

Do **not** use the Mac host for first real apply. Use:

```bash
./testenv/apple-container/scripts/up.sh --smoke
./testenv/apple-container/scripts/down.sh
```

See [testenv/apple-container/README.md](../testenv/apple-container/README.md).
