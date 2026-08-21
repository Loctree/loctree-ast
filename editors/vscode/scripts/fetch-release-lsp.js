/**
 * Fetch the same-version loctree-lsp release bundle for VSIX packaging.
 */

const crypto = require('crypto');
const fs = require('fs');
const https = require('https');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const extensionRoot = path.resolve(__dirname, '..');
const packageJson = require(path.join(extensionRoot, 'package.json'));

const releaseRepo = (process.env.LOCTREE_RELEASE_REPO || 'https://github.com/Loctree/loctree-release').replace(/\/$/, '');
const target = process.env.LOCTREE_TARGET;
const version = normalizeVersion(process.env.LOCTREE_VERSION || packageJson.version);
const tag = `v${version}`;
const refName = process.env.GITHUB_REF_NAME;
const refType = process.env.GITHUB_REF_TYPE;
const workDir = path.resolve(process.env.LOCTREE_DOWNLOAD_DIR || fs.mkdtempSync(path.join(os.tmpdir(), 'loctree-lsp-')));
const archiveName = `loctree-${version}-${target}.tar.gz`;
const archiveUrl = `${releaseRepo}/releases/download/${tag}/${archiveName}`;
const archivePath = path.join(workDir, archiveName);
const shaPath = `${archivePath}.sha256`;
const extractDir = path.join(workDir, 'extract');
const binaryName = target && target.includes('windows') ? 'loctree-lsp.exe' : 'loctree-lsp';
const extractedBinary = path.join(extractDir, `loctree-${version}-${target}`, 'bin', binaryName);
const stagedBinary = path.join(workDir, binaryName);

/** Strip a leading `v` and surrounding whitespace from a version string. */
function normalizeVersion(raw) {
  return String(raw || '').trim().replace(/^v/, '');
}

/** Abort the fetch when a required input is missing, naming it in the error. */
function requireValue(value, name) {
  if (!value) {
    throw new Error(`${name} is required`);
  }
}

/** Prefixed progress line on stdout, so CI logs are attributable. */
function log(message) {
  process.stdout.write(`[loctree-vscode] ${message}\n`);
}

/**
 * Download one URL to disk, following up to `redirectsLeft` redirects and
 * aborting after 30s. Partial files are unlinked before the promise rejects.
 */
function download(url, destination, redirectsLeft = 5) {
  return new Promise((resolve, reject) => {
    const fail = (error) => {
      try {
        if (fs.existsSync(destination)) fs.unlinkSync(destination);
      } catch {
        // Keep the original download error.
      }
      reject(error);
    };

    const request = https.get(url, { headers: { 'User-Agent': 'loctree-vscode-ci' } }, (response) => {
      if (response.statusCode && response.statusCode >= 300 && response.statusCode < 400) {
        const location = response.headers.location;
        response.resume();
        if (!location) {
          fail(new Error(`Redirect without location for ${url}`));
          return;
        }
        if (redirectsLeft <= 0) {
          fail(new Error(`Too many redirects for ${url}`));
          return;
        }
        download(location, destination, redirectsLeft - 1).then(resolve).catch(reject);
        return;
      }

      if (response.statusCode && response.statusCode >= 400) {
        response.resume();
        fail(new Error(`Download failed (${response.statusCode}) for ${url}`));
        return;
      }

      const file = fs.createWriteStream(destination);
      response.pipe(file);
      file.on('finish', () => file.close(resolve));
      file.on('error', fail);
    });

    request.setTimeout(30_000, () => {
      request.destroy(new Error(`Download timed out for ${url}`));
    });
    request.on('error', fail);
  });
}

/** SHA-256 of a file, as lowercase hex. */
function sha256(filePath) {
  const hash = crypto.createHash('sha256');
  hash.update(fs.readFileSync(filePath));
  return hash.digest('hex');
}

/** First token of a checksum sidecar, lowercased; throws when the sidecar is empty. */
function expectedSha(filePath) {
  const first = fs.readFileSync(filePath, 'utf8').trim().split(/\s+/)[0];
  if (!first) {
    throw new Error(`Empty checksum sidecar: ${filePath}`);
  }
  return first.toLowerCase();
}

/** Append a `key=value` line to `$GITHUB_OUTPUT` when running inside Actions. */
function writeGithubOutput(key, value) {
  const outputPath = process.env.GITHUB_OUTPUT;
  if (outputPath) {
    fs.appendFileSync(outputPath, `${key}=${value}\n`);
  }
}

/**
 * Fetch and stage the release LSP for packaging. Refuses to proceed when the
 * git tag disagrees with the VS Code package version, or when the downloaded
 * archive fails its checksum — a mismatched runtime never reaches a VSIX.
 */
async function main() {
  requireValue(target, 'LOCTREE_TARGET');
  requireValue(version, 'LOCTREE_VERSION or package.json version');
  if (refType === 'tag' && refName && refName !== tag) {
    throw new Error(`Git tag ${refName} does not match VSCode package version ${tag}`);
  }

  fs.rmSync(workDir, { recursive: true, force: true });
  fs.mkdirSync(workDir, { recursive: true });

  log(`Fetching ${archiveUrl}`);
  await download(archiveUrl, archivePath);
  await download(`${archiveUrl}.sha256`, shaPath);

  const expected = expectedSha(shaPath);
  const actual = sha256(archivePath);
  if (expected !== actual) {
    throw new Error(`Checksum mismatch for ${archiveName}. Expected ${expected}, got ${actual}`);
  }

  fs.mkdirSync(extractDir, { recursive: true });
  const tar = spawnSync('tar', ['-xzf', archivePath, '-C', extractDir], { stdio: 'inherit' });
  if (tar.status !== 0) {
    throw new Error(`tar -xzf failed for ${archiveName}`);
  }
  if (!fs.existsSync(extractedBinary)) {
    throw new Error(`Release archive did not contain ${path.relative(extractDir, extractedBinary)}`);
  }

  fs.copyFileSync(extractedBinary, stagedBinary);
  if (!binaryName.endsWith('.exe')) {
    fs.chmodSync(stagedBinary, 0o755);
  }

  writeGithubOutput('lsp_path', stagedBinary);
  writeGithubOutput('version', version);
  writeGithubOutput('tag', tag);
  writeGithubOutput('asset', archiveName);
  log(`Prepared ${stagedBinary}`);
}

main().catch((error) => {
  console.error(`[loctree-vscode] ${error instanceof Error ? error.message : String(error)}`);
  process.exit(1);
});
