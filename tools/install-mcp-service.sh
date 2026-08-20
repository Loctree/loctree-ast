#!/usr/bin/env bash
set -euo pipefail
# install-mcp-service.sh — background HTTP MCP server daemon for Loctree (macOS launchd).
#
# Installs a per-user LaunchAgent that runs
#   loctree-mcp --transport http --bind 127.0.0.1:5174 --snapshot-cache-capacity 20
# with KeepAlive=true, so the Universal Streamable HTTP MCP service stays available 24/7
# across machine restarts without manual intervention.
#
# Usage:
#   bash tools/install-mcp-service.sh              # install / refresh service
#   bash tools/install-mcp-service.sh --uninstall  # stop and remove service
#
# Auth: the server itself refuses to start on a NON-loopback bind unless bearer
# tokens are configured or LOCTREE_MCP_ALLOW_UNAUTHENTICATED=1 is set. This
# script only forwards the setting; it does not decide policy. There is no TLS
# in-process — put a reverse proxy or a tailnet in front of any non-loopback
# bind. Mint tokens with:
#   loctree-mcp token create --id <name> --scope context-read
#
# Env overrides:
#   LOCTREE_MCP_BIND                  bind address (default: 127.0.0.1:5174)
#   LOCTREE_MCP_CACHE_CAPACITY        in-memory snapshot cache size (default: 20)
#   LOCTREE_MCP_BIN                   explicit loctree-mcp binary path
#   LOCTREE_MCP_TOKEN_STORE           bearer token store path (forwarded to the agent)
#   LOCTREE_MCP_AUTH_TOKEN            single shared bearer token (forwarded)
#   LOCTREE_MCP_ALLOW_UNAUTHENTICATED forwarded; opens a non-loopback port

LABEL="io.vetcoders.loctree.mcp"
PLIST="$HOME/Library/LaunchAgents/$LABEL.plist"
LOG_DIR="$HOME/.loctree/logs"
BIND="${LOCTREE_MCP_BIND:-127.0.0.1:5174}"
CAPACITY="${LOCTREE_MCP_CACHE_CAPACITY:-20}"

note() { printf '  %s\n' "$*"; }

if [ "$(uname -s)" != "Darwin" ]; then
  note "loctree mcp service: skipped (launchd-only; this host is $(uname -s))"
  exit 0
fi

gui_domain() { printf 'gui/%s' "$(id -u)"; }

if [ "${1:-}" = "--uninstall" ]; then
  launchctl bootout "$(gui_domain)/$LABEL" 2>/dev/null || true
  rm -f "$PLIST"
  note "loctree mcp service: removed ($LABEL)"
  exit 0
fi

# Resolve binary
LOCTREE_MCP_BIN="${LOCTREE_MCP_BIN:-}"
if [ -z "$LOCTREE_MCP_BIN" ]; then
  if [ -x "$HOME/.local/bin/loctree-mcp" ]; then
    LOCTREE_MCP_BIN="$HOME/.local/bin/loctree-mcp"
  elif [ -x "$HOME/.cargo/bin/loctree-mcp" ]; then
    LOCTREE_MCP_BIN="$HOME/.cargo/bin/loctree-mcp"
  else
    LOCTREE_MCP_BIN="$(command -v loctree-mcp || true)"
  fi
fi

if [ -z "$LOCTREE_MCP_BIN" ] || [ ! -x "$LOCTREE_MCP_BIN" ]; then
  note "loctree mcp service: skipped (loctree-mcp not found — build/install loctree first)"
  exit 0
fi

LOCT_DIR="$(dirname "$LOCTREE_MCP_BIN")"

LOCTREE_MCP_BIN_XML="$(printf '%s' "$LOCTREE_MCP_BIN" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/"/\&quot;/g' -e "s/'/\&apos;/g")"
BIND_XML="$(printf '%s' "$BIND" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/"/\&quot;/g' -e "s/'/\&apos;/g")"
CAPACITY_XML="$(printf '%s' "$CAPACITY" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/"/\&quot;/g' -e "s/'/\&apos;/g")"
PATH_XML="$(printf '%s' "$LOCT_DIR:/usr/bin:/bin:$HOME/.local/bin:$HOME/.cargo/bin:/opt/homebrew/bin" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/"/\&quot;/g' -e "s/'/\&apos;/g")"

xml_escape() { printf '%s' "$1" | sed -e 's/&/\&amp;/g' -e 's/</\&lt;/g' -e 's/>/\&gt;/g' -e 's/"/\&quot;/g' -e "s/'/\&apos;/g"; }

# Forward the auth-relevant environment into the LaunchAgent. launchd does not
# inherit the installing shell's environment, so without this a token exported
# at install time would be invisible to the daemon — and a non-loopback bind
# would then refuse to start with no obvious cause.
AUTH_ENV_XML=""
for key in LOCTREE_MCP_TOKEN_STORE LOCTREE_MCP_AUTH_TOKEN LOCTREE_MCP_ALLOW_UNAUTHENTICATED; do
  value="${!key:-}"
  [ -n "$value" ] || continue
  AUTH_ENV_XML="$AUTH_ENV_XML
  <key>$key</key>
  <string>$(xml_escape "$value")</string>"
done

case "$BIND" in
  127.*|localhost:*|\[::1\]:*) ;;
  *)
    note "loctree mcp service: WARNING — $BIND is not loopback."
    note "  The server refuses to start unless bearer tokens exist or"
    note "  LOCTREE_MCP_ALLOW_UNAUTHENTICATED=1. There is no in-process TLS."
    ;;
esac

mkdir -p "$HOME/Library/LaunchAgents" "$LOG_DIR"

cat > "$PLIST" <<PLIST_EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key>
  <string>$LABEL</string>
  <key>ProgramArguments</key>
  <array>
    <string>$LOCTREE_MCP_BIN_XML</string>
    <string>--transport</string>
    <string>http</string>
    <string>--bind</string>
    <string>$BIND_XML</string>
    <string>--snapshot-cache-capacity</string>
    <string>$CAPACITY_XML</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key>
    <string>$PATH_XML</string>$AUTH_ENV_XML
  </dict>
  <key>RunAtLoad</key>
  <true/>
  <key>KeepAlive</key>
  <true/>
  <key>StandardOutPath</key>
  <string>$LOG_DIR/loctree-serve-http.log</string>
  <key>StandardErrorPath</key>
  <string>$LOG_DIR/loctree-serve-http.log</string>
</dict>
</plist>
PLIST_EOF

plutil -lint "$PLIST" >/dev/null

# Idempotent refresh
launchctl bootout "$(gui_domain)/$LABEL" 2>/dev/null || true
launchctl bootstrap "$(gui_domain)" "$PLIST"

if launchctl print "$(gui_domain)/$LABEL" >/dev/null 2>&1; then
  note "loctree mcp service: running on http://$BIND/mcp via $LABEL"
  note "loctree mcp logs: $LOG_DIR/loctree-serve-http.log"
else
  note "loctree mcp service: plist written to $PLIST (bootstrap failed or pending user session)"
fi
