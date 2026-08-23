/*
 * Persistent application-level settings for the Loctree plugin.
 *
 * Mirrors the VS Code configuration surface from
 * editors/vscode/package.json: serverPath, autoRefresh, showStatusBar,
 * autoDownload, downloadBaseUrl, downloadTag, diagnosticSeverity.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.settings

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage

/** Supported diagnostic severities, matching the VS Code enum. */
enum class DiagnosticSeverity { ERROR, WARNING, INFORMATION, HINT }

@State(
    name = "io.loct.intellij.settings.LoctreeSettingsState",
    storages = [Storage("loctree.xml")],
)
class LoctreeSettingsState : PersistentStateComponent<LoctreeSettingsState.State> {

    class State {
        @JvmField var serverPath: String = ""
        @JvmField var autoRefresh: Boolean = false
        @JvmField var showStatusBar: Boolean = true
        @JvmField var autoDownload: Boolean = true
        @JvmField var downloadBaseUrl: String = ""
        // Empty pins downloads to the plugin's own version (same-version
        // discipline); "latest" is an explicit user opt-in.
        @JvmField var downloadTag: String = ""
        @JvmField var diagnosticSeverity: DiagnosticSeverity = DiagnosticSeverity.WARNING
    }

    private var state = State()

    override fun getState(): State = state

    override fun loadState(loaded: State) {
        state = loaded
    }

    // Convenience accessors -------------------------------------------------

    var serverPath: String
        get() = state.serverPath
        set(value) { state.serverPath = value }

    var autoRefresh: Boolean
        get() = state.autoRefresh
        set(value) { state.autoRefresh = value }

    var showStatusBar: Boolean
        get() = state.showStatusBar
        set(value) { state.showStatusBar = value }

    var autoDownload: Boolean
        get() = state.autoDownload
        set(value) { state.autoDownload = value }

    var downloadBaseUrl: String
        get() = state.downloadBaseUrl
        set(value) { state.downloadBaseUrl = value }

    var downloadTag: String
        get() = state.downloadTag
        set(value) { state.downloadTag = value }

    var diagnosticSeverity: DiagnosticSeverity
        get() = state.diagnosticSeverity
        set(value) { state.diagnosticSeverity = value }

    companion object {
        fun getInstance(): LoctreeSettingsState =
            ApplicationManager.getApplication().getService(LoctreeSettingsState::class.java)
    }
}
