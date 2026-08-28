#!/bin/sh
# Fetch a prebuilt binary when Releases exist. Until then:
#   cargo install --path crates/rig
set -eu

echo "rig install"
echo "Prebuilt releases are not published yet."
echo "From a clone:"
echo "  cargo install --path crates/rig"
echo "  cd <rig-root> && rig init && rig apply --dry-run"
exit 1
