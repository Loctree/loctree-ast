# JetBrains Plugin

Loctree for JetBrains is a native IntelliJ Platform plugin powered by
`loctree-lsp`. It mirrors the VS Code integration with diagnostics,
navigation, custom Loctree actions, a findings tool window, and a status bar
surface inside JetBrains IDEs.

## Installation Status

The plugin is not yet published to JetBrains Marketplace. Build it from source
while the Marketplace listing, signing, and verifier matrix are being finished.

```bash
cd editors/jetbrains
./gradlew test
./gradlew buildPlugin
```

The distributable ZIP is written under `editors/jetbrains/build/distributions/`.
Install it from the IDE with **Settings > Plugins > Install Plugin from Disk**.

## Compatibility

- Target IDE line: IntelliJ Platform **2025.2.1+** (`since-build=252.1`).
- Required platform module: `com.intellij.modules.lsp`.
- Current verifier lane: IntelliJ IDEA Ultimate, with broader IDE coverage to
  add after the first stable Marketplace lane.
- Runtime: `loctree-lsp`, resolved from settings, SHA256-verified IDE cache,
  verified release download, or `PATH`.

## Source Layout

- `editors/jetbrains/src/main/kotlin/` — plugin code.
- `editors/jetbrains/src/main/resources/META-INF/plugin.xml` — Marketplace
  descriptor and module dependencies.
- `editors/jetbrains/src/test/kotlin/` — unit tests for binary resolution,
  protocol decoding, findings parsing, and workspace-safe writes.

See `editors/jetbrains/README.md` for the full build, settings, and release
notes.

---

*Vibecrafted with AI Agents by Vetcoders (c)2024-2026 LibraxisAI*
