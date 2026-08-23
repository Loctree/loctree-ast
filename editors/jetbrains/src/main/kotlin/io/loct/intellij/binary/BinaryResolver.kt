/*
 * loctree-lsp runtime resolution for JetBrains IDEs.
 *
 * Resolution order (mirrors editors/vscode/src/client.ts, then adds checksum
 * guards for downloads/cache):
 *   1. valid user-configured serverPath (file or directory override)
 *   2. bundled plugin binary
 *   3. IDE cache (previously verified download)
 *   4. verified download (SHA256 fail-closed)
 *   5. preferred user install (~/.local/bin)
 *   6. PATH fallback
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.binary

import com.intellij.ide.plugins.PluginManagerCore
import com.intellij.openapi.application.PathManager
import com.intellij.openapi.diagnostic.logger
import com.intellij.openapi.extensions.PluginId
import io.loct.intellij.settings.LoctreeSettingsState
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.util.concurrent.TimeUnit

/** Where a resolved runtime came from — surfaced to status/logging. */
enum class ResolutionSource { CONFIGURED, BUNDLED, CACHE, DOWNLOADED, PREFERRED_INSTALL, PATH_FALLBACK }

data class ResolvedRuntime(
    val command: String,
    val source: ResolutionSource,
    val identity: String,
    val warning: String? = null,
)

class BinaryResolver(
    private val settings: LoctreeSettingsState = LoctreeSettingsState.getInstance(),
    private val downloader: ReleaseDownloader = ReleaseDownloader(),
    private val os: OsFamily = PlatformAsset.detectOs(),
    private val arch: Arch = PlatformAsset.detectArch(),
    private val pathEnv: String = System.getenv("PATH").orEmpty(),
    private val userHome: String = System.getProperty("user.home").orEmpty(),
    private val pluginVersion: String? = detectPluginVersion(),
) {
    private val log = logger<BinaryResolver>()

    /** IDE cache directory used to store verified downloads. */
    fun cacheDir(): Path = Paths.get(PathManager.getSystemPath(), "loctree", "bin", downloadTag())

    /**
     * Resolve the runtime command. Never throws: when no binary can be
     * verified or located, falls back to the bare command name, but never
     * silently — the returned [ResolvedRuntime.warning] carries a
     * user-visible explanation (unsupported platform, failed download, or
     * missing local install) that the LSP descriptor surfaces as an IDE
     * notification.
     */
    fun resolve(): ResolvedRuntime {
        val configured = settings.serverPath.trim()
        if (configured.isNotEmpty()) {
            val configuredBinary = configuredBinary(configured)
            if (configuredBinary != null) {
                return resolvedRuntime(configuredBinary, ResolutionSource.CONFIGURED)
            }
            log.warn(
                "Configured loctree-lsp path did not resolve to an executable " +
                    "file or directory containing ${PlatformAsset.binaryName(os)}: $configured",
            )
        }

        val bundled = bundledBinary()
        if (bundled != null) {
            return resolvedRuntime(bundled, ResolutionSource.BUNDLED)
        }

        val cached = cachedBinary()
        if (cached != null) {
            return resolvedRuntime(cached, ResolutionSource.CACHE)
        }

        val assetAvailable = PlatformAsset.assetName(os, arch) != null
        var downloadFailure: String? = null
        if (!settings.autoDownload) {
            log.info("loctree-lsp auto-download disabled; falling back to PATH")
        } else if (!assetAvailable) {
            log.warn("No published loctree-lsp asset for $os/$arch; skipping download")
        } else {
            val downloaded = runCatching {
                downloader.downloadVerified(cacheDir(), repoBaseUrl(), downloadTag())
            }.onFailure {
                downloadFailure = it.message
                log.warn("loctree-lsp download failed: ${it.message}")
            }.getOrNull()
            if (downloaded != null) {
                return resolvedRuntime(downloaded, ResolutionSource.DOWNLOADED)
            }
        }

        val preferred = preferredInstalledBinary()
        val pathMatch = pathBinary()
        if (preferred != null) {
            val shadowed = pathMatch?.takeIf { canonical(it) != canonical(preferred) }
            val runtime = resolvedRuntime(preferred, ResolutionSource.PREFERRED_INSTALL)
            return if (shadowed == null) {
                runtime
            } else {
                val shadowedPath = canonical(shadowed).toString()
                runtime.copy(
                    warning = "$PATH_SHADOW_WARNING $shadowedPath (${probeVersion(shadowedPath)}) appears first on PATH, " +
                        "but the preferred install ${runtime.command} (${runtime.identity}) will be used. " +
                        "Remove or reorder the stale entry.",
                )
            }
        }
        if (pathMatch != null) {
            return resolvedRuntime(pathMatch, ResolutionSource.PATH_FALLBACK)
        }
        // Nothing resolved anywhere. Returning a bare command name keeps the
        // "never throws" contract, but a user must never be left with a plugin
        // that does nothing and says nothing — attach an actionable warning.
        val command = PlatformAsset.binaryName(os)
        val warning = when {
            !assetAvailable ->
                "$UNSUPPORTED_PLATFORM_WARNING loctree-lsp has no published binary for $os/$arch " +
                    "(published platforms: ${PlatformAsset.SUPPORTED_PLATFORMS}) and no local install was found. " +
                    "Build or install loctree-lsp manually, then set its path under Settings | Tools | Loctree."
            downloadFailure != null ->
                "$RUNTIME_MISSING_WARNING downloading loctree-lsp tag '${downloadTag()}' failed " +
                    "($downloadFailure) and no local install was found. The release tag may not be " +
                    "published yet — install loctree-lsp manually (for example into ~/.local/bin), " +
                    "set its path under Settings | Tools | Loctree, or set the release tag to 'latest'."
            else ->
                "$RUNTIME_MISSING_WARNING no loctree-lsp was found locally and automatic download is " +
                    "disabled. Install loctree-lsp manually or enable auto-download under " +
                    "Settings | Tools | Loctree."
        }
        log.warn(warning)
        return ResolvedRuntime(command, ResolutionSource.PATH_FALLBACK, "version unavailable", warning)
    }

    private fun resolvedRuntime(path: Path, source: ResolutionSource): ResolvedRuntime {
        val command = canonical(path).toString()
        return ResolvedRuntime(command, source, probeVersion(command))
    }

    private fun canonical(path: Path): Path =
        runCatching { path.toRealPath() }.getOrElse { path.toAbsolutePath().normalize() }

    private fun preferredInstalledBinary(): Path? {
        if (userHome.isBlank()) return null
        return executable(Paths.get(userHome, ".local", "bin", PlatformAsset.binaryName(os)))
    }

    private fun pathBinary(): Path? = pathEnv
        .split(java.io.File.pathSeparatorChar)
        .asSequence()
        .filter { it.isNotBlank() }
        .map { Paths.get(it).resolve(PlatformAsset.binaryName(os)) }
        .mapNotNull(::executable)
        .firstOrNull()

    private fun executable(path: Path): Path? =
        path.takeIf { Files.isRegularFile(it) && (os == OsFamily.WINDOWS || Files.isExecutable(it)) }

    private fun probeVersion(command: String): String = runCatching {
        val process = ProcessBuilder(command, "--version")
            .redirectErrorStream(true)
            .start()
        if (!process.waitFor(VERSION_TIMEOUT_SECONDS, TimeUnit.SECONDS)) {
            process.destroyForcibly()
            return@runCatching "version unavailable"
        }
        process.inputStream.bufferedReader().use { it.readLine()?.trim() }
            ?.takeIf { it.isNotEmpty() }
            ?: "version unavailable"
    }.getOrDefault("version unavailable")

    private fun cachedBinary(): Path? {
        val candidate = cacheDir().resolve(PlatformAsset.binaryName(os))
        return if (CachedBinaryVerifier.isVerified(candidate, os)) {
            candidate
        } else {
            if (Files.isRegularFile(candidate)) {
                log.warn("Ignoring unverified cached loctree-lsp at $candidate")
            }
            null
        }
    }

    private fun configuredBinary(raw: String): Path? {
        val normalized = expandHome(stripPathQuotes(raw)).takeIf { it.isNotBlank() } ?: return null
        val base = Paths.get(normalized)
        val candidates = buildList {
            if (Files.isDirectory(base)) {
                add(base.resolve(PlatformAsset.binaryName(os)))
            }
            add(base)
            if (base.fileName?.toString() != PlatformAsset.binaryName(os)) {
                add(base.resolve(PlatformAsset.binaryName(os)))
            }
        }
        return candidates.firstOrNull { Files.isRegularFile(it) }?.also { markExecutable(it) }
    }

    private fun bundledBinary(): Path? {
        val binaryName = PlatformAsset.binaryName(os)
        val resourcePath = "/bin/$binaryName"
        val bytes = javaClass.getResourceAsStream(resourcePath)?.use { it.readBytes() } ?: return null
        val target = cacheDir().resolve("bundled").resolve(binaryName)
        return runCatching {
            Files.createDirectories(target.parent)
            Files.write(target, bytes)
            markExecutable(target)
            CachedBinaryVerifier.writeSidecar(target, ChecksumVerifier.sha256Hex(bytes))
            log.info("Prepared bundled loctree-lsp at $target")
            target
        }.onFailure {
            log.warn("Failed to prepare bundled loctree-lsp from $resourcePath: ${it.message}", it)
        }.getOrNull()
    }

    private fun markExecutable(path: Path) {
        if (os != OsFamily.WINDOWS) {
            path.toFile().setExecutable(true, false)
        }
    }

    private fun stripPathQuotes(raw: String): String {
        val trimmed = raw.trim()
        if (trimmed.length >= 2) {
            val first = trimmed.first()
            val last = trimmed.last()
            if ((first == '"' && last == '"') || (first == '\'' && last == '\'')) {
                return trimmed.substring(1, trimmed.length - 1).trim()
            }
        }
        return trimmed
    }

    private fun expandHome(raw: String): String {
        val home = System.getProperty("user.home") ?: return raw
        return when {
            raw == "~" -> home
            raw.startsWith("~/") -> Paths.get(home, raw.removePrefix("~/")).toString()
            else -> raw
        }
    }

    private fun repoBaseUrl(): String {
        val configured = PlatformAsset.normalizeRepoUrl(settings.downloadBaseUrl)
        return configured ?: PlatformAsset.DEFAULT_REPO_URL
    }

    /**
     * Release tag used for downloads and the cache directory. An empty
     * setting pins to the plugin's own version (same-version discipline:
     * a 0.14.2 plugin must not silently fetch whatever `latest` points
     * at); `latest` remains an explicit user opt-in.
     */
    internal fun downloadTag(): String {
        val tag = settings.downloadTag.trim()
        if (tag.isNotEmpty()) return tag
        val version = pluginVersion?.trim().orEmpty()
        return if (version.isEmpty()) "latest" else "v${version.removePrefix("v")}"
    }

    companion object {
        const val PATH_SHADOW_WARNING = "Loctree runtime PATH shadowing:"
        const val UNSUPPORTED_PLATFORM_WARNING = "Loctree runtime unavailable for this platform:"
        const val RUNTIME_MISSING_WARNING = "Loctree runtime not found:"
        private const val VERSION_TIMEOUT_SECONDS = 5L

        /**
         * The installed plugin's own version, used to pin runtime downloads.
         * Null outside a running IDE (bare unit tests) — callers fall back
         * to `latest` in that case.
         */
        fun detectPluginVersion(): String? = runCatching {
            PluginManagerCore.getPlugin(PluginId.getId("io.loct.loctree"))?.version
        }.getOrNull()
    }
}
