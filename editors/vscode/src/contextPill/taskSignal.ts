import * as vscode from 'vscode';

/** Changed files from the built-in git extension (WIP diff): working-tree +
 * index changes, workspace-relative, capped. Empty array if git is unavailable
 * — callers must treat empty as "no signal" (never fabricate a task). */
export function getChangedFiles(): string[] {
    try {
        const git = vscode.extensions.getExtension('vscode.git')?.exports?.getAPI?.(1);
        // Pick the repo that owns the active file (or the opened workspace
        // folder) — NOT just repositories[0]. Multiple repos/worktrees can be
        // open at once (e.g. a sibling worktree with unrelated WIP), and [0]
        // would leak the wrong repo's changes into the agent task.
        const repos: Array<{ rootUri: vscode.Uri; state: { workingTreeChanges?: Array<{ uri: vscode.Uri }>; indexChanges?: Array<{ uri: vscode.Uri }> } }> =
            git?.repositories ?? [];
        if (repos.length === 0) return [];
        const owners = [
            vscode.window.activeTextEditor?.document.uri.fsPath,
            vscode.workspace.workspaceFolders?.[0]?.uri.fsPath,
        ].filter((p): p is string => typeof p === 'string');
        const repo =
            owners.map((p) => repos.find((r) => p.startsWith(r.rootUri.fsPath))).find((r) => r !== undefined) ??
            repos[0];
        const changes: Array<{ uri: vscode.Uri }> = [
            ...(repo.state.workingTreeChanges ?? []),
            ...(repo.state.indexChanges ?? []),
        ];
        return changes
            .map((c) => vscode.workspace.asRelativePath(c.uri))
            .slice(0, 20);
    } catch {
        return [];
    }
}
