/*
 * Status chips + metric tiles for the Loctree tool window.
 *
 * Editorial dark vocabulary (amber/teal/bone) over plain platform trees.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
 */

package io.loct.intellij.toolwindow

import com.intellij.ui.JBColor
import com.intellij.util.ui.JBUI
import com.intellij.util.ui.UIUtil
import io.loct.intellij.LoctreeColors
import java.awt.BasicStroke
import java.awt.BorderLayout
import java.awt.Color
import java.awt.Component
import java.awt.Cursor
import java.awt.Dimension
import java.awt.FlowLayout
import java.awt.Font
import java.awt.Graphics
import java.awt.Graphics2D
import java.awt.GridLayout
import java.awt.RenderingHints
import java.awt.geom.Ellipse2D
import java.awt.geom.RoundRectangle2D
import javax.swing.Box
import javax.swing.BoxLayout
import javax.swing.JComponent
import javax.swing.JLabel
import javax.swing.JPanel
import javax.swing.SwingConstants
import javax.swing.border.EmptyBorder

/** Visual tone for status chips / empty-state leaves (public for AtlasStatus). */
enum class ChipTone { NEUTRAL, LIVE, WARN, DANGER, MUTED }

internal data class StatusChipModel(
    val eyebrow: String,
    val value: String,
    val tone: ChipTone,
    val tooltip: String? = null,
)

internal data class MetricTileModel(
    val kind: GroupKind,
    val title: String,
    val count: Int,
    val subtitle: String,
)

/** Horizontal status strip: CONTEXT + LSP chips. */
internal class StatusChipStrip : JPanel(FlowLayout(FlowLayout.LEFT, 8, 0)) {
    private val contextChip = StatusChip()
    private val lspChip = StatusChip()

    init {
        isOpaque = false
        border = JBUI.Borders.empty(10, 12, 4, 12)
        add(contextChip)
        add(lspChip)
    }

    fun update(context: StatusChipModel, lsp: StatusChipModel) {
        contextChip.applyModel(context)
        lspChip.applyModel(lsp)
        revalidate()
        repaint()
    }
}

private class StatusChip : JPanel(BorderLayout(8, 0)) {
    private val eyebrow = JLabel().apply {
        font = UIUtil.getLabelFont().deriveFont(Font.BOLD, 10f)
        foreground = LoctreeColors.BONE_MUTE
    }
    private val value = JLabel().apply {
        font = UIUtil.getLabelFont().deriveFont(Font.BOLD, 12f)
    }
    private val textCol = JPanel().apply {
        isOpaque = false
        layout = BoxLayout(this, BoxLayout.Y_AXIS)
        add(eyebrow)
        add(Box.createVerticalStrut(1))
        add(value)
    }
    private var tone: ChipTone = ChipTone.NEUTRAL
    private val dot = object : JComponent() {
        override fun getPreferredSize(): Dimension = Dimension(JBUI.scale(8), JBUI.scale(8))
        override fun paintComponent(g: Graphics) {
            val g2 = g.create() as Graphics2D
            try {
                g2.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON)
                g2.color = toneColor(tone)
                val s = minOf(width, height).toFloat()
                g2.fill(Ellipse2D.Float((width - s) / 2f, (height - s) / 2f, s, s))
            } finally {
                g2.dispose()
            }
        }
    }

    init {
        isOpaque = false
        border = JBUI.Borders.empty(6, 10)
        add(dot, BorderLayout.WEST)
        add(textCol, BorderLayout.CENTER)
        cursor = Cursor.getDefaultCursor()
    }

    fun applyModel(model: StatusChipModel) {
        tone = model.tone
        eyebrow.text = model.eyebrow.uppercase()
        value.text = model.value
        value.foreground = when (model.tone) {
            ChipTone.LIVE -> LoctreeColors.TEAL
            ChipTone.WARN -> LoctreeColors.AMBER
            ChipTone.DANGER -> LoctreeColors.DANGER
            ChipTone.MUTED, ChipTone.NEUTRAL -> UIUtil.getLabelForeground()
        }
        toolTipText = model.tooltip
        repaint()
    }

    override fun paintComponent(g: Graphics) {
        val g2 = g.create() as Graphics2D
        try {
            g2.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON)
            val fill = when (tone) {
                ChipTone.LIVE -> LoctreeColors.withAlpha(LoctreeColors.TEAL, 28)
                ChipTone.WARN -> LoctreeColors.withAlpha(LoctreeColors.AMBER, 28)
                ChipTone.DANGER -> LoctreeColors.withAlpha(LoctreeColors.DANGER, 32)
                ChipTone.MUTED -> LoctreeColors.withAlpha(UIUtil.getLabelForeground(), 12)
                ChipTone.NEUTRAL -> LoctreeColors.withAlpha(UIUtil.getLabelForeground(), 16)
            }
            val border = when (tone) {
                ChipTone.LIVE -> LoctreeColors.withAlpha(LoctreeColors.TEAL, 90)
                ChipTone.WARN -> LoctreeColors.withAlpha(LoctreeColors.AMBER, 90)
                ChipTone.DANGER -> LoctreeColors.withAlpha(LoctreeColors.DANGER, 100)
                else -> LoctreeColors.withAlpha(UIUtil.getLabelForeground(), 40)
            }
            val r = JBUI.scale(10).toFloat()
            val shape = RoundRectangle2D.Float(0.5f, 0.5f, width - 1f, height - 1f, r, r)
            g2.color = fill
            g2.fill(shape)
            g2.color = border
            g2.stroke = BasicStroke(1f)
            g2.draw(shape)
        } finally {
            g2.dispose()
        }
        super.paintComponent(g)
    }
}

/** Three metric tiles for Dead / Cycles / Twins. */
internal class MetricStrip : JPanel(GridLayout(1, 3, 8, 0)) {
    private val tiles = GroupKind.entries.associateWith { MetricTile() }

    init {
        isOpaque = false
        border = JBUI.Borders.empty(6, 12, 8, 12)
        GroupKind.entries.forEach { add(tiles.getValue(it)) }
    }

    fun update(models: List<MetricTileModel>) {
        models.forEach { model ->
            tiles[model.kind]?.applyModel(model)
        }
        revalidate()
        repaint()
    }
}

private class MetricTile : JPanel(BorderLayout()) {
    private val title = JLabel().apply {
        font = UIUtil.getLabelFont().deriveFont(Font.BOLD, 10f)
        foreground = LoctreeColors.BONE_MUTE
        horizontalAlignment = SwingConstants.LEFT
    }
    private val count = JLabel("—").apply {
        font = UIUtil.getLabelFont().deriveFont(Font.BOLD, 20f)
        horizontalAlignment = SwingConstants.LEFT
    }
    private val subtitle = JLabel().apply {
        font = UIUtil.getLabelFont().deriveFont(Font.PLAIN, 10f)
        foreground = LoctreeColors.BONE_MUTE
    }
    private var accent: Color = LoctreeColors.TEAL
    private var active = false

    init {
        isOpaque = false
        border = JBUI.Borders.empty(8, 10)
        val col = JPanel().apply {
            isOpaque = false
            layout = BoxLayout(this, BoxLayout.Y_AXIS)
            add(title)
            add(Box.createVerticalStrut(4))
            add(count)
            add(Box.createVerticalStrut(2))
            add(subtitle)
        }
        add(col, BorderLayout.CENTER)
    }

    fun applyModel(model: MetricTileModel) {
        active = model.count > 0
        accent = when (model.kind) {
            GroupKind.DEAD -> LoctreeColors.DANGER
            GroupKind.CYCLES -> LoctreeColors.AMBER
            GroupKind.TWINS -> LoctreeColors.TEAL
        }
        title.text = model.title.uppercase()
        count.text = model.count.toString()
        count.foreground = if (active) accent else LoctreeColors.BONE_MUTE
        subtitle.text = model.subtitle
        toolTipText = "${model.title}: ${model.count} — ${model.subtitle}"
        repaint()
    }

    override fun paintComponent(g: Graphics) {
        val g2 = g.create() as Graphics2D
        try {
            g2.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON)
            val r = JBUI.scale(12).toFloat()
            val shape = RoundRectangle2D.Float(0.5f, 0.5f, width - 1f, height - 1f, r, r)
            g2.color = if (JBColor.isBright()) {
                Color(0xf6f4ee)
            } else {
                Color(0x161616)
            }
            g2.fill(shape)
            g2.color = if (active) {
                LoctreeColors.withAlpha(accent, 110)
            } else {
                LoctreeColors.withAlpha(UIUtil.getLabelForeground(), 28)
            }
            g2.stroke = BasicStroke(1f)
            g2.draw(shape)
            // left accent bar
            g2.color = if (active) accent else LoctreeColors.withAlpha(UIUtil.getLabelForeground(), 40)
            g2.fillRoundRect(0, JBUI.scale(8), JBUI.scale(3), height - JBUI.scale(16), 2, 2)
        } finally {
            g2.dispose()
        }
        super.paintComponent(g)
    }
}

internal fun toneColor(tone: ChipTone): Color = when (tone) {
    ChipTone.LIVE -> LoctreeColors.TEAL
    ChipTone.WARN -> LoctreeColors.AMBER
    ChipTone.DANGER -> LoctreeColors.DANGER
    ChipTone.MUTED, ChipTone.NEUTRAL -> LoctreeColors.BONE_MUTE
}

/** Mono-cap eyebrow for the command row. */
internal fun monoEyebrow(text: String): JLabel =
    JLabel(text.uppercase()).apply {
        font = UIUtil.getLabelFont().deriveFont(Font.BOLD, 10f)
        foreground = LoctreeColors.BONE_MUTE
        border = EmptyBorder(0, 2, 4, 0)
    }
