#!/usr/bin/env python3
"""Loctree-first PreToolUse guard — ZERO FALLBACK policy.

Blocks standalone grep/rg commands when they map the git repo containing the
session cwd. Since loct 0.14.x, `loct find --regex` covers the full raw-text
surface (metacharacters, (?i), docs/markdown/comments, coverage accounting
with trustworthy absence), so there is no deliberate-fallback escape hatch:
`command grep` / `command rg` are blocked the same as bare grep/rg.

Still allowed by design:
- grep as a downstream pipe filter (`cargo test | grep FAILED`)
- searches outside the working repo
- grep reading stdin with no repo target

Set LOCTREE_FIRST_GUARD=0 (or "off") to disable the guard entirely.
Any parsing uncertainty fails open: this is workflow guidance, not a
security boundary.
"""

import glob as globmod
import json
import os
import re
import shlex
import sys


GREP_RE = re.compile(r"(?:command\s+)?(grep|rg|egrep|fgrep)\b")
PREFIX_RE = re.compile(r"^(sudo\s+|env(\s+\w+=\S*)*\s+)+")

MESSAGE = (
    "LOCTREE FIRST — ZERO FALLBACK: grep/rg on the working repo is "
    "blocked; 'command grep'/'command rg' no longer bypass this. "
    "`loct find` covers the whole search surface now:\n"
    "  identifier      loct find NAME            "
    "(exact boundary truth; multi-OR: loct find A B)\n"
    "  real regex      loct find --regex 'PAT'   "
    "(full metachars, (?i) for case-insensitive; scans raw text "
    "incl. markdown/comments/configs)\n"
    "  grep -c         loct find --regex 'PAT' --count-only\n"
    "  grep -l         loct find NAME --group-by-file\n"
    "  machine output  --json  · paging: --limit N --offset N · "
    "everything: --all\n"
    "  definition      loct find NAME --where-symbol\n"
    "  importers       loct find path/to/file --who-imports\n"
    "Regex mode prints a coverage line (scanned X of Y indexed files) "
    "— absence is trustworthy for the scanned universe, unlike grep "
    "silence. When exclusions>0 the guarantee is conditional "
    "(unindexed/ignored paths may still match). "
    "Caveats: the scan universe is the indexed snapshot — run "
    "'loct scan' first if you just created files; in --regex mode "
    "ignore --lang/--file scoping (unreliable) and filter output "
    "instead.\n"
    "If loct genuinely cannot express the query, do NOT fall back "
    "to grep — append the gap to "
    "~/.vibecrafted/loctree/loctree-fail.md and report the blocker. "
    "Pipe filters (cmd | grep) and searches outside the working "
    "repo remain allowed. (Disable guard: LOCTREE_FIRST_GUARD=0)\n"
)


def repo_root(path: str):
    """Return the nearest ancestor containing .git, or None."""
    try:
        candidate = os.path.realpath(os.path.expanduser(path))
        if not os.path.isdir(candidate):
            candidate = os.path.dirname(candidate)
        while True:
            if os.path.exists(os.path.join(candidate, ".git")):
                return candidate
            parent = os.path.dirname(candidate)
            if parent == candidate:
                return None
            candidate = parent
    except Exception:
        return None


def grep_targets(head: str, cwd: str):
    """Return existing path arguments, or None when the command is not mapping."""
    try:
        tokens = shlex.split(head)
    except ValueError:
        return None

    if tokens and tokens[0] == "command":
        tokens = tokens[1:]
    command_name = os.path.basename(tokens[0]) if tokens else ""
    targets = []
    recursive = False
    for token in tokens[1:]:
        if token in ("-r", "-R", "--recursive") or (
            token.startswith("-")
            and not token.startswith("--")
            and ("r" in token or "R" in token)
        ):
            recursive = True
        if token.startswith("-"):
            continue
        expanded = os.path.expanduser(token)
        candidate = expanded if os.path.isabs(expanded) else os.path.join(cwd, expanded)
        if any(character in candidate for character in "*?["):
            targets.extend(globmod.glob(candidate)[:8])
        elif os.path.exists(candidate):
            targets.append(candidate)

    # ripgrep searches cwd by default; classic grep only walks cwd when asked
    # to recurse and otherwise commonly consumes stdin.
    if not targets and not recursive and command_name != "rg":
        return None
    return targets


def main() -> int:
    """Refuse the tool call (exit 2) when a segment of the command greps the
    working repo; every other shape, and any parsing doubt, exits 0."""
    if os.environ.get("LOCTREE_FIRST_GUARD", "1").lower() in ("0", "off", "false"):
        return 0

    try:
        payload = json.load(sys.stdin)
        command = (payload.get("tool_input") or {}).get("command", "")
        cwd = payload.get("cwd") or os.getcwd()
    except Exception:
        return 0

    if not command:
        return 0
    work_repo = repo_root(cwd)
    if work_repo is None:
        return 0

    for segment in re.split(r"&&|;|\n", command):
        head = segment.split("|", 1)[0]
        head = re.sub(r"^[\s({]+", "", head)
        head = PREFIX_RE.sub("", head)
        if not GREP_RE.match(head):
            continue
        targets = grep_targets(head, cwd)
        if targets is None:
            continue
        scope = targets if targets else [cwd]
        if any(repo_root(target) == work_repo for target in scope):
            sys.stderr.write(MESSAGE)
            return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
