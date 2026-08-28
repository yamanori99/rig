#!/bin/sh
# rig bootstrap — Releases 整備後はここからバイナリを取得する。
# 当面はチェックアウトからビルド:
#   cargo install --path crates/rig
set -eu

echo "rig install"
echo "まだ prebuilt Release は公開していません。"
echo "クローンしたリポジトリで:"
echo "  cargo install --path crates/rig"
echo "  cd <rig-root> && rig init && rig apply --dry-run"
exit 1
