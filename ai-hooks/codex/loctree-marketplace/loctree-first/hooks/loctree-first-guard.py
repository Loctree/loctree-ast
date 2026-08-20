#!/usr/bin/env python3
"""Keep repo mapping Loctree-first without policing deliberate fallbacks.

The hook blocks standalone grep/rg commands only when they map the git repo
containing the Codex session cwd. Searches outside that repo and grep used as
a downstream pipe filter remain valid. ``command grep`` and ``command rg`` are
explicit operator-grade fallbacks: the extra word proves the choice was
deliberate, so the hook allows them. Any uncertainty fails open because this is
workflow guidance, not a security boundary.
"""

import glob as globmod
import json
import os
import re
import shlex
import sys


GREP_RE = re.compile(r"(grep|rg|egrep|fgrep)\b")
DELIBERATE_FALLBACK_RE = re.compile(r"command\s+(grep|rg|egrep|fgrep)\b")
PREFIX_RE = re.compile(r"^(sudo\s+|env(\s+\w+=\S*)*\s+)+")


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
        if DELIBERATE_FALLBACK_RE.match(head):
            continue
        head = PREFIX_RE.sub("", head)
        if not GREP_RE.match(head):
            continue
        targets = grep_targets(head, cwd)
        if targets is None:
            continue
        scope = targets if targets else [cwd]
        if any(repo_root(target) == work_repo for target in scope):
            sys.stderr.write(
                "LOCTREE FIRST: first-choice repo mapping with grep/rg is paused, "
                "not forbidden. "
                "Use loct find '<pattern>' (literal by default), loct occurrences, "
                "loct body, or loct slice. If the scope is already mapped or "
                "Loctree cannot answer cleanly, retry deliberately with "
                "'command grep ...' or 'command rg ...'. When that fallback "
                "reveals a Loctree miss, append it to "
                "~/.vibecrafted/loctree/loctree-fail.md. Pipe filters and "
                "searches outside the working repo remain allowed.\n"
            )
            return 2
    return 0


if __name__ == "__main__":
    sys.exit(main())
