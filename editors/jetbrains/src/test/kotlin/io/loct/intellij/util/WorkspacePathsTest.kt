/*
 * Workspace path traversal-guard tests.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.util

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test
import java.nio.file.Path

class WorkspacePathsTest {

    private val root: Path = Path.of("/workspace/project").toAbsolutePath()

    @Test
    fun relativePathResolvesInsideWorkspace() {
        val resolved = WorkspacePaths.toWorkspaceAbsolutePath("src/App.kt", root)
        assertEquals(root.resolve("src/App.kt"), resolved)
    }

    @Test
    fun traversalOutsideWorkspaceRejected() {
        assertNull(WorkspacePaths.toWorkspaceAbsolutePath("../secret.txt", root))
        assertNull(WorkspacePaths.toWorkspaceAbsolutePath("/etc/passwd", root))
    }

    @Test
    fun controlCharactersRejected() {
        assertTrue(WorkspacePaths.hasUnsafeControlChars("bad\u0000name"))
        assertNull(WorkspacePaths.toWorkspaceAbsolutePath("bad\u0000name", root))
    }

    @Test
    fun rootItselfIsWithinWorkspace() {
        assertTrue(WorkspacePaths.isWithinWorkspace(root, root))
    }
}
