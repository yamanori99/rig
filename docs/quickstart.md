# クイックスタート

## ビルド

[Rust](https://rustup.rs/) (`cargo` が PATH にあること) が必要。

```bash
cd /path/to/rig
cargo build -p rig                # debug
cargo build -p rig --release      # release
cargo install --path crates/rig --force
```

- Debug: `target/debug/rig`
- Release: `target/release/rig`
- インストール先: `~/.cargo/bin/rig` (`~/.cargo/bin` を PATH に)

インストールせず動かす例:

```bash
cargo run -p rig -- roles
cargo run -p rig -- init --role workstation
```

ソースを pull / 編集したあとは再インストール
(または `cargo run`) しないと、古い `~/.cargo/bin/rig` のままになる。

## 使い方

1. `rig` バイナリを入れる (上記)。
2. `rig roles` — ロールごとのパッケージ / features / shell
   (例: `rig roles workstation --os macos`)。
3. `rig init --role workstation` または `--role compute`。
4. 生成された `hosts/<name>.toml` を編集 (ローカルのみ)。
5. `rig apply --dry-run` のあと `rig apply --yes`
   (shell スニペット + パッケージ + ssh-config + state。
   hostname / features は未実装)。
6. 任意: `rig ssh-config --write` だけ実行。

### Linux スモーク (Apple container)

初回の本適用は Mac ホストではなく、こちらを使う:

```bash
./testenv/apple-container/scripts/up.sh --smoke
./testenv/apple-container/scripts/down.sh
```

公開後の正しさ確認 (GitHub から clone):

```bash
./testenv/apple-container/scripts/up.sh --smoke --from-github
```

詳細は
[testenv/apple-container/README.md](../testenv/apple-container/README.md)。
