/*
 * SHA256 verification tests — fail-closed on missing/mismatched.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.binary

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChecksumVerifierTest {

    // sha256("loctree") computed independently.
    private val payload = "loctree".toByteArray()
    private val knownHash = ChecksumVerifier.sha256Hex(payload)

    @Test
    fun sha256IsStableLowercaseHex() {
        assertEquals(64, knownHash.length)
        assertEquals(knownHash, ChecksumVerifier.sha256Hex(payload))
        assertEquals(knownHash, knownHash.lowercase())
    }

    @Test
    fun verifyOkOnMatch() {
        assertTrue(ChecksumVerifier.verify(knownHash, knownHash) is ChecksumResult.Ok)
    }

    @Test
    fun verifyMatchToleratesSha256sumFormat() {
        val body = "$knownHash  loctree-lsp-darwin-arm64\n"
        assertTrue(ChecksumVerifier.verify(knownHash, body) is ChecksumResult.Ok)
    }

    @Test
    fun verifyMismatchReported() {
        val wrong = "0".repeat(64)
        val result = ChecksumVerifier.verify(knownHash, wrong)
        assertTrue(result is ChecksumResult.Mismatch)
        result as ChecksumResult.Mismatch
        assertEquals(wrong, result.expected)
        assertEquals(knownHash, result.actual)
    }

    @Test
    fun verifyFailsClosedOnMissingChecksum() {
        assertTrue(ChecksumVerifier.verify(knownHash, null) is ChecksumResult.MissingExpected)
        assertTrue(ChecksumVerifier.verify(knownHash, "") is ChecksumResult.MissingExpected)
        assertTrue(ChecksumVerifier.verify(knownHash, "no digest here") is ChecksumResult.MissingExpected)
    }

    @Test
    fun parseExpectedExtractsDigest() {
        assertEquals(knownHash, ChecksumVerifier.parseExpectedChecksum(knownHash.uppercase()))
        assertNull(ChecksumVerifier.parseExpectedChecksum("deadbeef"))
    }
}
