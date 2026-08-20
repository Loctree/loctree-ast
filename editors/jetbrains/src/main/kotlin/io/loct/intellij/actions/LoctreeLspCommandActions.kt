/*
 * Thin LSP CodeAction command handlers for loctree.* command IDs.
 *
 * IntelliJ resolves CodeAction commands to plugin action IDs (loctree-lsp.xml).
 * These actions delegate to [LoctreeLspCommandRouter] so LSP JSON arguments
 * (file path, cycle chain) are honored when present on the context file.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.actions

import com.intellij.openapi.actionSystem.ActionUpdateThread
import com.intellij.openapi.actionSystem.AnAction
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.CommonDataKeys
import io.loct.intellij.lsp.LoctreeLspCommandRouter
import org.eclipse.lsp4j.Command

open class LoctreeLspCommandAction(private val commandId: String) : AnAction() {
    override fun actionPerformed(e: AnActionEvent) {
        val project = e.project ?: return
        val contextFile = e.getData(CommonDataKeys.VIRTUAL_FILE) ?: return
        LoctreeLspCommandRouter.execute(
            project,
            contextFile,
            Command(commandId, commandId, null),
        )
    }

    override fun getActionUpdateThread(): ActionUpdateThread = ActionUpdateThread.BGT

    override fun update(e: AnActionEvent) {
        e.presentation.isEnabled = e.project != null && e.getData(CommonDataKeys.VIRTUAL_FILE) != null
    }
}

class LspAnalyzeImpactAction : LoctreeLspCommandAction("loctree.analyzeImpact")
class LspFindImportersAction : LoctreeLspCommandAction("loctree.findImporters")
class LspFindConsumersAction : LoctreeLspCommandAction("loctree.findConsumers")
class LspAnalyzeCycleAction : LoctreeLspCommandAction("loctree.analyzeCycle")
class LspOpenReportAction : LoctreeLspCommandAction("loctree.openReport")
class LspCheckDeadExportsAction : LoctreeLspCommandAction("loctree.checkDeadExports")
class LspShowCyclesAction : LoctreeLspCommandAction("loctree.showCycles")
class LspShowCycleDetailsAction : LoctreeLspCommandAction("loctree.showCycleDetails")
class LspNavigateToFileAction : LoctreeLspCommandAction("loctree.navigateToFile")
class LspIgnoreCycleAction : LoctreeLspCommandAction("loctree.ignoreCycle")