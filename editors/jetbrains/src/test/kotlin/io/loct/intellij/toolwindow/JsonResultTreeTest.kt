/*
 * Projected result tree tests for the Loctree tool window.
 *
 * Locks: (1) no raw JSON dumps, (2) Load More appends without wiping page 1.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
 */

package io.loct.intellij.toolwindow

import com.google.gson.JsonParser
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test
import javax.swing.tree.DefaultMutableTreeNode

class JsonResultTreeTest {

    private fun payload(parent: DefaultMutableTreeNode, index: Int): Any? =
        (parent.getChildAt(index) as DefaultMutableTreeNode).userObject

    private fun allPayloads(root: DefaultMutableTreeNode): List<Any> {
        val out = mutableListOf<Any>()
        fun walk(node: DefaultMutableTreeNode) {
            node.userObject?.let { out += it }
            for (i in 0 until node.childCount) {
                walk(node.getChildAt(i) as DefaultMutableTreeNode)
            }
        }
        walk(root)
        return out
    }

    @Test
    fun healthProjectsToHumanRowsNotRawJson() {
        val root = DefaultMutableTreeNode("Loctree")
        val page = JsonParser.parseString(
            """
            {
              "health_score": 82,
              "status": "yellow",
              "dead_exports": 4,
              "cycles": 1,
              "twins": 0,
              "hotspots": 2,
              "top_risks": [
                {"kind":"dead_export","file":"src/a.kt","severity":"high","message":"unused Foo"}
              ]
            }
            """.trimIndent(),
        )
        val projected = ResultProjector.project("loctree/health", page)
        renderProjectedInto(root, projected, append = false)

        val texts = allPayloads(root).filterIsInstance<StatusPayload>().map { it.text }
        assertTrue(texts.any { it.contains("82/100") })
        assertTrue(allPayloads(root).any { it is ItemPayload && it.finding.label.contains("Foo") })
        // No raw field dumps
        assertFalse(allPayloads(root).any { it.toString().contains("{") && it.toString().contains("fields") })
    }

    @Test
    fun findProjectsLiteralHitsAsNavigableItems() {
        val page = JsonParser.parseString(
            """
            {
              "literal_matches": {
                "data": [
                  {"file":"src/a.kt","line":10,"symbol":"Auth"},
                  {"file":"src/b.kt","line":3,"name":"Auth"}
                ]
              }
            }
            """.trimIndent(),
        )
        val projected = ResultProjector.project("loctree/find", page)
        assertEquals(1, projected.sections.size)
        assertEquals(2, projected.sections.first().items.size)
        assertTrue(projected.sections.first().items.all { it.file != null })
    }

    @Test
    fun freshRenderClearsStaleTreeThenAddsHeadline() {
        val root = DefaultMutableTreeNode("Loctree")
        root.add(DefaultMutableTreeNode(StatusPayload("stale page from a previous query")))

        val page = JsonParser.parseString("""{"symbol_matches":{"data":[{"file":"a.kt","line":1,"symbol":"X"}]}}""")
        val projected = ResultProjector.project("loctree/find", page)
        renderProjectedInto(root, projected, append = false)

        assertTrue(root.childCount >= 2)
        assertTrue(payload(root, 0) is StatusPayload)
        assertFalse(
            allPayloads(root).any {
                it is StatusPayload && it.text == "stale page from a previous query"
            },
        )
    }

    @Test
    fun loadMoreAppendsNextPageWithoutDiscardingPriorResults() {
        val root = DefaultMutableTreeNode("Loctree")
        val page1 = JsonParser.parseString(
            """{"literal_matches":{"data":[{"file":"a.kt","line":1,"symbol":"A"}]}}""",
        )
        renderProjectedInto(root, ResultProjector.project("loctree/find", page1), append = false)
        val afterFirstPage = root.childCount

        val page2 = JsonParser.parseString(
            """{"literal_matches":{"data":[{"file":"b.kt","line":2,"symbol":"B"}]}}""",
        )
        renderProjectedInto(root, ResultProjector.project("loctree/find", page2), append = true)

        assertTrue("Load More must add nodes, not replace", root.childCount > afterFirstPage)
        assertTrue(payload(root, 0) is StatusPayload)
        val moreHeaders = allPayloads(root).filterIsInstance<StatusPayload>().count { it.text.startsWith("More ·") }
        assertEquals(1, moreHeaders)
    }

    @Test
    fun appendDoesNotDuplicateOriginalHeadline() {
        val root = DefaultMutableTreeNode("Loctree")
        val page = JsonParser.parseString("""{"value":1}""")
        val projected = ResultProjector.project("loctree/diff", page)
        renderProjectedInto(root, projected, append = false)
        renderProjectedInto(root, projected, append = true)
        renderProjectedInto(root, projected, append = true)

        val headlines = allPayloads(root)
            .filterIsInstance<StatusPayload>()
            .map { it.text }
        // Original headline once; two "More ·" markers
        assertEquals(1, headlines.count { it == projected.headline })
        assertEquals(2, headlines.count { it.startsWith("More ·") })
    }
}
