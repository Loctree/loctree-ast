/*
 * Verified loctree-lsp release downloader.
 *
 * Downloads the platform asset AND its published `.sha256` checksum,
 * verifies the digest, and only then marks the binary executable. A
 * missing or mismatched checksum aborts and removes the partial file.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.binary

import com.intellij.openapi.diagnostic.logger
import java.io.IOException
import java.net.HttpURLConnection
import java.net.URI
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.StandardCopyOption

class ReleaseDownloadException(message: String) : IOException(message)

class ReleaseDownloader(
    private val os: OsFamily = PlatformAsset.detectOs(),
    private val arch: Arch = PlatformAsset.detectArch(),
) {
    private val log = logger<ReleaseDownloader>()

    /**
     * Download and verify the binary into [binDir]. Returns the verified
     * executable path. Throws [ReleaseDownloadException] when the
     * platform is unsupported, the download fails, or verification fails.
     */
    fun downloadVerified(
        binDir: Path,
        repoBaseUrl: String,
        tag: String,
    ): Path {
        val asset = PlatformAsset.assetName(os, arch)
            ?: throw ReleaseDownloadException(
                "No loctree-lsp release asset for this platform ($os/$arch)",
            )

        Files.createDirectories(binDir)
        val binaryName = PlatformAsset.binaryName(os)
        val target = binDir.resolve(binaryName)
        val tempBinary = binDir.resolve("$binaryName.download")

        val binaryUrl = PlatformAsset.releaseDownloadUrl(repoBaseUrl, tag, asset)
        val checksumUrl = PlatformAsset.releaseDownloadUrl(
            repoBaseUrl,
            tag,
            PlatformAsset.checksumAssetName(asset),
        )

        log.info("Downloading loctree-lsp from $binaryUrl")
        var targetReplaced = false
        try {
            downloadTo(binaryUrl, tempBinary)
            val actualHex = ChecksumVerifier.sha256Hex(Files.newInputStream(tempBinary))
            val expectedContent = runCatching { downloadText(checksumUrl) }.getOrNull()

            when (val result = ChecksumVerifier.verify(actualHex, expectedContent)) {
                is ChecksumResult.Ok -> Unit
                is ChecksumResult.MissingExpected ->
                    throw ReleaseDownloadException(
                        "Refusing to use loctree-lsp: ${result.reason} ($checksumUrl)",
                    )
                is ChecksumResult.Mismatch ->
                    throw ReleaseDownloadException(
                        "loctree-lsp checksum mismatch: expected ${result.expected}, " +
                            "got ${result.actual}",
                    )
            }

            Files.move(
                tempBinary,
                target,
                StandardCopyOption.REPLACE_EXISTING,
            )
            targetReplaced = true
            if (os != OsFamily.WINDOWS) {
                target.toFile().setExecutable(true, false)
            }
            CachedBinaryVerifier.writeSidecar(target, actualHex)
            log.info("Verified loctree-lsp at $target")
            return target
        } catch (e: ReleaseDownloadException) {
            runCatching { Files.deleteIfExists(tempBinary) }
            if (targetReplaced) {
                runCatching { Files.deleteIfExists(target) }
                runCatching { Files.deleteIfExists(CachedBinaryVerifier.sidecarPath(target)) }
            }
            throw e
        } catch (e: Exception) {
            runCatching { Files.deleteIfExists(tempBinary) }
            if (targetReplaced) {
                runCatching { Files.deleteIfExists(target) }
                runCatching { Files.deleteIfExists(CachedBinaryVerifier.sidecarPath(target)) }
            }
            throw ReleaseDownloadException("Failed to download loctree-lsp: ${e.message}")
        }
    }

    private fun downloadTo(url: String, dest: Path) {
        val connection = openFollowingRedirects(url)
        connection.inputStream.use { input ->
            Files.copy(input, dest, StandardCopyOption.REPLACE_EXISTING)
        }
    }

    private fun downloadText(url: String): String {
        val connection = openFollowingRedirects(url)
        return connection.inputStream.use { it.readBytes().toString(Charsets.UTF_8) }
    }

    private fun openFollowingRedirects(url: String, depth: Int = 0): HttpURLConnection {
        if (depth > MAX_REDIRECTS) {
            throw ReleaseDownloadException("Too many redirects for $url")
        }
        val connection = URI(url).toURL().openConnection() as HttpURLConnection
        connection.setRequestProperty("User-Agent", USER_AGENT)
        connection.connectTimeout = CONNECT_TIMEOUT_MS
        connection.readTimeout = READ_TIMEOUT_MS
        connection.instanceFollowRedirects = false

        val code = connection.responseCode
        if (code in 300..399) {
            val location = connection.getHeaderField("Location")
                ?: throw ReleaseDownloadException("Redirect without location for $url")
            connection.disconnect()
            return openFollowingRedirects(location, depth + 1)
        }
        if (code >= 400) {
            connection.disconnect()
            throw ReleaseDownloadException("Download failed ($code) for $url")
        }
        return connection
    }

    private companion object {
        const val USER_AGENT = "loctree-intellij"
        const val CONNECT_TIMEOUT_MS = 15_000
        const val READ_TIMEOUT_MS = 60_000
        const val MAX_REDIRECTS = 5
    }
}
