/*
 * Cycle suppression writer tests — append-only, idempotent, .loctree-bound.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.actions

import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.nio.file.Files

class CycleSuppressionWriterTest {

    @get:Rule
    val tempFolder = TemporaryFolder()

    @Test
    fun writesUnderLoctreeDirectory() {
        val root = tempFolder.root.toPath()
        val path = CycleSuppressionWriter.appendCycleSuppression(root, listOf("a.ts", "b.ts"))
        assertEquals(root.resolve(".loctree").resolve("suppressions.toml"), path)

        val content = Files.readString(path)
        assertTrue(content.contains("type = \"circular\""))
        assertTrue(content.contains("symbol = \"a.ts -> b.ts\""))
        assertTrue(content.contains("file = \"a.ts\""))
    }

    @Test
    fun secondIdenticalSuppressionIsIdempotent() {
        val root = tempFolder.root.toPath()
        val chain = listOf("a.ts", "b.ts")
        CycleSuppressionWriter.appendCycleSuppression(root, chain)
        val path = CycleSuppressionWriter.appendCycleSuppression(root, chain)

        val occurrences = Files.readString(path).split("[[suppress]]").size - 1
        assertEquals(1, occurrences)
    }

    @Test
    fun emptyChainRejected() {
        val root = tempFolder.root.toPath()
        assertThrows(IllegalArgumentException::class.java) {
            CycleSuppressionWriter.appendCycleSuppression(root, emptyList())
        }
    }

    @Test
    fun tomlStringEscapesQuotesAndStripsControlChars() {
        assertEquals("\"a\\\"b\"", CycleSuppressionWriter.tomlString("a\"b"))
        assertEquals("\"ab\"", CycleSuppressionWriter.tomlString("a\u0000b"))
    }
}
