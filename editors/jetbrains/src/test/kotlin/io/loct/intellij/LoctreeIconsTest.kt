package io.loct.intellij

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class LoctreeIconsTest {
    @Test
    fun logoResourceLoads() {
        assertEquals(16, LoctreeIcons.Logo.iconWidth)
        assertEquals(16, LoctreeIcons.Logo.iconHeight)
    }

    @Test
    fun statusLogoFitsStatusBarSlot() {
        assertEquals(16, LoctreeIcons.StatusLogo.iconWidth)
        assertEquals(16, LoctreeIcons.StatusLogo.iconHeight)
    }

    /**
     * JetBrains convention: pluginIcon.svg is the LIGHT-theme icon and
     * pluginIcon_dark.svg the dark one. The light icons must use dark ink
     * (a near-white mark is invisible on the white Marketplace card), the
     * dark variants carry the original light artwork, and the plugin icon
     * canvas is the 40x40 Marketplace spec.
     */
    @Test
    fun marketplaceIconsShipLightAndDarkVariantsWithCorrectInk() {
        val light = javaClass.getResource("/META-INF/pluginIcon.svg")!!.readText()
        val dark = javaClass.getResource("/META-INF/pluginIcon_dark.svg")!!.readText()

        assertTrue(light.contains("width=\"40\""))
        assertTrue(light.contains("height=\"40\""))
        assertTrue(light.contains("viewBox=\"0 0 40 40\""))
        assertTrue(dark.contains("width=\"40\""))
        assertTrue(dark.contains("viewBox=\"0 0 40 40\""))

        assertFalse("light plugin icon must not use near-white ink", light.contains("#e0e0e0"))
        assertTrue(dark.contains("#e0e0e0"))

        val actionLight = javaClass.getResource("/icons/loctree-action.svg")!!.readText()
        val actionDark = javaClass.getResource("/icons/loctree-action_dark.svg")!!.readText()
        assertFalse("light action icon must not use near-white ink", actionLight.contains("#e0e0e0"))
        assertTrue(actionDark.contains("#e0e0e0"))
    }

    @Test
    fun ideChromeUsesActionSizedIcon() {
        val actionIcon = javaClass.getResource("/icons/loctree-action.svg")!!.readText()
        assertTrue(actionIcon.contains("width=\"16\""))
        assertTrue(actionIcon.contains("height=\"16\""))

        val pluginXml = javaClass.getResource("/META-INF/loctree-lsp.xml")!!.readText()
        assertTrue(pluginXml.contains("icon=\"/icons/loctree-action.svg\""))
        assertFalse(pluginXml.contains("icon=\"/icons/loctree.svg\""))
    }
}
