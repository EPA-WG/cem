#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { copyFile, lstat, mkdir, readFile, readdir, unlink, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { dirname, extname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const defaultDestination = join(homedir(), 'cem-bin');
const args = process.argv.slice(2);

if (args.length === 1 && ['--help', '-h'].includes(args[0])) {
    console.log(`Usage: node tools/scripts/copy-demo-site.mjs [destination]

Build and copy the CEM elements, theme, component, and demo-viewer galleries with release WASM dependencies.
Default destination: ${defaultDestination}
Runs yarn build:demo first; Nx reuses cached outputs when up to date.
Shares one release WASM binary across all galleries.
Existing matching files are overwritten and superseded WASM copies are removed;
unrelated destination files are kept.`);
} else if (args.length > 1 || args.some((arg) => arg.startsWith('-'))) {
    console.error('Usage: node tools/scripts/copy-demo-site.mjs [destination]');
    process.exitCode = 1;
} else {
    try {
        await copyDemoSite(resolve(args[0] ?? defaultDestination));
    } catch (error) {
        console.error(`Demo copy failed: ${error.message}`);
        process.exitCode = 1;
    }
}

async function copyDemoSite(destination) {
    // Preserve existing module URLs with shims to one deployment-only runtime.
    const sharedDirectory = 'packages/cem_ql/dist/wasm';
    const sharedRuntime = `${sharedDirectory}/runtime.js`;
    const runtimeCopies = [
        ['packages/cem-elements/dist/lib/internal/runtime-support/vendor', 'cem_ql'],
        ['packages/cem-theme/dist/vendor/@epa-wg/cem-elements/dist/lib/internal/runtime-support/vendor', 'cem_ql'],
        ['packages/cem-ml-npm/dist/wasm/browser', 'cem_ml'],
    ];
    const supersededBinaries = new Set(runtimeCopies.map(([directory, name]) => `${directory}/${name}_bg.wasm`));
    const runtimeExtensions = new Set(['.js', '.mjs', '.wasm', '.json', '.css']);
    const demoExtensions = new Set([
        ...runtimeExtensions, '.html', '.xhtml', '.xml', '.xsl', '.cem', '.cemt',
        '.svg', '.png', '.jpg', '.jpeg', '.webp', '.gif', '.ico', '.woff', '.woff2',
        '.csv', '.yaml', '.yml', '.fixture',
    ]);
    const trees = [
        ['packages/cem-elements/demo', demoExtensions],
        ['packages/cem-elements/dist', runtimeExtensions],
        ['packages/cem-demo-element/demo', demoExtensions],
        ['packages/cem-demo-element/dist', runtimeExtensions],
        ['packages/cem-ml-npm/dist/wasm/browser', runtimeExtensions],
        ['packages/cem-components/examples', demoExtensions],
        ['packages/cem-components/src/components', demoExtensions],
        ['packages/cem-components/dist', demoExtensions],
        ['packages/cem-theme/dist/lib/css', demoExtensions],
        ['packages/cem-theme/dist/lib/css-generators', demoExtensions],
        ['packages/cem-theme/dist/lib/tokens', new Set([...demoExtensions, '.md'])],
        ['packages/cem-theme/dist/vendor/@epa-wg/cem-elements/dist', runtimeExtensions],
        ['packages/custom-element/material', demoExtensions],
    ];
    const navigation = [
        ['CEM elements demo gallery', 'packages/cem-elements/index.html'],
        ['Component workflow examples', 'packages/cem-components/index.html'],
        ['Theme CSS generators and demos', 'packages/cem-theme/dist/lib/css-generators/index.html'],
        ['Design token documentation', 'packages/cem-theme/dist/lib/tokens/index.xhtml'],
        ['Demo viewer examples', 'packages/cem-demo-element/demo/index.html'],
        ['Syntax coloring examples', 'packages/cem-demo-element/demo/syntax-coloring.html'],
        ['NPM version picker', 'packages/cem-elements/demo/npm-versions-demo.html'],
        ['String functions', 'packages/cem-elements/demo/functions/str.html'],
    ];
    const files = [
        'LICENSE',
        'packages/cem-elements/index.html',
        'packages/cem-elements/demo/framework-logos/README.md',
        'packages/cem-components/index.html',
        'packages/cem-components/demo.css',
        'packages/custom-element/demo/wc-square.svg',
        'docs/cem-elements-http-request-design.md',
        `${sharedDirectory}/cem_ql.js`,
        `${sharedDirectory}/cem_ql_bg.wasm`,
        `${sharedDirectory}/package.json`,
    ];
    const requiredBuildFiles = [
        ...navigation.map(([, file]) => file),
        'packages/cem-elements/dist/index.js',
        'packages/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql.js',
        'packages/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql_bg.wasm',
        'packages/cem-demo-element/dist/index.js',
        'packages/cem-ml-npm/dist/wasm/browser/cem_ml.js',
        'packages/cem-ml-npm/dist/wasm/browser/cem_ml_bg.wasm',
        'packages/cem-components/dist/lib/primitives.js',
        'packages/cem-components/dist/styles.css',
        'packages/cem-theme/dist/lib/css/cem-combined.css',
        'packages/cem-theme/dist/lib/css-generators/cem-css-generator.js',
        'packages/cem-theme/dist/lib/css-generators/cem-css-loader.js',
        'packages/cem-theme/dist/vendor/@epa-wg/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql.js',
        'packages/cem-theme/dist/vendor/@epa-wg/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql_bg.wasm',
    ];
    // Every authored generator/template needs its built file. Generator pages
    // also need their compiled token input.
    for (const name of await readdir(resolve(repoRoot, 'packages/cem-theme/src/lib/css-generators'))) {
        if ((!name.endsWith('.html') && !name.endsWith('.cemt')) || name === 'index.html') continue;
        requiredBuildFiles.push(`packages/cem-theme/dist/lib/css-generators/${name}`);
        if (name.endsWith('.html')) {
            requiredBuildFiles.push(`packages/cem-theme/dist/lib/tokens/${name.replace(/\.html$/, '.xhtml')}`);
        }
    }

    if (isWithin(repoRoot, destination) || destination === homedir()
        || [...trees.map(([tree]) => tree), sharedDirectory]
            .some((tree) => isWithin(destination, resolve(repoRoot, tree)))) {
        throw new Error('Choose an output directory separate from the repository inputs and home directory.');
    }

    console.log('Building demo dependencies with release WASM...');
    const build = spawnSync(process.platform === 'win32' ? 'yarn.cmd' : 'yarn', ['build:demo'], {
        cwd: repoRoot,
        stdio: 'inherit',
    });
    if (build.error || build.status !== 0) {
        throw new Error(`Release demo build failed: ${build.error?.message ?? build.status}. Destination was not updated.`);
    }

    // Check all sources before creating or overwriting destination files.
    for (const file of requiredBuildFiles) {
        try {
            const info = await lstat(resolve(repoRoot, file));
            if (!info.isFile() || info.size === 0) throw new Error('not a nonempty file');
        } catch {
            throw new Error(`Missing or invalid build output after yarn build:demo: ${file}.`);
        }
    }
    // Check both nested runtime copies against the release build. A stale
    // package/vendor cache must not silently reintroduce a debug binary.
    for (const [directory] of runtimeCopies.filter(([, name]) => name === 'cem_ql')) {
        for (const name of ['cem_ql.js', 'cem_ql_bg.wasm']) {
            const release = await readFile(resolve(repoRoot, sharedDirectory, name));
            const packaged = await readFile(resolve(repoRoot, directory, name));
            if (!release.equals(packaged)) {
                throw new Error(`${directory}/${name} differs from the release build. Rebuild its owning package before copying.`);
            }
        }
    }
    // Import the generated glue without initializing WASM. Fail before copying
    // if a future build stops exposing the CEM-ML API through CEM-QL.
    const ql = await import(pathToFileURL(resolve(repoRoot, sharedDirectory, 'cem_ql.js')).href);
    const ml = await import(pathToFileURL(resolve(repoRoot, 'packages/cem-ml-npm/dist/wasm/browser/cem_ml.js')).href);
    const missing = Object.keys(ml).filter((name) => typeof ql[name] !== typeof ml[name]);
    if (missing.length) {
        throw new Error(`The shared CEM-QL runtime cannot provide CEM-ML exports: ${missing.join(', ')}.`);
    }

    const generated = new Map([
        [sharedRuntime, `// Shared initialization preserves retained runtime state across concurrent consumers.
export * from './cem_ql.js';
import init from './cem_ql.js';
let ready;
export default function initialize(options) {
    return ready ??= init(options);
}
`],
    ]);
    for (const [directory, name] of runtimeCopies) {
        const file = `${directory}/${name}.js`;
        const modulePath = relative(dirname(file), sharedRuntime).split(sep).join('/');
        const specifier = JSON.stringify(modulePath.startsWith('.') ? modulePath : `./${modulePath}`);
        generated.set(file, `// Deployment adapter: the generated glue and WASM are shared by all demos.
export * from ${specifier};
export { default } from ${specifier};
`);
    }
    for (const [tree, extensions] of trees) {
        await collectFiles(tree, extensions, files);
    }
    const sources = [];
    for (const file of files.sort()) {
        if (supersededBinaries.has(file) || generated.has(file)) continue;
        if (extname(file) === '.wasm' && file !== `${sharedDirectory}/cem_ql_bg.wasm`) {
            throw new Error(`Unexpected WASM dependency; review shared runtime packaging: ${file}`);
        }
        const info = await lstat(resolve(repoRoot, file));
        if (!info.isFile()) throw new Error(`Expected a regular source file: ${file}`);
        sources.push({ file, size: info.size });
    }

    generated.set('index.html', `<!doctype html>
<html lang="en">
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>CEM demos</title>
<h1>CEM demos</h1>
<ul>
${navigation.map(([label, file]) => `  <li><a href="./${file}">${label}</a></li>`).join('\n')}
</ul>
</html>
`);
    generated.set('404.html', `<!doctype html>
<html lang="en">
<meta charset="utf-8">
<title>Not found</title>
<h1>404 — Not found</h1>
<p><a href="/">Return to the CEM demos</a></p>
</html>
`);
    generated.set('_headers', `/*
  Cache-Control: public, max-age=0, must-revalidate
  Access-Control-Allow-Origin: *
  X-Robots-Tag: noindex
`);

    // Refuse symlinks before writing or removing any destination file. Only
    // remove the three known obsolete binaries; never recursively clean up.
    const checked = new Set();
    async function checkDestination(path) {
        if (checked.has(path)) return;
        checked.add(path);
        const parent = dirname(path);
        if (parent !== path) await checkDestination(parent);
        const info = await lstat(path).catch((error) => {
            if (error.code !== 'ENOENT') throw error;
            return null;
        });
        if (info?.isSymbolicLink()) throw new Error(`Destination contains a symbolic link: ${path}`);
    }
    for (const file of new Set([...files, ...generated.keys(), ...supersededBinaries])) {
        await checkDestination(join(destination, file));
        const info = await lstat(join(destination, file)).catch((error) => {
            if (error.code !== 'ENOENT') throw error;
            return null;
        });
        if (info && !info.isFile()) throw new Error(`Expected a regular destination file: ${file}`);
    }

    for (const { file } of sources) {
        const target = join(destination, file);
        await mkdir(dirname(target), { recursive: true });
        await copyFile(resolve(repoRoot, file), target);
    }
    for (const [file, content] of generated) {
        const target = join(destination, file);
        await mkdir(dirname(target), { recursive: true });
        await writeFile(target, content);
    }
    let removed = 0;
    for (const file of supersededBinaries) {
        try {
            await unlink(join(destination, file));
            removed++;
        } catch (error) {
            if (error.code !== 'ENOENT') throw error;
        }
    }

    const bytes = sources.reduce((sum, { size }) => sum + size, 0)
        + [...generated.values()].reduce((sum, content) => sum + Buffer.byteLength(content), 0);
    console.log(`Copied ${sources.length} files and generated ${generated.size} files (${mib(bytes)} MiB) to ${destination}`);
    if (removed) console.log(`Removed ${removed} superseded WASM copies.`);
    for (const { file, size } of sources.filter(({ file }) => extname(file) === '.wasm')) {
        console.log(`WASM: ${file} (${mib(size)} MiB)`);
        if (size > 25 * 1024 * 1024) {
            console.warn(`  Exceeds the 25 MiB per-file hosting limit listed in docs/wasm-deployment-size.md.`);
        }
    }
    console.log(`Serve ${destination} as the HTTP document root; open /index.html.`);
}

async function collectFiles(directory, extensions, files) {
    const excluded = new Set([
        'node_modules', 'reports', 'testing', 'figma',
        'cem.tokens.intermediate.json', 'cem.tokens.resolved.json',
    ]);
    for (const entry of await readdir(resolve(repoRoot, directory), { withFileTypes: true })) {
        if (entry.name.startsWith('.') || entry.name.includes(':') || excluded.has(entry.name)) continue;
        const file = `${directory}/${entry.name}`;
        if (entry.isSymbolicLink()) throw new Error(`Source contains a symbolic link: ${file}`);
        if (entry.isDirectory()) await collectFiles(file, extensions, files);
        else if (entry.isFile() && extensions.has(extname(entry.name))) files.push(file);
    }
}

function isWithin(path, parent) {
    const child = relative(parent, path);
    return child === '' || (child !== '..' && !child.startsWith(`..${sep}`) && !child.startsWith(sep));
}

function mib(bytes) {
    return (bytes / 1024 / 1024).toFixed(2);
}
