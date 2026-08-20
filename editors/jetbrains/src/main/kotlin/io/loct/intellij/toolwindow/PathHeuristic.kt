/*
 * Multi-language path recognition for projected LSP rows.
 *
 * String locations must set FindingItem.file for navigation and copy-for-agent.
 * Hardcoding only .kt/.rs was a silent UX hole for TS/Python/Swift/Go repos.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents by Vetcoders (c)2024-2026 LibraxisAI
 */

package io.loct.intellij.toolwindow

import java.net.URI
import java.nio.file.Path

internal object PathHeuristic {

    /**
     * Extensions loctree actually indexes across the suite + common agent
     * product stacks. Keep lowercase; matching is case-insensitive.
     */
    private val KNOWN_EXTENSIONS: Set<String> = setOf(
        // Rust / C family
        "rs", "c", "h", "cc", "cpp", "hpp", "m", "mm",
        // JVM / Apple
        "kt", "kts", "java", "scala", "groovy", "swift",
        // Web / node
        "ts", "tsx", "js", "jsx", "mjs", "cjs", "vue", "svelte", "css", "scss", "less", "html", "htm",
        // Python / data
        "py", "pyi", "ipynb", "r", "jl",
        // Go / Zig / Nim
        "go", "zig", "nim",
        // Shell / config / docs that often appear in findings
        "sh", "bash", "zsh", "fish", "toml", "yaml", "yml", "json", "jsonc", "md", "mdx",
        "graphql", "gql", "sql", "proto", "tf", "hcl",
        // Build
        "gradle", "cmake", "makefile",
    )

    fun looksLikeFilePath(raw: String): Boolean {
        val s = raw.trim()
        if (s.isEmpty() || s.length > 1024) return false
        if (s.startsWith("file:")) return true
        if (s.contains('/') || s.contains('\\')) return true
        // bare "Main.swift" / "foo.ts" without directory
        val ext = extensionOf(s) ?: return false
        return ext in KNOWN_EXTENSIONS
    }

    /** Strip file:// and normalize for navigation / clipboard. */
    fun normalizeFileRef(raw: String): String {
        val s = raw.trim()
        if (s.startsWith("file:")) {
            return runCatching {
                Path.of(URI(s)).toString()
            }.getOrDefault(s.removePrefix("file://").removePrefix("file:"))
        }
        return s
    }

    private fun extensionOf(path: String): String? {
        val name = path.substringAfterLast('/').substringAfterLast('\\')
        val dot = name.lastIndexOf('.')
        if (dot <= 0 || dot == name.lastIndex) return null
        return name.substring(dot + 1).lowercase()
    }
}
