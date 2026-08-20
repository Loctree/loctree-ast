/*
 * Runtime resolver chain tests.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.binary

import io.loct.intellij.settings.LoctreeSettingsState
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeFalse
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder
import java.nio.file.Files

class BinaryResolverTest {

    @Rule
    @JvmField
    val tempDir = TemporaryFolder()

    @Test
    fun configuredExecutableWinsWhenValid() {
        val binary = tempDir.newFile("loctree-lsp").toPath()
        Files.write(binary, "test".toByteArray())

        val settings = LoctreeSettingsState()
        settings.serverPath = binary.toString()
        settings.autoDownload = false

        val resolved = BinaryResolver(settings = settings, os = OsFamily.MACOS).resolve()

        assertEquals(ResolutionSource.CONFIGURED, resolved.source)
        assertEquals(binary.toRealPath().toString(), resolved.command)
    }

    @Test
    fun configuredDirectoryResolvesBinaryInsideIt() {
        val dir = tempDir.newFolder("bin").toPath()
        val binary = dir.resolve("loctree-lsp")
        Files.write(binary, "test".toByteArray())

        val settings = LoctreeSettingsState()
        settings.serverPath = dir.toString()
        settings.autoDownload = false

        val resolved = BinaryResolver(settings = settings, os = OsFamily.MACOS).resolve()

        assertEquals(ResolutionSource.CONFIGURED, resolved.source)
        assertEquals(binary.toRealPath().toString(), resolved.command)
    }

    @Test
    fun invalidConfiguredPathFallsThroughInsteadOfBlockingResolution() {
        val invalid = tempDir.root.toPath().resolve("missing").toString()
        val settings = LoctreeSettingsState()
        settings.serverPath = invalid
        settings.autoDownload = false

        val resolved = BinaryResolver(
            settings = settings,
            os = OsFamily.MACOS,
            pathEnv = "",
            userHome = tempDir.newFolder("empty-home").toString(),
        ).resolve()

        assertNotEquals(ResolutionSource.CONFIGURED, resolved.source)
        assertNotEquals(invalid, resolved.command)
        assertTrue(resolved.command.endsWith("loctree-lsp"))
    }

    /**
     * Download-only invariant: a default build carries no runtime resource, so
     * `bundledBinary()` cannot hand a host-shaped binary to a foreign platform.
     * Skipped for the dev-only `-PbundleLsp=true` opt-in, which is never published.
     */
    @Test
    fun defaultBuildCarriesNoBundledRuntimeResource() {
        assumeFalse(System.getProperty("loctree.bundleLsp") == "true")

        assertNull(javaClass.getResource("/bin/loctree-lsp"))
        assertNull(javaClass.getResource("/bin/loctree-lsp.exe"))
    }

    /**
     * With no bundled resource and downloads disabled, resolution must still
     * end at the PATH command rather than throwing or returning a stale path.
     */
    @Test
    fun downloadOnlyBuildFallsBackToPathWhenNothingIsCached() {
        assumeFalse(System.getProperty("loctree.bundleLsp") == "true")

        val settings = LoctreeSettingsState()
        settings.serverPath = ""
        settings.autoDownload = false

        val resolved = BinaryResolver(
            settings = settings,
            os = OsFamily.MACOS,
            pathEnv = "",
            userHome = tempDir.newFolder("fallback-empty-home").toString(),
        ).resolve()

        assertEquals(ResolutionSource.PATH_FALLBACK, resolved.source)
        assertEquals("loctree-lsp", resolved.command)
        assertEquals("version unavailable", resolved.identity)
    }

    /**
     * A platform with no published asset must never end in a silent dead
     * plugin: the resolution carries a user-visible warning naming the
     * platform and the manual install path.
     */
    @Test
    fun unsupportedPlatformResolutionCarriesVisibleWarning() {
        assumeFalse(System.getProperty("loctree.bundleLsp") == "true")

        val settings = LoctreeSettingsState()
        settings.serverPath = ""
        settings.autoDownload = true

        val resolved = BinaryResolver(
            settings = settings,
            os = OsFamily.WINDOWS,
            arch = Arch.X64,
            pathEnv = "",
            userHome = tempDir.newFolder("win-empty-home").toString(),
        ).resolve()

        assertEquals(ResolutionSource.PATH_FALLBACK, resolved.source)
        assertEquals("loctree-lsp.exe", resolved.command)
        assertTrue(resolved.warning.orEmpty().startsWith(BinaryResolver.UNSUPPORTED_PLATFORM_WARNING))
        assertTrue(resolved.warning.orEmpty().contains("WINDOWS"))
        assertTrue(resolved.warning.orEmpty().contains("Settings | Tools | Loctree"))
    }

    @Test
    fun autoDownloadDisabledWithNothingFoundStillWarnsVisibly() {
        assumeFalse(System.getProperty("loctree.bundleLsp") == "true")

        val settings = LoctreeSettingsState()
        settings.serverPath = ""
        settings.autoDownload = false

        val resolved = BinaryResolver(
            settings = settings,
            os = OsFamily.MACOS,
            arch = Arch.ARM64,
            pathEnv = "",
            userHome = tempDir.newFolder("warn-empty-home").toString(),
        ).resolve()

        assertEquals(ResolutionSource.PATH_FALLBACK, resolved.source)
        assertTrue(resolved.warning.orEmpty().startsWith(BinaryResolver.RUNTIME_MISSING_WARNING))
    }

    /**
     * Same-version discipline: an empty tag setting pins the download to the
     * plugin's own version; `latest` stays an explicit opt-in; without a
     * plugin version (bare unit test / dev IDE), fall back to `latest`.
     */
    @Test
    fun downloadTagPinsToPluginVersionByDefault() {
        val settings = LoctreeSettingsState()
        settings.downloadTag = ""

        assertEquals(
            "v0.14.2",
            BinaryResolver(settings = settings, pluginVersion = "0.14.2").downloadTag(),
        )
        assertEquals(
            "v0.14.2",
            BinaryResolver(settings = settings, pluginVersion = "v0.14.2").downloadTag(),
        )
        assertEquals(
            "latest",
            BinaryResolver(settings = settings, pluginVersion = null).downloadTag(),
        )

        settings.downloadTag = "latest"
        assertEquals(
            "latest",
            BinaryResolver(settings = settings, pluginVersion = "0.14.2").downloadTag(),
        )

        settings.downloadTag = "v0.13.1"
        assertEquals(
            "v0.13.1",
            BinaryResolver(settings = settings, pluginVersion = "0.14.2").downloadTag(),
        )
    }

    @Test
    fun preferredUserInstallWinsAndReportsPathShadowingWithExactIdentity() {
        assumeFalse(System.getProperty("os.name").lowercase().contains("win"))
        val home = tempDir.newFolder("home").toPath()
        val preferred = home.resolve(".local/bin/loctree-lsp")
        val cargoBin = tempDir.newFolder("cargo-bin").toPath()
        val shadowed = cargoBin.resolve("loctree-lsp")
        Files.createDirectories(preferred.parent)
        Files.writeString(preferred, "#!/bin/sh\necho 'loctree-lsp 0.14.1+gpreferred'\n")
        Files.writeString(shadowed, "#!/bin/sh\necho 'loctree-lsp 0.12.2'\n")
        preferred.toFile().setExecutable(true)
        shadowed.toFile().setExecutable(true)

        val settings = LoctreeSettingsState().apply {
            serverPath = ""
            autoDownload = false
        }
        val resolved = BinaryResolver(
            settings = settings,
            os = OsFamily.MACOS,
            pathEnv = cargoBin.toString(),
            userHome = home.toString(),
        ).resolve()

        assertEquals(ResolutionSource.PREFERRED_INSTALL, resolved.source)
        assertEquals(preferred.toRealPath().toString(), resolved.command)
        assertEquals("loctree-lsp 0.14.1+gpreferred", resolved.identity)
        assertTrue(resolved.warning.orEmpty().startsWith(BinaryResolver.PATH_SHADOW_WARNING))
        assertTrue(resolved.warning.orEmpty().contains("loctree-lsp 0.12.2"))
    }
}
