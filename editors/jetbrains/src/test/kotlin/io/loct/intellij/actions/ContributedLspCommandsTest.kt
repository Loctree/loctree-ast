/*
 * W3 parity guard — ContributedLspCommands must match loctree-lsp.xml action ids.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.actions

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import java.nio.file.Files
import java.nio.file.Path

class ContributedLspCommandsTest {

    @Test
    fun everyContributedCommandIsRegisteredInPluginXml() {
        val repoRoot = Path.of(System.getProperty("user.dir")).parent.parent
        val xml = Files.readString(
            repoRoot.resolve("editors/jetbrains/src/main/resources/META-INF/loctree-lsp.xml"),
        )
        ContributedLspCommands.ALL.forEach { commandId ->
            assertTrue(
                "Missing <action id=\"$commandId\" in loctree-lsp.xml",
                xml.contains("""id="$commandId""""),
            )
        }
    }

    @Test
    fun lspCommandAdaptersStayOutOfPopupMenusAndHaveDefaultText() {
        val repoRoot = Path.of(System.getProperty("user.dir")).parent.parent
        val xml = Files.readString(
            repoRoot.resolve("editors/jetbrains/src/main/resources/META-INF/loctree-lsp.xml"),
        )
        val groupStart = xml.indexOf("<group id=\"io.loct.intellij.actions.LoctreeGroup\"")
        val groupEnd = xml.indexOf("</group>", groupStart)
        assertTrue("LoctreeGroup must exist in loctree-lsp.xml", groupStart >= 0 && groupEnd > groupStart)
        val popupGroup = xml.substring(groupStart, groupEnd)

        ContributedLspCommands.ALL.forEach { commandId ->
            assertFalse(
                "$commandId is a protocol adapter and must not appear as a duplicate popup action",
                popupGroup.contains("""id="$commandId"""),
            )
            val actionStart = xml.indexOf("""<action id="$commandId""", groupEnd)
            val actionEnd = xml.indexOf("/>", actionStart)
            assertTrue("Missing standalone action registration for $commandId", actionStart >= 0 && actionEnd > actionStart)
            val registration = xml.substring(actionStart, actionEnd)
            assertTrue(
                "$commandId must define non-empty default action text",
                Regex("""text="[^"]+"""").containsMatchIn(registration),
            )
        }
    }
}
