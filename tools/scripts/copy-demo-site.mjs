#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { lstat, readFile, readdir } from 'node:fs/promises';
import { homedir } from 'node:os';
import { dirname, extname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { publishDemoDeployment } from './demo-deployment.mjs';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const defaultDestination = join(homedir(), 'cem-bin');
const args = process.argv.slice(2);

if (args.length === 1 && ['--help', '-h'].includes(args[0])) {
    console.log(`Usage: node tools/scripts/copy-demo-site.mjs [destination]

Build and copy CEM Studio and the elements, theme, component, and demo-viewer galleries with release WASM dependencies.
Default destination: ${defaultDestination}
Runs yarn build:demo first; Nx reuses cached outputs when up to date.
Shares the CEM-QL gallery runtime; the CEM-ML bridge and Studio retain their own deployments.
Records managed files in demo-deployment.json, enforces a 999-file total and files below 25 MiB.
Unchanged obsolete managed files are removed; modified and unrelated files are kept.`);
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
    const studioSource = 'packages/cem-studio/dist/static';
    const themeVendor = 'packages/cem-theme/dist/vendor/@epa-wg/cem-elements/dist';
    const retired = file => file.startsWith(`${themeVendor}/`)
        || (file.startsWith('packages/cem-components/dist/') && extname(file) === '.html');
    const legacyEntries = [];
    const deploymentPath = file => file.startsWith(`${studioSource}/`) ? `studio/${file.slice(studioSource.length + 1)}` : file;
    // Preserve existing module URLs with shims to one deployment-only runtime.
    const sharedDirectory = 'packages/cem_ql/dist/wasm';
    const sharedRuntime = `${sharedDirectory}/runtime.js`;
    const runtimeCopies = [
        ['packages/cem-elements/dist/lib/internal/runtime-support/vendor', 'cem_ql'],
        ['packages/cem-theme/dist/vendor/@epa-wg/cem-elements/dist/lib/internal/runtime-support/vendor', 'cem_ql'],
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
        ['packages/cem-ml-cli-npm/dist', runtimeExtensions],
        ['packages/cem-components/examples', demoExtensions],
        ['packages/cem-components/playgrounds', demoExtensions],
        ['packages/cem-components/src/components', demoExtensions],
        ['packages/cem-components/dist', demoExtensions],
        ['packages/cem-theme/dist/lib/css', demoExtensions],
        ['packages/cem-theme/dist/lib/css-generators', demoExtensions],
        ['packages/cem-theme/dist/lib/tokens', new Set([...demoExtensions, '.md'])],
        ['packages/cem-theme/dist/vendor/@epa-wg/cem-elements/dist', runtimeExtensions],
        ['packages/custom-element/material', demoExtensions],
        [studioSource, null],
    ];
    const navigation = [
        ['CEM Studio', 'studio/', `${studioSource}/index.html`],
        ['CEM elements demo gallery', 'packages/cem-elements/index.html'],
        ['Component workflow examples', 'packages/cem-components/index.html'],
        ['Theme CSS generators and demos', 'packages/cem-theme/dist/lib/css-generators/index.html'],
        ['Design token documentation', 'packages/cem-theme/dist/lib/tokens/index.xhtml'],
        ['Demo viewer examples', 'packages/cem-demo-element/demo/index.html'],
        ['CEM playground', 'packages/cem-demo-element/demo/playground.html'],
        ['Syntax coloring examples', 'packages/cem-demo-element/demo/syntax-coloring.html'],
        ['NPM version picker', 'packages/cem-elements/demo/npm-versions-demo.html'],
        ['String functions', 'packages/cem-elements/demo/functions/str.html'],
    ];
    const files = [
        'LICENSE',
        'packages/cem-ml-npm/dist/cem-ml-runtime.json',
        'packages/cem-elements/index.html',
        'packages/cem-elements/demo/framework-logos/README.md',
        'packages/cem-components/index.html',
        'packages/cem-components/demo.css',
        'packages/custom-element/demo/wc-square.svg',
        'docs/cem-elements-http-request-design.md',
        `${sharedDirectory}/cem_ql.js`,
        `${sharedDirectory}/cem_ql_factory.js`,
        `${sharedDirectory}/cem_ql_bg.wasm`,
        `${sharedDirectory}/package.json`,
    ];
    const requiredBuildFiles = [
        ...navigation.map(([, file, source]) => source ?? file),
        ...['build.json', 'cache-inventory.json', 'security-headers.json', 'module-map.json', 'manifest.webmanifest', 'service-worker.js', 'samples/index.json'].map(file => `${studioSource}/${file}`),
        'packages/cem-elements/dist/index.js',
        'packages/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql.js',
        'packages/cem-elements/dist/lib/internal/runtime-support/vendor/cem_ql_bg.wasm',
        'packages/cem-demo-element/dist/index.js',
        'packages/cem-ml-npm/dist/wasm/browser/cem_ml.js',
        'packages/cem-ml-npm/dist/wasm/browser/cem_ml_bg.wasm',
        'packages/cem-components/dist/components.xhtml',
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
    // The common-query bridge installs native command adapters at startup; the
    // bare CEM-QL artifact is no longer a substitute for that paired deployment.
    // Share only identical CEM-QL bindings and keep each factory call isolated.
    const factory = await import(pathToFileURL(resolve(repoRoot, sharedDirectory, 'cem_ql_factory.js')).href);
    if (typeof factory.createCemQlWasmGeneration !== 'function') throw new Error('The shared CEM-QL generation factory is unavailable.');

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
        const shim = `// Deployment adapter: CEM-QL gallery consumers share matching glue and WASM.
export * from ${specifier};
export { default } from ${specifier};
`;
        const factoryFile = `${directory}/${name}_factory.js`;
        const factoryPath = relative(dirname(factoryFile), `${sharedDirectory}/cem_ql_factory.js`).split(sep).join('/');
        const factoryShim = `// Share factory code and binary URLs, preserving a new heap per generation.\nexport * from ${JSON.stringify(factoryPath.startsWith('.') ? factoryPath : `./${factoryPath}`)};\n`;
        if (retired(file)) {
            legacyEntries.push({ path: file, content: shim }, { path: factoryFile, content: factoryShim },
                { path: file, content: shim.replace('CEM-QL gallery consumers share matching glue and WASM.', 'the generated glue and WASM are shared by all demos.') });
        } else { generated.set(file, shim); generated.set(factoryFile, factoryShim); }
    }
    // Consumer-only rebasing: standalone package outputs retain their vendor tree.
    const generatorFile = 'packages/cem-theme/dist/lib/css-generators/cem-css-generator.js';
    const generator = await readFile(resolve(repoRoot, generatorFile), 'utf8');
    const oldPrefix = '../../vendor/@epa-wg/cem-elements/dist/';
    if (generator.split(oldPrefix).length - 1 !== 3) throw new Error('Review the theme generator runtime imports before deploying.');
    const newPrefix = `${relative(dirname(generatorFile), 'packages/cem-elements/dist').split(sep).join('/')}/`;
    generated.set(generatorFile, generator.replaceAll(oldPrefix, newPrefix));
    for (const [tree, extensions] of trees) {
        await collectFiles(tree, extensions, files);
    }
    const sources = [];
    for (const file of files.sort()) {
        if (retired(file) || supersededBinaries.has(file)) {
            legacyEntries.push({ path: deploymentPath(file), source: resolve(repoRoot, file) });
            continue;
        }
        if (supersededBinaries.has(file) || generated.has(file)) continue;
        if (extname(file) === '.wasm' && file !== `${sharedDirectory}/cem_ql_bg.wasm`
            && file !== 'packages/cem-ml-npm/dist/wasm/browser/cem_ml_bg.wasm' && !file.startsWith(`${studioSource}/`)) {
            throw new Error(`Unexpected WASM dependency; review shared runtime packaging: ${file}`);
        }
        const info = await lstat(resolve(repoRoot, file));
        if (!info.isFile()) throw new Error(`Expected a regular source file: ${file}`);
        sources.push({ file: deploymentPath(file), source: file, size: info.size });
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
    const studioSecurity = JSON.parse(await readFile(resolve(repoRoot, studioSource, 'security-headers.json'), 'utf8'));
    generated.set('_headers', `/*
  Cache-Control: public, max-age=0, must-revalidate
  Access-Control-Allow-Origin: *
  X-Robots-Tag: noindex

/studio/*
${Object.entries(studioSecurity.headers).map(([name, value]) => `  ${name}: ${value}`).join('\n')}
`);

    const publication = await publishDemoDeployment({ destination, legacyEntries, entries: [
        ...sources.map(({ file, source }) => ({ path: file, source: resolve(repoRoot, source) })),
        ...[...generated].map(([path, content]) => ({ path, content })),
    ] });
    console.log(`Published ${publication.inventory.fileCount} managed files, ${publication.totalFileCount} total (${mib(publication.bytes)} MiB) to ${destination}`);
    console.log(`Removed ${publication.removed.length} unchanged obsolete managed files.`);
    for (const file of publication.preserved) console.warn(`Preserved modified obsolete file: ${file}`);
    for (const { file, size } of sources.filter(({ file }) => extname(file) === '.wasm')) {
        console.log(`WASM: ${file} (${mib(size)} MiB)`);
    }
    console.log(`Serve ${destination} as the HTTP document root; open /index.html.`);
}

async function collectFiles(directory, extensions, files) {
    const excluded = new Set([
        'node_modules', 'reports', 'testing', 'figma',
        'cem.tokens.intermediate.json', 'cem.tokens.resolved.json',
    ]);
    for (const entry of await readdir(resolve(repoRoot, directory), { withFileTypes: true })) {
        if (extensions && (entry.name.startsWith('.') || entry.name.includes(':') || excluded.has(entry.name))) continue;
        const file = `${directory}/${entry.name}`;
        if (entry.isSymbolicLink()) throw new Error(`Source contains a symbolic link: ${file}`);
        if (entry.isDirectory()) await collectFiles(file, extensions, files);
        else if (entry.isFile() && (!extensions || extensions.has(extname(entry.name)))) files.push(file);
    }
}

function isWithin(path, parent) {
    const child = relative(parent, path);
    return child === '' || (child !== '..' && !child.startsWith(`..${sep}`) && !child.startsWith(sep));
}

function mib(bytes) {
    return (bytes / 1024 / 1024).toFixed(2);
}
