/*
 * Typed facade for Loctree's custom LSP requests.
 *
 * Single entry point the UI layer uses to talk to loctree-lsp. Locates
 * the running native LSP server for the project and dispatches typed
 * loctree custom requests plus the `loctree/refresh` notification. Heavy
 * responses come back as raw JSON envelopes decoded leniently into
 * Paginated<T> so unknown server-added fields never break the client.
 *
 * Uses the native LSP server proxy. The IntelliJ Platform changed the
 * request API between 2024.x and 2026.x, so calls are routed through a
 * tiny reflective shim rather than linking to a single binary shape.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

package io.loct.intellij.lsp

import com.google.gson.Gson
import com.google.gson.JsonElement
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.service
import com.intellij.openapi.diagnostic.logger
import com.intellij.openapi.project.Project
import com.intellij.platform.lsp.api.LspServer
import com.intellij.platform.lsp.api.LspServerManager
import io.loct.intellij.protocol.AicxParams
import io.loct.intellij.protocol.AstQueryParams
import io.loct.intellij.protocol.BodyParams
import io.loct.intellij.protocol.ContextAtlasParams
import io.loct.intellij.protocol.ContextPackParams
import io.loct.intellij.protocol.DiffParams
import io.loct.intellij.protocol.FileQueryParams
import io.loct.intellij.protocol.HealthParams
import io.loct.intellij.protocol.HealthResponse
import io.loct.intellij.protocol.Paginated
import io.loct.intellij.protocol.PaginatedJsonType
import io.loct.intellij.protocol.SemanticParams
import io.loct.intellij.protocol.WorkspacesParams
import org.eclipse.lsp4j.services.LanguageServer
import java.util.concurrent.CompletableFuture
import java.util.concurrent.TimeUnit

@Service(Service.Level.PROJECT)
class LoctreeLspGateway(private val project: Project) {

    private val log = logger<LoctreeLspGateway>()
    private val gson = Gson()

    /** True when at least one loctree-lsp server is running for this project. */
    fun isRunning(): Boolean = runningServers().isNotEmpty()

    /** Fire-and-forget `loctree/refresh` notification to all running servers. */
    fun refresh(): Boolean {
        val servers = runningServers()
        if (servers.isEmpty()) {
            log.debug("refresh skipped: no running loctree-lsp server")
            return false
        }
        servers.forEach { server ->
            runCatching {
                sendNotification(server) { it.refresh() }
            }
                .onFailure { log.warn("refresh notification failed: ${it.message}") }
        }
        return true
    }

    fun health(params: HealthParams = HealthParams()): HealthResponse? {
        val server = firstServer() ?: return null
        return runCatching {
            sendRequestSync(server) { it.health(params) }
        }.onFailure { log.warn("health request failed: ${it.message}") }.getOrNull()
    }

    fun find(params: FileQueryParams): Paginated<JsonElement>? =
        paginatedRequest("find") { it.find(params) }

    fun impact(params: FileQueryParams): Paginated<JsonElement>? =
        paginatedRequest("impact") { it.impact(params) }

    fun slice(params: FileQueryParams): Paginated<JsonElement>? =
        paginatedRequest("slice") { it.slice(params) }

    fun follow(params: FileQueryParams): Paginated<JsonElement>? =
        paginatedRequest("follow") { it.follow(params) }

    fun body(params: BodyParams): JsonElement? =
        rawRequest("body") { it.body(params) }

    fun contextAtlas(params: ContextAtlasParams): Paginated<JsonElement>? =
        paginatedRequest("contextAtlas") { it.contextAtlas(params) }

    fun contextPack(params: ContextPackParams): JsonElement? =
        rawRequest("contextPack") { it.contextPack(params) }

    fun workspaces(params: WorkspacesParams = WorkspacesParams()): JsonElement? =
        rawRequest("workspaces") { it.workspaces(params) }

    fun diff(params: DiffParams = DiffParams()): JsonElement? =
        rawRequest("diff") { it.diff(params) }

    fun semantic(params: SemanticParams): JsonElement? =
        rawRequest("semantic") { it.semantic(params) }

    fun aicx(params: AicxParams = AicxParams()): JsonElement? =
        rawRequest("aicx") { it.aicx(params) }

    fun astQuery(params: AstQueryParams): JsonElement? =
        rawRequest("astQuery") { it.astQuery(params) }

    /**
     * Forward-compatible custom request router for UI flows that do not need a
     * bespoke gateway wrapper. Params are converted through Gson so callers may
     * pass either the protocol data class or a map-like object.
     */
    fun customRequest(method: String, params: Any): JsonElement? =
        when (method) {
            "loctree/health" -> health(convert(params, HealthParams::class.java))?.let { gson.toJsonTree(it) }
            "loctree/find" -> rawRequest("find") { it.find(convert(params, FileQueryParams::class.java)) }
            "loctree/impact" -> rawRequest("impact") { it.impact(convert(params, FileQueryParams::class.java)) }
            "loctree/slice" -> rawRequest("slice") { it.slice(convert(params, FileQueryParams::class.java)) }
            "loctree/follow" -> rawRequest("follow") { it.follow(convert(params, FileQueryParams::class.java)) }
            "loctree/body" -> rawRequest("body") { it.body(convert(params, BodyParams::class.java)) }
            "loctree/contextAtlas" -> rawRequest("contextAtlas") { it.contextAtlas(convert(params, ContextAtlasParams::class.java)) }
            "loctree/contextPack" -> rawRequest("contextPack") { it.contextPack(convert(params, ContextPackParams::class.java)) }
            "loctree/workspaces" -> rawRequest("workspaces") { it.workspaces(convert(params, WorkspacesParams::class.java)) }
            "loctree/diff" -> rawRequest("diff") { it.diff(convert(params, DiffParams::class.java)) }
            "loctree/semantic" -> rawRequest("semantic") { it.semantic(convert(params, SemanticParams::class.java)) }
            "loctree/aicx" -> rawRequest("aicx") { it.aicx(convert(params, AicxParams::class.java)) }
            "loctree/astQuery" -> rawRequest("astQuery") { it.astQuery(convert(params, AstQueryParams::class.java)) }
            else -> {
                log.warn("unsupported Loctree custom request: $method")
                null
            }
        }

    // Internals -------------------------------------------------------------

    private fun paginatedRequest(
        name: String,
        call: (LoctreeLsp4jServer) -> CompletableFuture<JsonElement>,
    ): Paginated<JsonElement>? {
        val server = firstServer() ?: return null
        val raw = runCatching {
            sendRequestSync(server, call)
        }.onFailure { log.warn("$name request failed: ${it.message}") }.getOrNull()
            ?: return null
        return decodePaginated(raw)
    }

    private fun rawRequest(
        name: String,
        call: (LoctreeLsp4jServer) -> CompletableFuture<JsonElement>,
    ): JsonElement? {
        val server = firstServer() ?: return null
        return runCatching {
            sendRequestSync(server, call)
        }.onFailure { log.warn("$name request failed: ${it.message}") }.getOrNull()
    }

    private fun <T> convert(params: Any, type: Class<T>): T =
        gson.fromJson(gson.toJsonTree(params), type)

    /** Lenient decode of a Paginated<JsonElement> envelope (tolerates unknown fields). */
    @Suppress("UNCHECKED_CAST")
    private fun decodePaginated(raw: JsonElement): Paginated<JsonElement>? =
        runCatching { gson.fromJson<Paginated<JsonElement>>(raw, PaginatedJsonType.TYPE) }
            .onFailure { log.warn("failed to decode paginated envelope: ${it.message}") }
            .getOrNull()

    private fun runningServers(): Collection<LspServer> =
        LspServerManager.getInstance(project)
            .getServersForProvider(LoctreeLspServerSupportProvider::class.java)

    private fun firstServer(): LspServer? = runningServers().firstOrNull()

    private fun <R> sendRequestSync(
        server: LspServer,
        call: (LoctreeLsp4jServer) -> CompletableFuture<R>,
    ): R {
        val newApi = server.javaClass.methods.firstOrNull {
            it.name == "sendRequestSync" && it.parameterTypes.size == 2
        }
        if (newApi != null) {
            val requestCall = { languageServer: LanguageServer ->
                call(languageServer as LoctreeLsp4jServer)
            }
            // 2024.x declared the timeout as long; 2026.x narrowed it to int.
            // Method.invoke refuses Long->int narrowing ("argument type
            // mismatch"), so match the reflected parameter type explicitly.
            val timeoutArg: Any = when (newApi.parameterTypes[0]) {
                java.lang.Integer.TYPE, java.lang.Integer::class.java -> REQUEST_TIMEOUT_MS.toInt()
                else -> REQUEST_TIMEOUT_MS
            }
            @Suppress("UNCHECKED_CAST")
            return newApi.invoke(server, timeoutArg, requestCall) as R
        }

        return call(lsp4jServer(server)).get(REQUEST_TIMEOUT_MS, TimeUnit.MILLISECONDS)
    }

    private fun sendNotification(server: LspServer, call: (LoctreeLsp4jServer) -> Unit) {
        val newApi = server.javaClass.methods.firstOrNull {
            it.name == "sendNotification" && it.parameterTypes.size == 1
        }
        if (newApi != null) {
            val notificationCall = { languageServer: LanguageServer ->
                call(languageServer as LoctreeLsp4jServer)
                Unit
            }
            newApi.invoke(server, notificationCall)
            return
        }

        call(lsp4jServer(server))
    }

    private fun lsp4jServer(server: LspServer): LoctreeLsp4jServer {
        val oldApi = server.javaClass.methods.first { it.name == "getLsp4jServer" }
        return oldApi.invoke(server) as LoctreeLsp4jServer
    }

    companion object {
        private const val REQUEST_TIMEOUT_MS = 30_000L

        fun getInstance(project: Project): LoctreeLspGateway = project.service()
    }
}
