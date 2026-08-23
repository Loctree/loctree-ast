#!/usr/bin/env python3
"""Fail unless the live Codex hook registry contains the protected trusted pair."""

import json
import os
import subprocess
import sys


def send(proc: subprocess.Popen[str], payload: dict) -> None:
    """Write one newline-delimited JSON-RPC message to the app-server stdin."""
    assert proc.stdin is not None
    proc.stdin.write(json.dumps(payload) + "\n")
    proc.stdin.flush()


proc = subprocess.Popen(
    ["codex", "app-server", "--stdio"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
    bufsize=1,
)

try:
    send(
        proc,
        {
            "method": "initialize",
            "id": 1,
            "params": {
                "clientInfo": {
                    "name": "aicx-compact-recall-doctor",
                    "title": "AICX Compact Recall Doctor",
                    "version": "1",
                },
                "capabilities": {
                    "experimentalApi": True,
                    "requestAttestation": False,
                },
            },
        },
    )
    send(proc, {"method": "initialized"})
    send(proc, {"method": "hooks/list", "id": 2, "params": {"cwds": [os.getcwd()]}})

    result = None
    assert proc.stdout is not None
    for line in proc.stdout:
        message = json.loads(line)
        if message.get("id") == 2:
            result = message.get("result")
            break
    if result is None:
        raise RuntimeError("hooks/list returned no result")

    hooks = [
        hook
        for entry in result.get("data", [])
        for hook in entry.get("hooks", [])
        if hook.get("pluginId") == "aicx-compact-recall@personal"
    ]
    expected = {
        ("preCompact", None),
        ("sessionStart", "compact"),
    }
    actual = {(hook.get("eventName"), hook.get("matcher")) for hook in hooks}
    if actual != expected:
        raise RuntimeError(f"wrong hook pair: {sorted(actual)!r}")
    for hook in hooks:
        if not hook.get("enabled"):
            raise RuntimeError(f"hook disabled: {hook.get('key')}")
        if hook.get("trustStatus") != "trusted":
            raise RuntimeError(
                f"hook is {hook.get('trustStatus')}: {hook.get('key')}"
            )
    print("PASS: live hook registry (trusted preCompact + compact-only recall)")
except Exception as exc:
    print(f"FAIL: live hook registry: {exc}", file=sys.stderr)
    sys.exit(1)
finally:
    proc.terminate()
    try:
        proc.wait(timeout=2)
    except subprocess.TimeoutExpired:
        proc.kill()
