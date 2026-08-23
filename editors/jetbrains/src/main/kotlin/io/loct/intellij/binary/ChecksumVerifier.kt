/*
 * SHA256 verification for downloaded loctree-lsp binaries.
 *
 * The VS Code resolver (client.ts) performs NO checksum verification.
 * The JetBrains resolver closes that gap: a downloaded binary is only
 * marked executable after its SHA256 matches the published checksum.
 * Verification fails CLOSED — a missing or mismatched checksum is an
 * error, never a silent pass.
 *
 * Pure JVM logic, no IntelliJ Platform dependencies (unit-testable).
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.binary

import java.io.InputStream
import java.security.MessageDigest

/** Outcome of a checksum verification attempt. */
sealed interface ChecksumResult {
    object Ok : ChecksumResult
    data class Mismatch(val expected: String, val actual: String) : ChecksumResult
    data class MissingExpected(val reason: String) : ChecksumResult
}

object ChecksumVerifier {

    /** Lowercase hex SHA256 of a byte array. */
    fun sha256Hex(bytes: ByteArray): String {
        val digest = MessageDigest.getInstance("SHA-256").digest(bytes)
        return digest.joinToString("") { "%02x".format(it) }
    }

    /** Lowercase hex SHA256 of a stream (streamed, no full buffering). */
    fun sha256Hex(stream: InputStream): String {
        val digest = MessageDigest.getInstance("SHA-256")
        val buffer = ByteArray(8192)
        stream.use {
            while (true) {
                val read = it.read(buffer)
                if (read < 0) break
                digest.update(buffer, 0, read)
            }
        }
        return digest.digest().joinToString("") { "%02x".format(it) }
    }

    /**
     * Extract a 64-char hex SHA256 from a checksum file body. Handles the
     * common `sha256sum` shapes: `<hex>`, `<hex>  filename`, and
     * `SHA256(file)= <hex>`. Returns null when no valid digest is found.
     */
    fun parseExpectedChecksum(content: String?): String? {
        if (content.isNullOrBlank()) return null
        val hexToken = Regex("\\b[a-fA-F0-9]{64}\\b")
        val match = hexToken.find(content) ?: return null
        return match.value.lowercase()
    }

    /**
     * Verify [actualHex] against the published [expectedContent].
     * Fails closed: a missing/blank/unparseable expected checksum yields
     * [ChecksumResult.MissingExpected], never [ChecksumResult.Ok].
     */
    fun verify(actualHex: String, expectedContent: String?): ChecksumResult {
        val expected = parseExpectedChecksum(expectedContent)
            ?: return ChecksumResult.MissingExpected(
                "No SHA256 checksum published for this asset",
            )
        return if (expected.equals(actualHex, ignoreCase = true)) {
            ChecksumResult.Ok
        } else {
            ChecksumResult.Mismatch(expected = expected, actual = actualHex.lowercase())
        }
    }
}
