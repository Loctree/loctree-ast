/*
 * Verified cache sidecar tests.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.binary

import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.nio.file.Files

class CachedBinaryVerifierTest {

    @Rule
    @JvmField
    val tempDir = TemporaryFolder()

    @Test
    fun verifiedCacheRequiresMatchingSidecar() {
        val binary = tempDir.newFile("loctree-lsp.exe").toPath()
        val payload = "loctree".toByteArray()
        Files.write(binary, payload)

        assertFalse(CachedBinaryVerifier.isVerified(binary, OsFamily.WINDOWS))

        CachedBinaryVerifier.writeSidecar(binary, ChecksumVerifier.sha256Hex(payload))
        assertTrue(CachedBinaryVerifier.isVerified(binary, OsFamily.WINDOWS))
    }

    @Test
    fun modifiedCacheBinaryFailsClosed() {
        val binary = tempDir.newFile("loctree-lsp.exe").toPath()
        Files.write(binary, "original".toByteArray())
        CachedBinaryVerifier.writeSidecar(
            binary,
            ChecksumVerifier.sha256Hex("original".toByteArray()),
        )

        Files.write(binary, "tampered".toByteArray())

        assertFalse(CachedBinaryVerifier.isVerified(binary, OsFamily.WINDOWS))
    }
}
