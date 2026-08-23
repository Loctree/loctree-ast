/*
 * Platform/architecture asset selection and release URL building.
 *
 * The asset matrix lists ONLY the loctree-lsp binaries actually published
 * on Loctree/loctree-release: darwin-arm64, darwin-x64, linux-x64, and
 * windows-x64. Advertising an
 * asset that does not exist turns first run into a silent 404, so every
 * other OS/arch pair returns null and the resolver surfaces an explicit
 * unsupported-platform notification instead. Kept free of IntelliJ
 * Platform dependencies so it can be unit-tested in isolation.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.binary

/** Normalized operating system families the resolver understands. */
enum class OsFamily { MACOS, LINUX, WINDOWS, OTHER }

/** Normalized CPU architectures the resolver understands. */
enum class Arch { ARM64, X64, OTHER }

object PlatformAsset {

    // Public releases repo — loctree-suite is private and must never be
    // referenced by shipped surfaces (downloads there 404 for every user).
    const val DEFAULT_REPO_URL: String = "https://github.com/Loctree/loctree-release"

    /** Detect the current OS family from `os.name`. */
    fun detectOs(osName: String = System.getProperty("os.name").orEmpty()): OsFamily {
        val name = osName.lowercase()
        return when {
            name.contains("mac") || name.contains("darwin") -> OsFamily.MACOS
            name.contains("win") -> OsFamily.WINDOWS
            name.contains("nux") || name.contains("nix") || name.contains("aix") -> OsFamily.LINUX
            else -> OsFamily.OTHER
        }
    }

    /** Detect the current CPU architecture from `os.arch`. */
    fun detectArch(osArch: String = System.getProperty("os.arch").orEmpty()): Arch {
        val arch = osArch.lowercase()
        return when {
            arch == "aarch64" || arch == "arm64" -> Arch.ARM64
            arch == "x86_64" || arch == "amd64" || arch == "x64" -> Arch.X64
            else -> Arch.OTHER
        }
    }

    /** Local binary file name for the given OS. */
    fun binaryName(os: OsFamily = detectOs()): String =
        if (os == OsFamily.WINDOWS) "loctree-lsp.exe" else "loctree-lsp"

    /** Human-readable list of platforms with published loctree-lsp assets. */
    const val SUPPORTED_PLATFORMS: String = "macOS arm64, macOS x64, Linux x64, Windows x64"

    /**
     * Release asset name for the OS/arch pair, or `null` when the
     * platform has no published artifact. Linux arm64 and Windows arm64 are
     * deliberately absent: Loctree/loctree-release does not build them,
     * and a name that 404s is worse than an honest null.
     */
    fun assetName(os: OsFamily = detectOs(), arch: Arch = detectArch()): String? = when (os) {
        OsFamily.MACOS -> when (arch) {
            Arch.ARM64 -> "loctree-lsp-darwin-arm64"
            Arch.X64 -> "loctree-lsp-darwin-x64"
            else -> null
        }
        OsFamily.LINUX -> when (arch) {
            Arch.X64 -> "loctree-lsp-linux-x64"
            else -> null
        }
        OsFamily.WINDOWS -> when (arch) {
            Arch.X64 -> "loctree-lsp-windows-x64.exe"
            else -> null
        }
        OsFamily.OTHER -> null
    }

    /** Normalize a repository URL (strip git+ prefix, .git suffix, trailing slash). */
    fun normalizeRepoUrl(raw: String?): String? {
        if (raw.isNullOrBlank()) return null
        var url = raw.trim()
        if (url.startsWith("git+")) url = url.substring(4)
        if (url.endsWith(".git")) url = url.removeSuffix(".git")
        if (url.endsWith("/")) url = url.removeSuffix("/")
        if (!url.startsWith("http")) return null
        return url
    }

    /** Build a GitHub release download URL for an asset, mirroring client.ts. */
    fun releaseDownloadUrl(repoBase: String, tag: String, assetName: String): String {
        val releaseBase = if (repoBase.contains("/releases")) {
            repoBase.replace(Regex("/releases/?$"), "/releases")
        } else {
            "$repoBase/releases"
        }
        return if (tag == "latest") {
            "$releaseBase/latest/download/$assetName"
        } else {
            "$releaseBase/download/$tag/$assetName"
        }
    }

    /** Checksum asset name convention: `<asset>.sha256`. */
    fun checksumAssetName(assetName: String): String = "$assetName.sha256"
}
