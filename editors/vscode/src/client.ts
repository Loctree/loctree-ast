/**
 * Language Client for loctree-lsp
 *
 * Sets up the connection between VSCode and the loctree-lsp binary.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

import * as path from 'path';
import * as os from 'os';
import * as vscode from 'vscode';
import * as fs from 'fs';
import * as https from 'https';
import * as crypto from 'crypto';
import { execFile, spawn } from 'child_process';
import type { IncomingMessage } from 'http';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
    TransportKind,
} from 'vscode-languageclient/node';

/**
 * Find the loctree-lsp binary
 *
 * Search order:
 * 1. Valid user-configured path in settings (file or directory override)
 * 2. Bundled binary in extension
 * 3. Verified download/cache
 * 4. Preferred user install (~/.local/bin)
 * 5. System PATH (last-resort developer fallback)
 */
/**
 * Maximum number of HTTP redirects to follow before aborting. Mirrors the
 * JetBrains ReleaseDownloader.MAX_REDIRECTS so a redirect loop can never hang
 * the download indefinitely.
 */
const MAX_REDIRECTS = 5;

/**
 * Per-request timeout for a download. A hung/half-open connection that never
 * delivers data (or a server that accepts but never responds) is aborted after
 * this many milliseconds. Comparable to the JetBrains read timeout.
 */
const DOWNLOAD_TIMEOUT_MS = 30_000;

/**
 * Release repository the runtime auto-download falls back to when
 * `loctree.downloadBaseUrl` is unset or is not a usable http(s) URL.
 */
const DEFAULT_DOWNLOAD_REPO = 'https://github.com/Loctree/loctree-release';

/** Which of the five resolution steps produced the runtime that was launched. */
export type RuntimeResolutionSource =
    | 'configured'
    | 'bundled'
    | 'cache'
    | 'downloaded'
    | 'preferred-install'
    | 'path';

/** A resolved loctree-lsp runtime: exact path, how it was found, its
 *  `--version` identity, and any PATH-shadowing warning to surface. */
export interface ResolvedServerRuntime {
    command: string;
    source: RuntimeResolutionSource;
    version: string;
    warning?: string;
}

/** PATH-fallback outcome before the version probe: the chosen command plus the
 *  earlier-on-PATH binary it overrode, when one exists. */
interface PathRuntimeCandidate {
    command: string;
    source: 'preferred-install' | 'path';
    shadowedPath?: string;
}

/** Prefix of the warning raised when a stale PATH entry shadows ~/.local/bin. */
const PATH_SHADOW_WARNING = 'Loctree runtime PATH shadowing:';

/** Platform-correct loctree-lsp filename (`.exe` on Windows). */
function binaryNameForPlatform(): string {
    return process.platform === 'win32' ? 'loctree-lsp.exe' : 'loctree-lsp';
}

/**
 * Canonical path of `candidate` when it is an existing, executable regular
 * file; null otherwise. Symlinks are resolved so the identity is unambiguous.
 */
function executableFile(candidate: string): string | null {
    try {
        if (!fs.statSync(candidate).isFile()) return null;
        fs.accessSync(candidate, process.platform === 'win32' ? fs.constants.F_OK : fs.constants.X_OK);
        return fs.realpathSync(candidate);
    } catch {
        return null;
    }
}

/** Resolve the PATH fallback to an exact executable, preferring the canonical user install. */
export function resolvePathRuntime(
    binaryName = binaryNameForPlatform(),
    searchPath = process.env.PATH ?? '',
    home = os.homedir(),
): PathRuntimeCandidate {
    const safeBinaryName = binaryName === binaryNameForPlatform()
        ? binaryName
        : binaryNameForPlatform();
    const preferredRoot = safeChildPath(home, '.local', 'bin');
    const preferredCandidate = preferredRoot ? safeNamedChild(preferredRoot, safeBinaryName) : null;
    const preferred = preferredCandidate ? executableFile(preferredCandidate) : null;
    const pathMatch = searchPath
        .split(path.delimiter)
        .filter(Boolean)
        .map((entry) => safeNamedChild(entry, safeBinaryName))
        .map((entry) => entry ? executableFile(entry) : null)
        .find((entry): entry is string => entry !== null);

    if (preferred) {
        return {
            command: preferred,
            source: 'preferred-install',
            shadowedPath: pathMatch && pathMatch !== preferred ? pathMatch : undefined,
        };
    }
    return { command: pathMatch ?? safeBinaryName, source: 'path' };
}

/**
 * Run `<command> --version` (fixed argv, never a shell, killed after 5s) and
 * return its first line, or `'version unavailable'` when the probe cannot run.
 */
async function probeRuntimeVersion(command: string): Promise<string> {
    const executableCommand = executableFile(command)
        ?? (command === binaryNameForPlatform() ? command : null);
    if (!executableCommand) {
        return 'version unavailable';
    }
    return await new Promise<string>((resolve) => {
        // `executableCommand` is either a canonical executable file or the exact
        // platform binary name. `execFile` takes a fixed argv and by construction
        // never goes through a shell; the timeout replaces a hand-rolled kill timer.
        execFile(
            executableCommand,
            ['--version'],
            { timeout: 5_000, windowsHide: true },
            (error, stdout, stderr) => {
                const output = `${stdout ?? ''}${stderr ?? ''}`.trim();
                if (error && !output) {
                    resolve('version unavailable');
                    return;
                }
                resolve(output.split(/\r?\n/, 1)[0] || 'version unavailable');
            },
        );
    });
}

/**
 * Attach identity to a resolved command: probes its version and, when a PATH
 * entry shadows it, probes that too and composes the shadowing warning.
 */
async function runtimeIdentity(
    command: string,
    source: RuntimeResolutionSource,
    shadowedPath?: string,
): Promise<ResolvedServerRuntime> {
    const exactCommand = executableFile(command) ?? command;
    const version = await probeRuntimeVersion(exactCommand);
    if (!shadowedPath) return { command: exactCommand, source, version };

    const shadowedVersion = await probeRuntimeVersion(shadowedPath);
    return {
        command: exactCommand,
        source,
        version,
        warning: `${PATH_SHADOW_WARNING} ${shadowedPath} (${shadowedVersion}) appears first on PATH, ` +
            `but the preferred install ${exactCommand} (${version}) will be used. Remove or reorder the stale entry.`,
    };
}

/** Bare LSP asset marker name for the host platform; null on unsupported hosts. */
export function assetNameForPlatform(): string | null {
    const target = releaseTargetForPlatform();
    return target ? lspAssetNameForTarget(target) : null;
}

/** Rust target triple for the host platform, or null when no release is built for it. */
export function releaseTargetForPlatform(): string | null {
    const platform = process.platform;
    const arch = process.arch;

    if (platform === 'darwin') {
        if (arch === 'arm64') return 'aarch64-apple-darwin';
        if (arch === 'x64') return 'x86_64-apple-darwin';
    }
    if (platform === 'linux') {
        if (arch === 'arm64') return 'aarch64-unknown-linux-gnu';
        if (arch === 'x64') return 'x86_64-unknown-linux-gnu';
    }
    if (platform === 'win32') {
        if (arch === 'x64') return 'x86_64-pc-windows-msvc';
    }

    return null;
}

/** Asset marker name for a target triple; also names the cache version marker file. */
export function lspAssetNameForTarget(target: string): string {
    return `loctree-lsp-${target}`;
}

/** Strip a leading `v` and surrounding whitespace from a version string. */
function normalizeVersion(raw: string): string {
    return raw.trim().replace(/^v/, '');
}

/** Release tag used when no `loctree.downloadTag` is configured: the extension's
 *  own version, so the downloaded runtime stays in lockstep with the plugin. */
function defaultDownloadTag(extensionVersion: string): string {
    return `v${normalizeVersion(extensionVersion || '0.0.0')}`;
}

/** Tarball asset name inside a release: `loctree-<version>-<target>.tar.gz`. */
function archiveAssetName(version: string, target: string): string {
    return `loctree-${normalizeVersion(version)}-${target}.tar.gz`;
}

/** Top-level directory the release tarball unpacks into. */
function archiveRootDir(version: string, target: string): string {
    return `loctree-${normalizeVersion(version)}-${target}`;
}

/**
 * Normalize a configured repo URL (drop `git+`, `.git`, trailing slash).
 * Returns null for anything that is not http(s), so the default repo wins.
 */
function normalizeRepoUrl(raw?: string): string | null {
    if (!raw) return null;
    let url = raw.trim();
    if (url.startsWith('git+')) {
        url = url.slice(4);
    }
    if (url.endsWith('.git')) {
        url = url.slice(0, -4);
    }
    if (url.endsWith('/')) {
        url = url.slice(0, -1);
    }
    if (!url.startsWith('http')) {
        return null;
    }
    return url;
}

/**
 * Build the GitHub release asset URL. `latest` routes through
 * `/releases/latest/download/`; any other tag through `/releases/download/<tag>/`.
 */
export function releaseDownloadUrl(
    repoBase: string,
    tag: string,
    assetName: string
): string {
    const releaseBase = repoBase.includes('/releases')
        ? repoBase.replace(/\/releases\/?$/, '/releases')
        : `${repoBase}/releases`;
    if (tag === 'latest') {
        return `${releaseBase}/latest/download/${assetName}`;
    }
    return `${releaseBase}/download/${tag}/${assetName}`;
}

/** Path of the `.sha256` sidecar that guards a downloaded or cached file. */
function sidecarPath(binaryPath: string): string {
    return `${binaryPath}.sha256`;
}

/** Drop one layer of matching quotes a user may have pasted into a settings path. */
function stripPathQuotes(raw: string): string {
    const trimmed = raw.trim();
    if (trimmed.length >= 2) {
        const first = trimmed[0];
        const last = trimmed[trimmed.length - 1];
        if ((first === '"' && last === '"') || (first === "'" && last === "'")) {
            return trimmed.slice(1, -1).trim();
        }
    }
    return trimmed;
}

/** True when `candidate` resolves inside `root` (equal counts as inside). */
function isInsidePath(root: string, candidate: string): boolean {
    const normalizedRoot = path.normalize(root);
    const normalizedCandidate = path.normalize(candidate);
    const relativePath = path.relative(normalizedRoot, normalizedCandidate);
    return (
        relativePath === '' ||
        (!relativePath.startsWith('..') && !path.isAbsolute(relativePath))
    );
}

/** Split a relative path into segments, rejecting absolute paths, NUL bytes and
 *  any `..` traversal. Null means the input is not safe to join onto a root. */
function safeRelativeParts(raw: string): string[] | null {
    if (!raw || path.isAbsolute(raw) || raw.includes('\0')) {
        return null;
    }
    const parts = raw.split(/[\\/]+/).filter((part) => part !== '' && part !== '.');
    if (parts.some((part) => part === '..')) {
        return null;
    }
    return parts;
}

/** Join `segments` under `root`, returning null when a segment is unsafe or the
 *  result would escape the root. */
function safeChildPath(root: string, ...segments: string[]): string | null {
    const parts: string[] = [];
    for (const segment of segments) {
        const segmentParts = safeRelativeParts(segment);
        if (!segmentParts) {
            return null;
        }
        parts.push(...segmentParts);
    }

    const normalizedRoot = path.normalize(root);
    const child = path.normalize(vscode.Uri.joinPath(vscode.Uri.file(normalizedRoot), ...parts).fsPath);
    return isInsidePath(normalizedRoot, child) ? child : null;
}

/** {@link safeChildPath} for a single plain filename (no separators, no traversal). */
function safeNamedChild(root: string, name: string): string | null {
    if (!/^[A-Za-z0-9._-]+$/.test(name) || path.basename(name) !== name) {
        return null;
    }
    return safeChildPath(root, name);
}

/** {@link safeChildPath} that throws instead of returning null — an escape attempt
 *  aborts the download rather than degrading into an unchecked path. */
function requireSafeChildPath(root: string, ...segments: string[]): string {
    const child = safeChildPath(root, ...segments);
    if (!child) {
        throw new Error(`Refusing to resolve path outside trusted root: ${segments.join('/')}`);
    }
    return child;
}

/** {@link safeNamedChild} that throws on an unsafe filename. */
function requireSafeNamedChild(root: string, name: string): string {
    const child = safeNamedChild(root, name);
    if (!child) {
        throw new Error(`Refusing unsafe path segment: ${name}`);
    }
    return child;
}

/** Expand a leading `~` in a user-configured path; null when the remainder would
 *  escape the home directory. */
function expandHome(rawPath: string): string | null {
    if (rawPath === '~') {
        return os.homedir();
    }
    if (rawPath.startsWith('~/') || rawPath.startsWith('~\\')) {
        return safeChildPath(os.homedir(), rawPath.slice(2));
    }
    return rawPath;
}

/**
 * Turn a configured `loctree.serverPath` (a file OR a directory) into an existing
 * binary path, chmod +x on unix. Null when nothing usable is at that location.
 */
function resolveExistingBinary(candidate: string, binaryName: string): string | null {
    const normalized = expandHome(stripPathQuotes(candidate));
    if (!normalized || !path.isAbsolute(normalized)) {
        return null;
    }

    const candidates = [path.normalize(normalized)];
    try {
        if (fs.existsSync(normalized) && fs.statSync(normalized).isDirectory()) {
            const nestedBinary = safeNamedChild(normalized, binaryName);
            if (nestedBinary) {
                candidates.unshift(nestedBinary);
            }
        } else if (path.basename(normalized) !== binaryName) {
            const nestedBinary = safeNamedChild(normalized, binaryName);
            if (nestedBinary) {
                candidates.push(nestedBinary);
            }
        }
    } catch {
        // Keep the raw path candidate below; stat errors should not abort startup.
    }

    for (const possible of candidates) {
        try {
            if (fs.existsSync(possible) && fs.statSync(possible).isFile()) {
                if (process.platform !== 'win32') {
                    fs.chmodSync(possible, 0o755);
                }
                return possible;
            }
        } catch {
            // Try the next candidate.
        }
    }

    return null;
}

/** First whitespace-delimited token of a `.sha256` sidecar, lowercased; null when
 *  the sidecar is missing, empty or unreadable. */
function expectedShaFromSidecar(shaPath: string): string | null {
    try {
        const contents = fs.readFileSync(shaPath, 'utf8').trim();
        if (!contents) return null;
        const first = contents.split(/\s+/)[0];
        return first ? first.toLowerCase() : null;
    } catch {
        return null;
    }
}

/** Write the `<binary>.sha256` sidecar that later cache reuse is verified against. */
async function writeBinarySidecar(binaryPath: string): Promise<void> {
    fs.writeFileSync(sidecarPath(binaryPath), `${await computeSha256(binaryPath)}  ${path.basename(binaryPath)}\n`);
}

/** Unpack a release tarball with the system `tar`, rejecting on a non-zero exit. */
async function extractTarball(archivePath: string, extractDir: string): Promise<void> {
    await new Promise<void>((resolve, reject) => {
        const child = spawn('tar', ['-xzf', archivePath, '-C', extractDir], {
            stdio: 'ignore',
        });
        child.on('error', reject);
        child.on('close', (code) => {
            if (code === 0) {
                resolve();
            } else {
                reject(new Error(`tar -xzf failed with exit code ${code}`));
            }
        });
    });
}

/**
 * Download, verify, unpack and install one release archive. On any failure the
 * half-installed binary and its sidecar are removed; the temporary archive and
 * extract dir are cleaned up either way.
 */
async function downloadReleaseArchive(
    url: string,
    archivePath: string,
    extractDir: string,
    extractedBinaryPath: string,
    binaryPath: string
): Promise<void> {
    await downloadFile(url, archivePath);

    fs.rmSync(extractDir, { recursive: true, force: true });
    fs.mkdirSync(extractDir, { recursive: true });

    try {
        await extractTarball(archivePath, extractDir);
        if (!fs.existsSync(extractedBinaryPath) || !fs.statSync(extractedBinaryPath).isFile()) {
            throw new Error(`Archive did not contain ${path.relative(extractDir, extractedBinaryPath)}`);
        }

        fs.copyFileSync(extractedBinaryPath, binaryPath);
        if (process.platform !== 'win32') {
            fs.chmodSync(binaryPath, 0o755);
        }
        await writeBinarySidecar(binaryPath);
    } catch (err) {
        if (fs.existsSync(binaryPath)) fs.unlinkSync(binaryPath);
        if (fs.existsSync(sidecarPath(binaryPath))) fs.unlinkSync(sidecarPath(binaryPath));
        throw err;
    } finally {
        fs.rmSync(extractDir, { recursive: true, force: true });
        if (fs.existsSync(archivePath)) fs.unlinkSync(archivePath);
        if (fs.existsSync(sidecarPath(archivePath))) fs.unlinkSync(sidecarPath(archivePath));
    }
}

/**
 * Re-hash a cached binary against its `<binary>.sha256` sidecar.
 *
 * The version marker alone is not an integrity signal — it only records which
 * extension version last wrote the cache. A truncated/corrupted/tampered binary
 * can still carry a matching marker. Mirrors the JetBrains
 * CachedBinaryVerifier: trust the cache only when the binary still matches the
 * sidecar written after a verified download.
 */
async function isCachedBinaryVerified(binaryPath: string): Promise<boolean> {
    if (!fs.existsSync(binaryPath)) return false;
    const shaPath = sidecarPath(binaryPath);
    const expectedSha = expectedShaFromSidecar(shaPath);
    if (!expectedSha) return false;
    try {
        const actualSha = (await computeSha256(binaryPath)).toLowerCase();
        return actualSha === expectedSha;
    } catch {
        return false;
    }
}

/**
 * Fetch `url` plus its `.sha256` sidecar and refuse the result unless the hashes
 * match, so a corrupted or swapped asset never survives on disk.
 */
async function downloadFile(url: string, destPath: string): Promise<void> {
    const assetName = path.basename(destPath);
    const shaUrl = `${url}.sha256`;
    const shaPath = sidecarPath(destPath);

    console.log(`[Loctree] Downloading ${assetName}...`);
    await downloadRawFile(url, destPath);

    try {
        console.log(`[Loctree] Verifying checksum for ${assetName}...`);
        await downloadRawFile(shaUrl, shaPath);
        const expectedSha = expectedShaFromSidecar(shaPath);
        if (!expectedSha) {
            throw new Error(`Empty or unreadable checksum sidecar for ${assetName}`);
        }
        const actualSha = (await computeSha256(destPath)).toLowerCase();

        if (expectedSha !== actualSha) {
            throw new Error(`Checksum mismatch for ${assetName}. Expected: ${expectedSha}, Actual: ${actualSha}`);
        }
        console.log(`[Loctree] Checksum verified for ${assetName}.`);
        // Keep the sidecar so the cached binary can be re-hashed before reuse;
        // the version marker alone is not an integrity guarantee.
    } catch (err) {
        if (fs.existsSync(destPath)) fs.unlinkSync(destPath);
        if (fs.existsSync(shaPath)) fs.unlinkSync(shaPath);
        throw new Error(`Failed to verify ${assetName}: ${err instanceof Error ? err.message : String(err)}`, {
            cause: err,
        });
    }
}

/**
 * One HTTPS GET to a file, following up to {@link MAX_REDIRECTS} redirects and
 * aborting after {@link DOWNLOAD_TIMEOUT_MS}. Partial files are unlinked before
 * the promise rejects.
 */
async function downloadRawFile(
    url: string,
    destPath: string,
    redirectsLeft: number = MAX_REDIRECTS
): Promise<void> {
    await new Promise<void>((resolve, reject) => {
        // Drop any partial destination file when the attempt fails, mirroring
        // the unlink-on-failure cleanup in downloadFile().
        const failWith = (error: Error) => {
            try {
                if (fs.existsSync(destPath)) fs.unlinkSync(destPath);
            } catch {
                // Best-effort cleanup; surface the original error regardless.
            }
            reject(error);
        };

        const request = https.get(
            url,
            { headers: { 'User-Agent': 'loctree-vscode' } },
            (response: IncomingMessage) => {
                if (response.statusCode && response.statusCode >= 300 && response.statusCode < 400) {
                    const location = response.headers.location;
                    if (!location) {
                        failWith(new Error(`Redirect without location for ${url}`));
                        return;
                    }
                    if (redirectsLeft <= 0) {
                        failWith(new Error(`Too many redirects for ${url}`));
                        return;
                    }
                    response.resume();
                    downloadRawFile(location, destPath, redirectsLeft - 1)
                        .then(resolve)
                        .catch(reject);
                    return;
                }

                if (response.statusCode && response.statusCode >= 400) {
                    failWith(new Error(`Download failed (${response.statusCode}) for ${url}`));
                    return;
                }

                const file = fs.createWriteStream(destPath);
                response.pipe(file);
                file.on('finish', () => {
                    file.close();
                    resolve();
                });
                file.on('error', failWith);
            }
        );

        // Abort a hung/half-open connection so the promise can never wait
        // forever. destroy() emits 'error', which routes through failWith below.
        request.setTimeout(DOWNLOAD_TIMEOUT_MS, () => {
            request.destroy(new Error(`Download timed out for ${url}`));
        });
        request.on('error', failWith);
    });
}

/** Streaming SHA-256 of a file, as lowercase hex. */
async function computeSha256(filePath: string): Promise<string> {
    return new Promise((resolve, reject) => {
        const hash = crypto.createHash('sha256');
        const stream = fs.createReadStream(filePath);
        stream.on('data', (data) => hash.update(data));
        stream.on('end', () => resolve(hash.digest('hex')));
        stream.on('error', (err) => reject(err));
    });
}

/**
 * Cache-or-download path for the runtime. Reuses the cached binary only when the
 * version marker matches AND it still hashes to its sidecar; returns null when
 * auto-download is off, the platform is unsupported, or the download failed —
 * the caller then falls through to PATH.
 */
async function ensureDownloadedBinary(
    context: vscode.ExtensionContext,
    outputChannel: vscode.OutputChannel
): Promise<{ path: string; source: 'cache' | 'downloaded' } | null> {
    const config = vscode.workspace.getConfiguration('loctree');
    if (!config.get<boolean>('autoDownload', true)) {
        return null;
    }

    const target = releaseTargetForPlatform();
    if (!target) {
        outputChannel.appendLine('No compatible loctree-lsp asset for this platform');
        return null;
    }
    const lspAssetName = lspAssetNameForTarget(target);

    const storageDir = path.normalize(context.globalStorageUri.fsPath);
    const binDir = requireSafeChildPath(storageDir, 'bin');
    fs.mkdirSync(binDir, { recursive: true });

    const binaryName = binaryNameForPlatform();
    const binaryPath = requireSafeNamedChild(binDir, binaryName);
    const markerPath = requireSafeNamedChild(binDir, `${lspAssetName}.version`);
    const extensionVersion = context.extension.packageJSON?.version ?? '0.0.0';

    if (fs.existsSync(binaryPath) && fs.existsSync(markerPath)) {
        const marker = fs.readFileSync(markerPath, 'utf-8').trim();
        // Trust the cache only when BOTH the version marker matches AND the
        // binary still hashes to its `.sha256` sidecar. A matching marker over a
        // corrupted/tampered binary must not be used — re-download instead.
        if (marker === extensionVersion && (await isCachedBinaryVerified(binaryPath))) {
            return { path: binaryPath, source: 'cache' };
        }
        outputChannel.appendLine(
            'Cached loctree-lsp failed integrity re-check (version marker and/or sha256 sidecar mismatch); re-downloading'
        );
    }

    const configuredBase = config.get<string>('downloadBaseUrl')?.trim();
    const repoUrl = normalizeRepoUrl(configuredBase) || DEFAULT_DOWNLOAD_REPO;
    const tag = config.get<string>('downloadTag')?.trim() || defaultDownloadTag(extensionVersion);
    const archiveVersion = tag === 'latest' ? extensionVersion : tag;
    const assetName = archiveAssetName(archiveVersion, target);
    const archivePath = requireSafeNamedChild(binDir, assetName);
    const extractDir = requireSafeNamedChild(binDir, `${assetName}.extract`);
    const extractedBinaryPath = requireSafeChildPath(
        extractDir,
        archiveRootDir(archiveVersion, target),
        'bin',
        binaryName
    );
    const url = releaseDownloadUrl(repoUrl, tag, assetName);

    outputChannel.appendLine(`Downloading loctree-lsp from ${url}`);
    try {
        await downloadReleaseArchive(url, archivePath, extractDir, extractedBinaryPath, binaryPath);
        fs.writeFileSync(markerPath, extensionVersion);
        outputChannel.appendLine(`loctree-lsp downloaded to ${binaryPath}`);
        return { path: binaryPath, source: 'downloaded' };
    } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        outputChannel.appendLine(`Failed to download loctree-lsp: ${message}`);
        return null;
    }
}

/**
 * Resolve the loctree-lsp binary path, downloading it if needed.
 */
export async function resolveServerBinary(
    context: vscode.ExtensionContext,
    outputChannel: vscode.OutputChannel
): Promise<ResolvedServerRuntime> {
    const config = vscode.workspace.getConfiguration('loctree');
    const configuredPath = config.get<string>('serverPath');
    const binaryName = binaryNameForPlatform();

    if (configuredPath && configuredPath.trim() !== '') {
        const configuredBinary = resolveExistingBinary(configuredPath, binaryName);
        if (configuredBinary) {
            return runtimeIdentity(configuredBinary, 'configured');
        }
        outputChannel.appendLine(
            `Configured loctree.serverPath did not point to a usable ${binaryName}: ${configuredPath}; trying bundled/cache/download/PATH`
        );
    }

    const bundledPath = requireSafeChildPath(context.extensionPath, 'bin', binaryName);
    if (fs.existsSync(bundledPath)) {
        return runtimeIdentity(bundledPath, 'bundled');
    }

    const downloaded = await ensureDownloadedBinary(context, outputChannel);
    if (downloaded) {
        return runtimeIdentity(downloaded.path, downloaded.source);
    }

    const fallback = resolvePathRuntime(binaryName);
    return runtimeIdentity(fallback.command, fallback.source, fallback.shadowedPath);
}

/**
 * Create the language client
 */
export function createLanguageClient(
    context: vscode.ExtensionContext,
    outputChannel: vscode.OutputChannel,
    serverCommand: string,
    workspaceRoot: string
): LanguageClient {
    outputChannel.appendLine(`Using loctree-lsp binary: ${serverCommand}`);
    outputChannel.appendLine(`Starting loctree-lsp with root: ${workspaceRoot}`);
    const config = vscode.workspace.getConfiguration('loctree');
    const rootArgs = ['--root', workspaceRoot];

    const serverOptions: ServerOptions = {
        run: {
            command: serverCommand,
            transport: TransportKind.stdio,
            args: rootArgs,
            options: {
                cwd: workspaceRoot,
            },
        },
        debug: {
            command: serverCommand,
            transport: TransportKind.stdio,
            args: ['--debug', ...rootArgs],
            options: {
                cwd: workspaceRoot,
            },
        },
    };

    const clientOptions: LanguageClientOptions = {
        initializationOptions: {
            diagnosticSeverity: config.get<string>('diagnosticSeverity', 'warning'),
            // Loctree code lenses are off by default to avoid clutter with the
            // language server's own lenses; opt in via `loctree.codeLens`.
            codeLens: config.get<boolean>('codeLens', false),
        },
        // Register for supported file types
        documentSelector: [
            { scheme: 'file', language: 'typescript' },
            { scheme: 'file', language: 'typescriptreact' },
            { scheme: 'file', language: 'javascript' },
            { scheme: 'file', language: 'javascriptreact' },
            { scheme: 'file', language: 'rust' },
            { scheme: 'file', language: 'python' },
            { scheme: 'file', language: 'go' },
        ],
        synchronize: {
            // Watch for .loctree folder changes
            fileEvents: vscode.workspace.createFileSystemWatcher('**/.loctree/**'),
        },
        outputChannel,
        traceOutputChannel: outputChannel,
    };

    return new LanguageClient(
        'loctree',
        'Loctree Language Server',
        serverOptions,
        clientOptions
    );
}

/**
 * Start the language client
 */
export async function startClient(client: LanguageClient): Promise<void> {
    await client.start();
}

/**
 * Stop the language client
 */
export async function stopClient(client: LanguageClient): Promise<void> {
    if (client.isRunning()) {
        await client.stop();
    }
}
