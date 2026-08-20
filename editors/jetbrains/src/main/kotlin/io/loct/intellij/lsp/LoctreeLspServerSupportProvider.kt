/*
 * Native LSP support provider for loctree-lsp.
 *
 * Starts the loctree language server when a supported file is opened.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.lsp

import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.LspServerSupportProvider
import com.intellij.platform.lsp.api.LspServerSupportProvider.LspServerStarter

class LoctreeLspServerSupportProvider : LspServerSupportProvider {
    override fun fileOpened(
        project: Project,
        file: VirtualFile,
        serverStarter: LspServerStarter,
    ) {
        val ext = file.extension?.lowercase() ?: return
        if (ext !in LoctreeLspServerDescriptor.SUPPORTED_EXTENSIONS) {
            return
        }
        serverStarter.ensureServerStarted(LoctreeLspServerDescriptor(project))
    }
}
