# Security Policy

## Supported Versions

Loctree is pre-1.0 and moves fast. We support the **most recently published
release on each distribution channel** (crates.io, npm, Homebrew, the signed
`loct.io` bundles):

| Channel | Supported |
|---|---|
| crates.io (`loctree`, `loctree-mcp`) | latest published version only |
| npm (`@loctree/loctree`) | latest published version only |
| `loct.io/install.sh` bundle | latest published version only |
| Homebrew (`loctree/cli`, `loctree/mcp`) | latest published version only |

Older release lines do not receive backported security fixes. Upgrading to
the latest release is the supported remediation path.

## Reporting a Vulnerability

**Do not open a public GitHub issue for a suspected vulnerability.**

Email **support@loctree.com** with:

- A description of the issue and its potential impact.
- Steps to reproduce (a minimal repo or command sequence is ideal — loctree
  runs entirely on local file input, so most issues are reproducible from a
  small fixture).
- The affected component (`loctree`/`loct` CLI, `loctree-mcp`, `loctree-lsp`,
  the VS Code or JetBrains extension, or the npm/Homebrew packaging) and the
  version you tested.

If you want encrypted communication, say so in your first message and we will
arrange a channel — we do not maintain a published PGP key today.

## What to Expect

This is a small team, not a dedicated security function, so we cannot commit
to a contractual SLA. In practice:

- We aim to acknowledge reports within **5 business days**.
- We will tell you honestly if a fix is going to take longer than that,
  rather than go silent.
- We will credit reporters in the release notes if you want credit, or keep
  you anonymous if you prefer.

## Disclosure Policy

We ask for coordinated disclosure: please give us a reasonable window to
investigate and ship a fix before any public write-up. In turn, we will keep
you informed of progress and agree on a disclosure date with you rather than
impose one. For a genuinely actively-exploited issue, tell us that explicitly
so we can prioritize accordingly.

## Scope Notes

- Loctree analyzes source code you point it at; it does not execute that
  source code. Reports about the analyzer mis-parsing malicious/malformed
  source files as a **crash or hang** are in scope. Reports assuming loctree
  executes scanned code are not applicable — it doesn't.
- The MCP/LSP servers accept local stdio/HTTP connections from an editor or
  agent client. Reports about unauthenticated network exposure, path
  traversal outside the configured project root, or privilege escalation are
  in scope.
- Supply-chain concerns (compromised release artifact, dependency
  vulnerability with a real exploit path) are in scope — see `make semgrep`
  and `.github/workflows/semgrep.yml` for the automated gate we already run.
