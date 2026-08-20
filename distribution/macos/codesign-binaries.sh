#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Usage: $0 <bin-dir>"
  exit 1
fi

BIN_DIR="$1"

if [[ ! -d "$BIN_DIR" ]]; then
  echo "Missing bin dir: $BIN_DIR"
  exit 1
fi

# Two signing modes:
#   * Developer ID (release): MACOS_DEVELOPER_ID_APPLICATION set -> hardened,
#     timestamped, notarization-ready signature.
#   * Ad-hoc fallback (local install): env unset -> plain ad-hoc re-sign
#     (flags=0x2 adhoc). cargo/ld leave a linker-signed ad-hoc signature
#     (flags=0x20002 adhoc,linker-signed) on freshly built binaries, and macOS
#     taskgated can SIGKILL those after a reboot ("Code Signature Invalid") even
#     though `codesign --verify` reports them valid on disk. A full `codesign
#     -f -s -` re-sign strips the linker-signed bit and fixes it permanently.
developer_id="${MACOS_DEVELOPER_ID_APPLICATION:-}"

for bin in loct loctree loctree-mcp loctree-lsp aicx aicx-mcp; do
  target="$BIN_DIR/$bin"
  if [[ ! -f "$target" ]]; then
    echo "Skipping absent binary: $target"
    continue
  fi
  if [[ -n "$developer_id" ]]; then
    codesign --force --timestamp --options runtime --sign "$developer_id" "$target"
  else
    codesign --force --sign - "$target"
  fi
  codesign --verify --verbose=2 "$target" >/dev/null
  echo "Codesigned: $target"
done
