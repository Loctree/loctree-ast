/*
 * Verified cache marker for downloaded loctree-lsp binaries.
 *
 * A cached runtime is only trusted when the binary still matches the
 * SHA256 sidecar written after a successful verified download.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.binary

import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardOpenOption

object CachedBinaryVerifier {

    fun sidecarPath(binary: Path): Path =
        binary.resolveSibling("${binary.fileName}.sha256")

    fun writeSidecar(binary: Path, actualHex: String) {
        Files.writeString(
            sidecarPath(binary),
            "${actualHex.lowercase()}  ${binary.fileName}\n",
            StandardCharsets.UTF_8,
            StandardOpenOption.CREATE,
            StandardOpenOption.TRUNCATE_EXISTING,
            StandardOpenOption.WRITE,
        )
    }

    fun isVerified(binary: Path, os: OsFamily): Boolean {
        if (!Files.isRegularFile(binary)) return false
        if (os != OsFamily.WINDOWS && !Files.isExecutable(binary)) return false

        val expectedContent = runCatching {
            Files.readString(sidecarPath(binary), StandardCharsets.UTF_8)
        }.getOrNull() ?: return false

        val actualHex = runCatching {
            ChecksumVerifier.sha256Hex(Files.newInputStream(binary))
        }.getOrNull() ?: return false

        return ChecksumVerifier.verify(actualHex, expectedContent) is ChecksumResult.Ok
    }
}
