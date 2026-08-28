# rig 計画書

最終更新: 2026-08-29  
製品名: **rig**  
作業ツリー: `~/rig` (既存 `~/dotfiles` とは分離)

---

## 1. 目的

個人向け dotfiles 置き場ではなく、**誰でもそこそこに開発機 / 計算ノードを立てられる環境セットアップパッケージ**をつくる。

| やりたいこと | やりたくないこと |
| --- | --- |
| workstation / compute で用途を分ける | 全マシンに同じ Brewfile を流し込む |
| zsh / bash 両対応 | zsh + Oh My Zsh 前提の固定 |
| 適用・検証・同期・撤去を CLI で完結 | 個人 IP・実機一覧を GitHub に載せる |
| 利用者に Rust toolchain を要求しない | 既存 `dotfiles` をそのまま製品化する |

旧 `~/dotfiles` は当面残す。実 IP の追加 push はしない。製品の正は **`~/rig`**。

---

## 2. 確定した方針

| 項目 | 決定 |
| --- | --- |
| 名前 / CLI | `rig` |
| 実装言語 | Rust (適用エンジン本体)。利用者はバイナリのみ |
| bootstrap | 薄い `install.sh` (POSIX sh)。初回はバイナリ取得のみ |
| 設定 | TOML (`hosts/`, `roles/`) |
| シェル | zsh と bash 同格 |
| パッケージ | role がセットを束ねる (`common` + `workstation` または `compute`) |
| リポジトリ | **新規パブリック** GitHub (作成・push は確認後) |
| 個人情報 | 実 `hosts/*.toml` は gitignore。examples のみ track |
| バージョン | SemVer (`0.x` から)。`rig --version` / tag / Releases |

---

## 3. アーキテクチャ

```text
┌─────────────────────────────────────┐
│  製品コア (repo に載せる)            │
│  crates/rig, roles/, packages/,     │
│  templates/, hosts/examples/        │
└─────────────────────────────────────┘
              │ rig apply
              ▼
┌─────────────────────────────────────┐
│  利用者データ (載せない / ローカル)  │
│  hosts/<実機>.toml, overlay/        │
└─────────────────────────────────────┘
```

Rust (`rig`) が担うもの:

- `hosts` / `roles` の読み込み・検証
- 適用計画 (`apply --dry-run`) と冪等な実行
- SSH config 生成
- 複数ホストの status / sync (予定)
- マニフェストに基づく `clean` / `uninstall` (予定)
- Releases からの `self-update` (予定)

Rust が担わないもの:

- `templates/` の文章そのもの (データ)
- `brew` / `apt` / `ssh` の中身 (呼び出すだけ)
- 初回バイナリ入手 (`install.sh`)

薄いラッパ (旧 `setup.sh` 呼び出しだけ) にはしない。適用の状態機械は Rust に置く。

---

## 4. ディレクトリ構成

```text
~/rig/
├── README.md
├── install.sh                 # バイナリ取得 (Releases 整備後)
├── Cargo.toml                 # workspace
├── crates/rig/                # CLI
├── roles/
│   ├── workstation.toml
│   └── compute.toml
├── packages/
│   ├── brew/{common,workstation,compute}.Brewfile
│   └── apt/{common,workstation,compute}.list
├── templates/
│   ├── shell/{common,zsh,bash}/
│   └── ssh/config.header
├── hosts/
│   ├── examples/              # プレースホルダのみ track
│   └── *.toml                 # 実機 — gitignore
├── overlay/                   # 個人上書き — gitignore 推奨
└── docs/
    ├── plan.md                # 本資料
    └── quickstart.md
```

---

## 5. データモデル

### Host (`hosts/<name>.toml`)

```toml
name = "my-laptop"
role = "workstation"       # workstation | compute | (将来拡張)
schema_version = 1
# os = "macos" | "linux"
# shell = "zsh" | "bash"
# user = "..."             # 省略時 $USER
# vpn / lan / thunderbolt  # 任意。実値はローカルのみ
# [packages]
# add = []
# remove = []
```

### Role (`roles/*.toml`)

- `packages`: セット名の列 (`common`, `workstation`, …)
- `features`: gui / cursor / remote_login / tailscale / thunderbolt
- `default_shell`: 省略時のシェル傾向

### パッケージ解決

```text
role.packages + host.packages.add - host.packages.remove
  → OS に応じて brew または apt を実行
```

| セット | 内容の意図 |
| --- | --- |
| common | 全機材の薄い土台 |
| workstation | GUI・開発フル |
| compute | ヘッドレス・計算寄り。GUI cask なし |

---

## 6. CLI

| コマンド | 状態 | 役割 |
| --- | --- | --- |
| `rig init` | 実装済 | 実ホスト TOML を examples から生成 |
| `rig host list\|detect` | 実装済 | 登録一覧 / 自ホスト検出 |
| `rig roles [NAME] [--os …]` | 実装済 | role・features・パッケージ一覧 |
| `rig apply [--dry-run]` | dry-run 実装済 | 適用 (本番実行は次段) |
| `rig ssh-config` | 生成表示まで | hosts から SSH 定義生成 |
| `rig clean [--yes] [--packages]` | 枠のみ | マニフェスト逆順で撤去 |
| `rig uninstall` | 未 | clean + バイナリ削除 |
| `rig status\|sync\|run` | 未 | 複数マシン運用 |
| `rig self-update` | 未 | Releases から更新 |

Apply ステップ (計画上):

1. detect / validate  
2. link (shell templates)  
3. packages  
4. ssh-config  
5. hostname  
6. features (gui, cursor, remote login, tailscale, thunderbolt)  
7. keys (任意)

Apply 時に state マニフェスト (`~/.local/share/rig/...` 等) を書き、`clean` の正とする。

---

## 7. プライバシーと配布

- track: 製品コード、examples、共有 templates / packages  
- 非 track: 実 IP、実ホスト名一覧、Cursor 個人設定、鍵  
- 新 GitHub は **パブリック**想定。公開前に `gitleaks protect --staged`
  (実 IP・鍵は gitignore / examples のみ)  
- 将来パブリック化する場合は、examples 以外に個人情報が混ざっていないことを再確認する

既存 `yamanori99/dotfiles` のリネームではなく、**新規 repo** に載せる。

---

## 8. ロードマップ

### Phase 0 — 骨格 (完了)

- [x] `~/rig` ワークスペースと Cargo  
- [x] schema / roles / package sets / shell templates  
- [x] `init` / `host` / `roles` / `apply --dry-run` / `ssh-config` 表示  
- [x] 実 hosts の gitignore  
- [x] 本計画書

### Phase 1 — 適用エンジン

- [x] symlink / copy による shell・ssh header 配置  
- [x] brew bundle / apt の実行と結果解釈  
- [ ] hostname・features の macOS 実装 (Linux は枠)  
- [x] state マニフェスト書き込み  
- [ ] `rig clean --yes` の逆操作  
- [ ] unit test (schema / plan / ssh-config)  
- [x] Apple `container` Linux スモーク (`testenv/apple-container`)

### Phase 2 — 配布と同期

- [ ] GitHub パブリック repo 作成 (確認後; 事前 gitleaks)  
- [ ] Release CI (darwin/linux 各 arch)  
- [ ] `install.sh` の認証付き取得  
- [ ] `rig self-update`  
- [ ] `status` / `sync` / `run`

### Phase 3 — 厚み

- [ ] Linux features 本実装  
- [ ] overlay マージ  
- [ ] `uninstall`  
- [ ] スキーマ移行 (`schema_version`)  
- [ ] (任意) Homebrew tap

---

## 9. 成功条件

- 知らない人が README だけで `workstation` を dry-run → apply できる (招待・認証の範囲で)  
- `compute` を足せて、入るパッケージが workstation と明確に違う  
- zsh / bash どちらでも apply できる  
- rustup 不要 (Releases 整備後)  
- `apply` が旧 `setup.sh` に依存しない  
- 不正な hosts.toml は検証段階で落ちる  
- `clean --yes` で管理下の配置・features・(指定時) パッケージを撤去できる  
- 製品 repo に実ラボ IP が載っていない  

---

## 10. やらないこと (当面)

- Ansible / Nix への全面移行  
- GUI インストーラ  
- 全言語ランタイムの強制インストール  
- 適用ロジックの shell 丸投げ  
- 既存 public/private `dotfiles` への個人情報の追加 push  

---

## 11. 開発者向けメモ

```bash
cd ~/rig
cargo install --path crates/rig --force
rig init --role workstation   # hosts/<hostname>.toml は gitignore
rig apply --dry-run
```

GitHub への初回 push・`gh repo create` は、ユーザー確認後に行う。
