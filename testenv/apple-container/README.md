# Apple container スモーク (macOS)

Apple [`container`](https://github.com/apple/container) 上の Linux VM で
**rig** を検証する。Mac ホストの brew / 本物の `.zshrc` は触らない。

モードは 2 つ:

| モード | 検証すること |
| --- | --- |
| bind-mount (既定) | 手元の作業ツリーでの速いループ |
| `--from-github` | **公開ゲート**: ゲストが公開 repo を clone してスモーク |

**CI にはまだ載せない。** DistSSHKit 流のローカル用ハーネス。

## 要件

- macOS 26+、Apple silicon
- [`container`](https://github.com/apple/container) CLI
- `python3` (`container inspect` の JSON を読む)

## 推奨手順 (パブリック GitHub)

1. 必要なら `gh auth refresh -h github.com`
2. 初回コミット + `gitleaks protect --staged`
   ([docs/security.md](../../docs/security.md))
3. パブリック repo を作成・push (`yamanori99/rig` など)
4. 公開ゲートのスモーク:

```bash
./testenv/apple-container/scripts/up.sh --smoke --from-github
# または:
./testenv/apple-container/scripts/up.sh \
  --smoke --from-github=https://github.com/USER/rig.git
./testenv/apple-container/scripts/down.sh
```

## ローカルループ (bind-mount)

```bash
./testenv/apple-container/scripts/up.sh --smoke
./testenv/apple-container/scripts/down.sh
```

シェルだけ入る:

```bash
./testenv/apple-container/scripts/up.sh
ssh -F testenv/apple-container/.generated/ssh_config rig-smoke
```

ゲスト内で apt も回す (遅い):

```bash
./testenv/apple-container/scripts/up.sh --smoke --with-packages
```

## 構成

| パス | 役割 |
| --- | --- |
| `Dockerfile` / `start.sh` | Ubuntu 24.04 + sshd + Rust |
| `scripts/up.sh` | system start → build → `rig-smoke` → SSH config |
| `scripts/smoke.sh` | ホスト側ドライバ (mount または clone) |
| `scripts/smoke-guest.sh` | cargo install → init → dry-run → apply |
| `scripts/down.sh` | コンテナ停止・削除 |
| `.generated/` | gitignore 済みの鍵と ssh_config |

## 補足

- 既定スモークは `rig apply --yes --skip-packages`
  (shell スニペット + ssh-config + state)。apt は `--with-packages`。
- hostname / Tailscale / Thunderbolt は未実装で `[todo]` になる。
- DistSSHKit の `child-1` / `child-2` とは別物。
- SSH は生成 config でユーザー `dev`。
  `ssh <ip>` で Mac のユーザー名にしない。
