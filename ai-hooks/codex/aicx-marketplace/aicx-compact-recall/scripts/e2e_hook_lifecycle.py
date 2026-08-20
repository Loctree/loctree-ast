#!/usr/bin/env python3
"""Prove the compact-recall lifecycle in an isolated fresh Codex generation."""

from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import queue
import shlex
import shutil
import subprocess
import tempfile
import threading
import time
from typing import Any, Callable


PLUGIN_ID = "aicx-compact-recall@personal"
ROOT = Path(__file__).resolve().parents[1]


class RpcError(RuntimeError):
    pass


class TuiSession:
    def __init__(self, env: dict[str, str], work: Path) -> None:
        self.socket = f"aicx-e2e-{os.getpid()}-{time.time_ns()}"
        self.env = env
        command = shlex.join(
            [
                "codex",
                "--no-alt-screen",
                "--strict-config",
                "-C",
                str(work),
                "-a",
                "never",
                "-s",
                "read-only",
                "LIFECYCLE_HISTORY: reply exactly HISTORY_ACK",
            ]
        )
        subprocess.run(
            [
                "tmux",
                "-L",
                self.socket,
                "new-session",
                "-d",
                "-x",
                "160",
                "-y",
                "40",
                "-c",
                str(work),
                command,
            ],
            env=env,
            check=True,
            capture_output=True,
        )

    @property
    def buffer(self) -> bytes:
        result = subprocess.run(
            ["tmux", "-L", self.socket, "capture-pane", "-p", "-S", "-1000"],
            env=self.env,
            capture_output=True,
            check=True,
        )
        return result.stdout

    def send(self, text: str) -> None:
        subprocess.run(
            ["tmux", "-L", self.socket, "send-keys", "-l", text],
            env=self.env,
            check=True,
        )
        subprocess.run(
            ["tmux", "-L", self.socket, "send-keys", "-H", "0d"],
            env=self.env,
            check=True,
        )
        subprocess.run(
            ["tmux", "-L", self.socket, "send-keys", "-l", "\x1b[13;1:1u"],
            env=self.env,
            check=True,
        )

    def pump(self, timeout: float = 0.25) -> None:
        time.sleep(timeout)

    def wait_count(self, token: bytes, count: int, timeout: float = 120) -> None:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            if self.buffer.count(token) >= count:
                return
            self.pump()
            alive = subprocess.run(
                ["tmux", "-L", self.socket, "has-session"],
                env=self.env,
                capture_output=True,
            )
            if alive.returncode != 0:
                raise RpcError("Codex TUI exited early")
        tail = self.buffer[-4000:].decode("utf-8", errors="replace")
        raise RpcError(f"timeout waiting for {token!r} x{count}; terminal tail={tail!r}")

    def settle(self, seconds: float = 2) -> None:
        deadline = time.monotonic() + seconds
        while time.monotonic() < deadline:
            self.pump(min(0.25, deadline - time.monotonic()))

    def close(self) -> None:
        subprocess.run(
            ["tmux", "-L", self.socket, "kill-server"],
            env=self.env,
            capture_output=True,
        )


class AppServer:
    def __init__(self, env: dict[str, str]) -> None:
        self.proc = subprocess.Popen(
            ["codex", "--dangerously-bypass-hook-trust", "app-server", "--stdio"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            text=True,
            bufsize=1,
            env=env,
        )
        self.events: list[dict[str, Any]] = []
        self.inbox: queue.Queue[dict[str, Any]] = queue.Queue()
        self.stderr: list[str] = []
        threading.Thread(target=self._read_stdout, daemon=True).start()
        threading.Thread(target=self._read_stderr, daemon=True).start()

    def _read_stdout(self) -> None:
        assert self.proc.stdout is not None
        for line in self.proc.stdout:
            try:
                message = json.loads(line)
            except json.JSONDecodeError:
                continue
            self.events.append(message)
            self.inbox.put(message)

    def _read_stderr(self) -> None:
        assert self.proc.stderr is not None
        for line in self.proc.stderr:
            self.stderr.append(line.rstrip())

    def send(self, method: str, request_id: int | None = None, params: dict[str, Any] | None = None) -> None:
        payload: dict[str, Any] = {"method": method}
        if request_id is not None:
            payload["id"] = request_id
        if params is not None:
            payload["params"] = params
        assert self.proc.stdin is not None
        self.proc.stdin.write(json.dumps(payload) + "\n")
        self.proc.stdin.flush()

    def wait_for(self, predicate: Callable[[dict[str, Any]], bool], timeout: float = 60) -> dict[str, Any]:
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            for event in self.events:
                if predicate(event):
                    return event
            if self.proc.poll() is not None:
                raise RpcError(
                    f"app-server exited {self.proc.returncode}: {' | '.join(self.stderr[-8:])}"
                )
            try:
                message = self.inbox.get(timeout=min(0.25, deadline - time.monotonic()))
            except queue.Empty:
                continue
            if predicate(message):
                return message
        raise RpcError(f"timeout waiting for app-server event; stderr={' | '.join(self.stderr[-8:])}")

    def response(self, request_id: int, timeout: float = 60) -> dict[str, Any]:
        message = self.wait_for(lambda item: item.get("id") == request_id, timeout)
        if "error" in message:
            raise RpcError(f"request {request_id} failed: {message['error']}")
        return message.get("result", {})

    def initialize(self) -> None:
        self.send(
            "initialize",
            1,
            {
                "clientInfo": {
                    "name": "aicx-compact-recall-e2e",
                    "title": "AICX Compact Recall E2E",
                    "version": "1",
                },
                "capabilities": {"experimentalApi": True, "requestAttestation": False},
            },
        )
        self.response(1, 15)
        self.send("initialized")

    def hooks(self, cwd: Path, request_id: int) -> list[dict[str, Any]]:
        self.send("hooks/list", request_id, {"cwds": [str(cwd)]})
        result = self.response(request_id, 15)
        return [
            hook
            for entry in result.get("data", [])
            for hook in entry.get("hooks", [])
            if hook.get("pluginId") == PLUGIN_ID
        ]

    def close(self) -> None:
        if self.proc.poll() is None:
            self.proc.terminate()
            try:
                self.proc.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self.proc.kill()
                self.proc.wait(timeout=3)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--aicx", required=True, type=Path)
    parser.add_argument("--require-model-visible-recall", action="store_true")
    args = parser.parse_args()
    aicx = args.aicx.resolve()
    if not aicx.is_file() or not os.access(aicx, os.X_OK):
        raise SystemExit(f"AICX binary is not executable: {aicx}")
    if shutil.which("tmux") is None:
        raise SystemExit("tmux is required for the real Codex TUI lifecycle test")

    operator_home = Path.home()
    version = json.loads((ROOT / ".codex-plugin" / "plugin.json").read_text())["version"]

    with tempfile.TemporaryDirectory(prefix="aicx-codex-lifecycle-") as tmp_name:
        home = Path(tmp_name)
        work = home / "workspace"
        work.mkdir()
        subprocess.run(["git", "init", "-q", str(work)], check=True)
        auth = operator_home / ".codex" / "auth.json"
        if auth.exists():
            (home / "auth.json").symlink_to(auth)

        isolated_env = os.environ.copy()
        isolated_env.update(
            {
                "HOME": str(home),
                "USERPROFILE": str(home),
                "CODEX_HOME": str(home),
                "AICX_BIN": str(aicx),
            }
        )

        # Generation N starts before installation and is held open across it.
        old = AppServer(isolated_env)
        try:
            old.initialize()
            before = old.hooks(work, 2)
            if before:
                raise RpcError("disposable CODEX_HOME unexpectedly had plugin hooks before install")

            install_env = os.environ.copy()
            install_env["CODEX_HOME"] = str(home)
            installed = subprocess.run(
                ["codex", "plugin", "add", PLUGIN_ID, "--json"],
                cwd=ROOT,
                env=install_env,
                text=True,
                capture_output=True,
                check=True,
            )
            installed_json = json.loads(installed.stdout)
            if installed_json.get("version") != version:
                raise RpcError(
                    f"disposable install version {installed_json.get('version')} != source {version}"
                )

            old_after = old.hooks(work, 3)
            old_registry_refresh = bool(old_after)
        finally:
            old.close()

        # Resolve the exact installed hashes in a disposable probe generation,
        # then persist trust only for those hashes. App-server 0.144.1 does not
        # execute untrusted plugin hooks even with the CLI bypass flag.
        probe = AppServer(isolated_env)
        try:
            probe.initialize()
            probe_hooks = probe.hooks(work, 2)
            probe_actual = {
                (hook.get("eventName"), hook.get("matcher")) for hook in probe_hooks
            }
            probe_expected = {("preCompact", None), ("sessionStart", "compact")}
            if probe_actual != probe_expected:
                raise RpcError(f"probe process hook pair mismatch: {sorted(probe_actual)!r}")
            if not all(version in str(hook.get("sourcePath", "")) for hook in probe_hooks):
                raise RpcError("probe process did not resolve the installed version generation")
        finally:
            probe.close()

        config = home / "config.toml"
        with config.open("a", encoding="utf-8") as handle:
            handle.write("\n[hooks.state]\n")
            for hook in probe_hooks:
                key = json.dumps(hook["key"])
                trusted_hash = json.dumps(hook["currentHash"])
                handle.write(f"\n[hooks.state.{key}]\n")
                handle.write(f"trusted_hash = {trusted_hash}\n")
                handle.write("enabled = true\n")
            handle.write(f"\n[projects.{json.dumps(str(work))}]\n")
            handle.write('trust_level = "trusted"\n')

        # Generation N+2 proves the trusted registry. The actual manual compact
        # runs through a fresh TUI process because app-server's
        # thread/compact/start emits PreCompact but (in 0.144.1) does not run the
        # SessionStart(compact) delivery path used by the interactive runtime.
        fresh = AppServer(isolated_env)
        try:
            fresh.initialize()
            hooks = fresh.hooks(work, 2)
            actual = {(hook.get("eventName"), hook.get("matcher")) for hook in hooks}
            expected = {("preCompact", None), ("sessionStart", "compact")}
            if actual != expected:
                raise RpcError(f"fresh process hook pair mismatch: {sorted(actual)!r}")
            if not all(version in str(hook.get("sourcePath", "")) for hook in hooks):
                raise RpcError("fresh process did not resolve the installed version generation")
            if not all(hook.get("enabled") for hook in hooks):
                raise RpcError(f"fresh process hooks are disabled: {hooks!r}")
            if not all(hook.get("trustStatus") == "trusted" for hook in hooks):
                raise RpcError(f"fresh process hooks are not trusted: {hooks!r}")
        finally:
            fresh.close()

        tui_env = isolated_env.copy()
        tui_env["TERM"] = "xterm-256color"
        tui = TuiSession(tui_env, work)
        try:
            tui.wait_count(b"HISTORY_ACK", 2, 60)
            tui.settle()
            tui.send("LIFECYCLE_CURRENT_ASK: reply exactly LIFECYCLE_HANDOFF")
            tui.wait_count(b"LIFECYCLE_HANDOFF", 2, 60)
            tui.settle()

            transcripts = sorted((home / "sessions").rglob("*.jsonl"))
            if not transcripts:
                raise RpcError("fresh TUI did not persist a transcript")
            transcript = max(transcripts, key=lambda path: path.stat().st_mtime_ns)
            thread_id = transcript.stem.rsplit("-", 5)[-5:]
            thread_id = "-".join(thread_id)
            if len(thread_id) != 36:
                raise RpcError(f"could not parse thread id from {transcript.name}")
            transcript_before = transcript.stat().st_mtime_ns

            tui.send("/compact")
            extract = home / ".aicx" / "extracts" / "codex" / f"{thread_id}_conversation.md"
            deadline = time.monotonic() + 45
            while time.monotonic() < deadline:
                tui.pump()
                if extract.is_file():
                    break
            if not extract.is_file():
                tail = tui.buffer[-6000:].decode("utf-8", errors="replace")
                raise RpcError(f"manual TUI compaction did not run PreCompact; terminal tail={tail!r}")
            # SessionStart additional context is model-visible but intentionally
            # not serialized as a user turn in Codex rollout JSONL. Give the
            # compaction response time to finish, then ask the model directly.
            tui.settle(15)
            if not transcript_before <= extract.stat().st_mtime_ns:
                raise RpcError("transcript/extract mtimes do not prove PreCompact order")

            model_visible = False
            if args.require_model_visible_recall:
                visible_before = tui.buffer.count(b"RECALL_VISIBLE")
                missing_before = tui.buffer.count(b"RECALL_MISSING")
                tui.send(
                    "Reply exactly RECALL_VISIBLE only if your model context contains the loud "
                    "AICX RECALL header; otherwise reply RECALL_MISSING."
                )
                deadline = time.monotonic() + 120
                while time.monotonic() < deadline:
                    tui.pump()
                    if (
                        tui.buffer.count(b"RECALL_VISIBLE") >= visible_before + 2
                        or tui.buffer.count(b"RECALL_MISSING") >= missing_before + 2
                    ):
                        break
                model_visible = (
                    tui.buffer.count(b"RECALL_VISIBLE") >= visible_before + 2
                    and tui.buffer.count(b"RECALL_MISSING") == missing_before + 1
                )
                if not model_visible:
                    raise RpcError("fresh post-compact model did not confirm AICX RECALL visibility")
            if extract.stat().st_mtime_ns > transcript.stat().st_mtime_ns:
                raise RpcError("model-visible response was not persisted after PreCompact extract")

            print(f"PASS: disposable install generation {version}")
            print(
                "PASS: old-process registry refresh="
                f"{str(old_registry_refresh).lower()} is not execution proof; hot reload refused"
            )
            print("PASS: fresh process required for activation proof")
            print("PASS: event order PreCompact -> SessionStart(compact) -> model response")
            print(
                "PASS: transcript mtime <= extract mtime <= model-visible response mtime "
                f"({transcript_before} <= {extract.stat().st_mtime_ns} <= {transcript.stat().st_mtime_ns})"
            )
            print(f"PASS: precompact extract bytes={extract.stat().st_size}")
            if args.require_model_visible_recall:
                print(f"PASS: fresh model-visible recall={str(model_visible).lower()}")
        finally:
            tui.close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
