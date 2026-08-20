---
name: runtime-atlas-api-aicx-loctree
status: planning
project: loctree-suite
created: 2026-05-27
scope: cross-repo architectural implementation plan for API runtime incidents, Loctree atlas truth, and AICX memory overlay
source_context: /tmp/loct_context--full--json.json
---

# Plan 25 - Runtime Atlas for API, Loctree, and AICX

## 0. Reading Receipt

This plan is grounded in three inputs:

- `loct context --full --json > /tmp/loct_context--full--json.json`
- `docs/plans/lsp/TRACKER.md`
- `docs/plans/lsp/*.md`, especially the tracker, the single-run handoff, the AICX request, the semantic request, the watcher request, and the cursor pattern.

The deep-research hypothesis read immediately before this plan defines the north star:
Loctree should not be treated as one more analyzer. It should become the join
layer that links intent, source evolution, semantic structure, build artifacts,
binary or package identity, runtime execution, observability, and MIAZGA.

This document applies that thesis to the current operator pain:

- API side runtime incidents in `/Users/polyversai/Libraxis/lbrx-services`
- Loctree suite context truth and MCP transport reliability in this checkout
- AICX as the intent and historical evidence overlay

## 1. Executive Verdict

The next architectural step is not another local health check, another
backlog note, or another LSP request. The next step is a **Runtime Atlas Plane**:
a first-class evidence model that captures what was actually running, joins it
to source and release identity, then uses AICX to explain whether this failure
has happened before and why it mattered.

The current stack already contains most of the ingredients:

- Loctree has structural, runtime, risk, authority, action, and memory sections
  in `ContextPack`.
- AICX can retrieve prior intent, failure, and outcome chunks.
- lbrx-services exposes concrete runtime truth: ports, PIDs, watchdog logs,
  API health, model aliases, route configuration, and auth/license failure
  modes.
- The LSP roadmap already proves that Loctree can expose structured request
  surfaces, cursor pagination, workspace routing, semantic slices, AICX slices,
  diff surfaces, and health gates.

But the join is missing. Today a VLM incident can be debugged by a human using
`ps`, `lsof`, `curl`, logs, and memory. Tomorrow Loctree should be able to
produce an incident atlas:

```text
intent -> service -> process -> port -> model -> log slice -> artifact hash
       -> source files -> commit -> prior AICX failures -> MIAZGA score
```

The plan below builds that plane in waves.

## 2. Hard Evidence From The Current Atlas

The first generated context pack, captured before the version bump finished and
before the post-bump rescan, said:

- Project root: `/Users/polyversai/Libraxis/vc-runtime/loctree-suite`
- Branch: `fix/the-truth-of-findings`
- Commit: `44096df7`
- Snapshot: `fix_the-truth-of-findings@44096df7`
- Structural surface returned: `App.tsx` only
- Runtime facts returned: Tauri `invoke` orphans for `greet`,
  `missing_handler`, and `save_data`
- Risk: `snapshot_health = fresh`
- Risk: `cache_scope = Unknown`
- Risk: `cache_scope_authority = stale_or_unknown`
- Dirty worktree in context pack: `false`
- Actual git worktree after user version bump: dirty across versioned manifests,
  `CHANGELOG.md`, `Cargo.lock`, `loctree-rs/src/lib.rs`, and reports code.

That was a red flag, but the post-bump artifact changed the interpretation.
After `make version TYPE=minor FORCE=1`, the installed and release binaries both
reported `loct 0.11.0`, and:

```bash
loct context --full > loct-v0-11-0-context-full.json
```

rescanned the repository from stale snapshot `44096df7` to `b09286f6`.
The resulting artifact is real suite context, not the earlier fixture-like
surface:

- Artifact path: `loct-v0-11-0-context-full.json`
- Size: 8,486 lines / 275,081 bytes
- Commit: `b09286f6`
- Snapshot: `fix_the-truth-of-findings@b09286f6`
- Structural files: 123
- Symbols: 293
- Imports: 37
- Consumers: 114
- Runtime idiom tags: 74
- Env contracts: 75
- Memory entries: 50
- Risk: `snapshot_health = dirty`
- Risk: `cache_scope = DirtyWorktree`
- Risk authority: `repo_verified`

So the problem is narrower and sharper than "context is broken." The current
evidence says Loctree can recover into real suite context after a stale snapshot
rescan, but the pack still needs a receipt that makes this transition explicit:
which binary produced the pack, which HEAD it scanned, whether it auto-rescanned,
which cache scope it trusted, and why the earlier context collapsed to a tiny
surface. The plan therefore still starts with context receipts, but now the
receipt is a proof-of-truth mechanism, not a panic button.

## 3. Current Runtime Case To Encode

The live API-side incident observed immediately before this plan:

- `api-router` stayed healthy on `/health`.
- `proxy-router` stayed healthy.
- MLX text endpoints on ports `8100` and `8101` stayed healthy.
- VLM on port `8102` wedged.
- PID `81601` held `127.0.0.1:8102`.
- PID `81601` had about 36 GB RSS, `0.0% CPU`, and did not accept TCP connects.
- Watchdog attempted restarts while the old listener still owned the port.
- Restart attempts logged `address already in use`.
- A historical log slice showed a transient `mlx_vlm/utils.py` conflict marker,
  but the current file no longer contained conflict markers.
- Controlled restart of `mlx-batch-vlm` killed the wedged process and started
  PID `99743`.
- `/v1/models` on port `8102` returned to 200 in 1-14 ms.
- Independent API noise remained: external `/v1/responses` calls returned
  `401`, and `LIBRAXIS_LICENSE_PATH` was missing in production configuration.

This incident is exactly the runtime-to-source workflow from the research
document, except the runtime artifact is a Python/MLX service rather than a
native crash address. The equivalent of symbolication is:

```text
PID + port + command + venv path + log frame + package file hash
-> source module / package file / repo commit / config surface / intent
```

## 4. North Star

Loctree should answer these questions without requiring a human to manually
stitch shell output together:

1. What was running?
2. Which exact artifact was running?
3. Which endpoint, port, PID, and command were involved?
4. Which source files, config files, and package files explain the behavior?
5. Which prior AICX chunks mention similar failures or intent?
6. Is this a transient busy task, a stale process, a supervisor loop, a bad
   package artifact, a routing/auth failure, or systemic MIAZGA?
7. What should the next operator do?

The artifact should be durable, queryable, and small enough for agents to fetch
in chunks.

## 5. Proposed Domain Model

### 5.1 Runtime Incident Bundle

Add a normalized incident bundle type, initially in `loctree-rs` and later
surfaced through CLI, MCP, LSP, and HTML.

```rust
pub struct RuntimeIncidentBundle {
    pub incident_id: String,
    pub captured_at: String,
    pub project_root: PathBuf,
    pub host: HostIdentity,
    pub intent_refs: Vec<IntentRef>,
    pub services: Vec<ServiceObservation>,
    pub processes: Vec<ProcessObservation>,
    pub ports: Vec<PortObservation>,
    pub probes: Vec<ProbeObservation>,
    pub log_slices: Vec<LogSlice>,
    pub artifact_identities: Vec<ArtifactIdentity>,
    pub source_joins: Vec<SourceJoin>,
    pub aicx_overlay: Vec<AicxMemoryEntry>,
    pub diagnosis: RuntimeDiagnosis,
    pub miazga: Vec<MiazgaScore>,
    pub actions: Vec<RecommendedAction>,
    pub authority: AuthorityMap,
}
```

### 5.2 Service Observation

```rust
pub struct ServiceObservation {
    pub service_id: String,
    pub display_name: String,
    pub repo: Option<String>,
    pub workdir: Option<PathBuf>,
    pub command: Vec<String>,
    pub expected_ports: Vec<u16>,
    pub health_url: Option<String>,
    pub supervisor: Option<String>,
    pub status: ServiceStatus,
    pub authority: AuthorityLabel,
}
```

### 5.3 Process Observation

```rust
pub struct ProcessObservation {
    pub pid: u32,
    pub ppid: u32,
    pub pgid: u32,
    pub elapsed: String,
    pub cpu_percent: f32,
    pub rss_bytes: u64,
    pub state: String,
    pub command: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env_fingerprint: Option<String>,
    pub authority: AuthorityLabel,
}
```

### 5.4 Probe Observation

```rust
pub struct ProbeObservation {
    pub target: String,
    pub probe_kind: ProbeKind,
    pub started_at: String,
    pub duration_ms: u64,
    pub status: ProbeStatus,
    pub body_excerpt: Option<String>,
    pub error_excerpt: Option<String>,
    pub authority: AuthorityLabel,
}
```

### 5.5 Artifact Identity

For Rust/native code this may be build ID, debug ID, object path, symbol table
hash, or binary digest. For Python/Node services this is package version, file
hash, venv path, lockfile entry, editable install path, and source checkout.

```rust
pub struct ArtifactIdentity {
    pub artifact_id: String,
    pub kind: ArtifactKind,
    pub path: PathBuf,
    pub digest: Option<String>,
    pub version: Option<String>,
    pub source_repo: Option<PathBuf>,
    pub source_commit: Option<String>,
    pub package_manager: Option<String>,
    pub authority: AuthorityLabel,
}
```

### 5.6 Diagnosis

```rust
pub enum RuntimeDiagnosisKind {
    Healthy,
    BusyButResponsive,
    BusyAndUnresponsive,
    WedgedListener,
    SupervisorRestartLoop,
    ArtifactSyntaxFailure,
    AuthMisconfiguration,
    LicenseMisconfiguration,
    TransportClosed,
    MemoryPressure,
    Unknown,
}
```

The current VLM case should classify as:

```text
primary: WedgedListener
secondary: SupervisorRestartLoop
side_findings: AuthMisconfiguration, LicenseMisconfiguration
```

## 6. Tracks

### Track A - Context Truth Receipt

Goal: make every context pack prove its own scope.

Problem: `loct context --full --json` returned a tiny `App.tsx` surface for
the Loctree suite root, while the repo is a large workspace. A plan based on
that pack must treat it as partial, even when the pack says `fresh`.

Implementation cuts:

- Add `ContextReceipt` to `ContextPack`.
- Record invocation cwd, requested project path, canonical root, selected scan
  root, cache root, snapshot path, snapshot file count, snapshot edge count,
  source of project root selection, and whether fixture markers influenced
  selection.
- Add a hard `scope_confidence` enum:
  - `repo_root_verified`
  - `workspace_member_verified`
  - `fixture_or_subtree_suspected`
  - `cache_scope_unknown`
  - `stale_or_missing`
- Add warnings when `--full` returns fewer than a threshold of files for a
  known workspace root unless `--scope` or `--file` was explicit.
- Emit the receipt in JSON and Markdown.
- Teach MCP `/context_pack` pagination to include the receipt in section 0.
- Teach HTML report to show a compact receipt banner.

Candidate files:

- `loctree-rs/src/cli/dispatch/handlers/context/mod.rs`
- `loctree-rs/src/atlas.rs`
- `loctree-rs/src/types.rs`
- `loctree-mcp/src/http/context_pack.rs`
- report rendering code under `reports/src`

Acceptance:

- `loct context --full --json` on this repo reports the real workspace file
  count or explicitly flags scope confidence below `repo_root_verified`.
- A fixture-root context pack cannot silently masquerade as suite-root truth.
- `loct context --full --markdown` shows the receipt before semantic claims.
- Tests cover repo root, workspace member, fixture subtree, stale cache, and
  explicit `--scope`.

### Track B - Runtime Incident Capture CLI

Goal: give operators one command to freeze runtime truth before it disappears.

Proposed command:

```bash
loct incident capture \
  --project /Users/polyversai/Libraxis/lbrx-services \
  --service mlx-batch-vlm \
  --port 8102 \
  --health http://127.0.0.1:8102/v1/models \
  --logs logs/api-router.log,logs/proxy-router.log,logs/mlx-batch-vlm.log \
  --json > incident.json
```

Implementation cuts:

- Add `incident` command group.
- Implement local probes:
  - process table snapshot
  - port listener lookup
  - bounded TCP/HTTP probe
  - recent log slice
  - optional file digest for runtime artifact paths
  - optional git identity for detected repo roots
- Keep it read-only by default.
- Add `--action restart` later only behind explicit flag and operator approval.

Candidate files:

- `loctree-rs/src/cli/dispatch/mod.rs`
- `loctree-rs/src/cli/dispatch/handlers/incident.rs`
- `loctree-rs/src/runtime/incident.rs`
- `loctree-rs/tests/incident_capture.rs`

Acceptance:

- Captures the 8102 wedge class without killing anything.
- Distinguishes TCP connect timeout from HTTP 500 from model-level timeout.
- Reports stale restart probes as separate process observations.
- Emits a stable `incident_id`.
- Does not require lbrx-services-specific code in Loctree core.

### Track C - lbrx-services Runtime Adapter

Goal: make API-side service topology machine-readable enough for Loctree to
capture cleanly.

Current known topology:

- `proxy-router` on 8088
- `api-router` on 8089
- MLX text/model route on 8100
- MLX svetliq/small route on 8101
- MLX VLM route on 8102
- watchdog/guardian around service health

Implementation cuts in lbrx-services:

- Add or refine a service manifest export based on `services.yaml`.
- Expose a local-only `/internal/runtime/topology` endpoint or JSON artifact
  that lists service IDs, ports, health probes, process command expectations,
  log paths, and restart policy.
- Teach watchdog to mark maintenance/restart attempts with a unique run ID.
- Teach watchdog not to restart a service when:
  - a listener still owns the port,
  - an existing restart is in progress,
  - a health probe is stuck as a child of the same process group,
  - the service is known to be doing a long heavy task and still emits progress.
- Emit `restart_attempt_id`, old PID, new PID, kill signal, port ownership, and
  outcome to logs.
- Emit `auth_failure` and `license_failure` as structured events, not just log
  prose.

Candidate files in lbrx-services:

- `services.yaml`
- `scripts/lbrx-ctl.sh`
- `api-router/watchdog.py`
- `api-router/app/main.py`
- `api-router/app/routers/llm.py`
- `api-router/app/services/fallback_router.py`
- `api-router/app/core/model_capabilities.py`
- `api-router/tests`

Acceptance:

- Reproducing a wedged listener produces one structured topology plus one
  structured incident slice.
- Watchdog does not create restart storms around an already-owned port.
- `/metrics` 401 noise is classified separately from external `/v1/responses`
  401s.
- Missing `LIBRAXIS_LICENSE_PATH` is a configuration finding with a stable
  code, not a generic error line.

### Track D - AICX Intent And Failure Overlay

Goal: make prior decisions and prior incidents joinable to runtime bundles.

Implementation cuts:

- Add an AICX query profile for runtime incidents:
  - service name
  - port
  - route
  - model alias
  - error phrase
  - process command
  - repo root
  - known config keys
- Return bounded entries with `authority`, `source_chunk`, `session_id`,
  `agent`, `date`, and `relevance`.
- Add dedupe by source chunk and failure class.
- Add a typed `AicxOverlayStatus`:
  - `available`
  - `semantic_index_missing`
  - `cli_unavailable`
  - `mcp_unavailable`
  - `timeout`
  - `empty`
- Add a CLI fallback that uses `aicx search --no-semantic` when semantic index
  is unavailable, but records that fallback in authority.

Candidate files:

- `loctree-rs/src/aicx/`
- `loctree-rs/src/cli/dispatch/handlers/context/mod.rs`
- `loctree-rs/src/runtime/incident.rs`
- sibling `/Users/polyversai/Libraxis/vc-runtime/aicx`

Acceptance:

- For an incident mentioning `8102`, AICX returns prior multimodal fallback and
  watchdog context if present.
- For `Transport closed`, AICX returns prior Loctree/AICX MCP transport notes.
- Empty or unavailable AICX never blocks the incident bundle.
- The bundle says whether the overlay was semantic, lexical, MCP, or CLI.

### Track E - MCP And Stdio Lifecycle Evidence

Goal: stop treating `Transport closed` as an opaque curse.

Implementation cuts:

- Add a local wrapper mode for MCP servers:

```bash
loct mcp-wrap --server loctree-mcp -- /Users/polyversai/.cargo/bin/loctree-mcp
loct mcp-wrap --server aicx-mcp -- /Users/polyversai/.cargo/bin/aicx-mcp
```

- Record:
  - PID, PPID, PGID
  - cwd
  - argv
  - start timestamp
  - first JSON-RPC request observed
  - stdout byte count
  - stderr tail
  - exit code or signal
  - transport close timestamp
  - client name if initialization reveals it
- Avoid logging secrets.
- Add `loct mcp-health` to list old child processes, RSS, and stale sessions.
- Add `loct incident capture --mcp loctree-mcp,aicx-mcp` to join MCP state into
  the same incident bundle.

Acceptance:

- A future Codex `Transport closed` can be classified as child exit, client
  pipe close, protocol error, stdout pollution, startup timeout, or unknown.
- Wrapper logs are small enough to attach to AICX/Loctree reports.
- No secret-bearing environment variables are printed.

### Track F - MIAZGA Scoring

Goal: rank systemic pathologies, not only individual bugs.

Add a score for runtime incidents:

```text
RuntimeMIAZGA(node) =
  0.20 * restart_loop_score
+ 0.15 * memory_pressure_score
+ 0.15 * unresponsive_listener_score
+ 0.15 * auth_config_noise_score
+ 0.10 * prior_failure_density
+ 0.10 * source_churn_score
+ 0.10 * fan_in_or_route_centrality
+ 0.05 * manual_recovery_frequency
```

Initial nodes:

- service instance
- port
- route
- model alias
- source file
- config key
- MCP server
- AICX source chunk

Acceptance:

- 8102 wedge ranks above routine `/metrics` 401.
- Missing license path is visible but separate from VLM liveness.
- Repeated `Transport closed` across hosts becomes a high prior-failure-density
  finding even if direct stdio smoke passes.
- Score explanation lists evidence, not vibes.

### Track G - Context Pack Pagination And Runtime Cards

Goal: make the atlas retrievable by agents without giant single reads.

Build on the existing six card grain:

- core
- structural
- runtime
- memory
- verification
- risk

Add runtime incident cards:

- `06-runtime-incidents.md`
- `07-artifact-identity.md`
- `08-miazga.md`

Or, if the six-card grain must remain stable, fold them as sub-sections under:

- runtime
- memory
- risk

Acceptance:

- `/context_pack?project=<path>&cards=runtime,risk` returns runtime incident
  facts in bounded chunks.
- Cursor pagination preserves the context receipt.
- CLI `loct context --full --json` remains full and unchunked.
- MCP and HTTP clients can fetch a 1000+ line runtime atlas without timeouts.

### Track H - HTML And LSP Surfacing

Goal: make incident truth visible where operators and agents already look.

LSP additions:

- `loctree/runtimeIncidents`
- `loctree/runtimeHealth`
- `loctree/artifactIdentity`
- `loctree/miazga`

HTML report additions:

- runtime incident panel
- process/port table
- auth/license findings
- AICX prior-failure overlay
- action checklist
- scope receipt banner

Acceptance:

- A generated report can show that 8102 was wedged while 8100/8101 were
  healthy.
- The report distinguishes live runtime truth from historical AICX memory.
- The report does not imply a source fix when only an operational restart was
  performed.

## 7. Wave Plan

### Wave 1 - Truth Receipt And Capture Substrate

Parallel slots:

| Slot | Track | Deliverable |
|---|---|---|
| 1.A | A | `ContextReceipt` in context packs |
| 1.B | B | `loct incident capture` read-only MVP |
| 1.C | C | lbrx-services topology artifact |
| 1.D | D | AICX runtime query profile |
| 1.E | E | MCP wrapper design and smoke test |

Wave 1 gate:

- A context pack cannot hide scope ambiguity.
- A runtime incident can be captured without mutating services.
- AICX overlay can attach prior failures or explicitly say why it cannot.

### Wave 2 - Runtime Joins

Parallel slots:

| Slot | Track | Deliverable |
|---|---|---|
| 2.A | B | artifact identity for Python/venv and Rust binaries |
| 2.B | C | watchdog restart run IDs and structured status |
| 2.C | D | AICX dedupe and authority labels |
| 2.D | F | MIAZGA v0 score implementation |
| 2.E | G | runtime card sections in Context Atlas |

Wave 2 gate:

- 8102-style incidents classify as `WedgedListener`.
- restart loops classify separately as supervisor pathologies.
- source joins include package file hash or repo commit when available.

### Wave 3 - Operator Surfaces

Parallel slots:

| Slot | Track | Deliverable |
|---|---|---|
| 3.A | G | HTTP cursor-paginated runtime cards |
| 3.B | H | HTML runtime incident panel |
| 3.C | H | LSP `loctree/runtimeHealth` |
| 3.D | E | `loct mcp-health` |
| 3.E | C | API auth/license structured findings |

Wave 3 gate:

- Agents can fetch runtime incident cards without full-pack overload.
- Operators can inspect the same incident in HTML.
- MCP transport failures have process-level evidence.

### Wave 4 - Release-Grade Integration

Parallel slots:

| Slot | Track | Deliverable |
|---|---|---|
| 4.A | all | end-to-end incident fixture |
| 4.B | all | docs and examples |
| 4.C | all | release gates |
| 4.D | all | AICX corpus sync recipe |
| 4.E | all | regression suite for context scope |

Wave 4 gate:

- A fixture simulates wedged listener plus restart loop plus stale artifact log.
- `loct incident capture` produces deterministic JSON.
- `loct context --full --json` includes incident and scope receipt.
- `cargo clippy --workspace --all-targets -- -D warnings` is green.
- lbrx-services tests for watchdog/auth/license pass in its own repo.

## 8. Dependency Graph

```text
ContextReceipt
  -> context cards
  -> HTTP pagination
  -> HTML/LSP display

IncidentCapture
  -> ArtifactIdentity
  -> RuntimeDiagnosis
  -> MIAZGA
  -> Context cards

lbrx-services topology
  -> IncidentCapture adapter quality
  -> Watchdog structured events
  -> API auth/license findings

AICX runtime overlay
  -> MIAZGA prior failure density
  -> operator explanation

MCP wrapper
  -> TransportClosed classification
  -> incident evidence for Loctree/AICX tools themselves
```

Critical path:

```text
ContextReceipt -> IncidentCapture -> RuntimeDiagnosis -> Runtime cards -> HTML/MCP/LSP
```

## 9. Data Contracts

### 9.1 Context Receipt JSON

```json
{
  "requested_path": "/Users/polyversai/Libraxis/vc-runtime/loctree-suite",
  "canonical_root": "/Users/polyversai/Libraxis/vc-runtime/loctree-suite",
  "selected_scan_root": "/Users/polyversai/Libraxis/vc-runtime/loctree-suite",
  "snapshot_path": ".../snapshot.json",
  "snapshot_file_count": 0,
  "snapshot_edge_count": 0,
  "scope_confidence": "repo_root_verified",
  "warnings": []
}
```

### 9.2 Incident Summary JSON

```json
{
  "incident_id": "inc-20260527-8102-vlm",
  "diagnosis": {
    "primary": "WedgedListener",
    "secondary": ["SupervisorRestartLoop"],
    "side_findings": ["AuthMisconfiguration", "LicenseMisconfiguration"]
  },
  "services": [
    {
      "service_id": "mlx-batch-vlm",
      "port": 8102,
      "status": "unresponsive_listener"
    }
  ],
  "actions": [
    {
      "kind": "operator_restart",
      "safety": "manual_only",
      "reason": "listener held by idle 36GB process"
    }
  ]
}
```

### 9.3 AICX Overlay JSON

```json
{
  "status": "available",
  "mode": "lexical_fallback",
  "entries": [
    {
      "kind": "prior_failure",
      "summary": "multimodal fallback / 8102 instability",
      "source_chunk": "/Users/polyversai/.aicx/store/...",
      "session_id": "...",
      "authority": "aicx_agent"
    }
  ]
}
```

## 10. Verification Matrix

| Layer | Gate |
|---|---|
| Context truth | `loct context --full --json` on suite root shows receipt and non-fixture scope |
| Context regression | fixture root still works and is labeled as fixture/subtree |
| Incident CLI | `loct incident capture --port 8102 --json` emits deterministic bundle |
| Runtime probes | tests cover healthy, connect timeout, HTTP timeout, 401, 500, no listener |
| Artifact identity | tests cover Rust binary, Python venv file, package file hash |
| AICX overlay | unavailable semantic index returns typed fallback status |
| MCP wrapper | fake server exit and stdout pollution fixtures classify correctly |
| lbrx-services | watchdog tests cover owned port, in-progress restart, stuck curl child |
| HTML | report renders incident panel without overlapping cards |
| LSP | `loctree/runtimeHealth` returns bounded response |
| End-to-end | simulated 8102 incident joins service, process, log, source, AICX, MIAZGA |

## 11. First 48-Hour Implementation Cut

The first cut should be intentionally sharp:

1. Add `ContextReceipt` and make the current `App.tsx` false-scope impossible
   to miss.
2. Add read-only `loct incident capture` for ports/processes/probes/log slices.
3. Add one lbrx-services fixture or captured sample for the 8102 wedged
   listener class.
4. Add AICX overlay status, even if it initially shells out to `aicx search
   --no-semantic`.
5. Write one HTML/Markdown incident card from the captured bundle.

Do not start with full eBPF, native symbolication, or reverse-engineering
tooling. Those are correct later layers, but the immediate product pain is
runtime truth capture plus join discipline.

## 12. Non-Goals For The First Cut

- No automatic service restarts from Loctree.
- No secret capture.
- No replacement of lbrx-services watchdog.
- No claim that AICX memory is live truth.
- No binary-level DWARF symbolication as a blocker for Python/MLX incidents.
- No huge unpaginated MCP runtime dumps.

## 13. Risks

- The context pack scope bug can poison every downstream plan if treated as
  truth. Mitigation: receipt first.
- AICX search can be slow or unavailable. Mitigation: bounded fallback and
  overlay status.
- Watchdog changes can disrupt live heavy tasks. Mitigation: tests plus
  maintenance windows and no automatic kill from Loctree.
- Runtime capture can leak secrets if environment is dumped naively.
  Mitigation: env fingerprints and allowlisted keys only.
- Cross-repo plan drift can make implementation ambiguous. Mitigation:
  each wave writes a report and exact command receipts.

## 14. Exit Contract

This plan is complete when the following are true:

- `loct context --full --json` includes a scope receipt.
- `loct incident capture` exists and can capture the VLM wedge class.
- AICX overlay is typed and bounded.
- lbrx-services emits or exposes enough topology for capture to be reliable.
- HTML, MCP, and LSP have a path to show runtime incident truth.
- MIAZGA can rank at least three real classes:
  - wedged listener
  - supervisor restart loop
  - auth/license misconfiguration

Vibecrafted with AI Agents by vetcoders.
