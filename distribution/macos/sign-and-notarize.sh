#!/usr/bin/env bash
set -euo pipefail

# Sign every binary in <dist-dir> with the Developer ID certificate, zip the
# directory, and submit the zip for Apple notarization.
#
# publish.yml calls this from the macOS build jobs and provides:
#   MACOS_DEVELOPER_ID_APPLICATION  Developer ID Application signing identity
#   APPLE_API_KEY_BASE64            base64-encoded App Store Connect .p8 key
#   APPLE_API_KEY_ID                App Store Connect API key id
#   APPLE_API_ISSUER_ID             App Store Connect API issuer id
#
# History: the original v0.10.x script was deleted at the v0.10.2 baseline
# (6d398f11) while publish.yml kept calling it. It also authenticated with an
# Apple ID app-specific password (APPLE_ID / APPLE_TEAM_ID /
# APPLE_APP_SPECIFIC_PASSWORD) that publish.yml never passed, and signed only
# `loctree`/`loct` — leaving `loct-mcp` staged by the MCP jobs unsigned, which
# notarization rejects. This restored version consumes the API-key envs the
# workflow actually passes and signs every file in the dist dir.

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <dist-dir> <output-zip>" >&2
  exit 1
fi

DIST_DIR="$1"
OUTPUT_ZIP="$2"

: "${MACOS_DEVELOPER_ID_APPLICATION:?Set MACOS_DEVELOPER_ID_APPLICATION}"
: "${APPLE_API_KEY_BASE64:?Set APPLE_API_KEY_BASE64}"
: "${APPLE_API_KEY_ID:?Set APPLE_API_KEY_ID}"
: "${APPLE_API_ISSUER_ID:?Set APPLE_API_ISSUER_ID}"

if [[ ! -d "$DIST_DIR" ]]; then
  echo "Missing dist dir: $DIST_DIR" >&2
  exit 1
fi

shopt -s nullglob
signed=0
for target in "$DIST_DIR"/*; do
  [[ -f "$target" ]] || continue
  codesign --force --timestamp --options runtime \
    --sign "$MACOS_DEVELOPER_ID_APPLICATION" "$target"
  codesign --verify --verbose=2 "$target" >/dev/null
  echo "Codesigned: $target"
  signed=$((signed + 1))
done

if [[ "$signed" -eq 0 ]]; then
  echo "No binaries found to sign in $DIST_DIR" >&2
  exit 1
fi

rm -f "$OUTPUT_ZIP"
ditto -c -k --keepParent "$DIST_DIR" "$OUTPUT_ZIP"

KEY_DIR="$(mktemp -d)"
trap 'rm -rf "$KEY_DIR"' EXIT
KEY_PATH="$KEY_DIR/AuthKey_${APPLE_API_KEY_ID}.p8"
# `base64 --decode` is GNU-only; BSD base64 on macOS wants `-D`. openssl
# ships on every macOS runner and decodes both wrapped and single-line input.
printf '%s' "$APPLE_API_KEY_BASE64" | openssl base64 -d -A > "$KEY_PATH"

xcrun notarytool submit \
  "$OUTPUT_ZIP" \
  --key "$KEY_PATH" \
  --key-id "$APPLE_API_KEY_ID" \
  --issuer "$APPLE_API_ISSUER_ID" \
  --wait

echo "Notarized archive ready: $OUTPUT_ZIP"
