/*
 * Loctree design tokens mapped to IntelliJ JBColor.
 *
 * Source of truth: loctree-com / reports editorial dark
 * (ink/bone + amber narrative + teal interaction).
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
 */

package io.loct.intellij

import com.intellij.ui.JBColor
import java.awt.Color

object LoctreeColors {
    /** --amber: #c99a3b — narrative accent / warning */
    val AMBER = JBColor(Color(0xc99a3b), Color(0xc99a3b))

    /** --teal: #3d7a72 — interaction / success / live state */
    val TEAL = JBColor(Color(0x3d7a72), Color(0x3d7a72))

    /** --status-danger: #b86a5c */
    val DANGER = JBColor(Color(0xb86a5c), Color(0xb86a5c))

    /** --bone: #f5f1e7 — warm white (dark-theme foreground) */
    val BONE = JBColor(Color(0x1a1a18), Color(0xf5f1e7))

    /** Muted bone for secondary labels */
    val BONE_MUTE = JBColor(Color(0x6a6a62), Color(0xa39e92))

    /** Faint bone for borders / chip fills */
    val BONE_FAINT = JBColor(Color(0x1a1a18).brighter(), Color(0x2a2a26))

    /** --ink surfaces */
    val INK = JBColor(Color(0xfbfbf8), Color(0x0e0e0e))
    val INK2 = JBColor(Color(0xf3f1ea), Color(0x161616))
    val INK3 = JBColor(Color(0xeae6dc), Color(0x1e1e1e))

    fun withAlpha(base: Color, alpha: Int): Color =
        Color(base.red, base.green, base.blue, alpha.coerceIn(0, 255))
}
