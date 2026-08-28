# Apple container smoke (macOS)

Verify **rig** on a Linux VM via Apple
[`container`](https://github.com/apple/container) — not on the Mac host.

Two modes:

| Mode | What it tests |
| --- | --- |
| bind-mount (default) | Fast loop on your local working tree |
| `--from-github` | **Release gate**: guest clones public repo, then smoke |

**Not CI yet.** DistSSHKit-style local harness.

## Requirements

- macOS 26+, Apple silicon
- [`container`](https://github.com/apple/container) CLI
- `python3` (reads `container inspect` JSON)

## Recommended path (public GitHub)

1. `gh auth refresh -h github.com` (if `gh` token is stale)
2. First commit + `gitleaks protect --staged` (see [docs/security.md](../../docs/security.md))
3. Create/push public repo (`yamanori99/rig` or your fork)
4. Release-gate smoke:

```bash
./testenv/apple-container/scripts/up.sh --smoke --from-github
# or:
./testenv/apple-container/scripts/up.sh --smoke --from-github=https://github.com/USER/rig.git
./testenv/apple-container/scripts/down.sh
```

## Local loop (bind-mount)

```bash
./testenv/apple-container/scripts/up.sh --smoke
./testenv/apple-container/scripts/down.sh
```

Shell only:

```bash
./testenv/apple-container/scripts/up.sh
ssh -F testenv/apple-container/.generated/ssh_config rig-smoke
```

Also install packages inside the guest (apt; slower):

```bash
./testenv/apple-container/scripts/up.sh --smoke --with-packages
```

## Layout

| Path | Role |
| --- | --- |
| `Dockerfile` / `start.sh` | Ubuntu 24.04 + sshd + Rust |
| `scripts/up.sh` | system start → build → create `rig-smoke` → SSH config |
| `scripts/smoke.sh` | host driver (mount or clone, then guest smoke) |
| `scripts/smoke-guest.sh` | cargo install → init compute → dry-run → apply |
| `scripts/down.sh` | stop/rm container |
| `.generated/` | gitignored keys + ssh_config |

## Notes

- Default smoke uses `rig apply --yes --skip-packages` (shell snippet +
  ssh-config + state). Pass `--with-packages` for apt.
- Hostname / Tailscale / Thunderbolt features are still unimplemented;
  they show as `[todo]` in apply output.
- Do not confuse with DistSSHKit workers (`child-1` / `child-2`).
- SSH as `dev` with the generated config — not `ssh <ip>` as your Mac user.
