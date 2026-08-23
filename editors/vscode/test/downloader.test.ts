import assert from 'node:assert/strict';
import Module from 'node:module';
import test from 'node:test';

/**
 * Runtime download release-URL parity guard.
 *
 * The runtime downloader is a fallback for dev/unpacked installs and corrupted
 * bundles. It must request public `loctree-release` tarballs by exact tag, then
 * extract `<root>/bin/loctree-lsp`; otherwise auto-download can silently point
 * at private repos or asset names that releases never publish.
 *
 * `client.ts` does a top-level `import * as vscode` and pulls in
 * `vscode-languageclient/node`. Neither is loadable outside a real editor host:
 * `vscode` is editor-injected (only `@types/vscode` exists at compile time) and
 * the language client's runtime subclasses vscode API types. The parity seam
 * exercised here (assetNameForPlatform, releaseDownloadUrl) is pure and never
 * touches either, so we stub both on the module loader before importing the
 * client. This keeps the test runnable under `node --test` without an editor.
 */
type ModuleLoad = (request: string, parent: unknown, isMain: boolean) => unknown;
const STUBBED_MODULES = new Set(['vscode', 'vscode-languageclient/node']);
const moduleInternals = Module as unknown as { _load: ModuleLoad };
const originalLoad = moduleInternals._load;
moduleInternals._load = function patchedLoad(request, parent, isMain) {
    if (request === 'vscode') {
        const path = require('node:path') as typeof import('node:path');
        return {
            Uri: {
                file: (value: string) => ({ fsPath: value }),
                joinPath: (root: { fsPath: string }, ...segments: string[]) => ({
                    fsPath: path.join(root.fsPath, ...segments),
                }),
            },
        };
    }
    if (STUBBED_MODULES.has(request)) {
        return {};
    }
    return originalLoad.call(this, request, parent, isMain);
};

// eslint-disable-next-line @typescript-eslint/no-require-imports
const { assetNameForPlatform, lspAssetNameForTarget, releaseDownloadUrl, releaseTargetForPlatform, resolvePathRuntime } =
    require('../src/client') as typeof import('../src/client');

const REPO_BASE = 'https://github.com/Loctree/loctree-release';

const RELEASE_TARGETS = [
    'x86_64-unknown-linux-gnu',
    'aarch64-unknown-linux-gnu',
    'aarch64-apple-darwin',
    'x86_64-apple-darwin',
];

const FUTURE_TARGETS = [
    ...RELEASE_TARGETS,
    'x86_64-pc-windows-msvc',
];

test('releaseTargetForPlatform resolves the current platform to a release triple', () => {
    const target = releaseTargetForPlatform();
    if (target === null) {
        // Unsupported (platform, arch) is a valid outcome; nothing to assert.
        return;
    }
    assert.ok(
        FUTURE_TARGETS.includes(target),
        `releaseTargetForPlatform() returned an unexpected target: ${target}`,
    );
});

test('assetNameForPlatform keeps the bare loctree-lsp marker naming convention', () => {
    const asset = assetNameForPlatform();
    if (asset === null) {
        // Unsupported (platform, arch) is a valid outcome; nothing to assert.
        return;
    }
    assert.match(asset, /^loctree-lsp-/);
    assert.ok(
        FUTURE_TARGETS.map(lspAssetNameForTarget).includes(asset),
        `assetNameForPlatform() returned an unexpected name: ${asset}`,
    );
});

test('release target names produce loctree-lsp-* cache markers', () => {
    for (const target of RELEASE_TARGETS) {
        assert.equal(lspAssetNameForTarget(target), `loctree-lsp-${target}`);
    }
});

test('releaseDownloadUrl builds an exact-tag tarball path', () => {
    const url = releaseDownloadUrl(
        REPO_BASE,
        'v0.12.3',
        'loctree-0.12.3-aarch64-apple-darwin.tar.gz',
    );
    assert.equal(
        url,
        'https://github.com/Loctree/loctree-release/releases/download/v0.12.3/loctree-0.12.3-aarch64-apple-darwin.tar.gz',
    );
});

test('releaseDownloadUrl still supports explicit latest override with a versioned tarball name', () => {
    const url = releaseDownloadUrl(
        REPO_BASE,
        'latest',
        'loctree-0.12.3-x86_64-unknown-linux-gnu.tar.gz',
    );
    assert.equal(
        url,
        'https://github.com/Loctree/loctree-release/releases/latest/download/loctree-0.12.3-x86_64-unknown-linux-gnu.tar.gz',
    );
});

test('releaseDownloadUrl does not double up an existing /releases suffix', () => {
    const url = releaseDownloadUrl(
        `${REPO_BASE}/releases`,
        'v0.12.3',
        'loctree-0.12.3-x86_64-apple-darwin.tar.gz',
    );
    assert.equal(
        url,
        'https://github.com/Loctree/loctree-release/releases/download/v0.12.3/loctree-0.12.3-x86_64-apple-darwin.tar.gz',
    );
});

test('every release target yields a well-formed exact-tag tarball URL', () => {
    for (const target of RELEASE_TARGETS) {
        const asset = `loctree-0.12.3-${target}.tar.gz`;
        const url = releaseDownloadUrl(REPO_BASE, 'v0.12.3', asset);
        assert.equal(url, `${REPO_BASE}/releases/download/v0.12.3/${asset}`);
    }
});

test('PATH fallback prefers ~/.local/bin and reports a shadowing executable', () => {
    const fs = require('node:fs') as typeof import('node:fs');
    const os = require('node:os') as typeof import('node:os');
    const path = require('node:path') as typeof import('node:path');
    const root = fs.mkdtempSync(path.join(os.tmpdir(), 'loctree-path-contract-'));
    const home = path.join(root, 'home');
    const cargoBin = path.join(root, 'cargo-bin');
    fs.mkdirSync(path.join(home, '.local', 'bin'), { recursive: true });
    fs.mkdirSync(cargoBin, { recursive: true });
    const preferred = path.join(home, '.local', 'bin', 'loctree-lsp');
    const shadowed = path.join(cargoBin, 'loctree-lsp');
    fs.writeFileSync(preferred, '#!/bin/sh\n');
    fs.writeFileSync(shadowed, '#!/bin/sh\n');
    fs.chmodSync(preferred, 0o755);
    fs.chmodSync(shadowed, 0o755);

    const resolved = resolvePathRuntime('loctree-lsp', cargoBin, home);
    assert.equal(resolved.command, fs.realpathSync(preferred));
    assert.equal(resolved.source, 'preferred-install');
    assert.equal(resolved.shadowedPath, fs.realpathSync(shadowed));
    fs.rmSync(root, { recursive: true, force: true });
});

test('PATH fallback rejects a caller-supplied executable name', () => {
    const expected = process.platform === 'win32' ? 'loctree-lsp.exe' : 'loctree-lsp';
    const resolved = resolvePathRuntime('../other-binary', '', '/tmp/unused-loctree-home');

    assert.equal(resolved.command, expected);
    assert.equal(resolved.source, 'path');
});

test.after(() => {
    moduleInternals._load = originalLoad;
});
