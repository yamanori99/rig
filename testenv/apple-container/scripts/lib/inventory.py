#!/usr/bin/env python3
"""Parse testenv/apple-container/inventory.toml for fleet scripts.

Minimal TOML subset (no third-party deps): [[node]] tables with name/role.
"""
from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ASSIGN = re.compile(
    r'^(name|role|cpus|memory)\s*=\s*(?:"([^"]*)"|\'([^\']*)\'|(\S+))\s*$'
)


def load(path: Path) -> list[dict]:
    text = path.read_text()
    nodes: list[dict] = []
    cur: dict | None = None
    for raw in text.splitlines():
        line = raw.split("#", 1)[0].strip()
        if not line:
            continue
        if line == "[[node]]":
            if cur is not None:
                nodes.append(cur)
            cur = {}
            continue
        if cur is None:
            continue
        m = ASSIGN.match(line)
        if not m:
            continue
        key = m.group(1)
        val = m.group(2) if m.group(2) is not None else (
            m.group(3) if m.group(3) is not None else m.group(4)
        )
        cur[key] = val
    if cur is not None:
        nodes.append(cur)

    if not nodes:
        raise SystemExit(f"no [[node]] entries in {path}")

    out = []
    for i, n in enumerate(nodes):
        name = str(n.get("name", "")).strip()
        role = str(n.get("role", "")).strip()
        if not name or not role:
            raise SystemExit(f"node[{i}]: name and role are required")
        if role not in ("workstation", "compute"):
            raise SystemExit(
                f"node[{i}]: role must be workstation|compute, got {role!r}"
            )
        out.append(
            {
                "name": name,
                "role": role,
                "cpus": n.get("cpus"),
                "memory": n.get("memory"),
            }
        )
    names = [n["name"] for n in out]
    if len(names) != len(set(names)):
        raise SystemExit(f"duplicate node names in {path}")
    return out


def main() -> None:
    if len(sys.argv) < 2:
        print(
            "usage: inventory.py names|tsv|json [inventory.toml]",
            file=sys.stderr,
        )
        raise SystemExit(2)
    cmd = sys.argv[1]
    root = Path(__file__).resolve().parents[2]
    path = Path(sys.argv[2]) if len(sys.argv) > 2 else root / "inventory.toml"
    nodes = load(path)
    if cmd == "names":
        for n in nodes:
            print(n["name"])
    elif cmd == "tsv":
        for n in nodes:
            print(f"{n['name']}\t{n['role']}")
    elif cmd == "json":
        print(json.dumps(nodes))
    else:
        print(f"unknown command: {cmd}", file=sys.stderr)
        raise SystemExit(2)


if __name__ == "__main__":
    main()
