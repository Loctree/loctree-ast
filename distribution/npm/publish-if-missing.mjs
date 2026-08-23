#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { pathToFileURL } from "node:url";

const VERSION_RE = /^[0-9]+\.[0-9]+\.[0-9]+$/;

export function classifyNpmView(result, requestedVersion) {
  if (result.status === 0) {
    let published;
    try {
      published = JSON.parse(result.stdout.trim());
    } catch (error) {
      throw new Error(`npm view returned invalid JSON: ${error.message}`);
    }
    if (published === requestedVersion) return "already-published";
    throw new Error(
      `npm view returned unexpected version ${JSON.stringify(published)} for ${requestedVersion}`,
    );
  }

  const diagnostic = `${result.stdout || ""}\n${result.stderr || ""}`;
  if (/\bE404\b|404 Not Found/.test(diagnostic)) return "missing";
  throw new Error(`npm view failed without a registry 404:\n${diagnostic.trim()}`);
}

function parseArgs(argv) {
  const options = {};
  for (let index = 0; index < argv.length; index += 2) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (!flag?.startsWith("--") || value === undefined) {
      throw new Error(
        "usage: publish-if-missing.mjs --package-dir <dir> --version <x.y.z>",
      );
    }
    options[flag.slice(2)] = value;
  }
  if (!options["package-dir"] || !VERSION_RE.test(options.version || "")) {
    throw new Error(
      "usage: publish-if-missing.mjs --package-dir <dir> --version <x.y.z>",
    );
  }
  return options;
}

function npm(args, cwd, capture) {
  return spawnSync("npm", args, {
    cwd,
    encoding: "utf8",
    stdio: capture ? "pipe" : "inherit",
  });
}

export function main(argv = process.argv.slice(2)) {
  const options = parseArgs(argv);
  const packageDir = path.resolve(options["package-dir"]);
  const packageJson = JSON.parse(
    readFileSync(path.join(packageDir, "package.json"), "utf8"),
  );
  const packageName = packageJson.name;
  if (typeof packageName !== "string" || packageName.length === 0) {
    throw new Error(`package.json has no package name: ${packageDir}`);
  }

  const spec = `${packageName}@${options.version}`;
  const view = npm(["view", spec, "version", "--json"], packageDir, true);
  const state = classifyNpmView(view, options.version);
  if (state === "already-published") {
    console.log(`${spec} already published; preserving immutable npm version`);
    return 0;
  }

  console.log(`${spec} is absent; publishing with public access and provenance`);
  const published = npm(
    ["publish", "--access", "public", "--provenance"],
    packageDir,
    false,
  );
  if (published.error) throw published.error;
  if (published.status !== 0) {
    throw new Error(`npm publish failed for ${spec} with exit ${published.status}`);
  }
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    process.exitCode = main();
  } catch (error) {
    console.error(error.message);
    process.exitCode = 2;
  }
}
