#!/usr/bin/env bash
# Shebang + executable bit: a runtime entrypoint by role. Nothing imports a
# release script, and nothing ever will — being unimported is its normal state.
set -euo pipefail
swift build -c release
