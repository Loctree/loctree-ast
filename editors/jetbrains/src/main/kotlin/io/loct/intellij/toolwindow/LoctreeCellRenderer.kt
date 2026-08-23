/*
 * Tree cell renderer for the Loctree findings tool window.
 *
 * Brand-colored group headers, severity-aware finding rows, and calm
 * empty-state leaves (never the old "Clear" corpse label).
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
 */

package io.loct.intellij.toolwindow

import com.intellij.icons.AllIcons
import com.intellij.ui.ColoredTreeCellRenderer
import com.intellij.ui.JBColor
import com.intellij.ui.SimpleTextAttributes
import com.intellij.util.ui.JBUI
import io.loct.intellij.LoctreeBundle
import io.loct.intellij.LoctreeColors
import java.awt.Component
import java.awt.Graphics
import java.awt.Graphics2D
import java.awt.RenderingHints
import java.awt.geom.Ellipse2D
import javax.swing.Icon
import javax.swing.JTree
import javax.swing.tree.DefaultMutableTreeNode

internal class LoctreeCellRenderer : ColoredTreeCellRenderer() {
    override fun customizeCellRenderer(
        tree: JTree,
        value: Any?,
        selected: Boolean,
        expanded: Boolean,
        leaf: Boolean,
        row: Int,
        hasFocus: Boolean,
    ) {
        val node = value as? DefaultMutableTreeNode ?: return
        when (val payload = node.userObject) {
            is GroupPayload -> {
                val title = when (payload.kind) {
                    GroupKind.DEAD -> LoctreeBundle.message("toolwindow.group.dead")
                    GroupKind.CYCLES -> LoctreeBundle.message("toolwindow.group.cycles")
                    GroupKind.TWINS -> LoctreeBundle.message("toolwindow.group.twins")
                }
                val color = when (payload.kind) {
                    GroupKind.CYCLES -> LoctreeColors.AMBER
                    GroupKind.DEAD -> LoctreeColors.DANGER
                    GroupKind.TWINS -> LoctreeColors.TEAL
                }
                val muted = payload.count == 0
                val attrs = SimpleTextAttributes(
                    if (muted) SimpleTextAttributes.STYLE_PLAIN else SimpleTextAttributes.STYLE_BOLD,
                    if (muted) LoctreeColors.BONE_MUTE else color,
                )
                append(title, attrs)
                append(
                    "  ${payload.count}",
                    SimpleTextAttributes(
                        SimpleTextAttributes.STYLE_BOLD,
                        if (muted) LoctreeColors.BONE_MUTE else color,
                    ),
                )
                if (muted) {
                    append(
                        "  · clean",
                        SimpleTextAttributes(SimpleTextAttributes.STYLE_ITALIC, LoctreeColors.BONE_MUTE),
                    )
                }
                icon = DotIcon(if (muted) LoctreeColors.BONE_MUTE else color, filled = !muted)
                toolTipText = if (muted) {
                    LoctreeBundle.message("toolwindow.group.tooltip.clean", title)
                } else {
                    LoctreeBundle.message("toolwindow.group.tooltip.count", title, payload.count)
                }
            }

            is ItemPayload -> {
                val finding = payload.finding
                val accent = when (finding.severity) {
                    "high" -> LoctreeColors.DANGER
                    "warning" -> LoctreeColors.AMBER
                    "low" -> LoctreeColors.BONE_MUTE
                    else -> JBColor.foreground()
                }
                append(finding.label, SimpleTextAttributes(SimpleTextAttributes.STYLE_PLAIN, accent))
                finding.description?.let {
                    append("  $it", SimpleTextAttributes.GRAYED_ATTRIBUTES)
                }
                toolTipText = finding.tooltip ?: finding.label
                icon = when (finding.severity) {
                    "high" -> AllIcons.General.Error
                    "warning" -> AllIcons.General.Warning
                    else -> AllIcons.General.Information
                }
            }

            is StatusPayload -> {
                val color = toneColor(payload.tone)
                append(
                    payload.text,
                    SimpleTextAttributes(
                        if (payload.tone == ChipTone.LIVE) SimpleTextAttributes.STYLE_ITALIC
                        else SimpleTextAttributes.STYLE_PLAIN,
                        color,
                    ),
                )
                icon = when (payload.tone) {
                    ChipTone.LIVE -> AllIcons.General.InspectionsOK
                    ChipTone.WARN -> AllIcons.General.Warning
                    ChipTone.DANGER -> AllIcons.General.Error
                    ChipTone.MUTED, ChipTone.NEUTRAL -> AllIcons.General.Information
                }
            }

            else -> append(node.userObject?.toString().orEmpty())
        }
    }
}

/** Small brand-colored status/group dot — denser than stock warning icons. */
private class DotIcon(
    private val color: java.awt.Color,
    private val filled: Boolean,
) : Icon {
    private val size = JBUI.scale(10)

    override fun getIconWidth(): Int = size
    override fun getIconHeight(): Int = size

    override fun paintIcon(c: Component?, g: Graphics, x: Int, y: Int) {
        val g2 = g.create() as Graphics2D
        try {
            g2.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON)
            val pad = 1.5f
            val oval = Ellipse2D.Float(x + pad, y + pad, size - pad * 2, size - pad * 2)
            if (filled) {
                g2.color = color
                g2.fill(oval)
            } else {
                g2.color = LoctreeColors.withAlpha(color, 90)
                g2.fill(oval)
                g2.color = color
                g2.draw(oval)
            }
        } finally {
            g2.dispose()
        }
    }
}
