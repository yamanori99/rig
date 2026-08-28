# Security scan (gitleaks)

## Before the first public push

```bash
cd ~/rig
gh auth refresh -h github.com   # if gh reports invalid token
git add -A
gitleaks protect --staged -c .gitleaks.toml
# then commit, create public repo, push (confirm with yourself / agent)
```

## Prove the published tree works

Do **not** rely on bind-mount smoke alone. After push:

```bash
./testenv/apple-container/scripts/up.sh --smoke --from-github
./testenv/apple-container/scripts/down.sh
```

That clones `https://github.com/yamanori99/rig.git` (override with
`--from-github=URL`) inside Apple `container` and runs init/apply there.

## Filesystem scan (noisy)

Includes gitignored local keys:

```bash
gitleaks detect --source . --no-git -c .gitleaks.toml
```

Expected local-only hit: `testenv/**/.generated/` SSH private keys
(gitignored; created by `scripts/gen-keys.sh`).
