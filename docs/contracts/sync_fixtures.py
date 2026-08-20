#!/usr/bin/env python3
"""Mirror overlay-intent-v1 fixtures into the aicx repo, byte-identical.

Contract (C0-01): fixtures live canonically in
`loctree-suite/docs/contracts/fixtures/overlay-intent-v1/` and are mirrored
into `$AICX_ROOT/tests/fixtures/overlay-intent-v1/` with sha256 equality.

Usage:
    LOCTREE_SUITE_ROOT=... AICX_ROOT=... python3 sync_fixtures.py          # copy + verify
    LOCTREE_SUITE_ROOT=... AICX_ROOT=... python3 sync_fixtures.py --check  # verify only

stdlib only — no third-party deps.
"""

import hashlib
import os
import pathlib
import shutil
import sys


def sha256(path: pathlib.Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    check_only = "--check" in sys.argv[1:]
    suite_root = os.environ.get("LOCTREE_SUITE_ROOT")
    aicx_root = os.environ.get("AICX_ROOT")
    if not suite_root or not aicx_root:
        print("FAIL: export LOCTREE_SUITE_ROOT and AICX_ROOT (DRIVER §3)", file=sys.stderr)
        return 2

    src = pathlib.Path(suite_root) / "docs/contracts/fixtures/overlay-intent-v1"
    dst = pathlib.Path(aicx_root) / "tests/fixtures/overlay-intent-v1"
    fixtures = sorted(src.glob("*.json"))
    if not fixtures:
        print(f"FAIL: no fixtures under {src}", file=sys.stderr)
        return 1

    if not check_only:
        dst.mkdir(parents=True, exist_ok=True)
        for f in fixtures:
            shutil.copyfile(f, dst / f.name)

    bad = []
    for f in fixtures:
        mirror = dst / f.name
        if not mirror.exists():
            bad.append(f"{f.name}: missing in mirror")
        elif sha256(mirror) != sha256(f):
            bad.append(f"{f.name}: sha256 mismatch")
    stray = sorted(set(p.name for p in dst.glob("*.json")) - set(f.name for f in fixtures))
    for name in stray:
        bad.append(f"{name}: stray file in mirror (not in canon)")

    if bad:
        for line in bad:
            print(f"FAIL: {line}", file=sys.stderr)
        return 1
    print(f"SYNC OK: {len(fixtures)} fixtures byte-identical in both repos")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
