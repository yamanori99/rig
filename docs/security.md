# 秘密情報スキャン (gitleaks)

## 初回の公開 push の前に

```bash
cd ~/rig
gh auth refresh -h github.com   # gh の token が無効なとき
git add -A
gitleaks protect --staged -c .gitleaks.toml
# 問題なければコミット → パブリック repo 作成 → push
```

## 公開ツリーが動くことの確認

bind-mount スモークだけに頼らない。push 後:

```bash
./testenv/apple-container/scripts/up.sh --smoke --from-github
./testenv/apple-container/scripts/down.sh
```

ゲスト内で `https://github.com/yamanori99/rig.git` を clone して
init / apply する (URL は `--from-github=URL` で変更可)。

## ファイルシステム全体のスキャン (うるさい)

gitignore 済みのローカル鍵も見る:

```bash
gitleaks detect --source . --no-git -c .gitleaks.toml
```

想定されるローカル専用ヒット:
`testenv/**/.generated/` の SSH 秘密鍵
(`scripts/gen-keys.sh` が作り、gitignore 済み)。
