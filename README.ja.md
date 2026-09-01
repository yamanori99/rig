# rig

[English](README.md)

`rig` は、自分の Mac や Linux に開発環境を入れるコマンドである。
ノートでも Mini でも、各マシンで同じ手順を実行する。

入るのはシェル、そのマシン向けのパッケージ、マシン同士の SSH である。

Rust は不要である。リリースのバイナリを入れれば足りる。付属ファイルは
初回実行でディスクに書き出される。

## セットアップ

各マシンでこの順に進める。1 台だけなら apply まででよい。
複数台なら、ホストファイル、接続、`host check` / `host keys` まで続ける。

### 1. パッケージマネージャ

`rig apply` は、macOS では Homebrew (`brew`)、Debian/Ubuntu では
`apt` でパッケージを入れる。brew と apt 自体は rig が入れない。

- macOS: 先に [Homebrew](https://brew.sh/) を入れる
- Debian/Ubuntu: `apt` は既にある

パッケージを飛ばすなら:

```bash
rig apply --yes --skip-packages
```

### 2. マシン名

`rig init` / `rig apply` は、短いホスト名 (`hostname -s`) でこの
マシンを探す。その文字列がホストファイルの `name` になる。
OS のホスト名は rig が変えない。

Mac では init の前に、システム設定 > 一般 > 共有のコンピュータ名 /
ローカルホスト名を決める。`.local` は付けない (`m4-mini-tak` であり、
`m4-mini-tak.local` ではない)。

### 3. rig のインストール

`curl` と `tar` が必要である。

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

### 4. init と apply

```bash
rig init -R workstation   # or: -R compute
rig apply                 # preview
rig apply -y
rig status
```

init は `~/.rig-hosts/<name>.toml` が無いときだけ書く。
inventory が既にある (コピーや git) なら init は実行しない。
`File exists` と出たら、そのパスにファイルがある。init はやり直さない。

### 5. 他のマシン

マシン同士で SSH するときだけ必要である。

1. 各マシンの `~/.rig-hosts/` に、全ホストの toml を置く
   (`[[ssh]]` は接続先のマシンの toml に書く)。
2. 相手側でも同じようにインストール、名前、init (空なら)、
   `rig apply -y` を実行する。
3. 機器を接続できる状態にする (同じ LAN / Thunderbolt / VPN、SSH
   待ち受け)。macOS 12.1 以降、compute の apply はリモートマネジメント
   (`:5900`) を付ける。許可はシステム設定 > 一般 > 共有で
   リモートマネジメントをオン。画面共有スイッチではない。
4. 外に接続する各マシンで:

```bash
rig host check            # TCP/22、続いて ssh
rig host keys -y          # ssh が失敗したとき (新規ピアはパスワード 1 回)
rig host check            # ssh ok を確認
```

いっぺんに何機も設定するなら、check / keys / check を**各マシンで**
実行する。鍵は片方向なので、相手側も自分の鍵を配る必要がある。

ピアの toml を足したり直したら、もう一度 apply する
(または `rig host ssh-config -y`)。`~/.ssh/config.d/rig.conf` が
更新される。

`rig apply` が見るのは `~/.rig-hosts/` である。製品の `hosts/` ではない。
GitHub にログインする機材なら、`~/.rig-hosts/` を git で管理すると便利である。

## ロール

| Role | 内容 |
| --- | --- |
| `workstation` | GUI のあるノート / デスクトップ。シェルは zsh |
| `compute` | 画面なし。シェルは bash。SSH、画面共有、スリープしない |

macOS ではロールのシェルを Homebrew のバイナリにする。
workstation は zsh、compute は bash。`/bin/zsh` と `/bin/bash` は使わない。

## ファイルの場所

データは次のディレクトリに置かれる。

- macOS: `~/Library/Application Support/dev.rig.rig/product/`
- Linux: だいたい `~/.local/share/rig/product/`

正確なパスは `rig root` の 1 行目である。このリポジトリの checkout や
`--root` を使うと、そちらが根になる。

```text
~/.rig-hosts/              # マシン定義の正本。実ディレクトリ
  m4-mba-neva.toml
  m4-mini-tak.toml

$(rig root)/
  hosts/examples/          # サンプル。触らない
  overlay/                 # 自分用の shell / tmux / Cursor
  templates/               # 既定値。触らない
  roles/
  packages/
```

## コマンド

```text
init, i     ~/.rig-hosts/<name>.toml を書く
apply, a    このホストに載せる (preview。-y で実行)
            --undo -y   apply を戻す
status, s   このマシンを見る
host, h     list | check | keys

-v          バージョン
-r          データの根  (cwd / RIG_ROOT / 展開)
-h          help        (長い説明は --help)
```

多くのコマンドは、データのパスを stderr に出す。長い説明は
`rig COMMAND --help` である。

## 更新

```bash
rig update           # preview
rig update --yes
```

`~/.local/bin/rig` だけ入れ替わる。`~/.rig-hosts/` と `overlay/` はそのまま
残る。バージョンが上がっていれば、次の実行で templates が更新される。

## アンインストール

```bash
curl -fsSL \
  https://raw.githubusercontent.com/yamanori99/rig/main/uninstall.sh | sh
```

`~/.local/bin/rig` を消す。~/.rig-hosts / overlay / state は残る。まとめて
消すなら:

```bash
curl -fsSL \
  https://raw.githubusercontent.com/yamanori99/rig/main/uninstall.sh \
  | RIG_PURGE=1 sh
```

`rig apply` が入れたパッケージやシェル設定は、これでは元に戻らない。
戻すなら、バイナリが残っているうちに `rig apply --undo --yes` を先に実行する。

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

## License

[MIT](LICENSE)
