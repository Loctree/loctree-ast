#!/usr/bin/env node

const { execFileSync } = require('child_process');
const { existsSync } = require('fs');
const { basename, dirname, isAbsolute, normalize, relative, sep } = require('path');
const { fileURLToPath, pathToFileURL } = require('url');

// Loctree is one runtime. The npm wrapper exposes `loctree` (with `loct` as a
// short alias) and a narrow `loctree-mcp` adapter for stdio package runners.
// MCP and LSP remain co-process binaries the runtime can spawn on demand:
//
//   loct watch --http   -> loctree co-spawns `loctree-mcp` (streamable HTTP MCP)
//   loct watch --lsp    -> loctree co-spawns `loctree-lsp`  (editor language server)
//
// The runtime resolves those co-processes as SIBLINGS of its own executable,
// so the platform package must ship all three binaries side by side.
const RUNTIME_BINARY = 'loctree';
const BUNDLED_BINARIES = ['loctree', 'loctree-mcp', 'loctree-lsp'];

// Platform key -> the technical platform package that delivers the binaries.
// These are a delivery mechanism only; users never install them directly.
const PLATFORM_PACKAGES = {
  'darwin-arm64': '@loctree/loct-darwin-arm64',
  'darwin-x64': '@loctree/loct-darwin-x64',
  'linux-x64-gnu': '@loctree/loct-linux-x64-gnu',
  'win32-x64-msvc': '@loctree/loct-win32-x64-msvc',
};

function getPlatformKey() {
  const platform = process.platform;
  const arch = process.arch;

  const archMap = {
    'x64': 'x64',
    'arm64': 'arm64',
    'aarch64': 'arm64',
  };
  const normalizedArch = archMap[arch] || arch;

  if (platform === 'linux') {
    const libc = isMuslLibc() ? 'musl' : 'gnu';
    return `${platform}-${normalizedArch}-${libc}`;
  }
  if (platform === 'win32') {
    return `${platform}-${normalizedArch}-msvc`;
  }
  if (platform === 'darwin') {
    return `${platform}-${normalizedArch}`;
  }
  return null;
}

function isMuslLibc() {
  const { spawnSync } = require('child_process');
  try {
    const lddVersion = spawnSync('ldd', ['--version'], { encoding: 'utf8' });
    return lddVersion.stderr && lddVersion.stderr.includes('musl');
  } catch (err) {
    return false;
  }
}

function platformPackageName() {
  const key = getPlatformKey();
  if (!key) return null;
  return PLATFORM_PACKAGES[key] || null;
}

function binaryFileName(name) {
  return process.platform === 'win32' ? `${name}.exe` : name;
}

function childPathInsidePackage(pkgDir, fileName) {
  if (!/^[A-Za-z0-9._-]+$/.test(fileName) || basename(fileName) !== fileName) {
    throw new Error(`Unsafe Loctree binary file name: ${fileName}`);
  }

  const root = normalize(pkgDir);
  const rootUrl = pathToFileURL(root.endsWith(sep) ? root : `${root}${sep}`);
  const childPath = normalize(fileURLToPath(new URL(fileName, rootUrl)));
  const relativePath = relative(root, childPath);
  if (relativePath === '' || relativePath.startsWith('..') || isAbsolute(relativePath)) {
    throw new Error(`Loctree binary escaped platform package directory: ${fileName}`);
  }
  return childPath;
}

/**
 * Resolve the absolute path to one bundled binary inside the installed platform
 * package. Uses node module resolution (so it respects npm/pnpm/yarn hoisting),
 * then validates the file exists. The three bundled binaries always live in the
 * same directory, which is what lets `loctree` find `loctree-mcp` / `loctree-lsp`
 * as siblings at runtime.
 */
function getBinaryPath(name = RUNTIME_BINARY) {
  if (!BUNDLED_BINARIES.includes(name)) {
    throw new Error(`Unknown Loctree binary: ${name}`);
  }

  const packageName = platformPackageName();
  if (!packageName) {
    throw new Error(
      `Unsupported platform: ${process.platform}-${process.arch}. ` +
      `Loctree ships for: ${Object.keys(PLATFORM_PACKAGES).join(', ')}.`
    );
  }

  let pkgDir;
  try {
    // Resolve via the platform package's manifest, then sit the binary next to it.
    pkgDir = dirname(require.resolve(`${packageName}/package.json`));
  } catch (err) {
    throw new Error(
      `Loctree platform package "${packageName}" is not installed. ` +
      `This usually means optionalDependencies were disabled ` +
      `(--no-optional / --ignore-optional). Reinstall @loctree/loct with optional deps enabled.`
    );
  }

  const binaryPath = childPathInsidePackage(pkgDir, binaryFileName(name));
  if (!existsSync(binaryPath)) {
    throw new Error(
      `Loctree binary "${binaryFileName(name)}" not found in "${packageName}" ` +
      `(expected at ${binaryPath}). The platform package may be incomplete.`
    );
  }
  return binaryPath;
}

/**
 * Run the Loctree runtime (`loctree`), inheriting stdio. Both the `loctree` and
 * `loct` bin entries call this — `loct` is just a short alias for the same binary.
 * Exits with the binary's status code on failure.
 */
function runRuntime(args = []) {
  return runBinary(RUNTIME_BINARY, args);
}

/**
 * Run one bundled Loctree binary, inheriting stdio. This is intentionally
 * narrow: package entrypoints may expose a bundled co-process without adding a
 * second npm package or searching PATH for an unrelated install.
 */
function runBinary(name, args = []) {
  const binaryPath = getBinaryPath(name);
  try {
    return execFileSync(binaryPath, args, { stdio: 'inherit' });
  } catch (err) {
    if (typeof err.status === 'number') {
      process.exit(err.status);
    }
    throw err;
  }
}

// Backwards-compatible alias kept for any pre-existing importers of this module.
function execLoct(args = [], options = {}) {
  const binaryPath = getBinaryPath(RUNTIME_BINARY);
  try {
    return execFileSync(binaryPath, args, { stdio: 'inherit', ...options });
  } catch (err) {
    if (typeof err.status === 'number') {
      process.exit(err.status);
    }
    throw err;
  }
}

module.exports = {
  RUNTIME_BINARY,
  BUNDLED_BINARIES,
  platformPackageName,
  getPlatformKey,
  getBinaryPath,
  runBinary,
  runRuntime,
  execLoct,
};

if (require.main === module) {
  runRuntime(process.argv.slice(2));
}
