#!/usr/bin/env node
/**
 * sync-version.mjs
 *
 * Gen3 single-wrapper model. Bumps the shared version across the real npm
 * package set in distribution/npm — ONE user-facing wrapper plus its technical
 * platform packages (esbuild/swc pattern):
 *
 *   distribution/npm/loct/package.json                                   (wrapper @loctree/loctree)
 *   distribution/npm/loct/platform-packages/{4 platforms}/package.json   (@loctree/loctree-<platform>)
 *
 * That is 5 package.json files, not 15. The wrapper's optionalDependencies
 * (its @loctree/loctree-* platform set) are kept in lockstep with the same version.
 *
 *
 * Usage:
 *   node distribution/npm/sync-version.mjs <version>
 *   node distribution/npm/sync-version.mjs --check              # parse + verify only
 *   node distribution/npm/sync-version.mjs --check <version>    # verify ALL == <version>
 */

import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT = __dirname; // distribution/npm

// Directory of the single user-facing wrapper package under distribution/npm.
const WRAPPER = "loct";
// Platform packages published beside the wrapper as its optionalDependencies.
const PLATFORMS = ["darwin-arm64", "darwin-x64", "linux-x64-gnu", "win32-x64-msvc"];

/** Every package.json this script owns: the wrapper plus its 4 platform packages. */
function allPackageJsonPaths() {
  const paths = [path.join(ROOT, WRAPPER, "package.json")];
  for (const plat of PLATFORMS) {
    paths.push(path.join(ROOT, WRAPPER, "platform-packages", plat, "package.json"));
  }
  return paths;
}

/** Read and parse one package.json. */
function readJson(p) {
  return JSON.parse(fs.readFileSync(p, "utf8"));
}

/** Write one package.json back with 2-space indent and a trailing newline. */
function writeJson(p, obj) {
  fs.writeFileSync(p, `${JSON.stringify(obj, null, 2)}\n`);
}

/**
 * Stamp `version` into all five package.json files and into every @loctree/*
 * optionalDependency pin, aborting first if any of those files is missing.
 */
function bump(version) {
  const paths = allPackageJsonPaths();
  const missing = paths.filter((p) => !fs.existsSync(p));
  if (missing.length) {
    console.error("Missing package.json files:");
    for (const m of missing) console.error(`  ${m}`);
    process.exit(2);
  }

  for (const p of paths) {
    const pkg = readJson(p);
    pkg.version = version;
    if (pkg.optionalDependencies) {
      for (const dep of Object.keys(pkg.optionalDependencies)) {
        // Only bump our own scope; leave any external deps alone.
        if (dep.startsWith("@loctree/")) {
          pkg.optionalDependencies[dep] = version;
        }
      }
    }
    writeJson(p, pkg);
  }

  console.log(`distribution/npm synced to ${version} across ${paths.length} package.json files`);
}

/**
 * Verify the five package.json files carry one identical version (and
 * `expectedVersion` when given) with matching @loctree/* dep pins, exiting 1 on
 * any mismatch so a release cannot publish a split version set.
 */
function check(expectedVersion) {
  const paths = allPackageJsonPaths();
  const missing = paths.filter((p) => !fs.existsSync(p));
  if (missing.length) {
    console.error("Missing package.json files:");
    for (const m of missing) console.error(`  ${m}`);
    process.exit(2);
  }

  let ok = true;
  let firstVersion = null;
  for (const p of paths) {
    let pkg;
    try {
      pkg = readJson(p);
    } catch (err) {
      console.error(`Invalid JSON in ${p}: ${err.message}`);
      ok = false;
      continue;
    }
    if (typeof pkg.version !== "string" || !pkg.version.length) {
      console.error(`Missing version in ${p}`);
      ok = false;
      continue;
    }
    if (firstVersion === null) firstVersion = pkg.version;
    if (pkg.version !== firstVersion) {
      console.error(`Version mismatch: ${p} has ${pkg.version}, expected ${firstVersion}`);
      ok = false;
    }
    if (expectedVersion && pkg.version !== expectedVersion) {
      console.error(`Expected version ${expectedVersion} in ${p}, found ${pkg.version}`);
      ok = false;
    }
    if (pkg.optionalDependencies) {
      for (const [dep, ver] of Object.entries(pkg.optionalDependencies)) {
        if (dep.startsWith("@loctree/") && ver !== pkg.version) {
          console.error(`Dep version mismatch in ${p}: ${dep}=${ver} but pkg.version=${pkg.version}`);
          ok = false;
        }
      }
    }
  }

  if (!ok) process.exit(1);
  console.log(`distribution/npm: ${paths.length} package.json files all at version ${firstVersion}`);
}

/** Parse argv and dispatch to --check or a semver bump; bad input exits 1. */
function main() {
  const argv = process.argv.slice(2);
  if (argv.length === 0) {
    console.error("Usage: node distribution/npm/sync-version.mjs <version>");
    console.error("       node distribution/npm/sync-version.mjs --check [version]");
    process.exit(1);
  }

  if (argv[0] === "--check") {
    check(argv[1]);
    return;
  }

  const version = argv[0];
  if (!/^\d+\.\d+\.\d+/.test(version)) {
    console.error(`Not a valid semver-ish version: ${version}`);
    process.exit(1);
  }
  bump(version);
}

main();
