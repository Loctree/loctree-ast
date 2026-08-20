#!/usr/bin/env bash
# Install the Loctree-first discipline and AICX compaction continuity.
#
# This is a runtime installer, not a pile of copy commands. It owns the hook
# registrations it creates, removes the legacy Memex hook payload, preserves
# unrelated user hooks, and runs the AICX doctor before claiming success.

set -euo pipefail

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd "$SCRIPT_DIR/.." && pwd)
HOOKS_DIR="$REPO_ROOT/ai-hooks"

INSTALL_LOCTREE=false
INSTALL_AICX=false

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

info() { printf '%b\n' "${CYAN}i${NC} $*"; }
pass() { printf '%b\n' "${GREEN}OK${NC} $*"; }
warn() { printf '%b\n' "${YELLOW}WARN${NC} $*" >&2; }
fail() { printf '%b\n' "${RED}FAIL${NC} $*" >&2; exit 1; }
header() { printf '\n%b\n' "${BOLD}$*${NC}"; }

detect_claude() { command -v claude >/dev/null 2>&1 || [[ -d "$HOME/.claude" ]]; }
detect_codex() { command -v codex >/dev/null 2>&1 || [[ -d "$HOME/.codex" ]]; }
detect_gemini() { command -v gemini >/dev/null 2>&1 || [[ -d "$HOME/.gemini" ]]; }

select_packages() {
    header "Select hook packages"
    printf '  1) loctree  - map-first guard + structural augmentation\n'
    printf '  2) aicx     - protected pre/post-compact continuity\n'
    printf '  3) all      - both (recommended)\n'
    local choice
    read -rp 'Select [1/2/3]: ' choice
    case "${choice:-3}" in
        1) INSTALL_LOCTREE=true ;;
        2) INSTALL_AICX=true ;;
        3) INSTALL_LOCTREE=true; INSTALL_AICX=true ;;
        *) fail "Unknown selection: $choice" ;;
    esac
}

select_packages_from_env() {
    case "${HOOKS:-}" in
        '') select_packages ;;
        loctree) INSTALL_LOCTREE=true ;;
        aicx) INSTALL_AICX=true ;;
        all|both) INSTALL_LOCTREE=true; INSTALL_AICX=true ;;
        memex)
            fail "The legacy Memex hook package was removed. Use HOOKS=aicx for compaction continuity."
            ;;
        *) fail "Unknown HOOKS value: ${HOOKS}" ;;
    esac
}

require_payload() {
    local path="$1"
    [[ -e "$path" ]] || fail "Required hook payload is missing: $path"
}

backup_once() {
    local path="$1"
    [[ -f "$path" ]] || return 0
    cp "$path" "$path.backup.$(date +%Y%m%d%H%M%S)"
}

remove_legacy_memex_files() {
    local hooks_dir="$1"
    rm -f \
        "$hooks_dir/memex-context.sh" \
        "$hooks_dir/memex-startup.sh" \
        "$hooks_dir/memory-on-compact.sh"
}

update_claude_settings() {
    local settings="$1"
    command -v jq >/dev/null 2>&1 || fail "jq is required to update Claude hooks without destroying unrelated settings"
    [[ -f "$settings" ]] || printf '%s\n' '{}' > "$settings"
    backup_once "$settings"

    local tmp="${settings}.tmp.$$"
    jq \
        --argjson install_loctree "$INSTALL_LOCTREE" \
        --argjson install_aicx "$INSTALL_AICX" '
        def owned_or_legacy:
            ((.command // "") | test(
              "(memex-context|memex-startup|memory-on-compact)\\.sh"
            ))
            or ($install_loctree and ((.command // "") | test(
              "(loctree-first-guard|loct-grep-augment)\\.(sh|py)"
            )))
            or ($install_aicx and ((.command // "") | test(
              "(aicx-precompact|aicx-postcompact)\\.sh"
            )));
        def clean_groups:
            map(
              .hooks = ((.hooks // []) | map(select(owned_or_legacy | not)))
            ) | map(select((.hooks // []) | length > 0));

        .hooks = (.hooks // {})
        | .hooks |= with_entries(.value |= clean_groups)
        | if $install_loctree then
            .hooks.PreToolUse = ((.hooks.PreToolUse // []) + [{
              "matcher": "Bash",
              "hooks": [{
                "type": "command",
                "command": "python3 ~/.claude/hooks/loctree-first-guard.py",
                "timeout": 5
              }]
            }])
            | .hooks.PostToolUse = ((.hooks.PostToolUse // []) + [{
              "matcher": "Grep",
              "hooks": [{
                "type": "command",
                "command": "bash ~/.claude/hooks/loct-grep-augment.sh",
                "timeout": 15
              }]
            }])
          else . end
        | if $install_aicx then
            .hooks.PreCompact = ((.hooks.PreCompact // []) + [{
              "hooks": [{
                "type": "command",
                "command": "bash ~/.claude/hooks/aicx-precompact.sh",
                "timeout": 30,
                "statusMessage": "aicx extract before compact..."
              }]
            }])
            | .hooks.PostCompact = ((.hooks.PostCompact // []) + [{
              "hooks": [{
                "type": "command",
                "command": "bash ~/.claude/hooks/aicx-postcompact.sh",
                "timeout": 15,
                "statusMessage": "Restoring AICX recall after compact..."
              }]
            }])
          else . end
    ' "$settings" > "$tmp"
    mv "$tmp" "$settings"
}

install_claude() {
    header "Claude Code"
    local hooks_dir="$HOME/.claude/hooks"
    local settings="$HOME/.claude/settings.json"
    mkdir -p "$hooks_dir"
    remove_legacy_memex_files "$hooks_dir"

    if $INSTALL_LOCTREE; then
        local guard="$HOOKS_DIR/codex/loctree-marketplace/loctree-first/hooks/loctree-first-guard.py"
        require_payload "$guard"
        install -m 0755 "$guard" "$hooks_dir/loctree-first-guard.py"
        install -m 0755 "$HOOKS_DIR/loct-grep-augment.sh" "$hooks_dir/loct-grep-augment.sh"
        install -m 0755 "$HOOKS_DIR/loct-smart-suggest.sh" "$hooks_dir/loct-smart-suggest.sh"
        pass "installed Loctree-first guard and augmentation"
    fi

    if $INSTALL_AICX; then
        for name in aicx-precompact.sh aicx-postcompact.sh aicx-recall-selftest.sh; do
            require_payload "$HOOKS_DIR/claude/$name"
            install -m 0755 "$HOOKS_DIR/claude/$name" "$hooks_dir/$name"
        done
        pass "installed Claude AICX pre/post-compact continuity"
    fi

    update_claude_settings "$settings"
    pass "preserved unrelated Claude hooks and removed legacy Memex registrations"

    if $INSTALL_AICX; then
        if [[ "${AI_HOOKS_SKIP_DOCTOR:-0}" == "1" ]]; then
            warn "AICX Claude self-test skipped by AI_HOOKS_SKIP_DOCTOR=1"
        else
            bash "$hooks_dir/aicx-recall-selftest.sh"
            pass "Claude AICX recall self-test"
        fi
    fi
}

replace_codex_marketplace() {
    local name="$1" path="$2"
    require_payload "$path/.agents/plugins/marketplace.json"
    codex plugin marketplace remove "$name" >/dev/null 2>&1 || true
    codex plugin marketplace add "$path" >/dev/null
    pass "registered Codex marketplace $name from $path"
}

snapshot_codex_plugin_cache() {
    local marketplace="$1" plugin="$2" snapshot="$3"
    local base="$HOME/.codex/plugins/cache/$marketplace/$plugin"
    mkdir -p "$snapshot"
    if [[ -d "$base" ]]; then
        cp -R "$base/." "$snapshot/"
    fi
}

restore_running_codex_cache() {
    local marketplace="$1" plugin="$2" snapshot="$3"
    local base="$HOME/.codex/plugins/cache/$marketplace/$plugin"
    local saved version target
    [[ -d "$snapshot" ]] || return 0
    for saved in "$snapshot"/*; do
        [[ -d "$saved" ]] || continue
        version=$(basename "$saved")
        target="$base/$version"
        if [[ ! -d "$target" ]]; then
            mkdir -p "$base"
            cp -R "$saved" "$target"
            warn "restored cache $plugin/$version for an already-running Codex process"
        fi
    done
}

install_codex() {
    header "Codex"
    command -v codex >/dev/null 2>&1 || fail "codex is required for Codex plugin installation"

    if $INSTALL_LOCTREE; then
        local marketplace="$HOOKS_DIR/codex/loctree-marketplace"
        local loctree_snapshot
        loctree_snapshot=$(mktemp -d "${TMPDIR:-/tmp}/loctree-first-cache.XXXXXX")
        snapshot_codex_plugin_cache "loctree-local" "loctree-first" "$loctree_snapshot"
        replace_codex_marketplace "loctree-local" "$marketplace"
        if ! codex plugin add loctree-first@loctree-local >/dev/null; then
            restore_running_codex_cache "loctree-local" "loctree-first" "$loctree_snapshot"
            fail "failed to install loctree-first@loctree-local"
        fi
        restore_running_codex_cache "loctree-local" "loctree-first" "$loctree_snapshot"
        rm -rf "$loctree_snapshot"
        pass "installed loctree-first@loctree-local"
    fi

    if $INSTALL_AICX; then
        local marketplace="$HOOKS_DIR/codex/aicx-marketplace"
        local aicx_snapshot
        aicx_snapshot=$(mktemp -d "${TMPDIR:-/tmp}/aicx-compact-cache.XXXXXX")
        snapshot_codex_plugin_cache "personal" "aicx-compact-recall" "$aicx_snapshot"
        replace_codex_marketplace "personal" "$marketplace"
        if ! codex plugin add aicx-compact-recall@personal >/dev/null; then
            restore_running_codex_cache "personal" "aicx-compact-recall" "$aicx_snapshot"
            fail "failed to install aicx-compact-recall@personal"
        fi
        restore_running_codex_cache "personal" "aicx-compact-recall" "$aicx_snapshot"
        rm -rf "$aicx_snapshot"
        pass "installed aicx-compact-recall@personal"

        if [[ "${AI_HOOKS_SKIP_DOCTOR:-0}" == "1" ]]; then
            warn "AICX Codex doctor skipped by AI_HOOKS_SKIP_DOCTOR=1"
        else
            "$marketplace/aicx-compact-recall/scripts/doctor.sh"
            pass "Codex AICX compact-recall doctor"
        fi
    fi
}

install_gemini() {
    header "Gemini"
    local hooks_dir="$HOME/.gemini/hooks"
    mkdir -p "$hooks_dir"
    remove_legacy_memex_files "$hooks_dir"

    if $INSTALL_LOCTREE; then
        sed 's/PostToolUse/AfterTool/g' "$HOOKS_DIR/loct-grep-augment.sh" > "$hooks_dir/loct-grep-augment.sh"
        chmod 0755 "$hooks_dir/loct-grep-augment.sh"
        pass "installed Gemini Loctree augmentation payload"
    fi
    if $INSTALL_AICX; then
        warn "AICX compact lifecycle is not yet proven for Gemini; no fake continuity hook was installed"
    fi
}

run_for_cli() {
    case "$1" in
        claude)
            if detect_claude; then install_claude; else warn "Claude not detected"; fi
            ;;
        codex)
            if detect_codex; then install_codex; else warn "Codex not detected"; fi
            ;;
        gemini)
            if detect_gemini; then install_gemini; else warn "Gemini not detected"; fi
            ;;
        *) fail "Unknown CLI: $1" ;;
    esac
}

main() {
    header "AI Hooks Runtime Installer"
    printf 'Loctree gives sight. AICX preserves continuity.\n'
    select_packages_from_env

    local cli_arg="${CLI:-}"
    if [[ -n "$cli_arg" ]]; then
        if [[ "$cli_arg" == "all" ]]; then
            if detect_claude; then install_claude; fi
            if detect_codex; then install_codex; fi
            if detect_gemini; then install_gemini; fi
        else
            local cli
            IFS=',' read -ra clis <<< "$cli_arg"
            for cli in "${clis[@]}"; do run_for_cli "$cli"; done
        fi
    else
        detect_claude && install_claude
        detect_codex && install_codex
        detect_gemini && install_gemini
    fi

    header "Installation complete"
    $INSTALL_LOCTREE && info "Loctree-first: ordinary first-choice grep/rg is paused; command grep/rg is the deliberate fallback."
    $INSTALL_AICX && info "AICX continuity: installed only on runtimes with a verified lifecycle contract."
    info "Restart or resume running agents; hook registries are not hot-reloaded."
}

main "$@"
