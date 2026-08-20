# /// script
# requires-python = ">=3.10"
# dependencies = ["jsonschema>=4.21"]
# ///
"""Delivery verifier for the C0-01 contracts (loctree.overlay.intent.v1 +
loctree.anchors.v1). Superset of the brief §6 heredoc: adds the four-variant
revision matrix, schema-validation of the cross-revision pair and matrix
overlays, and the appended-refs assertion.

Run: LOCTREE_SUITE_ROOT=... AICX_ROOT=... uv run docs/contracts/verify_contracts.py
"""

import hashlib
import json
import os
import pathlib
import sys

try:
    import jsonschema
except ImportError:
    sys.exit("FAIL: pip install jsonschema (środowisko weryfikatora)")

suite_root = os.environ.get("LOCTREE_SUITE_ROOT")
aicx_root = os.environ.get("AICX_ROOT")
if not suite_root or not aicx_root:
    sys.exit("FAIL: export LOCTREE_SUITE_ROOT and AICX_ROOT (DRIVER §3)")

base = pathlib.Path(suite_root) / "docs/contracts"

# --- schemas are themselves valid draft 2020-12 ---
schema = json.loads((base / "loctree.overlay.intent.v1.schema.json").read_text())
jsonschema.Draft202012Validator.check_schema(schema)
v = jsonschema.Draft202012Validator(schema)
anchors_schema = json.loads((base / "loctree.anchors.v1.schema.json").read_text())
jsonschema.Draft202012Validator.check_schema(anchors_schema)

fx = base / "fixtures/overlay-intent-v1"
valid = sorted(fx.glob("valid_*.json"))
invalid = sorted(fx.glob("invalid_*.json"))
assert len(valid) >= 2 and len(invalid) >= 5, (
    f"FAIL: fixtures valid={len(valid)} invalid={len(invalid)} (min 2/5)"
)

# --- valid fixtures accepted, invalid fixtures provably rejected ---
for f in valid:
    v.validate(json.loads(f.read_text()))
for f in invalid:
    errs = list(v.iter_errors(json.loads(f.read_text())))
    assert errs, f"FAIL: {f.name} przeszedł walidację, a miał być odrzucony"

# --- identity + evidence obligations on the primary valid fixture ---
d = json.loads(valid[0].read_text())
assert any(e.get("attributions") for e in d["entries"]), (
    "FAIL: brak typed attribution w valid fixture"
)
for e in d["entries"]:
    assert "intent_id" in e and "content_hash" in e, f"FAIL: brak tożsamości w {e}"
    for r in e.get("relations", []):
        assert all(
            k in r for k in ("intent_id", "evidence_ref", "confidence", "observed_at")
        ), f"FAIL: relacja bez dowodu: {r}"

# --- cross-revision stability contract (4 assertions) ---
ra = json.loads((fx / "cross_revision_revA.json").read_text())
rb = json.loads((fx / "cross_revision_revB.json").read_text())
v.validate(ra)
v.validate(rb)
ida = {e["intent_id"] for e in ra["entries"]}
idb = {e["intent_id"] for e in rb["entries"]}
assert ida <= idb, "FAIL cross-rev: intent_id z revA zniknęły w revB"
common = ida & idb
cha = {e["intent_id"]: e["content_hash"] for e in ra["entries"]}
chb = {e["intent_id"]: e["content_hash"] for e in rb["entries"]}
assert any(cha[i] != chb[i] for i in common), (
    "FAIL cross-rev: append nie zmienił żadnego content_hash (fixture nie testuje mutacji)"
)
refs_a = {e["intent_id"]: {r["evidence_event_id"] for r in e["refs"]} for e in ra["entries"]}
refs_b = {e["intent_id"]: {r["evidence_event_id"] for r in e["refs"]} for e in rb["entries"]}
assert any(refs_a[i] < refs_b[i] for i in common), (
    "FAIL cross-rev: żaden wspólny klaster nie dopiął nowych refs (append bez śladu)"
)
for i in common:
    assert refs_a[i] <= refs_b[i], f"FAIL cross-rev: refs z revA zgubione w revB dla {i}"
for e in rb["entries"]:
    for r in e.get("relations", []):
        assert r["intent_id"] in idb, f"FAIL cross-rev: wisząca relacja {r['intent_id']}"

# --- four-variant revision matrix ---
matrix = json.loads((fx / "revision_matrix.json").read_text())
variants = matrix["variants"]
for name in ("baseline", "rerun_identical", "attribution_bump", "source_event_change"):
    assert name in variants, f"FAIL matrix: brak wariantu {name}"
    v.validate(variants[name]["overlay"])

def revs(name):
    o = variants[name]["overlay"]
    return o["store_revision"], o["overlay_revision"]

sr0, ov0 = revs("baseline")
sr1, ov1 = revs("rerun_identical")
sr2, ov2 = revs("attribution_bump")
sr3, ov3 = revs("source_event_change")
assert sr1 == sr0 and ov1 == ov0, "FAIL matrix: identyczny re-run zmienił rewizję"
assert sr2 == sr0 and ov2 != ov0, (
    "FAIL matrix: attribution_version-only MUSI zachować store_revision i zmienić overlay_revision"
)
assert variants["attribution_bump"]["inputs"]["attribution_version"] != variants["baseline"]["inputs"]["attribution_version"], (
    "FAIL matrix: attribution_bump nie zmienia attribution_version w inputs"
)
assert sr3 != sr0 and ov3 != ov0, (
    "FAIL matrix: zmiana źródłowego eventu MUSI zmienić store_revision ORAZ overlay_revision"
)
pairs = [("baseline", sr0, ov0), ("rerun_identical", sr1, ov1), ("attribution_bump", sr2, ov2), ("source_event_change", sr3, ov3)]
for na, sa, oa in pairs:
    for nb, sb, ob in pairs:
        assert not (sa != sb and oa == ob), (
            f"FAIL matrix: {na} vs {nb} — store_revision różny przy identycznym overlay_revision (zakazany kwadrant)"
        )

# --- evidence_event_id fidelity to frozen C0A derivation v1 ---
def all_evidence_ids(doc):
    for e in doc["entries"]:
        for r in e["refs"]:
            yield r["evidence_event_id"]

for f in valid + [fx / "cross_revision_revA.json", fx / "cross_revision_revB.json"]:
    doc = json.loads(f.read_text())
    for ev in all_evidence_ids(doc):
        assert ev.startswith("ev1:"), f"FAIL: {f.name}: evidence_event_id poza derivation v1: {ev}"
        assert "/" not in ev and "\\" not in ev, f"FAIL: {f.name}: ścieżka w identity: {ev}"
        tail = ev.rsplit(":", 1)[-1]
        assert len(tail) == 16 and all(c in "0123456789abcdef" for c in tail), (
            f"FAIL: {f.name}: evidence_event_id bez hex16 ogona: {ev}"
        )

# --- mirror byte-identity in both repos ---
mirror = pathlib.Path(aicx_root) / "tests/fixtures/overlay-intent-v1"
for f in sorted(fx.glob("*.json")):
    m = mirror / f.name
    assert m.exists() and hashlib.sha256(m.read_bytes()).digest() == hashlib.sha256(f.read_bytes()).digest(), (
        f"FAIL: mirror {f.name}"
    )

# --- card spec covers all cards + markers + Reachability ---
spec = (base / "atlas-card-format.md").read_text()
for h in (
    "## Reachability",
    "00-core",
    "01-structural",
    "02-runtime",
    "03-intent",
    "04-verification",
    "05-risk",
    "[V]",
    "[U]",
    "[R]",
):
    assert h.lower() in spec.lower(), f"FAIL: spec bez {h}"

print("VERIFIER GREEN")
