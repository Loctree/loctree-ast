/*
 * Safe, workspace-bound file navigation for the Loctree plugin.
 *
 * Rejects paths that resolve outside the project (mirrors the VS Code
 * navigateToFile safety rules). Opens the file in the editor and, when a
 * line is supplied, positions the caret on it.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.util

import com.intellij.openapi.fileEditor.OpenFileDescriptor
import com.intellij.openapi.fileEditor.ex.FileEditorManagerEx
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.LocalFileSystem
import io.loct.intellij.LoctreeBundle
import java.nio.file.Files
import java.nio.file.Path

object FileNavigator {

    /**
     * Open [filePath] (relative to the project root or absolute) at the
     * optional [line] (1-based). Returns true on success; shows a warning
     * and returns false on traversal/missing-file failures.
     */
    fun navigate(project: Project, filePath: String, line: Int? = null): Boolean {
        val root = project.basePath?.let { Path.of(it) }
        if (root == null) {
            LoctreeNotifier.warn(project, LoctreeBundle.message("notify.outsideWorkspace"))
            return false
        }

        val absolute = WorkspacePaths.toWorkspaceAbsolutePath(filePath, root)
        if (absolute == null) {
            LoctreeNotifier.warn(project, LoctreeBundle.message("notify.outsideWorkspace"))
            return false
        }

        if (!Files.isRegularFile(absolute)) {
            LoctreeNotifier.warn(project, "${LoctreeBundle.message("notify.fileNotFound")} $filePath")
            return false
        }

        val virtualFile = LocalFileSystem.getInstance().refreshAndFindFileByNioFile(absolute)
        if (virtualFile == null) {
            LoctreeNotifier.warn(project, "${LoctreeBundle.message("notify.fileNotFound")} $filePath")
            return false
        }

        val targetLine = line?.let { (it - 1).coerceAtLeast(0) } ?: 0
        val descriptor = OpenFileDescriptor(project, virtualFile, targetLine, 0)
        FileEditorManagerEx.getInstanceEx(project).openTextEditor(descriptor, true)
        return true
    }
}
