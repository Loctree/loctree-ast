/*
 * Project-wide LSP server descriptor for loctree-lsp.
 *
 * Resolves the runtime via BinaryResolver and launches it with
 * `--root <projectBasePath>` (mirroring the loctree-lsp `--root` flag).
 * Declares the supported file types matching VS Code parity and wires
 * the custom lsp4j server interface for loctree custom requests.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.lsp

import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.openapi.diagnostic.logger
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.platform.lsp.api.Lsp4jClient
import com.intellij.platform.lsp.api.LspServerNotificationsHandler
import com.intellij.platform.lsp.api.ProjectWideLspServerDescriptor
import io.loct.intellij.binary.BinaryResolver
import io.loct.intellij.statusbar.LoctreeStatusService
import io.loct.intellij.util.LoctreeNotifier
import org.eclipse.lsp4j.services.LanguageServer

class LoctreeLspServerDescriptor(project: Project) :
    ProjectWideLspServerDescriptor(project, "Loctree") {

    private val log = logger<LoctreeLspServerDescriptor>()

    override fun isSupportedFile(file: VirtualFile): Boolean {
        val ext = file.extension?.lowercase() ?: return false
        return ext in SUPPORTED_EXTENSIONS
    }

    override fun createCommandLine(): GeneralCommandLine {
        val resolved = BinaryResolver().resolve()
        log.info(
            "Starting loctree-lsp (${resolved.source}): ${resolved.command} | ${resolved.identity}",
        )
        LoctreeStatusService.getInstance(project).updateRuntime(resolved)
        resolved.warning?.let {
            log.warn(it)
            LoctreeNotifier.warn(project, it)
        }

        val commandLine = GeneralCommandLine(resolved.command)
        project.basePath?.let { base ->
            commandLine.addParameters("--root", base)
            commandLine.withWorkDirectory(base)
        }
        commandLine.withCharset(Charsets.UTF_8)
        return commandLine
    }

    /** Wire the custom server interface so the gateway can issue loctree custom requests. */
    override val lsp4jServerClass: Class<out LanguageServer> = LoctreeLsp4jServer::class.java

    /** Accept loctree custom server notifications (scanProgress) without log spam. */
    override fun createLsp4jClient(handler: LspServerNotificationsHandler): Lsp4jClient =
        LoctreeLsp4jClient(handler)

    companion object {
        val SUPPORTED_EXTENSIONS: Set<String> = setOf(
            "ts", "tsx", "js", "jsx", "mjs", "cjs", "rs", "py", "go",
        )
    }
}
