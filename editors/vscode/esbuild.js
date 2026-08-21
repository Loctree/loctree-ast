/**
 * esbuild bundler for the loctree VS Code extension.
 *
 * Bundles src/extension.ts (and its dependency tree, including
 * vscode-languageclient) into a single dist/extension.js. Only `vscode` is
 * external — it is provided by the editor runtime. Bundling lets the VSIX ship
 * without node_modules, which also removes the class of "Cannot find module"
 * activation failures the unbundled package was prone to.
 *
 * 𝚅𝚒𝚋𝚎𝚌𝚛𝚊𝚏𝚝𝚎𝚍. with AI Agents ⓒ 2025-2026 Vetcoders
 */

const esbuild = require('esbuild');

const production = process.argv.includes('--production');
const watch = process.argv.includes('--watch');

/**
 * Build (or watch) the extension bundle. `--production` minifies and drops the
 * source map; `--watch` keeps the context alive for incremental rebuilds.
 */
async function main() {
    const ctx = await esbuild.context({
        entryPoints: ['src/extension.ts'],
        bundle: true,
        format: 'cjs',
        platform: 'node',
        target: 'node16',
        outfile: 'dist/extension.js',
        external: ['vscode'],
        minify: production,
        sourcemap: !production,
        sourcesContent: false,
        logLevel: 'warning',
    });

    if (watch) {
        await ctx.watch();
    } else {
        await ctx.rebuild();
        await ctx.dispose();
    }
}

main().catch((err) => {
    console.error(err);
    process.exit(1);
});
