/*
 * Bridge for sending action results into the Loctree tool window.
 *
 * Context-menu actions should not grow separate renderers; they publish the
 * same routed query result that the inline tool-window console uses.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.toolwindow

import com.google.gson.JsonElement
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.ToolWindowManager
import java.util.WeakHashMap

internal object LoctreeToolWindowBridge {
    private const val TOOL_WINDOW_ID = "Loctree"
    private val panels = WeakHashMap<Project, LoctreeFindingsPanel>()

    fun register(project: Project, panel: LoctreeFindingsPanel) {
        panels[project] = panel
    }

    fun showResult(project: Project, request: LoctreeQueryRequest, result: JsonElement?) {
        ApplicationManager.getApplication().invokeLater {
            ToolWindowManager.getInstance(project).getToolWindow(TOOL_WINDOW_ID)?.show(null)
            panels[project]?.showExternalQueryResult(request, result)
        }
    }
}
