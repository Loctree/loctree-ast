/*
 * Findings reader tests — grouping + count derivation from agent/findings.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.toolwindow

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import java.nio.file.Files
import java.nio.file.Path

class FindingsReaderTest {

    private val root: Path = Path.of("/workspace/project").toAbsolutePath()

    private fun obj(json: String): JsonObject = JsonParser.parseString(json).asJsonObject

    @Test
    fun emptyInputsYieldZeroCounts() {
        val data = FindingsReader.build(root, null, null)
        assertEquals(0, data.counts.getValue(GroupKind.DEAD))
        assertEquals(0, data.counts.getValue(GroupKind.CYCLES))
        assertEquals(0, data.counts.getValue(GroupKind.TWINS))
        assertTrue(data.groups.getValue(GroupKind.DEAD).isEmpty())
    }

    @Test
    fun parsesGroupsFromFindingsJson() {
        val findings = obj(
            """
            {
              "dead_exports": [{"file":"src/a.ts","line":10,"symbol":"foo","confidence":"high"}],
              "cycles": [["src/a.ts","src/b.ts","src/a.ts"]],
              "twins": [{"symbol":"bar","files":[{"file":"src/c.ts","line":1},{"file":"src/d.ts"}]}]
            }
            """.trimIndent(),
        )
        val data = FindingsReader.build(root, null, findings)

        assertEquals(1, data.groups.getValue(GroupKind.DEAD).size)
        assertEquals(1, data.groups.getValue(GroupKind.CYCLES).size)
        assertEquals(1, data.groups.getValue(GroupKind.TWINS).size)

        val dead = data.groups.getValue(GroupKind.DEAD).first()
        assertTrue(dead.label.contains("foo"))
        assertEquals("high", dead.severity)

        val twin = data.groups.getValue(GroupKind.TWINS).first()
        assertTrue(twin.label.contains("2 locations"))
    }

    @Test
    fun summaryCountsTakePrecedenceOverParsedSize() {
        val agent = obj(
            """
            {"summary": {"dead_exports": 12, "circular_imports": 3,
              "twins_same_language": 1, "twins_cross_language": 1, "twins_dead_parrots": 0}}
            """.trimIndent(),
        )
        val data = FindingsReader.build(root, agent, null)
        assertEquals(12, data.counts.getValue(GroupKind.DEAD))
        assertEquals(3, data.counts.getValue(GroupKind.CYCLES))
        assertEquals(2, data.counts.getValue(GroupKind.TWINS))
    }

    @Test
    fun fromHealthPrefersLiveCounts() {
        val health = io.loct.intellij.protocol.HealthResponse(
            healthScore = 88,
            status = "green",
            cycles = 2,
            deadExports = 5,
            twins = 1,
            hotspots = 0,
            topRisks = listOf(
                io.loct.intellij.protocol.RiskItem(
                    kind = "dead_export",
                    file = "src/a.ts",
                    severity = "high",
                    message = "unused export foo",
                ),
            ),
        )
        val data = FindingsReader.fromHealth(root, health)
        assertEquals(5, data.counts.getValue(GroupKind.DEAD))
        assertEquals(2, data.counts.getValue(GroupKind.CYCLES))
        assertEquals(1, data.counts.getValue(GroupKind.TWINS))
        assertEquals(1, data.groups.getValue(GroupKind.DEAD).size)
        assertTrue(data.groups.getValue(GroupKind.DEAD).first().label.contains("foo"))
    }

    @Test
    fun atlasReaderSurfacesReadyManifest() {
        val temp = Files.createTempDirectory("loctree-atlas-test")
        val atlasDir = temp.resolve(".loctree/context-atlas")
        Files.createDirectories(atlasDir)
        Files.writeString(
            atlasDir.resolve("manifest.json"),
            """
            {"status":"atlas_ready","snapshot":"main@abc123","generated_at":"2026-06-03T12:00:00Z"}
            """.trimIndent(),
        )

        val status = AtlasReader.read(temp)
        assertEquals("Context ready", status.label)
        assertEquals("ready", status.shortValue)
        assertEquals(ChipTone.LIVE, status.tone)
        assertTrue(status.tooltip.orEmpty().contains("main@abc123"))
    }

    @Test
    fun atlasReaderHandlesMissingManifest() {
        val temp = Files.createTempDirectory("loctree-atlas-missing-test")

        val status = AtlasReader.read(temp)

        assertEquals(AtlasStatus.MISSING, status)
    }

    @Test
    fun atlasMissingBecomesScanningWhenLspIsLive() {
        val temp = Files.createTempDirectory("loctree-atlas-scanning-test")
        val status = AtlasReader.read(temp, lspRunning = true)
        assertEquals(AtlasStatus.SCANNING, status)
    }

    @Test
    fun atlasReaderHandlesMissingManifestAttributes() {
        val temp = Files.createTempDirectory("loctree-atlas-partial-test")
        val atlasDir = temp.resolve(".loctree/context-atlas")
        Files.createDirectories(atlasDir)
        Files.writeString(atlasDir.resolve("manifest.json"), "{}")

        val status = AtlasReader.read(temp)

        assertEquals("Context unknown", status.label)
        assertEquals("unknown", status.shortValue)
        assertEquals(null, status.tooltip)
    }

    @Test
    fun atlasReaderHandlesMalformedManifest() {
        val temp = Files.createTempDirectory("loctree-atlas-malformed-test")
        val atlasDir = temp.resolve(".loctree/context-atlas")
        Files.createDirectories(atlasDir)
        Files.writeString(atlasDir.resolve("manifest.json"), "{not-json")

        val status = AtlasReader.read(temp)

        assertEquals("Context unreadable", status.label)
        assertEquals("error", status.shortValue)
        assertEquals(ChipTone.DANGER, status.tone)
        assertTrue(status.tooltip.orEmpty().isNotBlank())
    }
}
