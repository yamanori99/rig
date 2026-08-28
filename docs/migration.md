# dotfiles → rig 移行メモ

最終更新: 2026-08-29

旧 `~/dotfiles` を一気に消すのではなく、**製品コアを rig に寄せてから**
dotfiles 側を薄くする。

## 原則

- 実 IP / 実ホスト一覧は rig の tracked ツリーに載せない (`hosts/*.toml` は local)
- Oh My Zsh / p10k 前提にロックしない (製品テンプレは薄い shell)
- 個人の厚い設定は当面 `~/dotfiles` か `overlay/` に残す
- `setup.sh` を rig から呼ばない (適用ロジックは Rust)

## 寄せる順

| 順 | 内容 | 状態 |
| --- | --- | --- |
| 1 | role 別パッケージ (Brewfile / apt) を実 Brewfile から再分割 | 済 |
| 2 | `rig apply`: shell スニペット配置 + brew/apt + state | 済 (core) |
| 3 | `rig ssh-config --write` (config.d / 管理ブロック) | 済 |
| 3b | Apple container で Linux スモーク | 済 |
| 4 | hostname / remote login / Tailscale / Thunderbolt (macOS) | 未 |
| 5 | Cursor settings link (`features.cursor`) | 未 |
| 6 | tmux 等の共有 templates | 未 |
| 7 | `sync` / 複数機 / 鍵配布 (旧 sync-all 相当) | 未 |
| 8 | dotfiles の `setup.sh` を「rig 呼び出し + 個人 overlay」に縮小 | 未 |

## 今の検証方針

1. gitleaks → 初回コミット → **パブリック** GitHub に push  
2. **正のスモーク**: guest が GitHub から clone して動かす  

```bash
./testenv/apple-container/scripts/up.sh --smoke --from-github
./testenv/apple-container/scripts/down.sh
```

手元ループ用 (bind-mount) は `--smoke` のみ。公開後の正しさは `--from-github`。

詳細: [testenv/apple-container/README.md](../testenv/apple-container/README.md) /
[docs/security.md](security.md)

## やらないこと (当面)

- `~/.zshrc` を薄いテンプレで丸ごと上書き
  - 代わりに `# >>> begin rig >>>` ブロックで `~/.config/rig/shell/` を source

## dotfiles 側の置き場 (残すもの)

- `.p10k.zsh` / OMZ プラグイン / 個人 aliases・tips
- lab 固有 network scripts (internet-sharing 等)
- 実 SSH inventory (→ 将来は各機 `hosts/*.toml` + 同期)

## 完了の目安

新マシンで `rig init` → hosts 編集 → `rig apply` だけで
workstation または compute の最低限が立ち、dotfiles は「好みの上乗せ」だけになる。
