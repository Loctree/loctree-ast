/**
 * Bundle loctree-lsp into the VSCode extension package.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const extensionRoot = path.resolve(__dirname, '..');
const repoRoot = path.resolve(extensionRoot, '..', '..');
const binDir = path.join(extensionRoot, 'bin');
const isWindows = process.platform === 'win32';
const binaryName = isWindows ? 'loctree-lsp.exe' : 'loctree-lsp';
const targetBinary = path.join(binDir, binaryName);

/** Prefixed progress line on stdout, so packaging logs are attributable. */
function log(msg) {
  process.stdout.write(`[loctree-vscode] ${msg}\n`);
}

/** Install one binary into `bin/`, making it executable on unix. */
function copyBinary(sourcePath) {
  fs.mkdirSync(binDir, { recursive: true });
  fs.copyFileSync(sourcePath, targetBinary);
  if (!isWindows) {
    fs.chmodSync(targetBinary, 0o755);
  }
  log(`Bundled ${binaryName} from ${sourcePath}`);
}

/** First PATH hit for a command via which/where, or null when it is not installed. */
function findOnPath(cmd) {
  const whichCmd = isWindows ? 'where' : 'which';
  const result = spawnSync(whichCmd, [cmd], { encoding: 'utf-8' });
  if (result.status !== 0) {
    return null;
  }
  const firstLine = (result.stdout || '').split(/\r?\n/).find(Boolean);
  return firstLine ? firstLine.trim() : null;
}

/** Last-resort `cargo build -p loctree-lsp --release` in the repo root; returns
 *  whether the build succeeded. */
function buildLspFromSource() {
  log('Building loctree-lsp from source for editor bundling...');
  const result = spawnSync(
    'cargo',
    ['build', '-p', 'loctree-lsp', '--release'],
    {
      cwd: repoRoot,
      encoding: 'utf-8',
      stdio: 'inherit',
    }
  );
  return result.status === 0;
}

const envPath = process.env.LOCTREE_LSP_PATH;
if (envPath && fs.existsSync(envPath)) {
  copyBinary(envPath);
  process.exit(0);
}

const repoRootBinary = path.join(repoRoot, 'target', 'release', binaryName);
if (fs.existsSync(repoRootBinary)) {
  copyBinary(repoRootBinary);
  process.exit(0);
}

if (buildLspFromSource() && fs.existsSync(repoRootBinary)) {
  copyBinary(repoRootBinary);
  process.exit(0);
}

const pathBinary = findOnPath(binaryName);
if (pathBinary && fs.existsSync(pathBinary)) {
  copyBinary(pathBinary);
  process.exit(0);
}

log(`No ${binaryName} found after LOCTREE_LSP_PATH, repo build, cargo build, and PATH probes.`);
// Fail the package/release build so a VSIX is never shipped without a bundled
// LSP binary. The runtime auto-download (client.ts ensureDownloadedBinary)
// remains a repair path for unpacked/dev installs.
process.exit(1);
