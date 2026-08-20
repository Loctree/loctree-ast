/*
 * Asset selection + release URL tests (VS Code parity).
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.binary

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PlatformAssetTest {

    @Test
    fun assetMatrixListsOnlyPublishedBinaries() {
        // Exactly the assets published on Loctree/loctree-release — the same
        // three platforms the VS Code extension ships for.
        assertEquals("loctree-lsp-darwin-arm64", PlatformAsset.assetName(OsFamily.MACOS, Arch.ARM64))
        assertEquals("loctree-lsp-darwin-x64", PlatformAsset.assetName(OsFamily.MACOS, Arch.X64))
        assertEquals("loctree-lsp-linux-x64", PlatformAsset.assetName(OsFamily.LINUX, Arch.X64))
    }

    @Test
    fun unsupportedPairsReturnNull() {
        // Windows and Linux arm64 have NO published loctree-lsp asset;
        // advertising one produced a guaranteed 404 and a silently dead
        // plugin. These must stay null until real binaries are published.
        assertNull(PlatformAsset.assetName(OsFamily.WINDOWS, Arch.X64))
        assertNull(PlatformAsset.assetName(OsFamily.WINDOWS, Arch.ARM64))
        assertNull(PlatformAsset.assetName(OsFamily.LINUX, Arch.ARM64))
        assertNull(PlatformAsset.assetName(OsFamily.OTHER, Arch.X64))
        assertNull(PlatformAsset.assetName(OsFamily.MACOS, Arch.OTHER))
    }

    @Test
    fun binaryNameByOs() {
        assertEquals("loctree-lsp", PlatformAsset.binaryName(OsFamily.MACOS))
        assertEquals("loctree-lsp", PlatformAsset.binaryName(OsFamily.LINUX))
        assertEquals("loctree-lsp.exe", PlatformAsset.binaryName(OsFamily.WINDOWS))
    }

    @Test
    fun osAndArchDetection() {
        assertEquals(OsFamily.MACOS, PlatformAsset.detectOs("Mac OS X"))
        assertEquals(OsFamily.WINDOWS, PlatformAsset.detectOs("Windows 11"))
        assertEquals(OsFamily.LINUX, PlatformAsset.detectOs("Linux"))
        assertEquals(Arch.ARM64, PlatformAsset.detectArch("aarch64"))
        assertEquals(Arch.X64, PlatformAsset.detectArch("amd64"))
    }

    @Test
    fun normalizeRepoUrlStripsDecoration() {
        assertEquals(
            "https://github.com/Loctree/loctree-release",
            PlatformAsset.normalizeRepoUrl("git+https://github.com/Loctree/loctree-release.git"),
        )
        assertEquals(
            "https://github.com/Loctree/loctree-release",
            PlatformAsset.normalizeRepoUrl("https://github.com/Loctree/loctree-release/"),
        )
        assertNull(PlatformAsset.normalizeRepoUrl("not-a-url"))
        assertNull(PlatformAsset.normalizeRepoUrl(""))
    }

    @Test
    fun releaseUrlForLatestAndTag() {
        val base = "https://github.com/Loctree/loctree-release"
        assertEquals(
            "$base/releases/latest/download/loctree-lsp-darwin-arm64",
            PlatformAsset.releaseDownloadUrl(base, "latest", "loctree-lsp-darwin-arm64"),
        )
        assertEquals(
            "$base/releases/download/v0.12.1/loctree-lsp-darwin-arm64",
            PlatformAsset.releaseDownloadUrl(base, "v0.12.1", "loctree-lsp-darwin-arm64"),
        )
    }

    @Test
    fun checksumAssetNameConvention() {
        assertEquals(
            "loctree-lsp-darwin-arm64.sha256",
            PlatformAsset.checksumAssetName("loctree-lsp-darwin-arm64"),
        )
    }
}
