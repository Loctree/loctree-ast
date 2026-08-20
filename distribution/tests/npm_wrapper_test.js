#!/usr/bin/env node

'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const packageRoot = path.resolve(__dirname, '../npm/loct');
const packageJson = require(path.join(packageRoot, 'package.json'));
const wrapper = require(packageRoot);

assert.strictEqual(packageJson.bin['loctree-mcp'], 'bin/loctree-mcp');
assert(packageJson.files.includes('bin/loctree-mcp'));
assert(wrapper.BUNDLED_BINARIES.includes('loctree-mcp'));
assert.strictEqual(typeof wrapper.runBinary, 'function');

if (process.platform === 'win32') {
  console.log('npm wrapper metadata passed; executable shim smoke skipped on Windows');
  process.exit(0);
}

const packageName = wrapper.platformPackageName();
assert(packageName, `unsupported test platform: ${process.platform}-${process.arch}`);

const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'loctree-npm-wrapper-'));
try {
  const fakePackageRoot = path.join(tempRoot, 'node_modules', ...packageName.split('/'));
  const fakeBinDir = path.join(fakePackageRoot, 'bin');
  fs.mkdirSync(fakeBinDir, { recursive: true });
  fs.writeFileSync(
    path.join(fakePackageRoot, 'package.json'),
    `${JSON.stringify({ name: packageName, version: packageJson.version })}\n`,
  );
  const fakeBinary = path.join(fakeBinDir, 'loctree-mcp');
  fs.writeFileSync(fakeBinary, '#!/bin/sh\nprintf \'fake-mcp:%s\\n\' "$*"\n');
  fs.chmodSync(fakeBinary, 0o755);

  const result = spawnSync(
    process.execPath,
    [path.join(packageRoot, 'bin/loctree-mcp'), '--version'],
    {
      encoding: 'utf8',
      env: {
        ...process.env,
        NODE_PATH: path.join(tempRoot, 'node_modules'),
      },
    },
  );
  assert.strictEqual(result.status, 0, result.stderr);
  assert.strictEqual(result.stdout.trim(), 'fake-mcp:--version');
  console.log('npm loctree-mcp wrapper smoke passed');
} finally {
  fs.rmSync(tempRoot, { recursive: true, force: true });
}
