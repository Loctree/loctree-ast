/*
 * Project startup hook for loctree-lsp.
 *
 * Starts the project-wide Loctree language server as soon as the IDE project
 * opens, so tool-window and agent-facing custom requests do not depend on a
 * supported source file being opened first.
 *
 * Parity guard: starting the server triggers a server-side auto-scan
 * (loctree-lsp `initialized` -> load_snapshot -> run_scan when no snapshot
 * exists). The VS Code client only auto-starts when the workspace already
 * looks like a Loctree project (a `.loctree/` folder or a cached snapshot);
 * we mirror that here so opening an arbitrary project does not scan it from
 * scratch. Opening a supported source file still starts the server on demand
 * through LoctreeLspServerSupportProvider.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.lsp

import com.intellij.openapi.diagnostic.logger
import com.intellij.openapi.project.Project
import com.intellij.openapi.startup.ProjectActivity
import com.intellij.platform.lsp.api.LspServerManager
import java.nio.file.Files
import java.nio.file.Path
import java.nio.file.Paths
import java.security.MessageDigest

class LoctreeLspStartupActivity : ProjectActivity {

    private val log = logger<LoctreeLspStartupActivity>()

    override suspend fun execute(project: Project) {
        if (project.isDisposed) {
            return
        }
        val basePath = project.basePath
        if (basePath.isNullOrBlank()) {
            return
        }

        if (!hasProjectSignal(basePath)) {
            log.info(
                "No Loctree project signal for $basePath; deferring LSP startup " +
                    "to file-open to avoid scanning an arbitrary project",
            )
            return
        }

        runCatching {
            LspServerManager.getInstance(project).ensureServerStarted(
                LoctreeLspServerSupportProvider::class.java,
                LoctreeLspServerDescriptor(project),
            )
        }.onSuccess {
            log.info("Requested automatic loctree-lsp startup for $basePath")
        }.onFailure {
            log.warn("Automatic loctree-lsp startup failed: ${it.message}", it)
        }
    }

    /**
     * A "project signal" mirrors the VS Code client's `hasProjectSignal`: the
     * workspace already has a `.loctree/` folder, or a cached snapshot exists in
     * the per-project global cache. Absent both, an auto-start would scan a
     * random workspace from scratch.
     */
    private fun hasProjectSignal(basePath: String): Boolean {
        val base = Paths.get(basePath)
        if (Files.isDirectory(base.resolve(LOCTREE_DIR))) {
            return true
        }
        val cacheDir = projectCacheDir(base) ?: return false
        return runCatching { Files.exists(cacheDir.resolve(SNAPSHOT_FILE)) }.getOrDefault(false)
    }

    /** Mirrors `loctree-rs::snapshot::project_cache_dir`. */
    private fun projectCacheDir(root: Path): Path? {
        val canonical = runCatching { root.toRealPath() }.getOrDefault(root)
        val projectId = sha256Hex(canonical.toString()).take(PROJECT_ID_LEN)

        val custom = System.getenv(LOCT_CACHE_DIR_ENV)?.trim()
        if (!custom.isNullOrEmpty()) {
            val customPath = Paths.get(custom)
            return if (!customPath.isAbsolute) {
                // Relative override lives under the project root (parity with Rust).
                canonical.resolve(customPath)
            } else {
                customPath.resolve("projects").resolve(projectId)
            }
        }

        val base = cacheBaseDir() ?: return null
        return base.resolve("projects").resolve(projectId)
    }

    /** Mirrors `loctree-rs::snapshot::cache_base_dir`. */
    private fun cacheBaseDir(): Path? {
        val custom = System.getenv(LOCT_CACHE_DIR_ENV)?.trim()
        if (!custom.isNullOrEmpty()) {
            return Paths.get(custom)
        }
        val home = System.getProperty("user.home")?.takeIf { it.isNotBlank() } ?: return null
        val osName = System.getProperty("os.name").orEmpty().lowercase()
        return when {
            osName.contains("mac") || osName.contains("darwin") ->
                Paths.get(home, "Library", "Caches", CACHE_NAME)

            osName.contains("win") -> {
                val localAppData = System.getenv("LOCALAPPDATA")?.trim()
                val winBase =
                    if (!localAppData.isNullOrEmpty()) Paths.get(localAppData)
                    else Paths.get(home, "AppData", "Local")
                winBase.resolve(CACHE_NAME)
            }

            else -> {
                val xdg = System.getenv("XDG_CACHE_HOME")?.trim()
                val unixBase =
                    if (!xdg.isNullOrEmpty()) Paths.get(xdg) else Paths.get(home, ".cache")
                unixBase.resolve(CACHE_NAME)
            }
        }
    }

    private fun sha256Hex(input: String): String =
        MessageDigest.getInstance("SHA-256")
            .digest(input.toByteArray(Charsets.UTF_8))
            .joinToString("") { "%02x".format(it) }

    companion object {
        private const val LOCTREE_DIR = ".loctree"
        private const val SNAPSHOT_FILE = "snapshot.json"
        private const val CACHE_NAME = "loctree"
        private const val LOCT_CACHE_DIR_ENV = "LOCT_CACHE_DIR"
        private const val PROJECT_ID_LEN = 16
    }
}
