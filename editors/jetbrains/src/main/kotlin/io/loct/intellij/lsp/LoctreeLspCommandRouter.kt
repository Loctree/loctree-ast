/*
 * Routes LSP CodeAction / CodeLens command payloads to Loctree tool-window queries.
 *
 * Mirrors editors/vscode/src/commands.ts pickFileForCommand + per-command
 * handlers. IntelliJ CodeAction commands resolve to loctree.* action IDs
 * in loctree-lsp.xml, which delegate here (W3 parity).
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.lsp

import com.google.gson.JsonArray
import com.google.gson.JsonElement
import com.google.gson.JsonObject
import com.google.gson.JsonPrimitive
import com.intellij.ide.BrowserUtil
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.diagnostic.logger
import com.intellij.openapi.project.Project
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VirtualFile
import io.loct.intellij.LoctreeBundle
import io.loct.intellij.toolwindow.LoctreeQueryRouter
import io.loct.intellij.toolwindow.LoctreeToolWindowBridge
import io.loct.intellij.toolwindow.QueryMode
import io.loct.intellij.util.LoctreeNotifier
import io.loct.intellij.util.WorkspacePaths
import org.eclipse.lsp4j.Command
import java.nio.file.Path

internal object LoctreeLspCommandRouter {
    private val log = logger<LoctreeLspCommandRouter>()

    /** @return true when the command was handled client-side (no server forward). */
    fun execute(project: Project, contextFile: VirtualFile, command: Command): Boolean {
        val id = command.command ?: return false
        if (!id.startsWith("loctree.")) return false

        val workspaceRoot = project.basePath?.let { Path.of(it) }
        if (workspaceRoot == null) {
            LoctreeNotifier.warn(project, LoctreeBundle.message("notify.noFileSelected"))
            return true
        }

        val args = command.arguments.orEmpty()
        val filePath = pickFileForCommand(args, contextFile, workspaceRoot)
            ?: run {
                LoctreeNotifier.warn(project, LoctreeBundle.message("notify.noFileSelected"))
                return true
            }

        val rel = workspaceRoot.relativize(Path.of(filePath)).toString().replace('\\', '/')

        return when (id) {
            "loctree.analyzeImpact" -> route(project, QueryMode.Impact, rel, transitive = true)
            "loctree.findImporters", "loctree.findConsumers" ->
                route(project, QueryMode.Impact, rel, transitive = false)
            "loctree.checkDeadExports" -> routeFollow(project, "dead")
            "loctree.showCycles" -> routeFollow(project, "cycles")
            "loctree.analyzeCycle" -> routeFollow(project, "cycles")
            "loctree.openReport" -> openReport(project, workspaceRoot)
            "loctree.showCycleDetails" -> {
                log.info("loctree.showCycleDetails args=${args.size}")
                true
            }
            "loctree.navigateToFile" -> navigate(project, args, workspaceRoot)
            "loctree.ignoreCycle" -> {
                log.info("loctree.ignoreCycle not yet wired in JetBrains parity surface")
                true
            }
            else -> {
                log.warn("Unhandled loctree LSP command: $id")
                false
            }
        }
    }

    private fun route(
        project: Project,
        mode: QueryMode,
        rel: String,
        transitive: Boolean,
    ): Boolean {
        ApplicationManager.getApplication().executeOnPooledThread {
            val request = LoctreeQueryRouter.requestFor(mode, rel)
            if (mode == QueryMode.Impact) {
                request.params["transitive"] = transitive
            }
            val response = LoctreeLspGateway.getInstance(project)
                .customRequest(request.method, request.params)
            if (response == null) {
                LoctreeNotifier.warn(project, LoctreeBundle.message("notify.lspNotRunning"))
            } else {
                LoctreeToolWindowBridge.showResult(project, request, response)
            }
        }
        return true
    }

    private fun routeFollow(project: Project, scope: String): Boolean {
        ApplicationManager.getApplication().executeOnPooledThread {
            val request = LoctreeQueryRouter.requestFor(QueryMode.Follow, scope)
            val response = LoctreeLspGateway.getInstance(project)
                .customRequest(request.method, request.params)
            if (response == null) {
                LoctreeNotifier.warn(project, LoctreeBundle.message("notify.lspNotRunning"))
            } else {
                LoctreeToolWindowBridge.showResult(project, request, response)
            }
        }
        return true
    }

    private fun openReport(project: Project, workspaceRoot: Path): Boolean {
        val report = workspaceRoot.resolve(".loctree").resolve("report.html")
        if (!report.toFile().isFile) {
            LoctreeNotifier.warn(project, LoctreeBundle.message("notify.reportMissing"))
            LoctreeLspGateway.getInstance(project).refresh()
            return true
        }
        BrowserUtil.browse(report.toUri())
        return true
    }

    private fun navigate(project: Project, args: List<Any>, workspaceRoot: Path): Boolean {
        val target = pickFileForCommand(args, null, workspaceRoot) ?: return true
        val vf = LocalFileSystem.getInstance().findFileByPath(target) ?: return true
        ApplicationManager.getApplication().invokeLater {
            com.intellij.openapi.fileEditor.FileEditorManager.getInstance(project).openFile(vf, true)
        }
        return true
    }

    private fun pickFileForCommand(
        args: List<Any>,
        contextFile: VirtualFile?,
        workspaceRoot: Path,
    ): String? {
        for (raw in args) {
            val element = raw as? JsonElement ?: continue
            extractFilePath(element, workspaceRoot)?.let { return it.toString() }
            extractCycleChain(element).firstOrNull()?.let { first ->
                WorkspacePaths.toWorkspaceAbsolutePath(first, workspaceRoot)?.let { return it.toString() }
            }
        }

        contextFile?.path?.let { path ->
            WorkspacePaths.toWorkspaceAbsolutePath(path, workspaceRoot)?.let { return it.toString() }
        }

        return null
    }

    private fun extractFilePath(element: JsonElement, workspaceRoot: Path): Path? {
        when (element) {
            is JsonPrimitive -> {
                if (element.isString) {
                    return WorkspacePaths.toWorkspaceAbsolutePath(element.asString, workspaceRoot)
                }
            }
            is JsonObject -> {
                element.get("file")?.asStringOrNull()?.let {
                    return WorkspacePaths.toWorkspaceAbsolutePath(it, workspaceRoot)
                }
                element.get("uri")?.asStringOrNull()?.let {
                    return WorkspacePaths.toWorkspaceAbsolutePath(it, workspaceRoot)
                }
            }
        }
        return null
    }

    private fun extractCycleChain(element: JsonElement): List<String> {
        if (element !is JsonObject) return emptyList()
        val cycle = element.get("cycle") ?: return emptyList()
        if (cycle !is JsonArray) return emptyList()
        return cycle.mapNotNull { it.asStringOrNull() }
    }

    private fun JsonElement.asStringOrNull(): String? =
        (this as? JsonPrimitive)?.takeIf { it.isString }?.asString
}