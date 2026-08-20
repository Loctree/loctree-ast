#!/usr/bin/env python3
"""W3-00 resident-authority spike: cold CLI vs resident LSP, same query.

Measures, on the REAL installed binaries and the REAL loctree-suite snapshot:

  1. `loct --version`            — process-startup baseline (no snapshot load)
  2. `loct slice <TARGET>`       — cold surface: new process, load+parse
                                   snapshot.json (+ compute slice) every run
  3. loctree-lsp `loctree/slice` — resident surface: one warm process,
                                   snapshot parsed once, per-request latency
  4. resident cold-start         — initialize→initialized→first successful
                                   slice (the one-time price of the daemon)

No production code is touched. Repro:

    python3 experiments/resident-authority-spike/spike.py [N]

𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
"""

import json
import os
import statistics
import subprocess
import sys
import time

REPO = os.environ.get(
    "SPIKE_REPO", "/Users/maciejgad/vc-workspace/Loctree/loctree-suite"
)
TARGET = os.environ.get("SPIKE_TARGET", "loctree-rs/src/types.rs")
N = int(sys.argv[1]) if len(sys.argv) > 1 else 7


def median_ms(samples):
    return round(statistics.median(samples) * 1000.0, 1)


def bench_subprocess(cmd, n):
    """Run cmd n times, wall-clock each run (cold: fresh process every time)."""
    samples = []
    for _ in range(n):
        t0 = time.monotonic()
        subprocess.run(
            cmd, cwd=REPO, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
            check=True,
        )
        samples.append(time.monotonic() - t0)
    return samples


class LspClient:
    """Minimal stdio JSON-RPC client speaking LSP framing."""

    def __init__(self, argv):
        self.proc = subprocess.Popen(
            argv, cwd=REPO, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
        )
        self._id = 0

    def _send(self, msg):
        body = json.dumps(msg).encode()
        self.proc.stdin.write(
            b"Content-Length: %d\r\n\r\n%s" % (len(body), body)
        )
        self.proc.stdin.flush()

    def _read_message(self):
        headers = {}
        while True:
            line = self.proc.stdout.readline()
            if not line:
                raise RuntimeError("LSP server closed stdout")
            line = line.strip()
            if not line:
                break
            k, _, v = line.partition(b":")
            headers[k.strip().lower()] = v.strip()
        length = int(headers[b"content-length"])
        return json.loads(self.proc.stdout.read(length))

    def request(self, method, params):
        self._id += 1
        rid = self._id
        self._send({"jsonrpc": "2.0", "id": rid, "method": method, "params": params})
        while True:
            msg = self._read_message()
            if msg.get("id") == rid:
                return msg

    def notify(self, method, params):
        self._send({"jsonrpc": "2.0", "method": method, "params": params})

    def close(self):
        try:
            self.request("shutdown", None)
            self.notify("exit", None)
        finally:
            self.proc.kill()


def bench_lsp(n):
    lsp = LspClient(["loctree-lsp", "--root", REPO])
    t_start = time.monotonic()
    lsp.request(
        "initialize",
        {
            "processId": os.getpid(),
            "rootUri": "file://" + REPO,
            "capabilities": {},
            # keep the fs-watcher out of the measurement
            "initializationOptions": {"watcher": {"enabled": False}},
        },
    )
    lsp.notify("initialized", {})

    # Cold start of the resident: poll until the first slice succeeds
    # (snapshot load happens in `initialized`).
    first = None
    deadline = time.monotonic() + 300
    while time.monotonic() < deadline:
        resp = lsp.request("loctree/slice", {"target": TARGET})
        if "result" in resp and resp["result"] is not None:
            first = time.monotonic() - t_start
            break
        time.sleep(0.2)
    if first is None:
        lsp.close()
        raise RuntimeError("resident LSP never served a slice within 300s")

    # Warm resident: per-request round-trip on the SAME live process.
    samples = []
    for _ in range(n):
        t0 = time.monotonic()
        resp = lsp.request("loctree/slice", {"target": TARGET})
        assert "result" in resp, resp
        samples.append(time.monotonic() - t0)
    lsp.close()
    return first, samples


def main():
    ver = subprocess.run(
        ["loct", "--version"], capture_output=True, text=True, check=True
    ).stdout.strip()

    baseline = bench_subprocess(["loct", "--version"], N)
    cold_cli = bench_subprocess(["loct", "slice", TARGET], N)
    resident_cold, resident_warm = bench_lsp(N)

    out = {
        "loct_version": ver,
        "repo": REPO,
        "target": TARGET,
        "n": N,
        "median_ms": {
            "cli_process_baseline": median_ms(baseline),
            "cold_cli_slice": median_ms(cold_cli),
            "resident_lsp_slice_warm": median_ms(resident_warm),
        },
        "resident_cold_start_ms": round(resident_cold * 1000.0, 1),
        "all_samples_ms": {
            "cold_cli_slice": [round(s * 1000, 1) for s in cold_cli],
            "resident_lsp_slice_warm": [round(s * 1000, 1) for s in resident_warm],
        },
    }
    print(json.dumps(out, indent=2))


if __name__ == "__main__":
    main()
