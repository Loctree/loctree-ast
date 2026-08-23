/*
 * Secondary findings model + Context Atlas status reader for the Loctree tool window.
 *
 * Mirrors editors/vscode/src/treeview.ts: reads `.loctree/agent.json`
 * and `.loctree/findings.json`, builds three groups (dead exports,
 * cycles, twins) with counts derived from the agent summary or the
 * parsed items. Pure JVM logic (unit-testable, no IntelliJ deps).
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.toolwindow

import com.google.gson.JsonArray
import com.google.gson.JsonElement
import com.google.gson.JsonObject
import com.google.gson.JsonParser
import java.nio.file.Files
import java.nio.file.Path

enum class GroupKind { DEAD, CYCLES, TWINS }

data class FindingItem(
    val label: String,
    val description: String? = null,
    val tooltip: String? = null,
    val file: String? = null,
    val line: Int? = null,
    val severity: String = "normal",
)

data class FindingsData(
    val counts: Map<GroupKind, Int>,
    val groups: Map<GroupKind, List<FindingItem>>,
) {
    companion object {
        val EMPTY = FindingsData(
            counts = GroupKind.entries.associateWith { 0 },
            groups = GroupKind.entries.associateWith { emptyList() },
        )
    }
}

data class AtlasStatus(
    val label: String,
    val shortValue: String,
    val tone: ChipTone,
    val tooltip: String? = null,
) {
    companion object {
        val MISSING = AtlasStatus(
            label = "Context not built yet",
            shortValue = "not built",
            tone = ChipTone.MUTED,
            tooltip = "No context atlas on disk. Run Context Pack or wait for LSP scan.",
        )
        val LOADING = AtlasStatus(
            label = "Context loading",
            shortValue = "loading",
            tone = ChipTone.WARN,
        )
        val SCANNING = AtlasStatus(
            label = "Context scanning",
            shortValue = "scanning",
            tone = ChipTone.WARN,
            tooltip = "LSP is starting or still building the snapshot.",
        )
    }
}

object AtlasReader {
    fun read(workspaceRoot: Path, lspRunning: Boolean = false): AtlasStatus {
        val manifest = workspaceRoot.resolve(".loctree/context-atlas/manifest.json")
        if (!Files.isRegularFile(manifest)) {
            return if (lspRunning) AtlasStatus.SCANNING else AtlasStatus.MISSING
        }

        return runCatching {
            val obj = JsonParser.parseString(Files.readString(manifest)).asJsonObject
            val status = obj.stringOrNull("status") ?: "unknown"
            val snapshot = obj.stringOrNull("snapshot")
            val generated = obj.stringOrNull("generated_at")
            val tooltip = listOfNotNull(snapshot, generated).joinToString("\n").takeIf { it.isNotBlank() }
            when (status) {
                "atlas_ready" -> AtlasStatus("Context ready", "ready", ChipTone.LIVE, tooltip)
                "atlas_stale" -> AtlasStatus("Context stale", "stale", ChipTone.WARN, tooltip)
                else -> AtlasStatus(
                    label = "Context ${status.replace('_', ' ')}",
                    shortValue = status.replace('_', ' '),
                    tone = ChipTone.MUTED,
                    tooltip = tooltip,
                )
            }
        }.getOrElse {
            AtlasStatus("Context unreadable", "error", ChipTone.DANGER, it.message)
        }
    }
}

object FindingsReader {

    private const val MAX_ITEMS_PER_GROUP = 100

    fun read(workspaceRoot: Path): FindingsData {
        val loctreeDir = workspaceRoot.resolve(".loctree")
        val agent = readJsonObject(loctreeDir.resolve("agent.json"))
        val findings = readJsonObject(loctreeDir.resolve("findings.json"))
        return build(workspaceRoot, agent, findings)
    }

    /**
     * Prefer live `loctree/health` (same source as VS Code tree) over disk
     * artifacts that often lag or live only in the project cache.
     */
    fun fromHealth(
        workspaceRoot: Path,
        health: io.loct.intellij.protocol.HealthResponse,
        diskFallback: FindingsData? = null,
    ): FindingsData {
        val fromRisks = mapOf(
            GroupKind.DEAD to health.topRisks.filter { it.kind == "dead_export" }.map { risk ->
                FindingItem(
                    label = risk.message.ifBlank { risk.file.ifBlank { "dead export" } },
                    description = risk.file.takeIf { it.isNotBlank() }?.let { toDisplayPath(it, workspaceRoot) },
                    tooltip = listOf(risk.severity.uppercase(), risk.message, risk.file)
                        .filter { it.isNotBlank() }
                        .joinToString("\n"),
                    file = risk.file.takeIf { it.isNotBlank() },
                    line = null,
                    severity = normalizeSeverity(risk.severity),
                )
            }.take(MAX_ITEMS_PER_GROUP),
            GroupKind.CYCLES to health.topRisks.filter { it.kind == "cycle" }.map { risk ->
                FindingItem(
                    label = risk.message.ifBlank { "cycle" },
                    description = risk.file.takeIf { it.isNotBlank() }?.let { toDisplayPath(it, workspaceRoot) },
                    tooltip = risk.message,
                    file = risk.file.takeIf { it.isNotBlank() },
                    severity = "warning",
                )
            }.take(MAX_ITEMS_PER_GROUP),
            GroupKind.TWINS to health.topRisks.filter { it.kind == "twin" }.map { risk ->
                FindingItem(
                    label = risk.message.ifBlank { "twin" },
                    description = risk.file.takeIf { it.isNotBlank() }?.let { toDisplayPath(it, workspaceRoot) },
                    tooltip = risk.message,
                    file = risk.file.takeIf { it.isNotBlank() },
                    severity = normalizeSeverity(risk.severity),
                )
            }.take(MAX_ITEMS_PER_GROUP),
        )

        val counts = mapOf(
            GroupKind.DEAD to health.deadExports,
            GroupKind.CYCLES to health.cycles,
            GroupKind.TWINS to health.twins,
        )

        // When health only has counts, keep disk drill-down items if present.
        val groups = GroupKind.entries.associateWith { kind ->
            val risks = fromRisks.getValue(kind)
            if (risks.isNotEmpty()) risks else diskFallback?.groups?.getValue(kind).orEmpty()
        }

        return FindingsData(counts = counts, groups = groups)
    }

    fun build(workspaceRoot: Path, agent: JsonObject?, findings: JsonObject?): FindingsData {
        val summary = agent?.getAsJsonObjectOrNull("summary")
        val bundle = agent?.getAsJsonObjectOrNull("bundle")

        val deadSource = firstArray(
            findings?.arrayOrNull("dead_exports"),
            findings?.arrayOrNull("dead_parrots"),
            bundle?.arrayOrNull("dead_exports"),
        )
        val cyclesSource = firstArray(
            findings?.arrayOrNull("cycles"),
            bundle?.arrayOrNull("cycles"),
        )
        val twinsSource = firstArray(
            findings?.arrayOrNull("twins"),
            findings?.arrayOrNull("duplicates"),
            bundle?.arrayOrNull("duplicates"),
        )

        val groups = mapOf(
            GroupKind.DEAD to parseDead(deadSource, workspaceRoot).take(MAX_ITEMS_PER_GROUP),
            GroupKind.CYCLES to parseCycles(cyclesSource, workspaceRoot).take(MAX_ITEMS_PER_GROUP),
            GroupKind.TWINS to parseTwins(twinsSource, workspaceRoot).take(MAX_ITEMS_PER_GROUP),
        )

        val summaryCounts = mapOf(
            GroupKind.DEAD to count(summary, "dead_exports"),
            GroupKind.CYCLES to count(summary, "circular_imports"),
            GroupKind.TWINS to count(summary, "twins_same_language") +
                count(summary, "twins_cross_language") +
                count(summary, "twins_dead_parrots"),
        )

        val counts = GroupKind.entries.associateWith { kind ->
            val fromSummary = summaryCounts.getValue(kind)
            if (fromSummary > 0) fromSummary else groups.getValue(kind).size
        }

        return FindingsData(counts = counts, groups = groups)
    }

    // Parsing ---------------------------------------------------------------

    private fun parseDead(values: JsonArray, root: Path): List<FindingItem> =
        values.objects().map { obj ->
            val file = obj.stringOrNull("file") ?: obj.stringOrNull("path")
            val line = obj.intOrNull("line")
            val symbol = obj.stringOrNull("symbol") ?: obj.stringOrNull("name")
                ?: obj.stringOrNull("export")
            val reason = obj.stringOrNull("reason") ?: obj.stringOrNull("message")
            val severity = normalizeSeverity(
                obj.stringOrNull("confidence") ?: obj.stringOrNull("severity"),
            )
            val label = buildString {
                append(file?.let { toDisplayPath(it, root) } ?: "(unknown file)")
                if (line != null) append(":$line")
                if (symbol != null) append(": $symbol")
                append(" ($severity)")
            }
            FindingItem(label, reason, reason ?: label, file, line, severity)
        }

    private fun parseCycles(values: JsonArray, root: Path): List<FindingItem> =
        values.mapNotNull { element ->
            val chain = parseCycleChain(element)
            if (chain.isEmpty()) return@mapNotNull null
            val obj = element as? JsonObject
            val display = chain.map { toDisplayPath(it, root) }
            val label = display.joinToString(" ↔ ")
            val description = obj?.let { it.stringOrNull("description") ?: it.stringOrNull("message") }
            FindingItem(
                label = label,
                description = description,
                tooltip = description ?: display.joinToString(" -> "),
                file = chain.first(),
                line = obj?.intOrNull("line"),
                severity = "warning",
            )
        }

    private fun parseTwins(values: JsonArray, root: Path): List<FindingItem> =
        values.objects().map { obj ->
            val symbol = obj.stringOrNull("symbol") ?: obj.stringOrNull("name")
                ?: "(unknown symbol)"
            val locations = extractLocations(obj.arrayOrNull("files") ?: obj.arrayOrNull("locations"))
            val count = if (locations.isNotEmpty()) {
                locations.size
            } else {
                obj.intOrNull("count") ?: obj.intOrNull("locations") ?: 0
            }
            val first = locations.firstOrNull()
            val file = first?.first ?: obj.stringOrNull("file") ?: obj.stringOrNull("canonical")
            val line = first?.second ?: obj.intOrNull("line") ?: obj.intOrNull("canonical_line")
            val label = "$symbol: $count locations"
            val description = file?.let { toDisplayPath(it, root) }
            FindingItem(
                label = label,
                description = description,
                tooltip = if (description != null) "$label\n$description" else label,
                file = file,
                line = line,
                severity = normalizeSeverity(obj.stringOrNull("severity")),
            )
        }

    private fun parseCycleChain(raw: JsonElement?): List<String> {
        if (raw == null) return emptyList()
        if (raw.isJsonPrimitive && raw.asJsonPrimitive.isString) {
            return raw.asString.split("->").map { it.trim() }.filter { it.isNotEmpty() }
        }
        if (raw.isJsonArray) {
            return raw.asJsonArray.mapNotNull {
                if (it.isJsonPrimitive && it.asJsonPrimitive.isString) it.asString.trim() else null
            }.filter { it.isNotEmpty() }
        }
        val obj = raw as? JsonObject ?: return emptyList()
        val nested = obj.get("cycle") ?: obj.get("chain") ?: obj.get("files")
        if (nested != null) {
            val parsed = parseCycleChain(nested)
            if (parsed.isNotEmpty()) return parsed
        }
        return obj.stringOrNull("file")?.let { listOf(it) } ?: emptyList()
    }

    private fun extractLocations(arr: JsonArray?): List<Pair<String?, Int?>> {
        if (arr == null) return emptyList()
        return arr.mapNotNull { element ->
            when {
                element.isJsonPrimitive && element.asJsonPrimitive.isString ->
                    element.asString.trim().takeIf { it.isNotEmpty() }?.let { it to null }
                element.isJsonObject -> {
                    val obj = element.asJsonObject
                    (obj.stringOrNull("file") ?: obj.stringOrNull("path")) to obj.intOrNull("line")
                }
                else -> null
            }
        }
    }

    private fun normalizeSeverity(raw: String?): String {
        if (raw == null) return "normal"
        val value = raw.lowercase()
        return when {
            value.contains("very-high") || value == "high" || value.contains("critical") -> "high"
            value.contains("low") -> "low"
            value.contains("warn") -> "warning"
            else -> "normal"
        }
    }

    private fun toDisplayPath(filePath: String, workspaceRoot: Path): String {
        var candidate = filePath
        if (candidate.startsWith("file://")) {
            candidate = runCatching { Path.of(java.net.URI(candidate)).toString() }
                .getOrDefault(candidate)
        }
        val path = runCatching { Path.of(candidate) }.getOrNull() ?: return candidate
        if (!path.isAbsolute) return candidate
        val relative = runCatching { workspaceRoot.toAbsolutePath().relativize(path) }.getOrNull()
        return if (relative != null && !relative.startsWith("..")) relative.toString() else candidate
    }

    // JSON helpers ----------------------------------------------------------

    private fun readJsonObject(path: Path): JsonObject? {
        if (!Files.isRegularFile(path)) return null
        return runCatching {
            JsonParser.parseString(Files.readString(path)).asJsonObject
        }.getOrNull()
    }

    private fun firstArray(vararg arrays: JsonArray?): JsonArray =
        arrays.firstOrNull { it != null && it.size() > 0 } ?: arrays.firstOrNull { it != null }
        ?: JsonArray()

    private fun count(obj: JsonObject?, key: String): Int = obj?.intOrNull(key) ?: 0
}

// JsonObject extensions -----------------------------------------------------

private fun JsonObject.stringOrNull(key: String): String? {
    val element = get(key) ?: return null
    if (!element.isJsonPrimitive || !element.asJsonPrimitive.isString) return null
    return element.asString.trim().takeIf { it.isNotEmpty() }
}

private fun JsonObject.intOrNull(key: String): Int? {
    val element = get(key) ?: return null
    if (!element.isJsonPrimitive || !element.asJsonPrimitive.isNumber) return null
    return element.asInt
}

private fun JsonObject.arrayOrNull(key: String): JsonArray? {
    val element = get(key) ?: return null
    return if (element.isJsonArray) element.asJsonArray else null
}

private fun JsonObject.getAsJsonObjectOrNull(key: String): JsonObject? {
    val element = get(key) ?: return null
    return if (element.isJsonObject) element.asJsonObject else null
}

private fun JsonArray.objects(): List<JsonObject> = mapNotNull { if (it.isJsonObject) it.asJsonObject else null }
