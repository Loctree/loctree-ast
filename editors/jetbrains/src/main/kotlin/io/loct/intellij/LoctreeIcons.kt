/*
 * Loctree UI icons.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij

import com.intellij.ui.JBColor
import java.awt.Component
import java.awt.Graphics
import java.awt.Graphics2D
import java.awt.RenderingHints
import java.awt.geom.RoundRectangle2D
import javax.swing.Icon

object LoctreeIcons {
    @JvmField
    val Logo: Icon = StatusIcon

    // Status bar slots are square and small; do not reuse the marketing SVG
    // here. Its large viewport metadata can perturb the status bar layout in
    // some IDE builds even when wrapped. Paint a fixed-size IDE icon instead.
    @JvmField
    val StatusLogo: Icon = StatusIcon
}

private object StatusIcon : Icon {
    private const val SIZE = 16

    override fun getIconWidth(): Int = SIZE

    override fun getIconHeight(): Int = SIZE

    override fun paintIcon(c: Component?, g: Graphics, x: Int, y: Int) {
        val g2 = g.create() as Graphics2D
        try {
            g2.setRenderingHint(RenderingHints.KEY_ANTIALIASING, RenderingHints.VALUE_ANTIALIAS_ON)
            g2.translate(x, y)
            g2.color = JBColor(0x2B2D30, 0xDFE1E5)
            g2.fill(RoundRectangle2D.Double(1.0, 1.0, 14.0, 14.0, 4.0, 4.0))
            g2.color = JBColor(0xA8A8A8, 0x2B2D30)
            g2.font = g2.font.deriveFont(10f)
            val metrics = g2.fontMetrics
            val text = "L"
            val tx = (SIZE - metrics.stringWidth(text)) / 2
            val ty = (SIZE - metrics.height) / 2 + metrics.ascent
            g2.drawString(text, tx, ty)
        } finally {
            g2.dispose()
        }
    }
}
