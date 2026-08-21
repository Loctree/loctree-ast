/**
 * TypeScript definitions for the @loctree/loctree npm package (the Loctree runtime wrapper).
 */

import { ExecFileSyncOptions } from 'child_process';

/** The runtime binary the wrapper runs (`loct` is just a short alias for it). */
export const RUNTIME_BINARY: 'loctree';

/**
 * Binaries shipped inside the platform package. `loctree` is the runtime;
 * `loct` is the short alias for that same runtime; `loctree-mcp` / `loctree-lsp`
 * are co-processes the runtime spawns as siblings. The npm package additionally
 * exposes `loctree-mcp` for MCP package runners; LSP remains an internal
 * editor co-process.
 */
export type BundledBinary = 'loct' | 'loctree' | 'loctree-mcp' | 'loctree-lsp';
export const BUNDLED_BINARIES: BundledBinary[];

/**
 * Name of the technical platform package for the current platform
 * (e.g. "@loctree/loctree-darwin-arm64"), or null if the platform is unsupported.
 */
export function platformPackageName(): string | null;

/** Normalized platform key (e.g. "darwin-arm64", "linux-x64-gnu"), or null. */
export function getPlatformKey(): string | null;

/**
 * Get the absolute path to one bundled binary for the current platform.
 * Defaults to the runtime (`loctree`).
 * @throws Error if the platform is unsupported or the binary is not installed.
 */
export function getBinaryPath(name?: BundledBinary): string;

/** Run one bundled binary, inheriting stdio. */
export function runBinary(name: BundledBinary, args?: string[]): Buffer;

/**
 * Run the Loctree runtime (`loctree`), inheriting stdio. Exits the process with
 * the binary's status code on failure.
 */
export function runRuntime(args?: string[]): Buffer;

/**
 * Backwards-compatible alias that runs the runtime.
 * @deprecated Use `runRuntime(args)`.
 */
export function execLoct(args?: string[], options?: ExecFileSyncOptions): Buffer;
