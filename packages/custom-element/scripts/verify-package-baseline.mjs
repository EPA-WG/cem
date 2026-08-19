import { access, readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const distRoot = join(projectRoot, 'dist');
const roots = [projectRoot, distRoot];

const requiredFiles = [
    'LICENSE',
    'README.md',
    'custom-element.d.ts',
    'custom-element.js',
    'http-request.js',
    'index.js',
    'local-storage.js',
    'location-element.js',
    'module-url.js',
    'package.json',
    'ide/web-types-dce.json',
    'ide/web-types-xsl.json',
];

await verifyProjectConfig();
for (const root of roots) {
    await verifyRoot(root);
}

await verifyDistRuntime(distRoot);

async function verifyProjectConfig() {
    const projectJson = JSON.parse(await readFile(join(projectRoot, 'project.json'), 'utf8'));
    assertEqual(projectJson.name, '@epa-wg/custom-element', 'project name');
    assertArrayIncludes(projectJson.targets?.verify?.dependsOn, 'build', 'verify target dependsOn');
    assertArrayIncludes(projectJson.targets?.verify?.dependsOn, 'test', 'verify target dependsOn');
    assertArrayIncludes(projectJson.targets?.verify?.dependsOn, 'lint', 'verify target dependsOn');
    assertArrayIncludes(projectJson.targets?.test?.dependsOn, 'build', 'test target dependsOn');
    assertCommandIncludes(projectJson.targets?.test?.options?.commands, 'node scripts/verify-browser-fixtures.mjs', 'test target commands');
    assertCommandIncludes(projectJson.targets?.test?.options?.commands, 'node scripts/verify-package-baseline.mjs', 'test target commands');
    assertCommandIncludes(projectJson.targets?.test?.options?.commands, 'node scripts/verify-theme-vendor-runtime.mjs', 'test target commands');
    assertEqual(
        projectJson.targets?.['nx-release-publish']?.options?.packageRoot,
        'packages/custom-element/dist',
        'release publish packageRoot'
    );
    assertArrayIncludes(
        projectJson.release?.version?.manifestRootsToUpdate,
        'packages/custom-element/dist',
        'release version manifestRootsToUpdate'
    );

    if (!dependsOnProjectTarget(projectJson.targets?.build?.dependsOn, 'cem-elements', 'build')) {
        throw new Error('build target dependsOn: expected cem-elements:build');
    }
}

async function verifyRoot(root) {
    const packageJson = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'));
    assertEqual(packageJson.name, '@epa-wg/custom-element', `${root}: package name`);
    assertEqual(packageJson.type, 'module', `${root}: package type`);
    assertEqual(packageJson.browser, 'custom-element.js', `${root}: browser entrypoint`);
    assertEqual(packageJson.module, 'custom-element.js', `${root}: module entrypoint`);
    assertEqual(packageJson.types, './custom-element.d.ts', `${root}: types entrypoint`);
    assertEqual(packageJson.exports?.['.'], './index.js', `${root}: root export`);
    assertEqual(packageJson.exports?.['./CustomElement'], './custom-element.js', `${root}: CustomElement export`);
    assertEqual(packageJson.exports?.['./package.json'], './package.json', `${root}: package export`);

    for (const file of requiredFiles) {
        await access(join(root, file));
    }

    const customElementSource = await readFile(join(root, 'custom-element.js'), 'utf8');
    assertIncludes(customElementSource, 'window.customElements.define(', `${root}: custom-element registration`);
    assertIncludes(customElementSource, "'custom-element'", `${root}: custom-element tag literal`);
    assertIncludes(customElementSource, 'CemElementRuntime', `${root}: substrate runtime import`);
    assertIncludes(
        customElementSource,
        "const LEGACY_TEMPLATE_LANG = 'custom-element-v0'",
        `${root}: browser legacy selector`
    );
    assertNotIncludes(
        customElementSource,
        'LEGACY_CUSTOM_ELEMENT_TEMPLATE_LANG',
        `${root}: native converter identity must not select browser legacy mode`
    );
    assertNotIncludes(customElementSource, 'XSLTProcessor', `${root}: adapter must not use XSLTProcessor`);
    assertNotIncludes(customElementSource, 'createXsltFromDom', `${root}: adapter must not keep XSLT compiler`);
    assertNotIncludes(customElementSource, 'class DceElement', `${root}: adapter must not define legacy produced class`);
    if (root === distRoot) {
        assertIncludes(
            customElementSource,
            "from './vendor/@epa-wg/cem-elements/dist/index.js'",
            `${root}: dist substrate import`
        );
        assertNotIncludes(
            customElementSource,
            "from '../cem-elements/dist/index.js'",
            `${root}: dist must not reference workspace runtime path`
        );
    } else {
        assertIncludes(
            customElementSource,
            "from '../cem-elements/dist/index.js'",
            `${root}: source substrate import`
        );
    }

    const httpRequestSource = await readFile(join(root, 'http-request.js'), 'utf8');
    assertIncludes(httpRequestSource, "window.customElements.define( 'http-request'", `${root}: http-request registration`);

    const localStorageSource = await readFile(join(root, 'local-storage.js'), 'utf8');
    assertIncludes(localStorageSource, "window.customElements.define( 'local-storage'", `${root}: local-storage registration`);

    const locationSource = await readFile(join(root, 'location-element.js'), 'utf8');
    assertIncludes(locationSource, "window.customElements.define( 'location-element'", `${root}: location-element registration`);

    const moduleUrlSource = await readFile(join(root, 'module-url.js'), 'utf8');
    assertIncludes(moduleUrlSource, "window.customElements.define( 'module-url'", `${root}: module-url registration`);
}

async function verifyDistRuntime(root) {
    const runtimeFiles = [
        'vendor/@epa-wg/cem-elements/dist/index.js',
        'vendor/@epa-wg/cem-elements/dist/lib/cem-elements.js',
        'vendor/@epa-wg/cem-elements/dist/lib/projection.js',
        'vendor/@epa-wg/cem-elements/dist/lib/internal/runtime-support/cem-ql-render.js',
        'vendor/@epa-wg/cem_ql/dist/wasm/cem_ql.js',
        'vendor/@epa-wg/cem_ql/dist/wasm/cem_ql_bg.wasm',
        'vendor/@epa-wg/cem_ql/dist/wasm/package.json',
    ];
    for (const file of runtimeFiles) {
        await access(join(root, file));
    }
}

function assertEqual(actual, expected, label) {
    if (actual !== expected) {
        throw new Error(`${label}: expected ${expected}, got ${actual}`);
    }
}

function assertIncludes(value, expected, label) {
    if (!value.includes(expected)) {
        throw new Error(`${label}: expected to include ${expected}`);
    }
}

function assertNotIncludes(value, expected, label) {
    if (value.includes(expected)) {
        throw new Error(`${label}: expected not to include ${expected}`);
    }
}

function assertArrayIncludes(value, expected, label) {
    if (!Array.isArray(value) || !value.includes(expected)) {
        throw new Error(`${label}: expected to include ${expected}`);
    }
}

function assertCommandIncludes(value, expected, label) {
    if (!Array.isArray(value) || !value.includes(expected)) {
        throw new Error(`${label}: expected command ${expected}`);
    }
}

function dependsOnProjectTarget(dependsOn, project, target) {
    return Array.isArray(dependsOn) && dependsOn.some((entry) =>
        typeof entry === 'object' &&
        Array.isArray(entry.projects) &&
        entry.projects.includes(project) &&
        entry.target === target
    );
}
