/*
 * Append-only writer for cycle suppressions.
 *
 * Ports appendCycleSuppression from editors/vscode/src/commands.ts:
 * writes only to `<workspace>/.loctree/suppressions.toml`, escapes TOML
 * strings, and is idempotent for an already-suppressed cycle. The target
 * directory is fixed to `.loctree` under the workspace root, so writes
 * can never escape the workspace. Pure JVM logic (unit-testable).
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.actions

import java.nio.file.Files
import java.nio.file.Path
import java.time.LocalDate

object CycleSuppressionWriter {

    private const val HEADER = "# Loctree suppressions - findings marked as reviewed/OK\n\n"

    /** Escape a string for safe inclusion inside a TOML double-quoted value. */
    fun tomlString(value: String): String {
        val sanitized = value
            .replace("\\", "\\\\")
            .replace("\"", "\\\"")
            .replace("\n", "\\n")
            .replace("\r", "\\r")
            .replace("\t", "\\t")
            .replace(Regex("[\\x00-\\x08\\x0B\\x0C\\x0E-\\x1F\\x7F]"), "")
        return "\"$sanitized\""
    }

    /**
     * Append a `[[suppress]]` entry for [cycleChain] under
     * `<workspaceRoot>/.loctree/suppressions.toml`. Returns the file path.
     * Throws [IllegalArgumentException] when the chain is empty.
     */
    fun appendCycleSuppression(workspaceRoot: Path, cycleChain: List<String>): Path {
        require(cycleChain.isNotEmpty()) { "Cycle chain must not be empty" }

        val suppressionsDir = workspaceRoot.resolve(".loctree")
        val suppressionsPath = suppressionsDir.resolve("suppressions.toml")
        val cycleSymbol = cycleChain.joinToString(" -> ")
        val filePath = cycleChain.first()
        val date = LocalDate.now().toString()

        Files.createDirectories(suppressionsDir)
        if (!Files.exists(suppressionsPath)) {
            Files.writeString(suppressionsPath, HEADER)
        }

        val existing = Files.readString(suppressionsPath)
        val symbolLine = "symbol = ${tomlString(cycleSymbol)}"
        if (existing.contains("type = \"circular\"") && existing.contains(symbolLine)) {
            return suppressionsPath
        }

        val entry = buildString {
            append('\n')
            append("[[suppress]]\n")
            append("type = \"circular\"\n")
            append("symbol = ${tomlString(cycleSymbol)}\n")
            append("file = ${tomlString(filePath)}\n")
            append("reason = \"Ignored via JetBrains quick action\"\n")
            append("added = ${tomlString(date)}\n")
        }
        Files.writeString(
            suppressionsPath,
            entry,
            java.nio.file.StandardOpenOption.APPEND,
        )
        return suppressionsPath
    }
}
