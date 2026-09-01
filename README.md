# rig

[日本語](README.ja.md)

`rig` installs a development environment on your Mac or Linux machines.
Use the same steps on a laptop and on a Mini.

It installs the shell, the packages for that machine, and SSH between
your machines.

Rust is not required. A release binary is enough. Extra files are
written to disk on first run.

## Setup

Do this on every machine. One machine only: stop after apply.
Several machines: continue through host files, reachability, then
`host check` / `host keys`.

### 1. Package manager

`rig apply` uses Homebrew (`brew`) on macOS and `apt` on
Debian/Ubuntu. Rig does not install brew or apt.

- macOS: install [Homebrew](https://brew.sh/) first
- Debian/Ubuntu: `apt` is already there

To skip packages later:

```bash
rig apply --yes --skip-packages
```

### 2. Machine name

`rig init` / `rig apply` match this machine by short hostname
(`hostname -s`). That string becomes `name` in the host file.
Rig does not change the OS hostname.

On a Mac, set it in System Settings > General > Sharing
(Computer Name / Local Hostname) **before** init. Use a short name
without `.local` (`m4-mini-tak`, not `m4-mini-tak.local`).

### 3. Install rig

You need `curl` and `tar`.

```bash
curl -fsSL https://raw.githubusercontent.com/yamanori99/rig/main/install.sh | sh
```

`rig` goes in `~/.local/bin`. That path is also added to zsh and bash
startup files. When it finishes, close the terminal and open a new one.

Latest release: <https://github.com/yamanori99/rig/releases>

To change the install directory or version, set the variable on the
right side of the pipe (`sh`). Setting it only on `curl` does nothing.

```bash
curl -fsSL https://raw.githubusercontent.com/yamanori99/rig/main/install.sh \
  | RIG_BIN_DIR=/tmp/rig-bin sh
```

`RIG_VERSION=vX.Y.Z` works the same way.

### 4. Init and apply

```bash
rig init -R workstation   # or: -R compute
rig apply                 # preview
rig apply -y
rig status
```

Init writes `~/.rig-hosts/<name>.toml` only when that file is missing.
If you already have inventory there (copy or git), skip init.
`File exists` means the file is already there. Do not run init again.

### 5. Other machines

Needed only if machines should SSH to each other.

1. Put every host toml in `~/.rig-hosts/` on each machine
   (`[[ssh]]` goes on the toml of the machine you connect **to**).
2. On each peer: same install, name, init (if empty), then `rig apply -y`.
3. Make the machines reachable (same LAN / Thunderbolt / VPN, SSH
   listening). On macOS 12.1+, compute apply turns on Remote
   Management (`:5900`). Permit it once in System Settings >
   General > Sharing > Remote Management (not Screen Sharing).
4. Then, on each machine that should connect out:

```bash
rig host check            # TCP/22, then ssh
rig host keys -y          # if ssh fail (password once per new peer)
rig host check            # confirm ssh ok
```

If you set up several machines at once, run check / keys / check
**on each of them**. Keys are one-way until the other side copies
its own key.

After you add or edit peer toml files, apply again (or
`rig host ssh-config -y`) so `~/.ssh/config.d/rig.conf` updates.

`rig apply` reads `~/.rig-hosts/`, not product `hosts/`. On a machine
that signs in to GitHub, keep `~/.rig-hosts/` in git.

## Roles

| Role | What it is |
| --- | --- |
| `workstation` | GUI laptop or desktop. Shell: zsh |
| `compute` | No display. Shell: bash. SSH, screen sharing, no sleep |

On macOS the login shell is Homebrew: zsh for workstation, bash for
compute. Not `/bin/zsh` or `/bin/bash` 3.2.

## Files

Data is stored here:

- macOS: `~/Library/Application Support/dev.rig.rig/product/`
- Linux: usually `~/.local/share/rig/product/`

The exact path is the first line of `rig root`. A checkout of this repo
or `--root` uses that tree instead.

```text
~/.rig-hosts/              # inventory. a real directory, not a link
  m4-mba-neva.toml
  m4-mini-tak.toml

$(rig root)/
  hosts/examples/          # samples. leave them
  overlay/                 # your shell / tmux / Cursor
  templates/               # defaults. leave them
  roles/
  packages/
```

## Commands

```text
init, i     Write ~/.rig-hosts/<name>.toml
apply, a    Apply this host (preview; -y writes)
            --undo -y   reverse apply
status, s   Show this machine
host, h     list | check | keys

-v          version
-r          product root  (cwd / RIG_ROOT / unpack)
-h          help          (--help for more)
```

Most commands print the data path on stderr. `rig COMMAND --help` has the long text.

## Update

```bash
rig update           # preview
rig update --yes
```

Only `~/.local/bin/rig` is replaced. `~/.rig-hosts/` and `overlay/` stay.
If the version went up, templates refresh on the next run.

## Uninstall

```bash
curl -fsSL \
  https://raw.githubusercontent.com/yamanori99/rig/main/uninstall.sh | sh
```

Removes `~/.local/bin/rig`. `~/.rig-hosts`, overlay, and state stay.
To delete those too:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/yamanori99/rig/main/uninstall.sh \
  | RIG_PURGE=1 sh
```

This does not undo packages or shell settings from `rig apply`. To undo
those, run `rig apply --undo --yes` first, while the binary is still there.

## For developers

You need [Rust](https://rustup.rs/). Skip this section for normal use.

```bash
git clone https://github.com/yamanori99/rig.git
cd rig
RIG_FORCE_SOURCE=1 ./install.sh
# or: cargo install --path crates/rig --force
```

Inside a checkout, `rig` uses that tree. Otherwise it unpacks the
embedded files. Tags `v*` publish binaries with GitHub Actions.

Linux check (Apple `container`):

```bash
./testenv/apple-container/scripts/up.sh --smoke
./testenv/apple-container/scripts/up.sh --smoke --from-release
```

[testenv/apple-container/README.md](testenv/apple-container/README.md)

Before push, run `gitleaks protect --staged -c .gitleaks.toml`.

## License

[MIT](LICENSE)
