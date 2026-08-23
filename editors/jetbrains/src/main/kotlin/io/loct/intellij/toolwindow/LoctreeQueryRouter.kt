/*
 * Query routing for the Loctree tool window.
 *
 * Keeps UI controls, context actions, and future ACP-facing adapters on the
 * same Loctree custom request shapes instead of baking method-specific maps into
 * Swing widgets.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.toolwindow

import com.google.gson.JsonElement
import com.google.gson.JsonObject

internal enum class QueryMode(val label: String, val method: String) {
    ContextPack("Context Pack", "loctree/contextPack"),
    Literal("Literal", "loctree/find"),
    Body("Body", "loctree/body"),
    Impact("Impact", "loctree/impact"),
    Slice("Slice", "loctree/slice"),
    Find("Find", "loctree/find"),
    Follow("Follow", "loctree/follow"),
    Health("Health", "loctree/health"),
    ContextAtlas("Context Atlas", "loctree/contextAtlas"),
    Workspaces("Workspaces", "loctree/workspaces"),
    Diff("Diff", "loctree/diff"),
    Semantic("Semantic", "loctree/semantic"),
    Aicx("AICX", "loctree/aicx"),
    AstQuery("AST Query", "loctree/astQuery");

    override fun toString(): String = label
}

internal data class LoctreeQueryRequest(
    val mode: QueryMode,
    val method: String,
    val params: MutableMap<String, Any?>,
)

internal object LoctreeQueryRouter {
    fun requestFor(mode: QueryMode, query: String): LoctreeQueryRequest =
        LoctreeQueryRequest(mode, mode.method, paramsFor(mode, query.trim()))

    fun continuationFor(
        request: LoctreeQueryRequest,
        result: JsonElement,
    ): LoctreeQueryRequest? {
        val obj = result.asObjectOrNull() ?: return null
        // Explicit fast-paths preserve top-level precedence for the envelope
        // shapes that put the cursor at a known location (contextPack/diff at
        // top, follow under `items`, semantic under flattened `pagination`).
        // The recursive fallback then covers the per-bucket cursors that
        // `loctree/find` (`symbol_matches`/`param_matches`/…) and
        // `loctree/slice` (`deps`/`consumers`) carry — without it, "Load More"
        // is silently dead for those modes even though each bucket paginates.
        val cursor = obj.stringOrNull("next_cursor")
            ?: obj.objOrNull("items")?.stringOrNull("next_cursor")
            ?: obj.objOrNull("pagination")?.stringOrNull("next_cursor")
            ?: obj.objOrNull("data")?.stringOrNull("next_cursor")
            ?: obj.objOrNull("data")?.objOrNull("items")?.stringOrNull("next_cursor")
            ?: obj.objOrNull("data")?.objOrNull("pagination")?.stringOrNull("next_cursor")
            ?: obj.findNestedString("next_cursor")
        if (cursor != null) {
            val next = request.params.toMutableMap()
            next["cursor"] = cursor
            return request.copy(params = next)
        }

        val literal = obj.objOrNull("literal_matches")
            ?: obj.objOrNull("data")?.objOrNull("literal_matches")
        val nextOffset = literal
            ?.objOrNull("page")
            ?.intOrNull("next_offset")
            ?: obj.findNestedInt("next_offset")
        if (nextOffset != null) {
            val next = request.params.toMutableMap()
            next["offset"] = nextOffset
            return request.copy(params = next)
        }
        return null
    }

    private fun paramsFor(mode: QueryMode, query: String): MutableMap<String, Any?> =
        when (mode) {
            QueryMode.Health -> mutableMapOf(
                "include_top_risks" to true,
            )
            QueryMode.Find -> mutableMapOf(
                "query" to query,
                "mode" to "single",
                "limit" to 25,
            )
            QueryMode.Literal -> mutableMapOf(
                "query" to query,
                "symbol" to query,
                "mode" to "literal",
                "whole_token" to true,
                "group_by_file" to true,
                "limit" to 25,
                "offset" to 0,
            )
            QueryMode.Body -> mutableMapOf(
                "symbol" to query,
                "max_lines" to 80,
            )
            QueryMode.Follow -> mutableMapOf(
                "scope" to query.ifBlank { "dead" },
                "limit" to 25,
            )
            QueryMode.Slice -> mutableMapOf(
                "target" to query,
                "consumers" to true,
            )
            QueryMode.Impact -> mutableMapOf(
                "target" to query,
                "transitive" to true,
            )
            QueryMode.ContextAtlas -> mutableMapOf()
            QueryMode.ContextPack -> mutableMapOf(
                "task" to query.ifBlank { null },
                "cards" to listOf("core", "structural", "runtime", "risk"),
            )
            QueryMode.Workspaces -> mutableMapOf()
            QueryMode.Diff -> mutableMapOf(
                "since" to query.ifBlank { "lastQuery" },
                "chunk_size" to 25,
            )
            QueryMode.Aicx -> mutableMapOf(
                "scope" to "project",
                "target" to query.ifBlank { null },
                "limit" to 25,
            )
            QueryMode.Semantic -> mutableMapOf(
                "scope" to if (query.isBlank()) "project" else "file",
                "target" to query.ifBlank { null },
                "chunk_size" to 25,
            )
            QueryMode.AstQuery -> mutableMapOf(
                "language" to "auto",
                "query" to query,
                "limit" to 100,
            )
        }
}

private fun JsonElement.asObjectOrNull(): JsonObject? =
    takeIf { it.isJsonObject }?.asJsonObject

private fun JsonObject.objOrNull(key: String): JsonObject? =
    get(key)?.takeIf { it.isJsonObject }?.asJsonObject

private fun JsonObject.stringOrNull(key: String): String? =
    get(key)?.takeIf { !it.isJsonNull && it.isJsonPrimitive }?.asString

private fun JsonObject.intOrNull(key: String): Int? =
    get(key)?.takeIf { !it.isJsonNull && it.isJsonPrimitive }?.asInt

/**
 * Depth-first search for the first non-blank string stored under [key]
 * anywhere in the response tree. Loctree's paginated custom responses carry
 * their continuation cursor inside per-bucket envelopes (`symbol_matches`,
 * `deps`, `consumers`, …) rather than at a single fixed path, so the explicit
 * fast-paths cannot enumerate every shape. A direct hit on [key] wins over a
 * deeper one, keeping top-level precedence; buckets are then visited in
 * response insertion order, which drains them progressively because each
 * cursor is self-describing (kind-tagged) on the server.
 */
private fun JsonElement?.findNestedString(key: String): String? {
    when {
        this == null || isJsonNull -> return null
        isJsonObject -> {
            val obj = asJsonObject
            obj.get(key)?.let { direct ->
                if (!direct.isJsonNull && direct.isJsonPrimitive &&
                    direct.asJsonPrimitive.isString && direct.asString.isNotBlank()
                ) {
                    return direct.asString
                }
            }
            for ((_, value) in obj.entrySet()) {
                value.findNestedString(key)?.let { return it }
            }
        }
        isJsonArray -> for (element in asJsonArray) {
            element.findNestedString(key)?.let { return it }
        }
    }
    return null
}

/**
 * Depth-first search for the first numeric value stored under [key] anywhere
 * in the response tree. Mirrors [findNestedString] for offset-based literal
 * pagination (`literal_matches.page.next_offset`) so the fallback stays robust
 * if the envelope nesting shifts.
 */
private fun JsonElement?.findNestedInt(key: String): Int? {
    when {
        this == null || isJsonNull -> return null
        isJsonObject -> {
            val obj = asJsonObject
            obj.get(key)?.let { direct ->
                if (!direct.isJsonNull && direct.isJsonPrimitive && direct.asJsonPrimitive.isNumber) {
                    return direct.asInt
                }
            }
            for ((_, value) in obj.entrySet()) {
                value.findNestedInt(key)?.let { return it }
            }
        }
        isJsonArray -> for (element in asJsonArray) {
            element.findNestedInt(key)?.let { return it }
        }
    }
    return null
}
