# Codex skill mirrors

This directory is intentionally empty in v0.1.0. The 9 skills live under `../../skills/<name>/SKILL.md` as canonical Claude Code skill bodies. Codex consumers read the SKILL.md files directly via the path map in `../AGENTS.md` — no duplication.

If you need codex-specific skill variants (e.g., to strip Claude Code-only frontmatter fields like `allowed-tools` and `argument-hint`), generate them here with a build step:

```bash
# Example: strip Claude Code frontmatter, write codex-flat skills
for skill in ../../skills/*/SKILL.md; do
  name=$(basename "$(dirname "$skill")")
  awk '/^---$/{f++;next} f==2{print}' "$skill" > "${name}.md"
done
```

Until codex shows it cannot parse SKILL.md frontmatter cleanly, this directory stays empty.
