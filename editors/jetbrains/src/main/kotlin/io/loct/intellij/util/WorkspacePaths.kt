/*
 * Workspace-bound path resolution and traversal guards.
 *
 * Ports the safety rules from editors/vscode/src/commands.ts:
 * control-character rejection, file:// handling, and rejection of any
 * target that resolves outside the workspace root. Used by file actions
 * and suppression writes. Pure JVM logic (unit-testable).
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.util

import java.net.URI
import java.nio.file.Path
import java.nio.file.Paths

object WorkspacePaths {

    private val CONTROL_CHARS = Regex("[\\x00-\\x1F\\x7F]")

    fun hasUnsafeControlChars(value: String): Boolean = CONTROL_CHARS.containsMatchIn(value)

    /** True when [absolutePath] is the workspace root or nested within it. */
    fun isWithinWorkspace(absolutePath: Path, workspaceRoot: Path): Boolean {
        val root = workspaceRoot.toAbsolutePath().normalize()
        val target = absolutePath.toAbsolutePath().normalize()
        return target == root || target.startsWith(root)
    }

    /**
     * Resolve [filePath] against [workspaceRoot], handling file:// URIs
     * and absolute/relative inputs. Returns null on control chars or
     * malformed URIs.
     */
    fun resolveFilePath(filePath: String, workspaceRoot: Path): Path? {
        if (hasUnsafeControlChars(filePath)) return null

        if (filePath.startsWith("file://")) {
            return runCatching { Paths.get(URI(filePath)) }.getOrNull()
        }

        val candidate = Paths.get(filePath)
        return if (candidate.isAbsolute) {
            candidate.normalize()
        } else {
            workspaceRoot.resolve(candidate).normalize()
        }
    }

    /**
     * Resolve [filePath] and confirm it stays inside [workspaceRoot].
     * Returns null when the path is invalid or escapes the workspace.
     */
    fun toWorkspaceAbsolutePath(filePath: String, workspaceRoot: Path): Path? {
        val resolved = resolveFilePath(filePath, workspaceRoot) ?: return null
        return if (isWithinWorkspace(resolved, workspaceRoot)) resolved else null
    }
}
