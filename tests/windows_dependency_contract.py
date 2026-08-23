#!/usr/bin/env python3
"""Ensure the MCP binary keeps its cross-platform dependencies on Windows."""

from __future__ import annotations

import json
from pathlib import Path
import subprocess


ROOT = Path(__file__).resolve().parents[1]
metadata = json.loads(
    subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    ).stdout
)
package = next(item for item in metadata["packages"] if item["name"] == "loctree-mcp")
dependencies = {dependency["name"]: dependency for dependency in package["dependencies"]}

for name in ("clap", "tracing", "tracing-subscriber"):
    dependency = dependencies[name]
    if dependency["target"] is not None:
        raise SystemExit(
            f"loctree-mcp dependency {name} became target-scoped: {dependency['target']}"
        )

libc_target = dependencies["libc"]["target"]
if libc_target != "cfg(unix)":
    raise SystemExit(f"loctree-mcp libc target drifted from cfg(unix): {libc_target}")

print("loctree-mcp Windows dependency contract: PASS")
