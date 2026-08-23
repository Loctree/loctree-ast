/*
 * Loctree Context-King tool window.
 *
 * Editorial dashboard (status chips + metric tiles + command bar) over the
 * agent query router. Findings stay secondary; empty groups say "clean",
 * never "Clear". Loading runs off the EDT; double-click navigates.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
 */

package io.loct.intellij.toolwindow

import com.google.gson.JsonElement
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.ide.CopyPasteManager
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.project.Project
import com.intellij.openapi.wm.ToolWindow
import com.intellij.openapi.wm.ToolWindowFactory
import com.intellij.ui.components.JBLabel
import com.intellij.ui.components.JBScrollPane
import com.intellij.ui.components.JBTextField
import com.intellij.ui.treeStructure.Tree
import com.intellij.util.ui.JBUI
import com.intellij.util.ui.UIUtil
import io.loct.intellij.LoctreeBundle
import io.loct.intellij.LoctreeColors
import io.loct.intellij.lsp.LoctreeLspGateway
import io.loct.intellij.protocol.HealthParams
import io.loct.intellij.protocol.HealthResponse
import io.loct.intellij.util.FileNavigator
import io.loct.intellij.util.LoctreeNotifier
import java.awt.BorderLayout
import java.awt.Font
import java.awt.GridBagConstraints
import java.awt.GridBagLayout
import java.awt.Insets
import java.awt.datatransfer.StringSelection
import java.awt.event.ActionEvent
import java.awt.event.MouseAdapter
import java.awt.event.MouseEvent
import java.nio.file.Path
import javax.swing.JButton
import javax.swing.JComboBox
import javax.swing.JPanel
import javax.swing.tree.DefaultMutableTreeNode
import javax.swing.tree.DefaultTreeModel
import javax.swing.tree.TreePath
import javax.swing.tree.TreeSelectionModel

class LoctreeToolWindowFactory : ToolWindowFactory, DumbAware {
    override fun createToolWindowContent(project: Project, toolWindow: ToolWindow) {
        val panel = LoctreeFindingsPanel(project)
        LoctreeToolWindowBridge.register(project, panel)
        val content = toolWindow.contentManager.factory.createContent(panel.component, "", false)
        toolWindow.contentManager.addContent(content)
        panel.reload()
    }
}
/** Marker payload distinguishing the kind of tree node. */
internal sealed interface NodePayload
internal data class GroupPayload(val kind: GroupKind, val count: Int) : NodePayload
internal data class ItemPayload(val finding: FindingItem) : NodePayload
internal data class StatusPayload(val text: String, val tone: ChipTone = ChipTone.MUTED) : NodePayload

class LoctreeFindingsPanel(private val project: Project) {

    private val root = DefaultMutableTreeNode("Loctree Context")
    private val model = DefaultTreeModel(root)
    private val tree = Tree(model).apply {
        isRootVisible = false
        showsRootHandles = true
        rowHeight = JBUI.scale(22)
        selectionModel.selectionMode = TreeSelectionModel.SINGLE_TREE_SELECTION
        border = JBUI.Borders.empty(0, 4, 4, 4)
    }

    private val statusStrip = StatusChipStrip()
    private val metricStrip = MetricStrip()
    private val healthHero = HealthHeroPanel(project).also { it.setMetricsComponent(metricStrip) }

    private val modeSelector = JComboBox(QueryMode.entries.toTypedArray()).apply {
        toolTipText = LoctreeBundle.message("toolwindow.query.mode.tooltip")
    }
    private val queryField = JBTextField().apply {
        emptyText.text = LoctreeBundle.message("toolwindow.query.placeholder")
        toolTipText = LoctreeBundle.message("toolwindow.query.field.tooltip")
    }
    private val submitButton = JButton(LoctreeBundle.message("toolwindow.query.run")).apply {
        isDefaultCapable = true
    }
    private val loadMoreButton = JButton(LoctreeBundle.message("toolwindow.query.loadMore")).apply {
        isEnabled = false
    }
    private val copyAgentButton = JButton(LoctreeBundle.message("toolwindow.copy.agent")).apply {
        toolTipText = LoctreeBundle.message("toolwindow.copy.agent.tooltip")
    }
    private var continuation: LoctreeQueryRequest? = null
    private var lastHealth: HealthResponse? = null
    private var lastFindings: FindingsData? = null
    private var lastProjected: ProjectedResult? = null
    private var lastProjectedMethod: String? = null
    private var clipboardMode: ClipboardMode = ClipboardMode.Findings

    private val footer = JBLabel(LoctreeBundle.message("toolwindow.footer")).apply {
        font = font.deriveFont(Font.PLAIN, 10f)
        foreground = LoctreeColors.BONE_MUTE
        border = JBUI.Borders.empty(6, 12, 10, 12)
        horizontalAlignment = javax.swing.SwingConstants.LEFT
    }

    private val commandBar = JPanel(GridBagLayout()).apply {
        isOpaque = false
        border = JBUI.Borders.empty(0, 12, 8, 12)
        val c = GridBagConstraints().apply {
            insets = Insets(0, 0, 0, 0)
            fill = GridBagConstraints.HORIZONTAL
            weightx = 0.0
            gridx = 0
            gridy = 0
        }
        add(monoEyebrow(LoctreeBundle.message("toolwindow.query.eyebrow")), c)
        c.gridy = 1
        c.insets = Insets(0, 0, 0, 6)
        add(modeSelector, c)
        c.gridx = 1
        c.weightx = 1.0
        c.insets = Insets(0, 0, 0, 6)
        add(queryField, c)
        c.gridx = 2
        c.weightx = 0.0
        c.insets = Insets(0, 0, 0, 4)
        add(submitButton, c)
        c.gridx = 3
        c.insets = Insets(0, 0, 0, 4)
        add(loadMoreButton, c)
        c.gridx = 4
        c.insets = Insets(0, 0, 0, 0)
        add(copyAgentButton, c)
    }

    private val northPanel = JPanel(BorderLayout()).apply {
        isOpaque = false
        add(statusStrip, BorderLayout.NORTH)
        add(healthHero, BorderLayout.CENTER)
        add(commandBar, BorderLayout.SOUTH)
    }

    private enum class ClipboardMode { Findings, Projected, Health }

    val component = JPanel(BorderLayout()).apply {
        background = UIUtil.getPanelBackground()
        add(northPanel, BorderLayout.NORTH)
        add(JBScrollPane(tree), BorderLayout.CENTER)
        add(footer, BorderLayout.SOUTH)
    }

    init {
        submitButton.addActionListener { runQueryFromControls() }
        queryField.addActionListener { _: ActionEvent -> runQueryFromControls() }
        loadMoreButton.addActionListener { continuation?.let { runQuery(it, append = true) } }
        copyAgentButton.addActionListener { copyForAgent() }
        tree.addMouseListener(object : MouseAdapter() {
            override fun mouseClicked(event: MouseEvent) {
                if (event.clickCount != 2) return
                val path: TreePath = tree.getPathForLocation(event.x, event.y) ?: return
                val node = path.lastPathComponent as? DefaultMutableTreeNode ?: return
                val payload = node.userObject as? ItemPayload ?: return
                val file = payload.finding.file ?: return
                FileNavigator.navigate(project, file, payload.finding.line)
            }
        })
        // Initial chrome before first load lands.
        statusStrip.update(
            StatusChipModel("Context", "…", ChipTone.MUTED),
            StatusChipModel("LSP", "…", ChipTone.MUTED),
        )
        metricStrip.update(zeroMetrics())
        healthHero.updateHealth(null)
    }

    private fun copyForAgent() {
        val text = when (clipboardMode) {
            ClipboardMode.Projected -> {
                val projected = lastProjected
                val method = lastProjectedMethod
                if (projected == null || method == null) null
                else AgentBrief.fromProjected(method, projected)
            }
            ClipboardMode.Health -> lastHealth?.let { AgentBrief.fromHealth(it) }
            ClipboardMode.Findings -> {
                val findings = lastFindings
                if (findings != null) AgentBrief.fromFindings(findings, lastHealth)
                else lastHealth?.let { AgentBrief.fromHealth(it) }
            }
        }
        if (text.isNullOrBlank()) {
            LoctreeNotifier.info(project, LoctreeBundle.message("notify.copy.empty"))
            return
        }
        CopyPasteManager.getInstance().setContents(StringSelection(text))
        LoctreeNotifier.info(project, LoctreeBundle.message("notify.copy.ok"))
    }

    fun runQueryFromControls() {
        val mode = modeSelector.selectedItem as? QueryMode ?: QueryMode.ContextPack
        val query = queryField.text.trim()
        runQuery(LoctreeQueryRouter.requestFor(mode, query))
    }

    internal fun showExternalQueryResult(
        request: LoctreeQueryRequest,
        result: JsonElement?,
        append: Boolean = false,
    ) {
        if (result == null) {
            continuation = null
            loadMoreButton.isEnabled = false
            if (!append) {
                val message = if (LoctreeLspGateway.getInstance(project).isRunning()) {
                    LoctreeBundle.message("notify.queryFailed")
                } else {
                    LoctreeBundle.message("notify.lspNotRunning")
                }
                showStatus(message, ChipTone.DANGER)
            }
            return
        }
        continuation = LoctreeQueryRouter.continuationFor(request, result)
        loadMoreButton.isEnabled = continuation != null
        renderProjectedResult(request.method, result, append)
    }

    /** Reload findings asynchronously and rebuild the dashboard. */
    fun reload() {
        showStatus(LoctreeBundle.message("toolwindow.loading"), ChipTone.WARN)
        val lspRunning = LoctreeLspGateway.getInstance(project).isRunning()
        updateChrome(AtlasStatus.LOADING, lspRunning = lspRunning, healthScore = null)
        metricStrip.update(zeroMetrics(subtitle = LoctreeBundle.message("toolwindow.metric.loading")))
        val basePath = project.basePath
        if (basePath == null) {
            updateChrome(AtlasStatus.MISSING, lspRunning = false, healthScore = null)
            showStatus(LoctreeBundle.message("toolwindow.empty"), ChipTone.MUTED)
            return
        }
        ApplicationManager.getApplication().executeOnPooledThread {
            val rootPath = Path.of(basePath)
            val gateway = LoctreeLspGateway.getInstance(project)
            val running = gateway.isRunning()
            val health = if (running) {
                gateway.health(HealthParams(includeTopRisks = true))
            } else {
                null
            }
            val disk = runCatching { FindingsReader.read(rootPath) }.getOrNull()
            val data = when {
                health != null -> FindingsReader.fromHealth(rootPath, health, disk)
                disk != null -> disk
                else -> null
            }
            val atlas = AtlasReader.read(rootPath, lspRunning = running)
            ApplicationManager.getApplication().invokeLater {
                val nowRunning = gateway.isRunning()
                lastHealth = health
                healthHero.updateHealth(health)
                updateChrome(atlas, lspRunning = nowRunning, healthScore = health?.healthScore)
                if (data == null) {
                    lastFindings = null
                    metricStrip.update(zeroMetrics())
                    showStatus(LoctreeBundle.message("toolwindow.error"), ChipTone.DANGER)
                } else {
                    render(data, health?.healthScore)
                }
            }
        }
    }

    private fun runQuery(request: LoctreeQueryRequest, append: Boolean = false) {
        if (!append) {
            showStatus(LoctreeBundle.message("toolwindow.query.loading"), ChipTone.WARN)
        }
        loadMoreButton.isEnabled = false
        ApplicationManager.getApplication().executeOnPooledThread {
            val result = LoctreeLspGateway.getInstance(project).customRequest(request.method, request.params)
            ApplicationManager.getApplication().invokeLater {
                showExternalQueryResult(request, result, append)
            }
        }
    }

    private fun showStatus(text: String, tone: ChipTone = ChipTone.MUTED) {
        root.removeAllChildren()
        root.add(DefaultMutableTreeNode(StatusPayload(text, tone)))
        model.reload()
        tree.cellRenderer = LoctreeCellRenderer()
    }

    private fun render(data: FindingsData, healthScore: Int? = null) {
        root.removeAllChildren()
        lastFindings = data
        clipboardMode = ClipboardMode.Findings
        lastProjected = null
        lastProjectedMethod = null
        metricStrip.update(metricModels(data))
        if (healthScore != null) {
            footer.text = LoctreeBundle.message("toolwindow.footer.score", healthScore)
        } else {
            footer.text = LoctreeBundle.message("toolwindow.footer")
        }

        for (kind in GroupKind.entries) {
            val count = data.counts.getValue(kind)
            val groupNode = DefaultMutableTreeNode(GroupPayload(kind, count))
            val items = data.groups.getValue(kind)
            if (items.isEmpty()) {
                val emptyLabel = if (count == 0) {
                    LoctreeBundle.message("toolwindow.group.empty")
                } else {
                    LoctreeBundle.message("toolwindow.group.countOnly", count)
                }
                groupNode.add(
                    DefaultMutableTreeNode(
                        StatusPayload(
                            emptyLabel,
                            if (count == 0) ChipTone.LIVE else ChipTone.MUTED,
                        ),
                    ),
                )
            } else {
                items.forEach { groupNode.add(DefaultMutableTreeNode(ItemPayload(it))) }
            }
            root.add(groupNode)
        }
        model.reload()
        tree.cellRenderer = LoctreeCellRenderer()
        // Expand only groups that still have findings — zero-count groups stay collapsed.
        for (i in 0 until tree.rowCount) {
            val path = tree.getPathForRow(i) ?: continue
            val node = path.lastPathComponent as? DefaultMutableTreeNode ?: continue
            val payload = node.userObject as? GroupPayload ?: continue
            if (payload.count > 0) {
                tree.expandPath(path)
            }
        }
    }

    private fun renderProjectedResult(method: String, result: JsonElement, append: Boolean) {
        val projected = ResultProjector.project(method, result)
        if (append && lastProjected != null) {
            lastProjected = ProjectedResult(
                headline = lastProjected!!.headline,
                summaries = lastProjected!!.summaries,
                sections = lastProjected!!.sections + projected.sections,
            )
        } else {
            lastProjected = projected
        }
        lastProjectedMethod = method
        clipboardMode = if (method.endsWith("/health")) ClipboardMode.Health else ClipboardMode.Projected
        renderProjectedInto(root, projected, append)
        model.reload()
        tree.cellRenderer = LoctreeCellRenderer()
        for (i in 0 until minOf(tree.rowCount, 24)) {
            tree.expandRow(i)
        }
    }

    private fun updateChrome(atlas: AtlasStatus, lspRunning: Boolean, healthScore: Int?) {
        val contextChip = StatusChipModel(
            eyebrow = "Context",
            value = atlas.shortValue,
            tone = atlas.tone,
            tooltip = atlas.tooltip ?: atlas.label,
        )
        val lspChip = if (lspRunning) {
            StatusChipModel(
                eyebrow = "LSP",
                value = LoctreeBundle.message("toolwindow.lsp.running.short"),
                tone = ChipTone.LIVE,
                tooltip = if (healthScore != null) {
                    LoctreeBundle.message("toolwindow.lsp.running.tooltip.score", healthScore)
                } else {
                    LoctreeBundle.message("toolwindow.lsp.running.tooltip")
                },
            )
        } else {
            StatusChipModel(
                eyebrow = "LSP",
                value = LoctreeBundle.message("toolwindow.lsp.waiting.short"),
                tone = ChipTone.WARN,
                tooltip = LoctreeBundle.message("toolwindow.lsp.waiting.tooltip"),
            )
        }
        statusStrip.update(contextChip, lspChip)
    }

    private fun metricModels(data: FindingsData): List<MetricTileModel> =
        GroupKind.entries.map { kind ->
            val count = data.counts.getValue(kind)
            MetricTileModel(
                kind = kind,
                title = groupTitle(kind),
                count = count,
                subtitle = if (count == 0) {
                    LoctreeBundle.message("toolwindow.metric.clean")
                } else {
                    LoctreeBundle.message("toolwindow.metric.review")
                },
            )
        }

    private fun zeroMetrics(subtitle: String = LoctreeBundle.message("toolwindow.metric.idle")): List<MetricTileModel> =
        GroupKind.entries.map {
            MetricTileModel(it, groupTitle(it), 0, subtitle)
        }

    private fun groupTitle(kind: GroupKind): String = when (kind) {
        GroupKind.DEAD -> LoctreeBundle.message("toolwindow.group.dead")
        GroupKind.CYCLES -> LoctreeBundle.message("toolwindow.group.cycles")
        GroupKind.TWINS -> LoctreeBundle.message("toolwindow.group.twins")
    }
}
