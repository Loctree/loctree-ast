/*
 * Query router tests for the Loctree tool window.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.toolwindow

import com.google.gson.JsonParser
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LoctreeQueryRouterTest {

    @Test
    fun exposesAllLspCustomQueryModesUsedByIdeParitySurface() {
        val methods = QueryMode.entries.map { it.method }.toSet()

        assertTrue(methods.contains("loctree/health"))
        assertTrue(methods.contains("loctree/find"))
        assertTrue(methods.contains("loctree/body"))
        assertTrue(methods.contains("loctree/impact"))
        assertTrue(methods.contains("loctree/slice"))
        assertTrue(methods.contains("loctree/follow"))
        assertTrue(methods.contains("loctree/contextAtlas"))
        assertTrue(methods.contains("loctree/contextPack"))
        assertTrue(methods.contains("loctree/workspaces"))
        assertTrue(methods.contains("loctree/diff"))
        assertTrue(methods.contains("loctree/semantic"))
        assertTrue(methods.contains("loctree/aicx"))
        assertTrue(methods.contains("loctree/astQuery"))
    }

    @Test
    fun routesLiteralSearchWithAgentFacingKnobs() {
        val request = LoctreeQueryRouter.requestFor(QueryMode.Literal, "backdrop")

        assertEquals("loctree/find", request.method)
        assertEquals("backdrop", request.params["query"])
        assertEquals("backdrop", request.params["symbol"])
        assertEquals("literal", request.params["mode"])
        assertEquals(true, request.params["whole_token"])
        assertEquals(true, request.params["group_by_file"])
        assertEquals(25, request.params["limit"])
        assertEquals(0, request.params["offset"])
    }

    @Test
    fun routesContextPackWithTaskAndStableCardDefaults() {
        val request = LoctreeQueryRouter.requestFor(QueryMode.ContextPack, "literal search UI")

        assertEquals("loctree/contextPack", request.method)
        assertEquals("literal search UI", request.params["task"])
        assertEquals(listOf("core", "structural", "runtime", "risk"), request.params["cards"])
    }

    @Test
    fun routesBodyWithBoundedPreviewDefaults() {
        val request = LoctreeQueryRouter.requestFor(QueryMode.Body, "resolveServerBinary")

        assertEquals("loctree/body", request.method)
        assertEquals("resolveServerBinary", request.params["symbol"])
        assertEquals(80, request.params["max_lines"])
    }

    @Test
    fun derivesCursorContinuationFromContextPackResponse() {
        val request = LoctreeQueryRouter.requestFor(QueryMode.ContextPack, "IDE task")
        val result = JsonParser.parseString("""{"next_cursor":"cursor-2","card":"core"}""")

        val next = LoctreeQueryRouter.continuationFor(request, result)

        assertNotNull(next)
        assertEquals("loctree/contextPack", next!!.method)
        assertEquals("cursor-2", next.params["cursor"])
    }

    @Test
    fun routesFileScopedSliceThroughTargetForContextActions() {
        val request = LoctreeQueryRouter.requestFor(QueryMode.Slice, "src/Main.kt")

        assertEquals("loctree/slice", request.method)
        assertEquals("src/Main.kt", request.params["target"])
        assertEquals(true, request.params["consumers"])
    }

    @Test
    fun routesFileScopedImpactWithTransitiveForContextActions() {
        val request = LoctreeQueryRouter.requestFor(QueryMode.Impact, "src/Main.kt")

        assertEquals("loctree/impact", request.method)
        assertEquals("src/Main.kt", request.params["target"])
        assertEquals(true, request.params["transitive"])
    }

    @Test
    fun routesDeadExportCheckThroughFollowDeadScope() {
        val request = LoctreeQueryRouter.requestFor(QueryMode.Follow, "dead")

        assertEquals("loctree/follow", request.method)
        assertEquals("dead", request.params["scope"])
    }

    @Test
    fun derivesOffsetContinuationFromLiteralMatchesResponse() {
        val request = LoctreeQueryRouter.requestFor(QueryMode.Literal, "backdrop")
        val result = JsonParser.parseString(
            """{"literal_matches":{"page":{"next_offset":25},"occurrences":[]}}""",
        )

        val next = LoctreeQueryRouter.continuationFor(request, result)

        assertNotNull(next)
        assertEquals("loctree/find", next!!.method)
        assertEquals(25, next.params["offset"])
    }

    @Test
    fun derivesCursorContinuationFromFindSymbolMatchesBucket() {
        // `loctree/find` (non-literal) carries its cursor per match bucket, not
        // at the envelope root. Without the recursive fallback the continuation
        // was never derived and "Load More" stayed dead for Find mode.
        val request = LoctreeQueryRouter.requestFor(QueryMode.Find, "Auth")
        val result = JsonParser.parseString(
            """
            {
              "query": "Auth",
              "symbol_matches": {"chunk": 0, "next_cursor": "sym-cursor-2", "data": {"files": []}},
              "param_matches": {"chunk": 0, "next_cursor": null, "data": []}
            }
            """.trimIndent(),
        )

        val next = LoctreeQueryRouter.continuationFor(request, result)

        assertNotNull(next)
        assertEquals("loctree/find", next!!.method)
        assertEquals("sym-cursor-2", next.params["cursor"])
    }

    @Test
    fun derivesCursorContinuationFromSliceConsumersBucket() {
        // `loctree/slice` paginates `deps` and `consumers` independently; the
        // first exhausted bucket reports a null cursor while the other still
        // advances. The fallback must skip the null and find the live cursor.
        val request = LoctreeQueryRouter.requestFor(QueryMode.Slice, "src/Main.kt")
        val result = JsonParser.parseString(
            """
            {
              "deps": {"chunk": 0, "next_cursor": null, "data": []},
              "consumers": {"chunk": 0, "next_cursor": "consumers-cursor-2", "data": []}
            }
            """.trimIndent(),
        )

        val next = LoctreeQueryRouter.continuationFor(request, result)

        assertNotNull(next)
        assertEquals("loctree/slice", next!!.method)
        assertEquals("consumers-cursor-2", next.params["cursor"])
    }
}
