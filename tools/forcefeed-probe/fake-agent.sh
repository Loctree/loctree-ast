#!/usr/bin/env bash
# fake-agent.sh — no-op agent that captures the ACTUAL bootstrap payload fed by runner
# Usage: echo "$PAYLOAD" | ./fake-agent.sh --runner claude --capture /tmp/ff-capture.txt
# Before any reply, it writes the full received stdin+args to capture file.
# Then emits a deterministic no-op so harness knows it "ran".

set -euo pipefail

RUNNER="unknown"
CAPTURE=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --runner) RUNNER="$2"; shift 2 ;;
    --capture) CAPTURE="$2"; shift 2 ;;
    *) shift ;;
  esac
done

mkdir -p "$(dirname "${CAPTURE:-/tmp/ff-capture-${RUNNER}.txt}")"
CAPTURE="${CAPTURE:-/tmp/ff-capture-${RUNNER}.txt}"

# Capture EVERYTHING the runner actually fed (stdin is the prompt/context, args may carry metadata)
{
  echo "=== FAKE-AGENT CAPTURE ==="
  echo "runner: ${RUNNER}"
  echo "timestamp: $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "argv: $*"
  echo "=== PAYLOAD START ==="
  cat -
  echo "=== PAYLOAD END ==="
} > "${CAPTURE}"

# Prove we received something substantial
BYTES=$(wc -c < "${CAPTURE}" | tr -d ' ')
echo "FAKE-AGENT: captured ${BYTES} bytes for ${RUNNER}. NO-OP reply (no model call)."
exit 0
