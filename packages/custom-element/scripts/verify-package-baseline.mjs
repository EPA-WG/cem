import { access, readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const roots = [projectRoot, join(projectRoot, 'dist')];

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

for (const root of roots) {
    await verifyRoot(root);
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
    assertIncludes(customElementSource, "window.customElements.define( 'custom-element'", `${root}: custom-element registration`);

    const httpRequestSource = await readFile(join(root, 'http-request.js'), 'utf8');
    assertIncludes(httpRequestSource, "window.customElements.define( 'http-request'", `${root}: http-request registration`);

    const localStorageSource = await readFile(join(root, 'local-storage.js'), 'utf8');
    assertIncludes(localStorageSource, "window.customElements.define( 'local-storage'", `${root}: local-storage registration`);

    const locationSource = await readFile(join(root, 'location-element.js'), 'utf8');
    assertIncludes(locationSource, "window.customElements.define( 'location-element'", `${root}: location-element registration`);

    const moduleUrlSource = await readFile(join(root, 'module-url.js'), 'utf8');
    assertIncludes(moduleUrlSource, "window.customElements.define( 'module-url'", `${root}: module-url registration`);
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
