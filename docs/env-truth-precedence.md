# `loct env-truth` — Precedence Doctrine (Cut 8 / Lane 4)

> Higher rank means "more likely to win at deploy time." This is a heuristic
> ranking, NOT a verified runtime claim. The orchestrator labels every fact
> as `SemanticGuess` so downstream agents do not mistake the ranking for
> ground truth.

## Why Lane 4 exists

Vista (April 2026) shipped 10 days with broken auth because a stale
`SealedSecret` payload silently overrode a freshly-edited `.env`. Nobody
caught it. Reason: every project that combines plain `.env` + Docker
Compose + k8s manifests + sealed/SOPS secrets + GitHub Actions secrets has
the same blind spot — there is no tool that surfaces the **precedence
chain** of env declarations and warns when **stale source overrides fresh
source**.

`loct env-truth` is that tool. Lane 4 of the LOCTREE_NEXT.md doctrine:

- Lane 1: structural truth.
- Lane 2: runtime semantics (idiom + dispatch + env READ contracts).
- Lane 3: memory / intent overlay (AICX).
- **Lane 4: config truth — env DECLARATION-side audit + drift.**
- Lane 5: live runtime introspection (out of scope here).

## What we never do

We never decode encrypted/sealed payloads, even when local keys could in
principle do it. SealedSecret, SOPS, `ENC[AES256_GCM,...]` blobs surface as
`ValuePresence::Encrypted { marker }` — operators can see the file is
present, can see how old it is, and can compare against fresher plain
declarations, but the content stays opaque. This is by design: env-truth
is a read-only audit and decoded values would leak through any output
channel (stdout, logs, agent context packs, CI artifacts).

## Default precedence table

The orchestrator ships with this default ranking. Operator can override
the whole table via `.loctree/config.toml` (see "Override" below).

| Rank | `EnvSourceKind`                  | What it represents                                      |
|-----:|----------------------------------|---------------------------------------------------------|
| 100  | `sealed_secret`                  | bitnami SealedSecret, applied last in deploy chain      |
|  95  | `external_secret`                | External Secrets Operator                               |
|  92  | `k8s_secret_string_data`         | k8s `Secret.stringData` (plain value)                   |
|  90  | `k8s_secret`                     | k8s `Secret.data` (base64, never decoded)               |
|  85  | `k8s_deployment_env`             | container `env:` literal in Deployment / StatefulSet    |
|  82  | `k8s_deployment_env_from`        | container `envFrom:` reference                          |
|  80  | `k8s_config_map`                 | `ConfigMap.data` env entry                              |
|  78  | `sops_file`                      | SOPS-encrypted file (presence + age only)               |
|  65  | `helm_values`                    | Helm `values*.yaml` env block                           |
|  50  | `docker_compose`                 | docker-compose `environment:` literal                   |
|  45  | `docker_compose_env_file`        | docker-compose `env_file:` resolved entries             |
|  40  | `dockerfile`                     | Dockerfile `ENV` directive                              |
|  35  | `npm_script`                     | `package.json` script with `KEY=value` prefix           |
|  30  | `dot_env`                        | `.env` (base; refined per filename — see below)         |
|  20  | `github_actions_secret`          | `${{ secrets.X }}` reference                            |
|  15  | `github_actions_env`             | `.github/workflows/*.yml` `env:` block                  |
|  12  | `tauri_conf`                     | `tauri.conf.json` env-related field (best-effort)       |
|   8  | `env_rc`                         | `.envrc` (direnv shell-level override)                  |

### Dotenv file-name refinement

Within `dot_env` we further differentiate by file name (the base rank 30
adjusts up or down):

| Suffix                          | Adjustment | Reasoning                          |
|---------------------------------|------------|------------------------------------|
| `.env.example` / `.env.template`/ `.env.sample` | rank 5      | Intent only — not a real value      |
| `.env.production` / `.env.prod` | +8 → 38    | Higher precedence in prod deploys  |
| `.env.staging` / `.env.stage`   | +4 → 34    | Mid-tier                           |
| `.env`                          | 30         | Default                            |
| `.env.local`                    | -5 → 25    | Local-only override                |
| `.env.test` / `.env.dev`        | -8 → 22    | Test/dev overrides                 |

## Override via `.loctree/config.toml`

Per-repo customization. Unknown keys log a warning and are ignored.

```toml
[env_truth]
precedence = { sealed_secret = 50, dot_env = 99 }
stale_threshold_days = 14
```

Effect: this repo treats `.env` as authoritative (rank 99) and SealedSecret
as advisory (rank 50). Useful for monorepos where dotenv files ship with
the deploy artifact and SealedSecrets are decorative.

`stale_threshold_days` sets the minimum age delta (in days) before a
`stale-overrides-fresh` warning fires. The CLI `--stale-threshold-days` flag
takes precedence over this config value; the hardcoded default is 7 days.

## Read side — how a variable earns a reader

The precedence table above ranks **declarations**. The other half of the audit
is the read side, and for a long time it came from one place only:
`semantic_facts.env_contracts`, which covers the shell / Python / Make readers.
A repo whose live contract is consumed by Rust therefore looked like a repo
with no consumers at all — live keys got no heading, and keys that *were*
declared got libelled `orphan-declaration: declared but never read`. That is a
catalogue that actively invites an agent to delete a working flag.

Source files are now scanned directly. Rust hides its env reads behind three
shapes, and each carries a different strength of proof, so the required name
shape tightens as the proof weakens:

| Tier | Shape | Example | Name requirement |
|---|---|---|---|
| 1 | the env API itself | `std::env::var("STT_ENDPOINT")` | any SCREAMING_SNAKE literal |
| 2 | accessor whose identifier says `env` | `effective_env_string("STT_ENGINE", ..)`, `env_bool("X", ..)` | any SCREAMING_SNAKE literal |
| 3 | `const` / `static` key registry | `const PROMOTED_SETTINGS_KEYS: &[&str] = &["ASR_MODE", ..]` | binding named `ENV`/`KEY`/`SETTING`/`VAR` **and** key literal with ≥1 underscore |

Tier 3 is the **promoted-key** shape: the key never reaches `env::var` because
the app routes it through a settings brain. It is the weakest evidence, hence
the double fence — a `const ALLOWED_METHODS: &[&str] = &["GET", "POST"]` can
never contribute an env variable.

Two shapes are deliberately **not** reads, because calling them reads would
re-create the same class of lie in the opposite direction:

- **Mutation verbs** — an accessor with a `set` / `remove` / `unset` / `save` /
  `persist` / `restore` / `seed` / `inject` / `reset` / `clear` / `export` /
  `write` / `store` segment writes the environment. Matching is segment-wise on
  `_`, so `env_settings(..)` stays a reader while `set_env_var(..)` does not.
- **Child-process builders** — `command.env("K", "V")` / `.envs(..)` populate a
  child environment. Only the bare method name is rejected; every genuine
  reader wrapper has a more descriptive name.

Calls that rustfmt broke across lines are followed with exactly one line of
lookahead, and the read is attributed to the call line. One line, because
rustfmt's first argument is the key or nothing is.

Detection is line-oriented and therefore comment- and string-blind. This is an
inventory that must not miss a live contract key, not a reachability proof.

### Coverage is stated, never implied

`source_reads` in the JSON report (and the **Read-side coverage** block in the
Markdown) names the access classes recognised, the number of source files
opened, and the read sites found. `null` / "scan did not run" means there was no
snapshot to enumerate source files — not that nothing was found. An empty
`Reads:` block is only actionable next to a statement of where we looked;
without it, an operator reads silence as proof that a key is dead.

## Authority labeling

Every declaration emits an `authority: AuthorityLabel`:

- `RepoVerified` — the file exists and was read from disk this scan.
- `SemanticGuess` — the precedence rank is heuristic (always for env-truth).
- `StaleOrUnknown` — declaration appears via `orphan_reads` (read site
  exists in `semantic_facts.env_contracts` but no source file declared it).

CI gates and downstream agents should treat `SemanticGuess` as advisory:
"this is likely the resolution order, but you should verify before
making destructive decisions."

## Stale-overrides-fresh threshold

A stale-overrides-fresh warning fires when:

- the highest-precedence source is materially older than a lower-rank source by
- at least `stale_threshold_days` (default: 7).

The Vista-specific specialization (`SealedSecretSuspectedStale`) fires when
the highest-precedence source is `SealedSecret` / `SOPS` / `ExternalSecret`
**and** any lower-rank `Plain` source is materially fresher. That covers
the "stale ciphertext masks rotated plaintext" pattern even when the
fresh plain source is several precedence layers below the sealed payload.

## Out of scope

- AWS Secrets Manager / GCP Secret Manager / HashiCorp Vault — runtime
  introspection, Lane 5.
- Live process env (`/proc/PID/environ`) — runtime introspection.
- Multi-environment matrix diff (dev / staging / prod) — possible
  follow-up `loct env-diff <env-a> <env-b>`.
- Cross-repo scan — env-truth audits a single repo tree.
- Pulumi / Terraform variable extraction — possible follow-up.
- Auto-fix / auto-rotate suggestions — surfacing only.

## Output discipline

JSON schema is pinned at version `"1.2"`. Additive fields are minor;
breaking changes are major (`"2.0"`). Fields consumers can rely
on:

- top-level: `schema_version`, `generated_at`, `roots`, `declarations`,
  `orphan_reads`, `template_drift` (1.1), `source_reads` (1.2), `summary`.
- per-declaration: `name`, `sources` (sorted descending by `precedence_rank`),
  `reads`, `precedence_warnings`, `authority`.
- `value_present.kind` discriminator: `plain` / `encrypted` / `env_from` /
  `secret` / `empty`.

Markdown output is operator-runbook friendly: H1 / H2 hierarchy, tables,
authority labels rendered in italics. Suitable for `loct env-truth --md > ENV.md`
and pasting into a runbook.

## See also

- `loctree-rs/src/analyzer/env_truth/` — implementation.
- `loctree-rs/tests/fixtures/env_drift/` — drift fixture replicating the
  Vista pattern.
- `LOCTREE_NEXT.md` — Lane doctrine.
- `docs/semantic-spec.md` — Lane 2/3 (read-side env_contracts, AICX overlay).
