# Apple container smoke

Linux VM via Apple [`container`](https://github.com/apple/container)
(macOS 26+, Apple silicon). Does not mutate the Mac host.

## Single node

```bash
# local working tree
./testenv/apple-container/scripts/up.sh --smoke
./testenv/apple-container/scripts/down.sh

# after push — clone from GitHub inside the guest
./testenv/apple-container/scripts/up.sh --smoke --from-github
```

```bash
ssh -F testenv/apple-container/.generated/ssh_config rig-smoke
```

Use user `dev` and that config (not `ssh <ip>` as your Mac user).

Interactive shells: `CARGO_HOME` must be `~/.cargo` (writable). New images set
this in `.bashrc`. On an old guest already running:

```bash
export CARGO_HOME="$HOME/.cargo"
export PATH="$CARGO_HOME/bin:/opt/cargo/bin:$PATH"
```

| Flag | Meaning |
| --- | --- |
| `--smoke` | install + init + apply dry-run + apply `--skip-packages` |
| `--with-packages` | also run apt |
| `--from-github[=URL]` | guest clones public repo (source gate) |
| `--from-release` | guest `curl \| sh` installs Release binary (no Rust) |

Default URL: `https://github.com/yamanori99/rig.git`

## Fleet (N nodes)

Inventory: [`inventory.toml`](inventory.toml). Default is one `workstation` + one
`compute`. Add `[[node]]` rows to grow the fleet.

```bash
./testenv/apple-container/scripts/fleet-down.sh
./testenv/apple-container/scripts/fleet-up.sh --smoke
# release gate:
./testenv/apple-container/scripts/fleet-up.sh --smoke --from-github
```

```bash
ssh -F testenv/apple-container/.generated/ssh_config rig-ws
ssh -F testenv/apple-container/.generated/ssh_config rig-compute
```

What smoke checks:

1. Each node: `rig` install + init + apply (`--skip-packages`)
2. Each workstation: peer `hosts/<name>.toml` with `[[ssh]]` (`link = lan`),
   then `rig ssh-config --write`
3. From each workstation: `ssh <peer>-lan echo ok` to every other node
   (alias chosen by fleet-smoke; product lets you pick any alias)

Mac → guests uses the controller key. Workstation → peers uses the client key
(`~/.ssh/id_ed25519` on workstation nodes only).
