/*
 * Project loctree LSP JSON envelopes into human tree rows.
 *
 * The tool window must never dump raw `{N fields}` JSON trees at operators.
 * Extract navigable findings + short summaries; ignore noise keys.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
 */

package io.loct.intellij.toolwindow

import com.google.gson.JsonArray
import com.google.gson.JsonElement
import com.google.gson.JsonObject
import javax.swing.tree.DefaultMutableTreeNode

internal data class ProjectedSection(
    val title: String,
    val items: List<FindingItem>,
    val note: String? = null,
)

internal data class ProjectedResult(
    val headline: String,
    val summaries: List<String> = emptyList(),
    val sections: List<ProjectedSection> = emptyList(),
)

internal object ResultProjector {

    private val NOISE_KEYS = setOf(
        "protocol",
        "schema",
        "authority",
        "bundle_id",
        "snapshot_fp",
        "diagnostic",
        "suggested_next",
        "next_cursor",
        "cursor",
        "chunk",
        "total_chunks",
        "offset",
        "limit",
        "page",
        "pagination",
    )

    fun project(method: String, result: JsonElement): ProjectedResult {
        val obj = result.asObjectOrNull()
            ?: return ProjectedResult(
                headline = shortMethod(method),
                summaries = listOf(result.toDisplayString().take(240)),
            )

        return when {
            method.endsWith("/health") || looksLikeHealth(obj) -> projectHealth(method, obj)
            method.endsWith("/body") || obj.has("bodies") || obj.has("source") -> projectBody(method, obj)
            method.endsWith("/impact") || obj.has("consumers") || obj.has("transitive") ->
                projectImpact(method, obj)
            method.endsWith("/slice") || (obj.has("deps") && obj.has("consumers")) ->
                projectSlice(method, obj)
            method.endsWith("/find") || obj.has("literal_matches") || obj.has("symbol_matches") ->
                projectFind(method, obj)
            method.endsWith("/follow") || obj.has("items") && obj.get("items")?.isJsonArray == true ->
                projectItemsEnvelope(method, obj, "items")
            method.endsWith("/contextPack") || method.endsWith("/contextAtlas") ->
                projectContext(method, obj)
            else -> projectGeneric(method, obj)
        }
    }

    // ── shapes ────────────────────────────────────────────────────────────

    private fun projectHealth(method: String, obj: JsonObject): ProjectedResult {
        val score = obj.intOrNull("health_score")
        val status = obj.stringOrNull("status") ?: "unknown"
        val headline = if (score != null) {
            "Health · $score/100 · $status"
        } else {
            "Health · $status"
        }
        val summaries = buildList {
            add("dead ${obj.intOrNull("dead_exports") ?: 0} · cycles ${obj.intOrNull("cycles") ?: 0} · twins ${obj.intOrNull("twins") ?: 0} · hotspots ${obj.intOrNull("hotspots") ?: 0}")
            if (obj.boolOrNull("snapshot_stale") == true) add("snapshot stale")
            obj.arrayOrNull("recommended_actions")?.strings()?.take(4)?.forEach { add("→ $it") }
        }
        val risks = obj.arrayOrNull("top_risks")?.objects().orEmpty().map { risk ->
            FindingItem(
                label = risk.stringOrNull("message") ?: risk.stringOrNull("kind") ?: "risk",
                description = risk.stringOrNull("file"),
                tooltip = listOfNotNull(
                    risk.stringOrNull("severity")?.uppercase(),
                    risk.stringOrNull("kind"),
                    risk.stringOrNull("message"),
                    risk.stringOrNull("file"),
                ).joinToString("\n"),
                file = risk.stringOrNull("file"),
                line = risk.intOrNull("line"),
                severity = normalizeSeverity(risk.stringOrNull("severity")),
            )
        }
        return ProjectedResult(
            headline = headline,
            summaries = summaries,
            sections = listOf(
                ProjectedSection(
                    title = "Top risks",
                    items = risks,
                    note = if (risks.isEmpty()) "No top risks in this health page" else null,
                ),
            ),
        )
    }

    private fun projectBody(method: String, obj: JsonObject): ProjectedResult {
        val bodies = obj.arrayOrNull("bodies")?.objects().orEmpty()
        if (bodies.isEmpty()) {
            val source = obj.stringOrNull("source")
            val file = obj.stringOrNull("file") ?: obj.stringOrNull("path")
            val item = if (source != null || file != null) {
                listOf(
                    FindingItem(
                        label = obj.stringOrNull("symbol") ?: file ?: "body",
                        description = source?.lines()?.take(3)?.joinToString(" · ")?.take(120),
                        tooltip = source?.take(2000),
                        file = file,
                        line = obj.intOrNull("line") ?: obj.intOrNull("start_line"),
                    ),
                )
            } else {
                emptyList()
            }
            return ProjectedResult(
                headline = shortMethod(method),
                sections = listOf(ProjectedSection("Body", item, if (item.isEmpty()) "No body returned" else null)),
            )
        }
        val items = bodies.map { body ->
            val file = body.stringOrNull("file") ?: body.stringOrNull("path")
            val source = body.stringOrNull("source")
            FindingItem(
                label = body.stringOrNull("symbol") ?: file ?: "body",
                description = buildString {
                    body.intOrNull("start_line")?.let { append("L$it") }
                    body.intOrNull("end_line")?.let { append("–$it") }
                    if (body.boolOrNull("truncated") == true) append(" truncated")
                }.ifBlank { null },
                tooltip = source?.take(2000) ?: file,
                file = file,
                line = body.intOrNull("start_line") ?: body.intOrNull("line"),
            )
        }
        return ProjectedResult(
            headline = "Body · ${items.size}",
            sections = listOf(ProjectedSection("Bodies", items)),
        )
    }

    private fun projectImpact(method: String, obj: JsonObject): ProjectedResult {
        val file = obj.stringOrNull("file") ?: obj.stringOrNull("path")
        val sections = listOfNotNull(
            locationSection("Direct consumers", obj.arrayOrNull("consumers") ?: obj.arrayOrNull("direct_consumers")),
            locationSection("Transitive", obj.arrayOrNull("transitive") ?: obj.arrayOrNull("transitive_consumers")),
            locationSection("Dependencies", obj.arrayOrNull("deps") ?: obj.arrayOrNull("dependencies")),
        )
        val total = sections.sumOf { it.items.size }
        return ProjectedResult(
            headline = if (file != null) "Impact · $file" else shortMethod(method),
            summaries = listOf("$total related paths"),
            sections = sections.ifEmpty {
                listOf(ProjectedSection("Impact", emptyList(), "No consumers or deps in response"))
            },
        )
    }

    private fun projectSlice(method: String, obj: JsonObject): ProjectedResult {
        val file = obj.stringOrNull("file") ?: obj.stringOrNull("path")
        val sections = listOfNotNull(
            locationSection("Dependencies", obj.arrayOrNull("deps") ?: obj.arrayOrNull("dependencies")),
            locationSection("Consumers", obj.arrayOrNull("consumers")),
            locationSection("Exports", obj.arrayOrNull("exports") ?: obj.arrayOrNull("symbols")),
        )
        return ProjectedResult(
            headline = if (file != null) "Slice · $file" else shortMethod(method),
            sections = sections.ifEmpty {
                listOf(ProjectedSection("Slice", emptyList(), "Empty slice"))
            },
        )
    }

    private fun projectFind(method: String, obj: JsonObject): ProjectedResult {
        val sections = mutableListOf<ProjectedSection>()
        for (bucket in listOf(
            "literal_matches" to "Literal",
            "symbol_matches" to "Symbols",
            "param_matches" to "Parameters",
            "matches" to "Matches",
            "results" to "Results",
            "occurrences" to "Occurrences",
        )) {
            val node = obj.get(bucket.first) ?: continue
            val items = extractLocations(node)
            if (items.isNotEmpty() || node.isJsonObject || node.isJsonArray) {
                sections += ProjectedSection(bucket.second, items, if (items.isEmpty()) "No hits" else null)
            }
        }
        // Nested data envelopes: { data: { files: [...] } }
        if (sections.isEmpty()) {
            val data = obj.objOrNull("data")
            if (data != null) {
                val items = extractLocations(data)
                if (items.isNotEmpty()) {
                    sections += ProjectedSection("Matches", items)
                }
            }
        }
        val total = sections.sumOf { it.items.size }
        return ProjectedResult(
            headline = if (total > 0) "Find · $total hits" else shortMethod(method),
            sections = sections.ifEmpty {
                listOf(ProjectedSection("Find", emptyList(), "No matches"))
            },
        )
    }

    private fun projectItemsEnvelope(method: String, obj: JsonObject, key: String): ProjectedResult {
        val items = extractLocations(obj.get(key) ?: JsonArray())
        return ProjectedResult(
            headline = "${shortMethod(method)} · ${items.size}",
            sections = listOf(
                ProjectedSection("Results", items, if (items.isEmpty()) "Empty page" else null),
            ),
        )
    }

    private fun projectContext(method: String, obj: JsonObject): ProjectedResult {
        val summaries = buildList {
            obj.stringOrNull("status")?.let { add("status · $it") }
            obj.stringOrNull("snapshot")?.let { add("snapshot · $it") }
            obj.stringOrNull("project")?.let { add("project · $it") }
            obj.objOrNull("identity")?.let { id ->
                id.stringOrNull("branch")?.let { add("branch · $it") }
                id.stringOrNull("commit")?.let { add("commit · $it") }
            }
            obj.objOrNull("risk")?.let { risk ->
                risk.stringOrNull("snapshot_health")?.let { add("snapshot health · $it") }
            }
        }
        val sections = mutableListOf<ProjectedSection>()
        // Common context pack tables
        for ((key, title) in listOf(
            "files" to "Files",
            "hubs" to "Hubs",
            "hotspots" to "Hotspots",
            "entrypoints" to "Entrypoints",
            "cards" to "Cards",
            "safe_next_commands" to "Safe next",
        )) {
            val node = obj.get(key) ?: obj.objOrNull("structural")?.get(key) ?: continue
            val items = extractLocations(node)
            if (items.isNotEmpty()) {
                sections += ProjectedSection(title, items.take(40))
            } else if (node.isJsonArray) {
                val lines = node.asJsonArray.strings().take(12)
                if (lines.isNotEmpty()) {
                    sections += ProjectedSection(
                        title,
                        lines.map { FindingItem(label = it.take(160), severity = "normal") },
                    )
                }
            }
        }
        return ProjectedResult(
            headline = shortMethod(method),
            summaries = summaries.ifEmpty { listOf("Context pack received") },
            sections = sections,
        )
    }

    private fun projectGeneric(method: String, obj: JsonObject): ProjectedResult {
        val locationItems = extractLocations(obj)
        if (locationItems.isNotEmpty()) {
            return ProjectedResult(
                headline = shortMethod(method),
                sections = listOf(ProjectedSection("Results", locationItems)),
            )
        }
        val summaries = obj.entrySet()
            .asSequence()
            .filter { it.key !in NOISE_KEYS }
            .mapNotNull { (key, value) ->
                when {
                    value.isJsonPrimitive -> "$key · ${value.toDisplayString().take(100)}"
                    value.isJsonArray -> "$key · ${value.asJsonArray.size()} items"
                    value.isJsonObject -> {
                        val nested = extractLocations(value)
                        if (nested.isNotEmpty()) null
                        else "$key · ${value.asJsonObject.size()} fields"
                    }
                    else -> null
                }
            }
            .take(16)
            .toList()

        val nestedSections = obj.entrySet()
            .asSequence()
            .filter { it.key !in NOISE_KEYS }
            .mapNotNull { (key, value) ->
                val items = extractLocations(value)
                if (items.isEmpty()) null else ProjectedSection(key, items.take(40))
            }
            .take(8)
            .toList()

        return ProjectedResult(
            headline = shortMethod(method),
            summaries = summaries.ifEmpty { listOf("Empty or unrecognized response") },
            sections = nestedSections,
        )
    }

    // ── extractors ────────────────────────────────────────────────────────

    private fun locationSection(title: String, arr: JsonArray?): ProjectedSection? {
        if (arr == null) return null
        val items = extractLocations(arr)
        return ProjectedSection(
            title = title,
            items = items,
            note = if (items.isEmpty()) "none" else null,
        )
    }

    private fun extractLocations(node: JsonElement?): List<FindingItem> {
        if (node == null || node.isJsonNull) return emptyList()
        if (node.isJsonArray) {
            return node.asJsonArray.flatMap { extractLocations(it) }
        }
        if (node.isJsonPrimitive && node.asJsonPrimitive.isString) {
            val path = node.asString.trim()
            if (path.isEmpty()) return emptyList()
            // Multi-language path truth: separators, URI, OR known extension.
            // Never hardcode only .kt/.rs — TS/Python/Swift/Go must navigate too.
            if (PathHeuristic.looksLikeFilePath(path)) {
                return listOf(FindingItem(label = path, file = PathHeuristic.normalizeFileRef(path)))
            }
            return listOf(FindingItem(label = path))
        }
        if (!node.isJsonObject) return emptyList()
        val obj = node.asJsonObject

        // Prefer nested collections
        for (key in listOf("data", "files", "items", "results", "matches", "occurrences", "locations")) {
            val nested = obj.get(key)
            if (nested != null && (nested.isJsonArray || nested.isJsonObject)) {
                val fromNested = extractLocations(nested)
                if (fromNested.isNotEmpty()) return fromNested
            }
        }

        val rawFile = obj.stringOrNull("file") ?: obj.stringOrNull("path") ?: obj.stringOrNull("uri")
        val file = rawFile?.let { PathHeuristic.normalizeFileRef(it) }
        val line = obj.intOrNull("line") ?: obj.intOrNull("start_line")
        val symbol = obj.stringOrNull("symbol")
            ?: obj.stringOrNull("name")
            ?: obj.stringOrNull("export")
            ?: obj.stringOrNull("kind")
        val message = obj.stringOrNull("message")
            ?: obj.stringOrNull("description")
            ?: obj.stringOrNull("context")
            ?: obj.stringOrNull("reason")

        if (file == null && symbol == null && message == null) {
            // object without location — try array-ish children only
            return obj.entrySet().flatMap { (_, v) ->
                if (v.isJsonArray) extractLocations(v) else emptyList()
            }
        }

        val label = buildString {
            when {
                symbol != null && file != null -> {
                    append(symbol)
                    append(" · ")
                    append(file)
                    if (line != null) append(":$line")
                }
                symbol != null -> append(symbol)
                file != null -> {
                    append(file)
                    if (line != null) append(":$line")
                }
                message != null -> append(message)
                else -> append("item")
            }
        }
        return listOf(
            FindingItem(
                label = label,
                description = message?.takeIf { it != label },
                tooltip = listOfNotNull(message, file, line?.let { "line $it" }).joinToString("\n").ifBlank { label },
                file = file,
                line = line,
                severity = normalizeSeverity(obj.stringOrNull("severity") ?: obj.stringOrNull("confidence")),
            ),
        )
    }

    private fun looksLikeHealth(obj: JsonObject): Boolean =
        obj.has("health_score") || (obj.has("dead_exports") && obj.has("cycles") && obj.has("status"))

    private fun shortMethod(method: String): String =
        method.removePrefix("loctree/").ifBlank { method }

    private fun normalizeSeverity(raw: String?): String {
        if (raw == null) return "normal"
        val value = raw.lowercase()
        return when {
            value.contains("very-high") || value == "high" || value.contains("critical") -> "high"
            value.contains("low") -> "low"
            value.contains("warn") || value.contains("medium") -> "warning"
            else -> "normal"
        }
    }
}

// ── local JSON helpers (package-private duplicates avoided via top-level) ─

private fun JsonElement.asObjectOrNull(): JsonObject? =
    takeIf { it.isJsonObject }?.asJsonObject

private fun JsonObject.objOrNull(key: String): JsonObject? =
    get(key)?.takeIf { it.isJsonObject }?.asJsonObject

private fun JsonObject.arrayOrNull(key: String): JsonArray? =
    get(key)?.takeIf { it.isJsonArray }?.asJsonArray

private fun JsonObject.stringOrNull(key: String): String? =
    get(key)?.takeIf { it.isJsonPrimitive && it.asJsonPrimitive.isString }?.asString?.trim()?.takeIf { it.isNotEmpty() }

private fun JsonObject.intOrNull(key: String): Int? =
    get(key)?.takeIf { it.isJsonPrimitive && it.asJsonPrimitive.isNumber }?.asInt

private fun JsonObject.boolOrNull(key: String): Boolean? =
    get(key)?.takeIf { it.isJsonPrimitive && it.asJsonPrimitive.isBoolean }?.asBoolean

private fun JsonArray.objects(): List<JsonObject> =
    mapNotNull { if (it.isJsonObject) it.asJsonObject else null }

private fun JsonArray.strings(): List<String> =
    mapNotNull {
        if (it.isJsonPrimitive && it.asJsonPrimitive.isString) it.asString.trim().takeIf { s -> s.isNotEmpty() }
        else null
    }

private fun JsonElement.toDisplayString(): String =
    when {
        isJsonNull -> "null"
        isJsonPrimitive && asJsonPrimitive.isString -> asString
        else -> toString()
    }

/**
 * Render a projected result into the tool-window tree.
 * Never writes JsonPayload rows (raw key/value dumps).
 */
internal fun renderProjectedInto(
    root: DefaultMutableTreeNode,
    projected: ProjectedResult,
    append: Boolean,
) {
    if (!append) {
        root.removeAllChildren()
        root.add(DefaultMutableTreeNode(StatusPayload(projected.headline, ChipTone.LIVE)))
        projected.summaries.forEach { line ->
            root.add(DefaultMutableTreeNode(StatusPayload(line, ChipTone.MUTED)))
        }
    } else {
        root.add(DefaultMutableTreeNode(StatusPayload("More · ${projected.headline}", ChipTone.MUTED)))
    }

    for (section in projected.sections) {
        val group = DefaultMutableTreeNode(
            StatusPayload(
                if (section.items.isEmpty()) {
                    "${section.title} · ${section.note ?: "empty"}"
                } else {
                    "${section.title} · ${section.items.size}"
                },
                if (section.items.isEmpty()) ChipTone.MUTED else ChipTone.NEUTRAL,
            ),
        )
        if (section.items.isEmpty()) {
            section.note?.let {
                group.add(DefaultMutableTreeNode(StatusPayload(it, ChipTone.LIVE)))
            }
        } else {
            section.items.forEach { group.add(DefaultMutableTreeNode(ItemPayload(it))) }
        }
        root.add(group)
    }
}
