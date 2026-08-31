# rig

[English](README.md)

`rig` は、自分の Mac や Linux に開発環境を入れるコマンドである。
ノートでも Mini でも、各マシンで同じ手順を実行する。

入るのはシェル、そのマシン向けのパッケージ、マシン同士の SSH である。

Rust は不要である。リリースのバイナリを入れれば足りる。付属ファイルは
初回実行でディスクに書き出される。

## 必要なもの

インストールには `curl` と `tar` が必要である。

`rig apply` は、macOS では Homebrew (`brew`)、Debian/Ubuntu では
`apt` でパッケージを入れる。brew と apt 自体は rig が入れない。
パッケージを飛ばすなら:

```bash
rig apply --yes --skip-packages
```

## インストール

```bash
curl -fsSL https://raw.githubusercontent.com/yamanori99/rig/main/install.sh | sh
```

`rig` は `~/.local/bin` に入る。そのパスは zsh / bash の起動ファイルにも
追加される。終わったら端末を閉じ、開き直す。

最新版: <https://github.com/yamanori99/rig/releases>

インストール先やバージョンを変える変数は、パイプの右側 (`sh`) に
付ける。`curl` だけに付けても意味がない。

```bash
curl -fsSL https://raw.githubusercontent.com/yamanori99/rig/main/install.sh \
  | RIG_BIN_DIR=/tmp/rig-bin sh
```

`RIG_VERSION=vX.Y.Z` も同じである。

## 更新

```bash
rig update           # preview
rig update --yes
```

`~/.local/bin/rig` だけ入れ替わる。`hosts/` と `overlay/` はそのまま
残る。バージョンが上がっていれば、次の実行で templates が更新される。

## アンインストール

```bash
curl -fsSL \
  https://raw.githubusercontent.com/yamanori99/rig/main/uninstall.sh | sh
```

`~/.local/bin/rig` を消す。hosts / overlay / state は残る。まとめて
消すなら:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/yamanori99/rig/main/uninstall.sh \
  | RIG_PURGE=1 sh
```

`rig apply` が入れたパッケージやシェル設定は、これでは元に戻らない。
戻すなら、バイナリが残っているうちに `rig clean --yes` を先に実行する。

## 使い方

1 台目:

```bash
rig init --role workstation   # or: --role compute
rig apply            # preview
rig apply --yes
rig status
```

続けて:

```bash
rig ssh-config --yes
rig keys distribute --yes
rig check
```

### ファイルの場所

データは次のディレクトリに置かれる。

- macOS: `~/Library/Application Support/dev.rig.rig/product/`
- Linux: だいたい `~/.local/share/rig/product/`

正確なパスは `rig root` の 1 行目である。このリポジトリの checkout や
`--root` を使うと、そちらが根になる。

```text
$(rig root)/
  hosts/
    examples/              # サンプル。触らない
      workstation.toml
      compute.toml
    m4-mba-neva.toml       # 自分のマシン。編集する
    m4-mini-tak.toml
  overlay/                 # 自分用の shell / tmux / Cursor
  templates/               # 既定値。触らない
  roles/
  packages/
```

`[[ssh]]` は、接続先のマシンの toml に書く。

`name` は短いホスト名である。`m4-mini-tak` であり、
`m4-mini-tak.local` ではない。

### 2 台目

`rig apply` が見るのは、上の `hosts/` だけである。マシン定義を入れた
git (`~/rig-hosts` など) は、symlink するまで別ディレクトリである。

```text
~/rig-hosts/               # 非公開の git
  m4-mba-neva.toml
  m4-mini-tak.toml

$(rig root)/hosts  ->  ~/rig-hosts
```

`~/rig-hosts` で `git pull` しても、symlink していないマシンでは
`rig apply` に反映されない。2 台目では:

```bash
ln -sfn ~/rig-hosts "$(rig root | head -1)/hosts"
rig host detect            # このマシンの name が出ること
rig apply --yes
```

git に toml がすでにあるなら `rig init` は実行しない。init は
`hosts/` が空のときにファイルを作るコマンドである。

`File exists` と出たら、そのパスにファイルか壊れた symlink がある。
リンクを直す。init はやり直さない。

## ロール

| Role | 内容 |
| --- | --- |
| `workstation` | GUI のあるノート / デスクトップ。シェルは zsh |
| `compute` | 画面なし。シェルは bash。SSH、画面共有、スリープしない |

## コマンド

```text
rig -h                           # 概要 (ロゴ)
rig -v                           # バージョン
rig s | status                   # このマシン
rig a -y | apply -y              # preview; -y で書く
rig c | check
rig i | init [-R role] [-n HOST]
rig h list | host list
rig roles [NAME] [-o macos|linux]
rig k distribute -y | keys ...
rig ssh-config -y | ssh -y
rig u -y | update -y [-f]
rig root                         # データのパス (stdout 1 行目)
rig apply [-y] [-S]              # -S で packages を飛ばす
rig clean [-y] [-p]              # -p でパッケージ削除
```

多くのコマンドは、データのパスを stderr に出す。長い説明は
`rig COMMAND --help` を見る。

## 開発者向け

[Rust](https://rustup.rs/) が必要である。通常の利用では読まなくてよい。

```bash
git clone https://github.com/yamanori99/rig.git
cd rig
RIG_FORCE_SOURCE=1 ./install.sh
# or: cargo install --path crates/rig --force
```

checkout の中ではそのツリーを使う。そうでなければ埋め込みファイルを
展開する。タグ `v*` で GitHub Actions がバイナリを出す。

Linux の確認 (Apple `container`):

```bash
./testenv/apple-container/scripts/up.sh --smoke
./testenv/apple-container/scripts/up.sh --smoke --from-release
```

[testenv/apple-container/README.md](testenv/apple-container/README.md)

push の前に `gitleaks protect --staged -c .gitleaks.toml` を実行する。

## Status

`v0.2.18` — help のロゴ、`-v`、短縮、値を黄色。

## License

[MIT](LICENSE)
