/**
 * Loctree Context Pill — hardened webview shell
 *
 * The VS Code shell for the Context Pill: a `WebviewViewProvider` that owns
 * webview SECURITY (strict CSP + per-render nonce + an inbound message
 * allowlist), ambient binding to the active editor, and posting the
 * `ContextPillViewModel` to the rendered surface (Task 6 ships the actual
 * `media/contextPill.{js,css}`).
 *
 * This shell does NOT compute analysis: it delegates to `assembleViewModel`
 * over the real `LoctreeGateway` (which structurally satisfies `PillGateway`)
 * and reuses the existing `loctree.navigateToFile` / `loctree.showBody`
 * commands for navigation, so we never reach outside the workspace or ship a
 * bespoke file-opening path.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

import * as vscode from 'vscode';
import type { LoctreeGateway } from '../gateway';
import { assembleViewModel, noEditorViewModel } from './viewModel';
import { parseScopeInput, type Scope } from './scope';
import { buildAgentContextMarkdown, inferTask } from './agentContext';
import {
    groupOccurrencesByFile,
    mergeLiteralGroups,
    applyLiteralHarvest,
    resolveSymbol,
    LITERAL_FIRST_PAGE_LIMIT,
    type LiteralHarvest,
} from './viewModel';

/** Inbound message types the webview is allowed to send. Anything else is dropped. */
const ALLOWED_MESSAGE_TYPES = new Set<string>([
    'rescope',
    'copyAgentContext',
    'openFile',
    'showFullBody',
    'openProjectFile',
    'scanWorkspace',
    // Task 5 — scope-switcher symbol resolution.
    'rescopeCandidate',
    'showAsLiteral',
    // Task 6 — literal-scope pill paging.
    'loadMoreLiteral',
]);

/** Occurrence page size for each Load-more (matches the first-page size). */
const LITERAL_PAGE_LIMIT = LITERAL_FIRST_PAGE_LIMIT;

/** A raw inbound webview message. `type` is validated against the allowlist. */
interface InboundMessage {
    type?: unknown;
    query?: unknown;
    task?: unknown;
    file?: unknown;
    symbol?: unknown;
    line?: unknown;
    offset?: unknown;
}

export class ContextPillViewProvider implements vscode.WebviewViewProvider {
    public static readonly viewId = 'loctree.context';

    private view?: vscode.WebviewView;
    private readonly webviewChannel = getNonce();
    private currentScope: Scope = { kind: 'literal', value: '' };
    /** Monotonic rescope generation. Each `rescope` bumps it before awaiting and
     *  drops its render if a newer rescope started meanwhile (fast editor
     *  switches must not paint a stale VM over the current one). */
    private rescopeGen = 0;
    /** Accumulated literal occurrences for the current literal scope, so a later
     *  Copy Agent Context harvests every page loaded via Load-more — not just
     *  assembleViewModel's first page. Null whenever the scope is not literal. */
    private literalAccum: LiteralHarvest | null = null;

    constructor(
        private readonly context: vscode.ExtensionContext,
        private readonly gateway: LoctreeGateway,
        private readonly getTaskSignal: () => { changedFiles: string[]; selection: string },
    ) { }

    public resolveWebviewView(view: vscode.WebviewView): void {
        this.view = view;

        view.webview.options = {
            enableScripts: true,
            localResourceRoots: [vscode.Uri.joinPath(this.context.extensionUri, 'media')],
        };
        view.webview.html = this.html(view.webview);

        view.webview.onDidReceiveMessage(
            (msg: unknown) => {
                void this.onMessage(msg as InboundMessage);
            },
            undefined,
            this.context.subscriptions,
        );

        void this.rescopeToActiveEditor();
    }

    /** Ambient binding: point the pill at the active editor's file, if any. */
    public async rescopeToActiveEditor(): Promise<void> {
        const editor = vscode.window.activeTextEditor;
        if (!editor) {
            // No active editor: render a minimal "No file selected" card that
            // STILL carries the scope switcher, so the user can type to re-scope
            // instead of staring at a blank pill with no escape hatch.
            this.currentScope = { kind: 'literal', value: '' };
            // No real literal scope here (this is the empty card), so drop any
            // prior harvest — a later Copy must not export a stale set.
            this.literalAccum = null;
            // Bump the generation so any in-flight rescope drops its stale render
            // and doesn't paint over this no-editor card.
            this.rescopeGen++;
            this.postMessage({ type: 'render', vm: noEditorViewModel(this.gateway.lspState()) });
            return;
        }
        // A file the snapshot can't speak to: either it lives outside every open
        // workspace folder, or it isn't a real on-disk file (git/output/etc.
        // virtual schemes). Bind to a dedicated out-of-workspace scope so the
        // pill shows an honest empty state instead of all-zeros repo numbers.
        const folder = vscode.workspace.getWorkspaceFolder(editor.document.uri);
        if (folder === undefined || editor.document.uri.scheme !== 'file') {
            await this.rescope({
                kind: 'out-of-workspace',
                value: editor.document.uri.fsPath,
            });
            return;
        }
        const rel = vscode.workspace.asRelativePath(editor.document.uri);
        await this.rescope({ kind: 'file', value: rel });
    }

    private async rescope(scope: Scope): Promise<void> {
        this.currentScope = scope;
        const gen = ++this.rescopeGen;
        const vm = await assembleViewModel(this.gateway, scope);
        // Drop a stale render: a newer rescope started while we awaited.
        if (gen !== this.rescopeGen) {
            return;
        }
        // Seed/clear the literal harvest accumulator so a later Copy exports the
        // full loaded set. Stale renders are dropped above, so the newest rescope
        // owns the accumulator.
        this.literalAccum =
            scope.kind === 'literal'
                ? {
                    query: scope.value,
                    groups: vm.literalGroups,
                    total: vm.literalTotal,
                    hasMore: vm.literalHasMore,
                }
                : null;
        this.postMessage({ type: 'render', vm });
    }

    /**
     * Route a scope-switcher submission. File/literal inputs rescope directly.
     * A `symbol` input is RESOLVED via `gateway.body()` (clean identity — `find`
     * only returns context strings): one body → symbol scope; many → an inline
     * candidate list in the webview; zero → a literal-occurrence count probe and
     * a "show as a pill?" prompt. Resolution lives here (the panel owns the
     * gateway); the webview just renders what we post.
     */
    private async handleRescopeInput(raw: string): Promise<void> {
        const scope = parseScopeInput(raw);
        if (scope.kind !== 'symbol') {
            await this.rescope(scope);
            return;
        }

        const query = scope.value;
        let resolution: ReturnType<typeof resolveSymbol>;
        try {
            const resp = await this.gateway.body(query);
            resolution = resolveSymbol(resp.bodies);
        } catch {
            // Resolution failed (e.g. snapshot warming): fall back to a direct
            // symbol rescope rather than a dead end.
            await this.rescope(scope);
            return;
        }

        if (resolution.kind === 'one') {
            await this.rescope({ kind: 'symbol', value: query });
            return;
        }
        if (resolution.kind === 'many') {
            this.postMessage({ type: 'candidates', query, items: resolution.candidates });
            return;
        }

        // Zero bodies: probe how many literal occurrences exist so the prompt can
        // be honest about the count, then offer "show as a pill?".
        let literalCount = 0;
        try {
            const lit = await this.gateway.literal(query, { count_only: true });
            literalCount = lit.literal_matches?.total ?? 0;
        } catch {
            // Probe failed (e.g. snapshot warming): literalCount stays 0 — the
            // "show as a pill?" prompt is still honest about an unknown count.
        }
        this.postMessage({ type: 'noSymbol', query, literalCount });
    }

    /**
     * Fetch the next literal occurrence page and post an append message. The
     * webview merges + de-dups (file/line). Offset/limit paging mirrors the
     * `OccurrencePage` the LSP returns on the literal request.
     */
    private async loadMoreLiteral(query: string, offset: number): Promise<void> {
        if (!query) {
            return;
        }
        let resp;
        try {
            resp = await this.gateway.literal(query, {
                group_by_file: true,
                offset,
                limit: LITERAL_PAGE_LIMIT,
            });
        } catch {
            return;
        }
        const matches = resp.literal_matches;
        const groups = groupOccurrencesByFile(
            (matches?.occurrences ?? []).map((o) => ({ file: o.file, line: o.line })),
        );
        const hasMore = !!matches?.page?.has_more;
        // Fold this page into the accumulator so Copy Agent Context harvests the
        // full set, not just the first page. Guard on the query matching the
        // current literal scope so a stale Load-more cannot pollute a re-scoped
        // pill's harvest.
        if (this.literalAccum && this.literalAccum.query === query) {
            this.literalAccum.groups = mergeLiteralGroups(this.literalAccum.groups, groups);
            this.literalAccum.hasMore = hasMore;
        }
        this.postMessage({
            type: 'appendLiteral',
            groups,
            hasMore,
            nextOffset: hasMore ? matches?.page?.next_offset ?? null : null,
        });
    }

    /**
     * Single copy path shared by the webview CTA (`copyAgentContext` message)
     * and the `loctree.copyAgentContext` command. Assembles the VM for `scope`,
     * renders the agent-context markdown (task from the explicit hint or
     * inferred from IDE signals), and writes it to the clipboard. Returns the
     * markdown so callers can surface their own confirmation.
     */
    private async buildAndCopy(scope: Scope, taskHint?: string): Promise<string> {
        // Overlay the accumulated literal harvest so the copy exports every page
        // the user loaded via Load-more, not just assembleViewModel's first page.
        const vm = applyLiteralHarvest(
            await assembleViewModel(this.gateway, scope),
            this.literalAccum,
        );
        const task = (taskHint ?? '').trim() || inferTask(this.getTaskSignal());
        const markdown = buildAgentContextMarkdown(vm, task);
        await vscode.env.clipboard.writeText(markdown);
        return markdown;
    }

    /**
     * Command entrypoint (`loctree.copyAgentContext`): copy the agent context
     * for the current scope to the clipboard and confirm. Unlike the old
     * binding (which only rescoped), this actually copies.
     */
    public async copyAgentContextForCurrent(): Promise<void> {
        await this.buildAndCopy(this.currentScope);
        void vscode.window.showInformationMessage('Copied Agent Context');
    }

    private postMessage(message: Record<string, unknown>): void {
        void this.view?.webview.postMessage({
            ...message,
            loctreeChannel: this.webviewChannel,
        });
    }

    private async onMessage(msg: InboundMessage): Promise<void> {
        // SECURITY: enforce the inbound allowlist before any handler runs.
        const type = typeof msg?.type === 'string' ? msg.type : '';
        if (!ALLOWED_MESSAGE_TYPES.has(type)) {
            return;
        }

        switch (type) {
            case 'rescope': {
                await this.handleRescopeInput(String(msg.query ?? ''));
                return;
            }
            case 'rescopeCandidate': {
                // A candidate row was clicked: pin symbol scope, hint the file so
                // bodyRanked previews the right definition.
                const symbol = String(msg.symbol ?? '');
                if (!symbol) {
                    return;
                }
                // Thread the chosen file through so bodyRanked previews the right
                // definition (disambiguation); empty file → no hint.
                const file = typeof msg.file === 'string' && msg.file ? msg.file : undefined;
                await this.rescope({ kind: 'symbol', value: symbol, file });
                return;
            }
            case 'showAsLiteral': {
                // "Show as a pill?" on the no-symbol prompt → literal scope.
                const query = String(msg.query ?? '');
                if (!query) {
                    return;
                }
                await this.rescope({ kind: 'literal', value: query });
                return;
            }
            case 'loadMoreLiteral': {
                await this.loadMoreLiteral(
                    String(msg.query ?? ''),
                    typeof msg.offset === 'number' ? msg.offset : 0,
                );
                return;
            }
            case 'copyAgentContext': {
                await this.buildAndCopy(this.currentScope, String(msg.task ?? ''));
                this.postMessage({ type: 'copied' });
                return;
            }
            case 'openFile': {
                const file = String(msg.file ?? '');
                if (!file) {
                    return;
                }
                // Delegate to the existing command: it resolves the path against
                // the workspace, blocks escapes, and opens with caret support.
                // `navigateToFile` accepts a `{ filePath, line }` payload
                // (commands.ts → extractLocationTarget); pass the line for the
                // literal occurrence chips so the caret lands on the match.
                const line = typeof msg.line === 'number' ? msg.line : undefined;
                await vscode.commands.executeCommand('loctree.navigateToFile', {
                    filePath: file,
                    line,
                });
                return;
            }
            case 'showFullBody': {
                // `loctree.showBody` takes the symbol name as its first arg
                // (string seed), then prompts for confirmation (commands.ts).
                await vscode.commands.executeCommand(
                    'loctree.showBody',
                    String(msg.symbol ?? ''),
                );
                return;
            }
            case 'openProjectFile': {
                // Out-of-workspace affordance: surface the Explorer so the user
                // can open a file that IS part of an open Loctree project.
                await vscode.commands.executeCommand('workbench.view.explorer');
                return;
            }
            case 'scanWorkspace': {
                // Not-in-snapshot affordance: `loctree.initialize` starts the LSP
                // client (or triggers a refresh if already running), re-scanning
                // the workspace so the file gets indexed.
                await vscode.commands.executeCommand('loctree.initialize');
                return;
            }
        }
    }

    private html(webview: vscode.Webview): string {
        const nonce = getNonce();
        const scriptUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this.context.extensionUri, 'media', 'contextPill.js'),
        );
        const styleUri = webview.asWebviewUri(
            vscode.Uri.joinPath(this.context.extensionUri, 'media', 'contextPill.css'),
        );

        const csp = [
            `default-src 'none'`,
            `style-src ${webview.cspSource}`,
            `script-src 'nonce-${nonce}'`,
            `img-src ${webview.cspSource}`,
        ].join('; ');

        return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta http-equiv="Content-Security-Policy" content="${csp}">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <link href="${styleUri}" rel="stylesheet">
    <title>Loctree Context</title>
</head>
<body>
    <div id="pill" data-state="loading" data-channel="${this.webviewChannel}"></div>
    <script nonce="${nonce}" src="${scriptUri}"></script>
</body>
</html>`;
    }
}

/** 32 random alphanumeric chars, regenerated per render (CSP nonce). */
function getNonce(): string {
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    let text = '';
    for (let i = 0; i < 32; i++) {
        text += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return text;
}
