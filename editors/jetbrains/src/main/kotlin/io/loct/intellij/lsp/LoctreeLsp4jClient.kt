/*
 * lsp4j client endpoint accepting Loctree's custom server notifications.
 *
 * loctree-lsp emits `loctree/scanProgress` during workspace rescans
 * (see loctree-lsp/src/watcher.rs). Without a declared handler, lsp4j's
 * GenericEndpoint logs "Unsupported notification method" for every ping,
 * flooding idea.log. This endpoint accepts the payload and keeps the
 * JSON-RPC channel quiet; no UI surface consumes the progress yet.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.lsp

import com.google.gson.JsonElement
import com.intellij.openapi.diagnostic.logger
import com.intellij.platform.lsp.api.Lsp4jClient
import com.intellij.platform.lsp.api.LspServerNotificationsHandler
import org.eclipse.lsp4j.jsonrpc.services.JsonNotification

class LoctreeLsp4jClient(handler: LspServerNotificationsHandler) : Lsp4jClient(handler) {

    private val log = logger<LoctreeLsp4jClient>()

    @JsonNotification("loctree/scanProgress")
    fun scanProgress(params: JsonElement?) {
        if (log.isTraceEnabled) {
            log.trace("loctree/scanProgress: $params")
        }
    }
}
