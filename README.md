# rig

[日本語](README.ja.md)

`rig` installs a development environment on your Mac or Linux machines.
Use the same steps on a laptop and on a Mini.

It installs the shell, the packages for that machine, and SSH between
your machines.

Rust is not required. A release binary is enough. Extra files are
written to disk on first run.

## Requirements

You need `curl` and `tar` to install.

`rig apply` installs packages with Homebrew (`brew`) on macOS and
`apt` on Debian/Ubuntu. Rig does not install brew or apt. To skip
packages:

```bash
rig apply --yes --skip-packages
```

## Install

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

## Update

```bash
rig update           # preview
rig update --yes
```

Only `~/.local/bin/rig` is replaced. `hosts/` and `overlay/` stay.
If the version went up, templates refresh on the next run.

## Uninstall

```bash
curl -fsSL \
  https://raw.githubusercontent.com/yamanori99/rig/main/uninstall.sh | sh
```

Removes `~/.local/bin/rig`. Hosts, overlay, and state stay. To delete
those too:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/yamanori99/rig/main/uninstall.sh \
  | RIG_PURGE=1 sh
```

This does not undo packages or shell settings from `rig apply`. To undo
those, run `rig apply --undo --yes` first, while the binary is still there.

## Use

First machine:

```bash
rig init -R workstation   # or: -R compute
rig apply                 # preview
rig apply -y
rig status
```

Then:

```bash
rig host keys -y
rig host check
```

### Where files live

Data is stored here:

- macOS: `~/Library/Application Support/dev.rig.rig/product/`
- Linux: usually `~/.local/share/rig/product/`

The exact path is the first line of `rig root`. A checkout of this repo
or `--root` uses that tree instead.

```text
$(rig root)/
  hosts/
    examples/              # samples. leave them
      workstation.toml
      compute.toml
    m4-mba-neva.toml       # your machines. edit these
    m4-mini-tak.toml
  overlay/                 # your shell / tmux / Cursor
  templates/               # defaults. leave them
  roles/
  packages/
```

Put `[[ssh]]` on the toml of the machine you connect **to**.

`name` is the short hostname: `m4-mini-tak`, not
`m4-mini-tak.local`.

### A second machine

`rig apply` only looks at `hosts/` above. A git repo of machine files
(`~/rig-hosts` or similar) is a different directory until you symlink
it.

```text
~/rig-hosts/               # private git
  m4-mba-neva.toml
  m4-mini-tak.toml

$(rig root)/hosts  ->  ~/rig-hosts
```

`git pull` in `~/rig-hosts` does not update `rig apply` on a machine
without that symlink. On the second machine:

```bash
ln -sfn ~/rig-hosts "$(rig root | head -1)/hosts"
rig status                 # this machine should match a host file
rig apply --yes
```

If the toml is already in git, do not run `rig init`. Init only creates
a file when `hosts/` is empty.

If you see `File exists`, there is already a file or a broken symlink
at that path. Fix the link. Do not run init again.

## Roles

| Role | What it is |
| --- | --- |
| `workstation` | GUI laptop or desktop. Shell: zsh |
| `compute` | No display. Shell: bash. SSH, screen sharing, no sleep |

On macOS 12.1+, `rig apply` starts VNC `:5900` but cannot permit
Screen Sharing.app. Turn Screen Sharing off and on once in
System Settings > General > Sharing.

## Commands

```text
init, i     Write hosts/<name>.toml
apply, a    Apply this host (preview; -y writes)
            --undo -y   reverse apply
status, s   Show this machine
host, h     list | check | keys

-v          version
-r          product root  (cwd / RIG_ROOT / unpack)
-h          help          (--help for more)
```

Most commands print the data path on stderr. `rig COMMAND --help` has the long text.

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
