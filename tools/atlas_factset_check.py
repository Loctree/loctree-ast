#!/usr/bin/env python3
"""atlas_factset_check.py — machine proof that atlas cards are decision-complete.

Primary mode (L1-01, kanon v4)::

    python3 tools/atlas_factset_check.py <atlas_dir> [--seed N] [--no-mutation]

For every card in <atlas_dir> (``.loctree/context-atlas``) with a coverage
receipt, three FactSets are compared with SET EQUALITY in both directions:

    rendered  — parsed from the markdown card via the base-fact line grammar
                frozen in ``docs/contracts/atlas-card-format.md``
    receipt   — ``coverage_receipt`` from ``receipt.json`` (Rust materializer)
    derived   — re-derived independently from the card's ``.full.json`` payload
                (double-entry bookkeeping against the Rust receipt logic)

A fact missing from the markdown is a FAIL. A fact rendered in markdown
without receipt coverage is ALSO a FAIL (renderer and receipt drifted apart).
Any ``` ```json ``` fence inside a ``0*-*.md`` card is a FAIL.

After the checks pass, a built-in mutation self-test removes >=3
seeded-random fact lines (in memory, source files untouched) and asserts the
parser notices every removal. A parser that cannot see a missing fact is
theatre — the self-test keeps this tool honest on every invocation.

Exit code: 0 only when every comparison and the mutation self-test pass.

Legacy mode (E1-01a forcefeed probe)::

    python3 tools/atlas_factset_check.py --payload <file> [--receipt <manifest.json>] [--out <file>]

Substring completeness check over a captured bootstrap feed. Exit 0 always
(the probe's caller decides). Kept verbatim so ``tools/forcefeed-probe/run.sh``
continues to work.
"""

from __future__ import annotations

import argparse
import json
import random
import re
import sys
from pathlib import Path

DEFAULT_MUTATION_SEED = 0x5EEDCAFE
MUTATION_SAMPLES = 3

# One thesis = one line (karta 03, M1-01): `lifecycle[dowodowość] · data ·
# authority · teza · ref`. Its fact id (`thesis:<intent_id>`) comes from the
# overlay contract and is NOT derivable from the line (kanon v4) — thesis
# facts verify cross-payload: receipt ↔ payload id equality plus markdown
# thesis-line COUNT parity (both directions).
THESIS_LINE_RE = re.compile(r"^\s*[✓⊘✗]\[[VUR]\] \d{4}-\d{2}-\d{2} · ")

# --------------------------------------------------------------------------
# FactSet grammar — mirror of the test-side parser in
# loctree-rs/src/cli/dispatch/handlers/context/atlas.rs (mod tests) and the
# grammar frozen in docs/contracts/atlas-card-format.md (kanon v4).
# --------------------------------------------------------------------------


def _head(rest: str) -> str:
    return rest.split(" · ", 1)[0].strip()


def parse_line_facts(line: str) -> set[str]:
    """Base-fact ids carried by a single rendered card line."""
    facts: set[str] = set()
    line = line.rstrip()
    if line.startswith("entry:"):
        facts.add(f"entry:{_head(line[len('entry:'):])}")
    elif line.startswith("env:"):
        parts = line[len("env:"):].split(" · ")
        if len(parts) >= 2:
            facts.add(f"env:{parts[0].strip()}:{parts[1].strip()}")
    elif line.startswith("dispatch:"):
        head = _head(line[len("dispatch:"):])
        if "→" in head:
            source, target = head.split("→", 1)
            facts.add(f"dispatch:{source}:{target}")
    elif line.startswith("reachability:"):
        facts.add(f"reachability:{_head(line[len('reachability:'):])}")
    elif line.startswith("hotspots:"):
        facts.add(f"hotspots:{_head(line[len('hotspots:'):])}")
    elif line.startswith("authority:"):
        # One counter line carries one fact per label with count > 0
        # (`authority:RepoVerified 1 · LoctreeDerived 148 · ...`).
        for part in line[len("authority:"):].split(" · "):
            tokens = part.split()
            if len(tokens) >= 2 and tokens[1].isdigit() and int(tokens[1]) > 0:
                facts.add(f"authority:{tokens[0]}")
    elif line.startswith("gate:"):
        facts.add(f"gate:{_head(line[len('gate:'):])}")
    elif line.startswith("test:"):
        facts.add(f"test:{_head(line[len('test:'):])}")
    elif " ← " in line:
        target, groups = line.split(" ← ", 1)
        for group in groups.split(" · "):
            if "{" in group:
                prefix, inner = group.split("{", 1)
                inner = inner.rstrip("}")
                for entry in inner.split(","):
                    facts.add(f"edge:{target}:{prefix}{entry}")
            else:
                facts.add(f"edge:{target}:{group}")
    elif line.startswith("|"):
        cells = [cell.strip() for cell in line.split("|")]
        if len(cells) > 2 and cells[1].isdigit():
            facts.add(f"hub:{cells[2]}")
    return facts


def parse_card_facts(text: str) -> set[str]:
    facts: set[str] = set()
    for line in text.splitlines():
        facts |= parse_line_facts(line)
    return facts


def count_thesis_lines(text: str) -> int:
    """Number of spec-grammar thesis lines rendered on a card."""
    return sum(1 for line in text.splitlines() if THESIS_LINE_RE.match(line))


# --------------------------------------------------------------------------
# Independent derivation from the canonical .full.json payload — the second
# entry of the double-entry bookkeeping. Mirrors the *_coverage_receipt
# functions in atlas.rs without reading any Rust output but receipt.json.
# --------------------------------------------------------------------------


# Snake-case AuthoritySlice keys -> CamelCase label names (mirror of
# `authority_name` / `authority_counter_pairs` in atlas.rs).
AUTHORITY_LABELS = [
    ("repo_verified", "RepoVerified"),
    ("loctree_derived", "LoctreeDerived"),
    ("aicx_operator", "AicxOperator"),
    ("aicx_agent", "AicxAgent"),
    ("aicx_failure", "AicxFailure"),
    ("semantic_guess", "SemanticGuess"),
    ("stale_or_unknown", "StaleOrUnknown"),
]


def derive_expected(card_name: str, payloads: dict[str, dict]) -> set[str]:
    """Facts a card's receipt must carry, derived from the canonical payloads.

    ``payloads`` maps the two-digit card prefix (``"01"``) to its parsed
    ``.full.json``. Since L1-02 the receipt follows the manifest
    ``domain_owners`` map, not payload locality: karta 01 owns the hotspots /
    authority / reachability domains whose machine data still lives in the
    02/05 payloads (payload duplication is legal, markdown/receipt is not).
    """
    facts: set[str] = set()
    payload = payloads.get(card_name[:2], {})
    if card_name.startswith("01"):
        for imported in payload.get("structural", {}).get("imports", []):
            target = imported.get("resolved_path")
            if target:
                facts.add(f"edge:{target}:{imported['file']}")
        for hub in payload.get("high_fan_in", []):
            facts.add(f"hub:{hub['file']}")
        risk_payload = payloads.get("05", {})
        for hotspot in risk_payload.get("risk", {}).get("hotspots", []):
            facts.add(f"hotspots:{hotspot['file']}")
        for key, label in AUTHORITY_LABELS:
            if risk_payload.get("authority", {}).get(key):
                facts.add(f"authority:{label}")
        for claim in payloads.get("02", {}).get("reachability", []):
            if not claim.get("reached") and "::" in claim.get("symbol", ""):
                facts.add(f"reachability:{claim['symbol'].split('::', 1)[0]}")
    elif card_name.startswith("02"):
        for edge in payload.get("dispatch_edges", []):
            target = edge.get("handler_file") or edge["handler_symbol"]
            facts.add(f"dispatch:{edge['from_file']}:{target}")
        for contract in payload.get("env_contracts", []):
            for file in contract.get("used_in_files", []):
                facts.add(f"env:{contract['name']}:{file}")
        for hint in payload.get("framework_hints", []):
            if hint.get("kind") == "entrypoint":
                facts.add(f"entry:{hint['file']}")
    elif card_name.startswith("03"):
        # Intent domain (M1-01): fact ids come from the overlay contract via
        # the payload's rendered-theses list, never from line content.
        for thesis in payload.get("rendered_theses", []):
            intent_id = thesis.get("intent_id")
            if intent_id:
                facts.add(f"thesis:{intent_id}")
    elif card_name.startswith("04"):
        for gate in payload.get("verification_gates", []):
            facts.add(f"gate:{gate}")
        for test in payload.get("likely_tests", []):
            facts.add(f"test:{test}")
    # 00 (core) is a projection card; 05 is a reference-only card since
    # L1-02 (hotspots domain owned by karta 01).
    return facts


# --------------------------------------------------------------------------
# Primary check
# --------------------------------------------------------------------------


def _sample(diff: set[str], cap: int = 5) -> str:
    shown = sorted(diff)[:cap]
    extra = len(diff) - len(shown)
    tail = f" (+{extra} more)" if extra > 0 else ""
    return ", ".join(shown) + tail


def check_atlas(atlas_dir: Path, seed: int, run_mutation: bool) -> int:
    receipt_path = atlas_dir / "receipt.json"
    if not receipt_path.exists():
        print(f"FAIL: {receipt_path} missing — atlas without receipt is unverifiable", file=sys.stderr)
        return 1
    receipt_doc = json.loads(receipt_path.read_text())
    entries = receipt_doc.get("coverage_receipts")
    if not isinstance(entries, list):
        print("FAIL: receipt.json carries no coverage_receipts (pre-L1-00 atlas?)", file=sys.stderr)
        return 1
    receipts: dict[str, set[str]] = {
        entry["path"]: set(entry["coverage_receipt"]) for entry in entries
    }

    cards = sorted(path for path in atlas_dir.glob("0*.md"))
    if not cards:
        print(f"FAIL: no cards matching 0*.md in {atlas_dir}", file=sys.stderr)
        return 1

    # Preload every canonical payload: cross-card domains (L1-02) derive the
    # owner card's facts from sibling payloads, not just its own.
    payloads: dict[str, dict] = {}
    for card in cards:
        full = atlas_dir / card.name.replace(".md", ".full.json")
        if full.exists():
            payloads[card.name[:2]] = json.loads(full.read_text())

    failures: list[str] = []
    fact_lines: list[tuple[Path, int]] = []
    checked = 0
    total_facts = 0

    for card in cards:
        text = card.read_text()
        if "```json" in text:
            failures.append(f"{card.name}: forbidden ```json fence in a dense card")
        if card.name not in receipts:
            failures.append(f"{card.name}: no coverage_receipt entry in receipt.json")
            continue
        rendered = parse_card_facts(text)
        receipt = receipts[card.name]
        # Thesis facts (karta 03, M1-01) verify cross-payload: their ids are
        # not derivable from line content, so markdown honesty is proven by
        # thesis-line COUNT parity while receipt ↔ payload compare per id.
        thesis_receipt = {fact for fact in receipt if fact.startswith("thesis:")}
        line_receipt = receipt - thesis_receipt
        thesis_lines = count_thesis_lines(text)

        full = atlas_dir / card.name.replace(".md", ".full.json")
        derived = derive_expected(card.name, payloads)
        if not full.exists() and receipt:
            failures.append(f"{card.name}: canonical payload {full.name} missing")

        missing_in_md = line_receipt - rendered
        excess_in_md = rendered - line_receipt
        drift = receipt ^ derived
        if missing_in_md:
            failures.append(
                f"{card.name}: {len(missing_in_md)} receipt fact(s) absent from markdown — {_sample(missing_in_md)}"
            )
        if excess_in_md:
            failures.append(
                f"{card.name}: {len(excess_in_md)} markdown fact(s) without receipt coverage — {_sample(excess_in_md)}"
            )
        if thesis_lines != len(thesis_receipt):
            failures.append(
                f"{card.name}: {thesis_lines} thesis line(s) in markdown vs {len(thesis_receipt)} thesis fact(s) in receipt"
            )
        if drift:
            failures.append(
                f"{card.name}: receipt vs payload-derived drift on {len(drift)} fact(s) — {_sample(drift)}"
            )
        checked += 1
        total_facts += len(receipt)
        for idx, line in enumerate(text.splitlines()):
            if parse_line_facts(line) or THESIS_LINE_RE.match(line):
                fact_lines.append((card, idx))

    if failures:
        for failure in failures:
            print(f"FAIL: {failure}", file=sys.stderr)
        return 1

    print(
        f"OK: {checked} card(s) checked — FactSet(markdown) == FactSet(receipt) == FactSet(payload), "
        f"{total_facts} base fact(s), zero json fences"
    )

    if not run_mutation:
        print("NOTE: mutation self-test skipped (--no-mutation)")
        return 0

    # Mutation self-test: the parser must notice every removed fact line.
    if not fact_lines:
        print("NOTE: no fact lines in this atlas (all receipts empty) — mutation self-test vacuous")
        return 0
    rng = random.Random(seed)
    samples = min(MUTATION_SAMPLES, len(fact_lines))
    for card, idx in rng.sample(fact_lines, samples):
        lines = card.read_text().splitlines()
        removed = lines.pop(idx)
        mutated_text = "\n".join(lines)
        receipt = receipts[card.name]
        thesis_receipt = {fact for fact in receipt if fact.startswith("thesis:")}
        noticed = parse_card_facts(mutated_text) != (receipt - thesis_receipt) or count_thesis_lines(
            mutated_text
        ) != len(thesis_receipt)
        if not noticed:
            print(
                f"FAIL: mutation self-test — parser did not notice removal of "
                f"{card.name}:{idx + 1} ({removed[:80]!r})",
                file=sys.stderr,
            )
            return 1
    print(f"OK: mutation self-test — {samples} seeded fact-line removal(s) all detected (seed={seed:#x})")
    return 0


# --------------------------------------------------------------------------
# Legacy mode — E1-01a forcefeed probe (substring completeness over a captured
# bootstrap feed). Kept verbatim; exit 0 always, the caller decides.
# --------------------------------------------------------------------------

DEFAULT_FACTS = [
    "core-map",
    "structural-map",
    "runtime-map",
    "memory-trail",
    "verification-gates",
    "risk-register",
    "loct-context",
    "hubs-types-rs",
    "entrypoints",
    "aicx-memory",
    "full-atlas-present",
    "structure-before-task",
]

FACT_MARKERS = {
    "core-map": [r"Core Map", r"00-core-map", r"repo identity.*risk"],
    "structural-map": [r"Structural Map", r"01-structural-map", r"files.*symbols.*imports", r"top_hubs"],
    "runtime-map": [r"Runtime Map", r"02-runtime-map", r"runtime behavior", r"framework_hints"],
    "memory-trail": [r"Intent Map", r"03-intent-map", r"Memory Trail", r"03-memory-trail", r"AICX", r"prior decisions"],
    "verification-gates": [r"Verification Gates", r"04-verification-gates", r"likely_tests"],
    "risk-register": [r"Risk Register", r"05-risk-register", r"hotspots", r"dirty_worktree"],
    "loct-context": [r"loct context", r"Agent Context Pack", r"context-atlas"],
    "hubs-types-rs": [r"loctree-rs/src/types.rs", r"importers.*82", r"types\.rs", r"loctree-rs/src/types"],
    "entrypoints": [r"entrypoints", r"next_safe_commands", r"power_path"],
    "aicx-memory": [r"AICX", r"memory-trail", r"intent", r"outcome", r"memory"],
    "full-atlas-present": [r"atlas_ready", r"context_atlas", r"manifest\.md", r"Core Map"],
    "structure-before-task": [r"structure_before_task|Core Map.*Brief|Core Map.*Mission", r"Core Map"],
}


def fact_present(fact_id: str, payload: str) -> bool:
    for marker in FACT_MARKERS.get(fact_id, [fact_id]):
        if re.search(marker, payload, re.I):
            return True
    return fact_id.lower() in payload.lower()


def legacy_check(payload_path: Path, receipt_path: Path | None = None) -> dict:
    payload = payload_path.read_text(errors="ignore") if payload_path.exists() else ""
    expected = DEFAULT_FACTS[:]
    if receipt_path and receipt_path.exists():
        try:
            rec = json.loads(receipt_path.read_text())
            if isinstance(rec, dict):
                cards = rec.get("cards") or rec.get("atlas", {}).get("cards", [])
                for card in cards:
                    cid = card.get("id") or card.get("path", "").replace(".md", "")
                    if cid and cid not in expected:
                        expected.append(cid)
        except Exception:
            pass

    present = [fid for fid in expected if fact_present(fid, payload)]
    missing = [fid for fid in expected if fid not in present]
    if "structure-before-task" in missing and ("Core Map" in payload or "Structural Map" in payload):
        if "Brief E1" in payload or "Mission" in payload or "Operator prompt" in payload:
            missing = [fid for fid in missing if fid != "structure-before-task"]

    return {
        "missing_fact_ids": missing,
        "present_fact_ids": present,
        "total_expected": len(expected),
        "receipt_used": str(receipt_path) if receipt_path else "builtin-atlas-facts-v1",
    }


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("atlas_dir", nargs="?", type=Path, help="context-atlas directory (primary FactSet mode)")
    parser.add_argument("--seed", type=lambda v: int(v, 0), default=DEFAULT_MUTATION_SEED, help="mutation self-test seed")
    parser.add_argument("--no-mutation", action="store_true", help="skip the built-in mutation self-test")
    parser.add_argument("--payload", type=Path, default=None, help="legacy forcefeed mode: captured feed file")
    parser.add_argument("--receipt", type=Path, default=None, help="legacy forcefeed mode: manifest.json")
    parser.add_argument("--out", type=Path, default=None, help="legacy forcefeed mode: JSON output path")
    args = parser.parse_args()

    if args.payload is not None:
        result = legacy_check(args.payload, args.receipt)
        rendered = json.dumps(result, indent=2, ensure_ascii=False)
        if args.out:
            args.out.write_text(rendered)
        else:
            print(rendered)
        sys.exit(0)

    if args.atlas_dir is None:
        parser.error("pass an atlas directory (FactSet mode) or --payload (legacy forcefeed mode)")
    sys.exit(check_atlas(args.atlas_dir, args.seed, not args.no_mutation))


if __name__ == "__main__":
    main()
