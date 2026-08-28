# Apple container smoke

Linux VM via Apple [`container`](https://github.com/apple/container)
(macOS 26+, Apple silicon). Does not mutate the Mac host.

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

| Flag | Meaning |
| --- | --- |
| `--smoke` | install + init + apply dry-run + apply `--skip-packages` |
| `--with-packages` | also run apt |
| `--from-github[=URL]` | guest clones public repo (release gate) |

Default URL: `https://github.com/yamanori99/rig.git`
