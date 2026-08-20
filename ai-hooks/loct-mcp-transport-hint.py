#!/usr/bin/env python3
"""Agent PostToolUse hint for stale MCP transports.

Updated: 2026-06-06

This hook is intentionally informative-only. It watches tool results for MCP
transport failure signatures and tells the agent how to recover without
blocking or mutating the tool call.
"""

from __future__ import annotations

import json
import sys
from typing import Any


MCP_FAILURE_NEEDLES = (
    "broken pipe",
    "connection closed",
    "transport closed",
    "transport down",
    "transport error",
    "mcp transport",
    "handshaking",
    "initialize response",
    "mcp startup failed",
)


def load_payload() -> dict[str, Any]:
    try:
        payload = json.load(sys.stdin)
    except Exception:
        return {}
    return payload if isinstance(payload, dict) else {}


def stringify(value: object) -> str:
    try:
        return json.dumps(value, ensure_ascii=False, default=str)
    except Exception:
        return str(value)


def main() -> int:
    payload_text = stringify(load_payload()).lower()
    if any(needle in payload_text for needle in MCP_FAILURE_NEEDLES):
        print(
            json.dumps(
                {
                    "systemMessage": (
                        "MCP transport likely went stale/down. Refresh tool "
                        "discovery with tool_search for the affected MCP, then "
                        "rerun your call. Fallback to CLI <command> if still broken."
                    )
                }
            )
        )
    else:
        print(json.dumps({"continue": True}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
