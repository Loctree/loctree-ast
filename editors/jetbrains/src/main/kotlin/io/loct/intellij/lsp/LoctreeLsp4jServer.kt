/*
 * Custom lsp4j server interface for Loctree's experimental requests.
 *
 * Declares the loctree custom methods registered by loctree-lsp
 * (see loctree-lsp/src/lib.rs). lsp4j sends these over the same stdio
 * channel the native LSP client manages. Request results are decoded by
 * lsp4j's Gson, which honors the @SerializedName annotations on the
 * protocol models.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.lsp

import com.google.gson.JsonElement
import io.loct.intellij.protocol.AicxParams
import io.loct.intellij.protocol.AstQueryParams
import io.loct.intellij.protocol.BodyParams
import io.loct.intellij.protocol.ContextAtlasParams
import io.loct.intellij.protocol.ContextPackParams
import io.loct.intellij.protocol.DiffParams
import io.loct.intellij.protocol.FileQueryParams
import io.loct.intellij.protocol.HealthParams
import io.loct.intellij.protocol.HealthResponse
import io.loct.intellij.protocol.SemanticParams
import io.loct.intellij.protocol.WorkspacesParams
import org.eclipse.lsp4j.jsonrpc.services.JsonNotification
import org.eclipse.lsp4j.jsonrpc.services.JsonRequest
import org.eclipse.lsp4j.services.LanguageServer
import java.util.concurrent.CompletableFuture

/**
 * Extends the standard [LanguageServer] with Loctree's custom methods.
 *
 * Heavy responses (`find`, `impact`, `slice`, `follow`, `contextAtlas`,
 * `contextPack`, `workspaces`, `diff`, `semantic`, `aicx`, `astQuery`)
 * are returned as raw [JsonElement] envelopes so the gateway can decode
 * the `Paginated<T>` shape leniently and tolerate unknown fields. The
 * `health` response is fully typed.
 */
interface LoctreeLsp4jServer : LanguageServer {

    @JsonNotification("loctree/refresh")
    fun refresh()

    @JsonRequest("loctree/health")
    fun health(params: HealthParams): CompletableFuture<HealthResponse>

    @JsonRequest("loctree/find")
    fun find(params: FileQueryParams): CompletableFuture<JsonElement>

    @JsonRequest("loctree/impact")
    fun impact(params: FileQueryParams): CompletableFuture<JsonElement>

    @JsonRequest("loctree/slice")
    fun slice(params: FileQueryParams): CompletableFuture<JsonElement>

    @JsonRequest("loctree/follow")
    fun follow(params: FileQueryParams): CompletableFuture<JsonElement>

    @JsonRequest("loctree/body")
    fun body(params: BodyParams): CompletableFuture<JsonElement>

    @JsonRequest("loctree/contextAtlas")
    fun contextAtlas(params: ContextAtlasParams): CompletableFuture<JsonElement>

    @JsonRequest("loctree/contextPack")
    fun contextPack(params: ContextPackParams): CompletableFuture<JsonElement>

    @JsonRequest("loctree/workspaces")
    fun workspaces(params: WorkspacesParams): CompletableFuture<JsonElement>

    @JsonRequest("loctree/diff")
    fun diff(params: DiffParams): CompletableFuture<JsonElement>

    @JsonRequest("loctree/semantic")
    fun semantic(params: SemanticParams): CompletableFuture<JsonElement>

    @JsonRequest("loctree/aicx")
    fun aicx(params: AicxParams): CompletableFuture<JsonElement>

    @JsonRequest("loctree/astQuery")
    fun astQuery(params: AstQueryParams): CompletableFuture<JsonElement>
}
