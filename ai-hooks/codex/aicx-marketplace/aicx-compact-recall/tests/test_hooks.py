#!/usr/bin/env python3
"""Isolated fixture contracts for the compact-recall shell hooks."""

from __future__ import annotations

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
PRE = ROOT / "scripts" / "aicx-precompact.sh"
POST = ROOT / "scripts" / "aicx-postcompact.sh"
FIXTURE = ROOT / "tests" / "fixtures" / "adversarial_conversation.md"
SID = "11111111-2222-4333-8444-555555555555"


class HookFixtures(unittest.TestCase):
    def setUp(self) -> None:
        self.tmp = tempfile.TemporaryDirectory(prefix="aicx-hook-fixture-")
        self.home = Path(self.tmp.name)
        self.transcript = self.home / ".codex" / "sessions" / "fixture.jsonl"
        self.transcript.parent.mkdir(parents=True)
        self.transcript.write_text('{"type":"session_meta"}\n', encoding="utf-8")
        self.argv_log = self.home / "argv.json"
        self.fake = self.home / "fake-aicx"
        self.fake.write_text(
            """#!/usr/bin/env python3
import json, os, pathlib, shutil, sys
pathlib.Path(os.environ["FAKE_AICX_ARGV"]).write_text(json.dumps(sys.argv[1:]))
if os.environ.get("FAKE_AICX_FAIL") == "1":
    raise SystemExit(7)
out = pathlib.Path(sys.argv[sys.argv.index("-o") + 1])
shutil.copyfile(os.environ["FAKE_AICX_EXTRACT"], out)
""",
            encoding="utf-8",
        )
        self.fake.chmod(0o755)

    def tearDown(self) -> None:
        self.tmp.cleanup()

    def env(self, **extra: str) -> dict[str, str]:
        env = os.environ.copy()
        env.update(
            {
                "HOME": str(self.home),
                "AICX_BIN": str(self.fake),
                "AICX_HOOK_AGENT": "codex",
                "FAKE_AICX_ARGV": str(self.argv_log),
                "FAKE_AICX_EXTRACT": str(FIXTURE),
            }
        )
        env.update(extra)
        return env

    def precompact(self, **extra: str) -> subprocess.CompletedProcess[bytes]:
        payload = json.dumps(
            {
                "session_id": SID,
                "transcript_path": str(self.transcript),
                "hook_event_name": "PreCompact",
            }
        ).encode()
        return subprocess.run(
            ["bash", str(PRE)], input=payload, env=self.env(**extra), capture_output=True
        )

    def postcompact(self, **payload_extra: object) -> subprocess.CompletedProcess[bytes]:
        payload_obj: dict[str, object] = {
            "session_id": SID,
            "hook_event_name": "SessionStart",
            "source": "compact",
            "transcript_path": str(self.transcript),
        }
        payload_obj.update(payload_extra)
        payload = json.dumps(payload_obj).encode()
        return subprocess.run(
            ["bash", str(POST)], input=payload, env=self.env(), capture_output=True
        )

    def test_precompact_uses_exact_direct_file_argv_and_atomic_output(self) -> None:
        result = self.precompact()
        self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
        argv = json.loads(self.argv_log.read_text(encoding="utf-8"))
        self.assertEqual(argv[:5], ["extract", "codex", "--file", str(self.transcript), "--conversation"])
        self.assertEqual(argv[5], "-o")
        self.assertRegex(
            argv[6],
            rf"^.+/\.aicx/extracts/codex/{SID}_conversation\.md\.tmp\.\d+$",
        )
        extract = self.home / ".aicx" / "extracts" / "codex" / f"{SID}_conversation.md"
        self.assertEqual(extract.read_bytes(), FIXTURE.read_bytes())

    def test_digest_deduplicates_blocks_and_preserves_latest_p0_state(self) -> None:
        self.assertEqual(self.precompact().returncode, 0)
        result = self.postcompact()
        self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
        text = result.stdout.decode("utf-8")
        self.assertLess(len(result.stdout), 12_000)
        self.assertIn("CURRENT ASK: preserve the canonical direct-file contract", text)
        self.assertIn("CURRENT HANDOFF: fixtures are green and activation is next", text)
        self.assertNotIn("dedup-ref", text)
        self.assertNotIn("SECRET_REASONING_MUST_NOT_LEAK", text)
        recent = text.split("RECENT TURNS", 1)[1].split("APPENDIX", 1)[0]
        lines = [
            line.strip()
            for line in recent.splitlines()
            if line.strip().startswith("[") and " — " in line
        ]
        self.assertEqual(
            lines,
            [
                "[10:00:00] user — First ask",
                "[10:01:00] assistant — First handoff",
                "[10:02:00] user — CURRENT ASK: preserve the canonical direct-file contract",
                "[10:03:00] assistant — CURRENT HANDOFF: fixtures are green and activation is next",
            ],
        )

    def test_invalid_utf8_controls_and_private_reasoning_are_not_emitted(self) -> None:
        unsafe = self.home / "unsafe.md"
        unsafe.write_bytes(
            b"**[11:00:00] user:**\n\n> SAFE ASK\x00\x1b\xff\n\n"
            b"**[11:01:00] assistant analysis:**\n\n> PRIVATE_CHAIN\n\n"
            b"**[11:02:00] assistant:**\n\n> SAFE HANDOFF\n"
        )
        result = self.precompact(FAKE_AICX_EXTRACT=str(unsafe))
        self.assertEqual(result.returncode, 0)
        result = self.postcompact()
        self.assertEqual(result.returncode, 0)
        text = result.stdout.decode("utf-8")
        self.assertIn("SAFE ASK", text)
        self.assertIn("SAFE HANDOFF", text)
        self.assertNotIn("PRIVATE_CHAIN", text)
        self.assertNotIn("\x00", text)
        self.assertNotIn("\x1b", text)

    def test_failures_are_fail_open_and_loud_in_diagnostics(self) -> None:
        result = self.precompact(FAKE_AICX_FAIL="1")
        self.assertEqual(result.returncode, 0)
        log = self.home / ".aicx" / "state" / f"precompact-codex-{SID}.log"
        self.assertTrue(log.exists())
        fallback = self.postcompact()
        self.assertEqual(fallback.returncode, 0)
        self.assertIn(b"POSTCOMPACT RECALL DEGRADED", fallback.stdout)

    def test_append_growth_after_seal_still_recalls(self) -> None:
        """Codex appends multi-MB `compacted` events after PreCompact seals."""
        self.assertEqual(self.precompact().returncode, 0)
        sealed = self.transcript.stat().st_size
        with self.transcript.open("ab") as fh:
            fh.write(
                b'{"type":"compacted","payload":{"replacement_history":"'
                + (b"x" * 4096)
                + b'"}}\n'
            )
        self.assertGreater(self.transcript.stat().st_size, sealed)
        result = self.postcompact()
        self.assertEqual(result.returncode, 0, result.stderr.decode(errors="replace"))
        text = result.stdout.decode("utf-8")
        self.assertIn("AICX RECALL", text)
        self.assertNotIn("POSTCOMPACT RECALL DEGRADED", text)
        self.assertNotIn("freshness mismatch", text)

    def test_transcript_shrink_degrades(self) -> None:
        # Seed a larger rollout so the post-seal rewrite is a true shrink.
        self.transcript.write_text('{"type":"session_meta"}\n' * 40, encoding="utf-8")
        self.assertEqual(self.precompact().returncode, 0)
        sealed = self.transcript.stat().st_size
        self.transcript.write_text('{"type":"session_meta"}\n', encoding="utf-8")
        self.assertLess(self.transcript.stat().st_size, sealed)
        result = self.postcompact()
        self.assertEqual(result.returncode, 0)
        text = result.stdout.decode("utf-8")
        self.assertIn("POSTCOMPACT RECALL DEGRADED", text)
        self.assertIn("freshness shrink", text)

    def test_seal_is_one_shot_after_successful_recall(self) -> None:
        self.assertEqual(self.precompact().returncode, 0)
        first = self.postcompact()
        self.assertEqual(first.returncode, 0, first.stderr.decode(errors="replace"))
        self.assertIn(b"AICX RECALL", first.stdout)
        second = self.postcompact()
        self.assertEqual(second.returncode, 0)
        self.assertIn(b"POSTCOMPACT RECALL DEGRADED", second.stdout)
        self.assertIn(b"freshness sidecar missing", second.stdout)


if __name__ == "__main__":
    unittest.main(verbosity=2)
