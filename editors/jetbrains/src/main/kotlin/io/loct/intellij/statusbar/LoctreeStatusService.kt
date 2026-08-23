/*
 * Tracks and broadcasts the Loctree status-bar state for a project.
 *
 * States mirror the VS Code status bar plus the verified-download stage:
 * disconnected, downloading, running, healthy, error.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.statusbar

import com.intellij.openapi.components.Service
import com.intellij.openapi.components.service
import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.WindowManager
import io.loct.intellij.binary.ResolvedRuntime

enum class LoctreeStatus { DISCONNECTED, DOWNLOADING, RUNNING, HEALTHY, ERROR }

@Service(Service.Level.PROJECT)
class LoctreeStatusService(private val project: Project) {

    @Volatile
    var status: LoctreeStatus = LoctreeStatus.DISCONNECTED
        private set

    @Volatile
    var runtime: ResolvedRuntime? = null
        private set

    fun updateRuntime(resolved: ResolvedRuntime) {
        runtime = resolved
        WindowManager.getInstance().getStatusBar(project)
            ?.updateWidget(LoctreeStatusBarWidgetFactory.ID)
    }

    fun update(newStatus: LoctreeStatus) {
        status = newStatus
        WindowManager.getInstance().getStatusBar(project)
            ?.updateWidget(LoctreeStatusBarWidgetFactory.ID)
    }

    companion object {
        fun getInstance(project: Project): LoctreeStatusService = project.service()
    }
}
