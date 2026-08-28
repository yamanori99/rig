# rig

[English](README.md) | [日本語](README.ja.md)

**workstation** (ノート / デスクトップ) と **compute** ノード向けの、意見の入ったセットアップ。

1 つの CLI でシェル (zsh/bash)、ロール別パッケージ、SSH ホストエントリ、
そして (将来) sync/clean を設定する。
個人の IP やプライベートな在庫情報はプロダクトリポジトリに置かない。

## ビルド

[Rust](https://rustup.rs/) (`cargo`) が必要。

```bash
cd /path/to/rig
cargo build -p rig                  # debug: target/debug/rig
cargo build -p rig --release        # release: target/release/rig
cargo install --path crates/rig --force   # ~/.cargo/bin/rig
cargo run -p rig -- --help          # インストールせず実行
```

ソースを直したら `cargo install --path crates/rig --force` をやり直し
(または `cargo run -p rig -- …`) しないと PATH のバイナリは古いまま。

手順の詳細は [docs/quickstart.md](docs/quickstart.md)。
Linux 検証は [testenv/apple-container/README.md](testenv/apple-container/README.md)
(Apple `container`; ホストの brew は触らない)。
公開前の秘密情報チェックは [docs/security.md](docs/security.md)。

## ステータス

`v0.1.0`: スキーマ、ロール、パッケージ、`init` / `host` / `roles`、
`apply --dry-run` / `apply --yes` (shell スニペット、brew/apt、ssh-config、state)。
hostname / features / clean / self-update はこれから。

## クイックスタート (開発)

```bash
cargo install --path crates/rig
cd /path/to/rig
rig init --role workstation    # hosts/<hostname>.toml を書く (gitignore 済み)
rig host list
rig apply --dry-run
rig ssh-config
```

## ロール

| ロール | 意図 |
| --- | --- |
| `workstation` | GUI 向けノート / デスクトップ、zsh デフォルト |
| `compute` | ヘッドレスノード、bash デフォルト、remote/tailscale 向け機能 |

パッケージセット: `packages/brew/{common,workstation,compute}.Brewfile`
と `packages/apt/*.list`。

## プライバシー

- 追跡する: `hosts/examples/` 配下の例のみ
- 追跡しない: `hosts/*.toml` (実機)、`overlay/`
- ラボや自宅ネットの VPN/LAN/Thunderbolt アドレスはコミットしない

## コマンド

```text
rig init [--role workstation|compute] [--name HOST]
rig host list | detect
rig roles [NAME] [--os macos|linux]
rig apply [--dry-run] [-y]
rig clean [--dry-run] [-y] [--packages]
rig ssh-config [--write]
```

`rig roles` でロールごとのパッケージ (brew / apt)、features、
デフォルトシェルを一覧できる。

## ライセンス

MIT (予定)
