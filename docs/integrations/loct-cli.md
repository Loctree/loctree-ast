# Loct CLI

## `loct context --scope`

`--scope` is the deterministic structural narrowing primitive for `loct context`.
Use it when the operator already knows the region of the codebase to inspect.
Use `--task` on top of `--scope` when prose intent should rank or highlight
signals inside that region.

| Selector  | Example                          | Meaning                                                                             |
|-----------|----------------------------------|-------------------------------------------------------------------------------------|
| `path:`   | `path:loctree-rs/src/cli/`       | Match project-root-relative path prefix or glob                                     |
| `tag:`    | `tag:rust-reexport`              | Match files with the idiom tag in semantic facts                                    |
| `import:` | `import:loctree-rs/src/types.rs` | Match downstream consumers importing the target                                     |
| `reach:`  | `reach:compose_context_pack`     | Match files reachable from a known symbol via available dispatch/reachability facts |

Named scopes live in `<project-root>/.loctree/scopes.toml`:

```toml
[scopes."context-pipeline"]
description = "Files that compose ContextPack output"
selectors = ["path:loctree-rs/src/cli/dispatch/handlers/context/"]
```

Examples:

```bash
loct context --scope 'path:loctree-rs/src/cli/'
loct context --scope 'context-pipeline'
loct context --scope 'context-pipeline' --task 'cache invalidation'
loct context --file Cargo.toml --scope 'context-pipeline'
```

When `--file` and `--scope` are both present, `--file` wins and the CLI emits a
warning. When `--scope` and `--task` are both present, scope chooses the file
set and task only ranks within that set.
