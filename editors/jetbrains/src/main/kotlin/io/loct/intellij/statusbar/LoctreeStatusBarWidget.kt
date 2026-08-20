/*
 * Loctree status bar widget + factory.
 *
 * Reflects the current LoctreeStatus and, on click, opens the Loctree
 * tool window and triggers a health refresh via the LSP gateway.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.statusbar

import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.StatusBar
import com.intellij.openapi.wm.StatusBarWidget
import com.intellij.openapi.wm.StatusBarWidgetFactory
import com.intellij.openapi.wm.ToolWindowManager
import com.intellij.util.Consumer
import io.loct.intellij.LoctreeBundle
import io.loct.intellij.LoctreeIcons
import io.loct.intellij.lsp.LoctreeLspGateway
import io.loct.intellij.settings.LoctreeSettingsState
import java.awt.event.MouseEvent

class LoctreeStatusBarWidgetFactory : StatusBarWidgetFactory {
    override fun getId(): String = ID

    override fun getDisplayName(): String = LoctreeBundle.message("statusbar.name")

    override fun isAvailable(project: Project): Boolean =
        LoctreeSettingsState.getInstance().showStatusBar

    override fun createWidget(project: Project): StatusBarWidget = LoctreeStatusBarWidget(project)

    override fun isConfigurable(): Boolean = true

    companion object {
        const val ID = "io.loct.intellij.statusbar.LoctreeStatusBarWidget"
    }
}

class LoctreeStatusBarWidget(private val project: Project) :
    StatusBarWidget, StatusBarWidget.IconPresentation {

    private var statusBar: StatusBar? = null

    override fun ID(): String = LoctreeStatusBarWidgetFactory.ID

    override fun getPresentation(): StatusBarWidget.WidgetPresentation = this

    override fun install(statusBar: StatusBar) {
        this.statusBar = statusBar
    }

    override fun dispose() {
        statusBar = null
    }

    // Use the status-bar-scaled logo, not the raw SVG, so the full mark fits
    // the 16x16 IDE status slot instead of rendering as cropped top dots.
    override fun getIcon(): javax.swing.Icon = LoctreeIcons.StatusLogo

    override fun getTooltipText(): String {
        val service = LoctreeStatusService.getInstance(project)
        val status = service.status
        val statusText = when (status) {
            LoctreeStatus.DISCONNECTED -> LoctreeBundle.message("statusbar.disconnected")
            LoctreeStatus.DOWNLOADING -> LoctreeBundle.message("statusbar.downloading")
            LoctreeStatus.RUNNING -> LoctreeBundle.message("statusbar.running")
            LoctreeStatus.HEALTHY -> LoctreeBundle.message("statusbar.healthy")
            LoctreeStatus.ERROR -> LoctreeBundle.message("statusbar.error")
        }
        val runtime = service.runtime
        val provenance = runtime?.let {
            "\nBinary: ${it.command}\nIdentity: ${it.identity}\nSource: ${it.source}" +
                (it.warning?.let { warning -> "\nWarning: $warning" } ?: "")
        }.orEmpty()
        return "${LoctreeBundle.message("statusbar.tooltip")}: $statusText$provenance"
    }

    override fun getClickConsumer(): Consumer<MouseEvent> = Consumer {
        ToolWindowManager.getInstance(project).getToolWindow("Loctree")?.show(null)
        LoctreeLspGateway.getInstance(project).refresh()
    }
}
