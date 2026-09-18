import { access, readFile } from 'node:fs/promises';
import { dirname, join, relative } from 'node:path';
import { fileURLToPath } from 'node:url';
import { glob } from 'glob';

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = dirname(dirname(projectRoot));
const themeDistRoot = join(workspaceRoot, 'packages/cem-theme/dist');
const vendorRoot = join(themeDistRoot, 'vendor/@epa-wg/custom-element');

const runtimeFiles = [
    'custom-element.js',
];

for (const file of runtimeFiles) {
    const source = await readFile(join(projectRoot, file), 'utf8');
    const vendored = await readFile(join(vendorRoot, file), 'utf8');
    assertEqual(vendored, source, `${file} vendored into cem-theme`);
}

for (const retired of [join(vendorRoot, 'http-request.js'), join(themeDistRoot, 'lib/css-generators/cem-http-request.js')]) {
    if (await access(retired).then(() => true, () => false)) {
        throw new Error(`retired HTTP implementation remains in the theme distribution: ${retired}`);
    }
}

const htmlFiles = await glob('**/*.html', { cwd: themeDistRoot });
if (htmlFiles.length === 0) {
    throw new Error('cem-theme dist did not contain compiled HTML files');
}

for (const file of htmlFiles) {
    const html = await readFile(join(themeDistRoot, file), 'utf8');
    assertNotIncludes(
        html,
        'node_modules/@epa-wg/custom-element/',
        `${relative(workspaceRoot, join(themeDistRoot, file))}: custom-element runtime path`
    );
}

function assertEqual(actual, expected, label) {
    if (actual !== expected) {
        throw new Error(`${label}: expected vendored file to match workspace source`);
    }
}

function assertNotIncludes(value, expected, label) {
    if (value.includes(expected)) {
        throw new Error(`${label}: expected not to include ${expected}`);
    }
}
