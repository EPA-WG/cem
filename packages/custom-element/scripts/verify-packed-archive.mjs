import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { createReadStream, readFileSync } from 'node:fs';
import { cp, mkdir, mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { dirname, extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

import { chromium } from 'playwright';

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = dirname(dirname(projectRoot));
const distRoot = join(projectRoot, 'dist');
const archiveManifest = readJson(join(projectRoot, 'package-archive.json'));
const sourcePackage = readJson(join(projectRoot, 'package.json'));
const temporaryRoot = await mkdtemp(join(tmpdir(), 'custom-element-packed-archive-'));
const stagedRoot = join(temporaryRoot, 'release-root');
const archiveRoot = join(temporaryRoot, 'archive');
const npmCacheRoot = join(temporaryRoot, 'npm-cache');
const consumerRoot = join(temporaryRoot, 'consumer');
const forbiddenSentinels = [
    { path: '.codex/session.json', content: '{}' },
    { path: '.github/workflows/private.yml', content: 'name: private\n' },
    { path: 'coverage/index.html', content: '<!doctype html>' },
    { path: 'node_modules/private/package.json', content: '{}' },
    { path: 'reports/private.json', content: '{}' },
    { path: 'scripts/private.mjs', content: 'export {};' },
    { path: 'test-fixtures/private.html', content: '<!doctype html>' },
    { path: 'package-lock.json', content: '{}' },
    { path: 'project.json', content: '{}' },
    { path: 'private.tsbuildinfo', content: '{}' },
];

try {
    validateArchiveManifest();
    await cp(distRoot, stagedRoot, { recursive: true });
    await addForbiddenSentinels();
    await mkdir(archiveRoot, { recursive: true });

    const stagedPackage = readJson(join(stagedRoot, 'package.json'));
    assert.deepEqual(stagedPackage.files, archiveManifest.packageFiles, 'generated package files allowlist');
    assert.equal(stagedPackage.scripts, undefined, 'generated package must not retain workspace scripts');
    assertDependencyFree(stagedPackage);
    assert.deepEqual(stagedPackage.exports, {
        '.': {
            types: './custom-element.d.ts',
            import: './index.js',
            default: './index.js',
        },
        './package.json': './package.json',
        './CustomElement': {
            types: './custom-element.d.ts',
            import: './custom-element.js',
            default: './custom-element.js',
        },
    });
    assert.equal(
        Object.keys(stagedPackage.exports).some((key) => key.includes('vendor')),
        false,
        'private vendor paths must not be exported',
    );

    const pack = runCapture(
        npmCommand(),
        ['pack', '--json', '--ignore-scripts', '--pack-destination', archiveRoot, '--cache', npmCacheRoot],
        stagedRoot,
    );
    const packResult = JSON.parse(pack)[0];
    assert(packResult?.filename, 'npm pack must report an archive filename');
    const archivePath = join(archiveRoot, packResult.filename);
    await stat(archivePath);

    const packedPaths = (packResult.files ?? []).map(({ path }) => path).sort();
    assert.equal(new Set(packedPaths).size, packedPaths.length, 'archive paths must be unique');
    assert.equal(packedPaths.length, archiveManifest.archive.fileCount, 'archive file count');
    assert.equal(
        createHash('sha256').update(packedPaths.join('\n')).digest('hex'),
        archiveManifest.archive.pathDigestSha256,
        'archive path digest',
    );
    for (const requiredFile of archiveManifest.archive.requiredFiles) {
        assert(packedPaths.includes(requiredFile), `archive must include ${requiredFile}`);
    }
    for (const path of packedPaths) {
        assert.equal(isForbiddenArchivePath(path), false, `archive must exclude private/generated path ${path}`);
    }
    for (const sentinel of forbiddenSentinels) {
        assert.equal(packedPaths.includes(sentinel.path), false, `archive must exclude sentinel ${sentinel.path}`);
    }

    await verifyCleanConsumer(archivePath);
    console.log(
        `Verified packed ${stagedPackage.name}@${stagedPackage.version}: ${packedPaths.length} locked files, ` +
            'clean root/subpath JavaScript and type contracts, and Chromium rendering.',
    );
} finally {
    assert(temporaryRoot.startsWith(`${tmpdir()}${sep}custom-element-packed-archive-`));
    await rm(temporaryRoot, { recursive: true, force: true });
}

async function addForbiddenSentinels() {
    for (const sentinel of forbiddenSentinels) {
        const target = join(stagedRoot, sentinel.path);
        await mkdir(dirname(target), { recursive: true });
        await writeFile(target, sentinel.content);
    }
}

function validateArchiveManifest() {
    assert.equal(archiveManifest.schemaVersion, 1, 'archive manifest schema');
    assertUniqueStrings(archiveManifest.sourceEntries, 'archive manifest sourceEntries');
    assertUniqueStrings(archiveManifest.packageFiles, 'archive manifest packageFiles');
    assertUniqueStrings(archiveManifest.archive?.requiredFiles, 'archive manifest requiredFiles');
    assert.deepEqual(sourcePackage.files, archiveManifest.packageFiles, 'source package files allowlist');
    assert.deepEqual(
        archiveManifest.packageFiles,
        [...archiveManifest.sourceEntries, 'vendor'],
        'package files must be source entries plus the private runtime vendor root',
    );
    assert(Number.isSafeInteger(archiveManifest.archive.fileCount) && archiveManifest.archive.fileCount > 0);
    assert.match(archiveManifest.archive.pathDigestSha256, /^[0-9a-f]{64}$/u);
}

async function verifyCleanConsumer(archivePath) {
    await mkdir(consumerRoot, { recursive: true });
    await writeFile(
        join(consumerRoot, 'package.json'),
        `${JSON.stringify({ name: 'custom-element-clean-consumer', private: true, type: 'module' }, null, 4)}\n`,
    );
    run(
        npmCommand(),
        [
            'install',
            archivePath,
            '--ignore-scripts',
            '--no-audit',
            '--no-fund',
            '--package-lock=false',
            '--cache',
            npmCacheRoot,
        ],
        consumerRoot,
    );

    const installedRoot = join(consumerRoot, 'node_modules/@epa-wg/custom-element');
    const installedPackage = readJson(join(installedRoot, 'package.json'));
    assert.equal(installedPackage.name, sourcePackage.name);
    assert.equal(installedPackage.version, sourcePackage.version);
    assert.deepEqual(installedPackage.files, archiveManifest.packageFiles);
    assert.equal(installedPackage.scripts, undefined);
    assertDependencyFree(installedPackage);

    await verifyTypeContracts();
    await verifyBrowserContract();
}

async function verifyTypeContracts() {
    await writeFile(
        join(consumerRoot, 'probe.ts'),
        [
            "import RootDefault, { CustomElement, diagnosticsFor, normalizeLegacyDeclaration, whenDeclarationSettled, whenRenderSettled } from '@epa-wg/custom-element';",
            "import SubpathDefault, { CustomElement as SubpathNamed } from '@epa-wg/custom-element/CustomElement';",
            '',
            'const rootDefault: typeof CustomElement = RootDefault;',
            'const subpathDefault: typeof CustomElement = SubpathDefault;',
            'const subpathNamed: typeof CustomElement = SubpathNamed;',
            "const declaration = document.createElement('custom-element');",
            "const instance = document.createElement('packed-type-card');",
            'const declarationSettled: Promise<void> = whenDeclarationSettled(declaration);',
            'const renderSettled: Promise<void> = whenRenderSettled(instance);',
            'const normalized: HTMLElement = normalizeLegacyDeclaration(declaration);',
            'const diagnostics: readonly unknown[] = diagnosticsFor(instance);',
            'void [rootDefault, subpathDefault, subpathNamed, declarationSettled, renderSettled, normalized, diagnostics];',
            '',
        ].join('\n'),
    );
    await writeFile(
        join(consumerRoot, 'tsconfig.json'),
        `${JSON.stringify(
            {
                compilerOptions: {
                    lib: ['ES2022', 'DOM'],
                    module: 'NodeNext',
                    moduleResolution: 'NodeNext',
                    noEmit: true,
                    strict: true,
                    target: 'ES2022',
                },
                files: ['probe.ts'],
            },
            null,
            4,
        )}\n`,
    );
    run(
        process.execPath,
        [join(workspaceRoot, 'node_modules/typescript/bin/tsc'), '--project', 'tsconfig.json'],
        consumerRoot,
    );
}

async function verifyBrowserContract() {
    await writeFile(
        join(consumerRoot, 'probe.html'),
        `<!doctype html>
<html lang="en">
    <head>
        <meta charset="utf-8" />
        <script type="importmap">
            {
                "imports": {
                    "@epa-wg/custom-element": "/node_modules/@epa-wg/custom-element/index.js",
                    "@epa-wg/custom-element/CustomElement": "/node_modules/@epa-wg/custom-element/custom-element.js"
                }
            }
        </script>
    </head>
    <body>
        <script type="module">
            const errors = [];
            const check = (label, condition) => {
                if (!condition) errors.push(label);
            };
            try {
                const root = await import('@epa-wg/custom-element');
                const subpath = await import('@epa-wg/custom-element/CustomElement');
                check('root and subpath default exports match', root.default === subpath.default);
                check('root named/default exports match', root.default === root.CustomElement);
                check('subpath named/default exports match', subpath.default === subpath.CustomElement);
                check('custom-element registration uses the public class', customElements.get('custom-element') === root.CustomElement);
                check('root import registers http-request', customElements.get('http-request') !== undefined);
                check('root import registers local-storage', customElements.get('local-storage') !== undefined);
                check('root import registers location-element', customElements.get('location-element') !== undefined);

                const declaration = document.createElement('custom-element');
                declaration.hidden = true;
                declaration.setAttribute('tag', 'packed-consumer-card');
                const template = document.createElement('template');
                template.setAttribute('type', 'text/cem-ml');
                template.textContent = '{article @data-role=output | {$datadom.attributes.label}}';
                declaration.append(template);
                document.body.append(declaration);
                await root.whenDeclarationSettled(declaration);

                const instance = document.createElement('packed-consumer-card');
                instance.setAttribute('label', 'Packed consumer');
                document.body.append(instance);
                await root.whenRenderSettled(instance);
                check('packed canonical CEM-ML renders through the private runtime', instance.querySelector('[data-role="output"]')?.textContent === 'Packed consumer');
                check('packed rendering keeps the substrate data island', instance.querySelector('template[data-cem-island="instance"]') !== null);
            } catch (error) {
                errors.push(error instanceof Error ? error.stack ?? error.message : String(error));
            }
            globalThis.__packedArchiveConsumer = { done: true, errors };
        </script>
    </body>
</html>
`,
    );

    const server = createConsumerServer();
    await new Promise((resolvePromise) => server.listen(0, '127.0.0.1', resolvePromise));
    const address = server.address();
    const port = typeof address === 'object' && address ? address.port : 0;
    const browser = await chromium.launch({ headless: true });
    try {
        const pageErrors = [];
        const page = await browser.newPage();
        page.on('pageerror', (error) => pageErrors.push(error.message));
        page.on('console', (message) => {
            if (message.type() === 'error') pageErrors.push(message.text());
        });
        await page.goto(`http://127.0.0.1:${port}/probe.html`);
        await page.waitForFunction(() => globalThis.__packedArchiveConsumer?.done === true);
        const result = await page.evaluate(() => globalThis.__packedArchiveConsumer);
        assert.deepEqual([...pageErrors, ...(result.errors ?? [])], []);
        await page.close();
    } finally {
        await browser.close();
        await new Promise((resolvePromise) => server.close(resolvePromise));
    }
}

function createConsumerServer() {
    return createServer(async (request, response) => {
        try {
            const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');
            const pathname = decodeURIComponent(requestUrl.pathname === '/' ? '/probe.html' : requestUrl.pathname);
            const filePath = normalize(join(consumerRoot, pathname));
            if (!filePath.startsWith(`${consumerRoot}${sep}`)) {
                response.writeHead(403);
                response.end('Forbidden');
                return;
            }
            const fileStat = await stat(filePath);
            if (!fileStat.isFile()) throw new Error('not a file');
            response.writeHead(200, { 'content-type': contentType(filePath) });
            createReadStream(filePath).pipe(response);
        } catch {
            response.writeHead(404);
            response.end('Not found');
        }
    });
}

function assertDependencyFree(packageJson) {
    for (const field of ['dependencies', 'optionalDependencies', 'peerDependencies', 'bundledDependencies']) {
        const value = packageJson[field];
        assert(value === undefined || Object.keys(value).length === 0, `package must not contain ${field}`);
    }
}

function assertUniqueStrings(value, label) {
    assert(Array.isArray(value) && value.length > 0, `${label} must be a non-empty array`);
    assert(
        value.every((item) => typeof item === 'string' && item.length > 0),
        `${label} must contain strings`,
    );
    assert.equal(new Set(value).size, value.length, `${label} must be unique`);
}

function isForbiddenArchivePath(path) {
    return (
        /(^|\/)(?:\.claude|\.codex|\.git|\.github|\.idea|\.nx|\.vs|\.vscode|coverage|node_modules|reports|scripts|storybook-static|test-fixtures)(?:\/|$)/u.test(
            path,
        ) ||
        /(^|\/)(?:\.editorconfig|\.gitignore|package-lock\.json|project\.json)$/u.test(path) ||
        /(?:\.tsbuildinfo|\.tmp|\.cache)$/u.test(path)
    );
}

function contentType(filePath) {
    switch (extname(filePath)) {
        case '.html':
            return 'text/html; charset=utf-8';
        case '.js':
            return 'text/javascript; charset=utf-8';
        case '.json':
            return 'application/json; charset=utf-8';
        case '.wasm':
            return 'application/wasm';
        case '.css':
            return 'text/css; charset=utf-8';
        case '.svg':
            return 'image/svg+xml';
        case '.png':
            return 'image/png';
        default:
            return 'application/octet-stream';
    }
}

function npmCommand() {
    return process.platform === 'win32' ? 'npm.cmd' : 'npm';
}

function run(command, args, cwd) {
    const result = spawnSync(command, args, { cwd, encoding: 'utf8' });
    if (result.status !== 0) {
        throw new Error(`${command} ${args.join(' ')} failed:\n${result.stderr || result.stdout || result.error}`);
    }
}

function runCapture(command, args, cwd) {
    const result = spawnSync(command, args, { cwd, encoding: 'utf8' });
    if (result.status !== 0) {
        throw new Error(`${command} ${args.join(' ')} failed:\n${result.stderr || result.stdout || result.error}`);
    }
    return result.stdout.trim();
}

function readJson(path) {
    return JSON.parse(readFileSync(path, 'utf8'));
}
