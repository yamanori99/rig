#!/bin/bash
# Authorize mounted controller key, then run sshd.
set -euo pipefail
SSH_DIR="/home/dev/.ssh"
AUTH_KEYS="${SSH_DIR}/authorized_keys"

shopt -s nullglob
for key in /mounted-keys/*.pub; do
  line="$(tr -d '\n\r' < "${key}")"
  [[ -n "${line}" ]] || continue
  grep -qxF "${line}" "${AUTH_KEYS}" 2>/dev/null || echo "${line}" >> "${AUTH_KEYS}"
done
shopt -u nullglob

chmod 600 "${AUTH_KEYS}"
chown -R dev:dev "${SSH_DIR}"
exec /usr/sbin/sshd -D
