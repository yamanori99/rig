#!/bin/sh
# rig bootstrap — fetch prebuilt binary when Releases exist.
# Until then, build from a checkout:
#   cargo install --path crates/rig
set -eu

echo "rig install"
echo "Prebuilt Relases are not published yet."
echo "From a clone:"
echo "  cargo install --path crates/rig"
echo "  cd <rig-root> && rig init && rig apply --dry-run"
exit 1
