#!/usr/bin/env node

import { existsSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/markdown/v1');
const readmePath = join(packageRoot, 'README.md');
const packageLabel = 'Markdown';

const examples = [
    { id: 'basic-document', file: 'basic-document.md' },
    { id: 'gfm-worklog', file: 'gfm-worklog.md' },
    { id: 'markdown-html-svg', file: 'markdown1.md' },
    { id: 'invalid-embedded-html', file: 'invalid-embedded-html.md' },
    { id: 'unknown-variant', file: 'unknown-variant.md' },
];

const readme = readFileSync(readmePath, 'utf8');
const failures = [];

if (readme.includes('examples/previews/') || readme.includes('.svg)')) {
    failures.push('README must not reference Markdown SVG previews.');
}

const previewRoot = join(packageRoot, 'examples/previews');
if (directoryHasSvgFiles(previewRoot)) {
    failures.push('Markdown examples/previews must not contain generated SVG files.');
}

for (const example of examples) {
    const source = readFileSync(join(packageRoot, 'examples', example.file), 'utf8')
        .replace(/\r\n/g, '\n')
        .replace(/\r/g, '\n')
        .trimEnd();
    const delimiter = markdownFenceDelimiter(source);
    const expectedFence = `${delimiter}markdown\n${source}\n${delimiter}`;
    if (!readme.includes(expectedFence)) {
        failures.push(
            `${example.id} README source fence is missing or drifted from examples/${example.file}.`,
        );
    }
}

if (failures.length > 0) {
    for (const failure of failures) {
        console.error(`${packageLabel} README source-fence drift: ${failure}`);
    }
    console.error(
        'Refresh previews with: yarn nx run cem_ml_schema_package_markdown_v1:samples2readme',
    );
    process.exit(1);
}

console.log('Markdown README source fences verified.');

function markdownFenceDelimiter(source) {
    const longestRun = Math.max(0, ...[...source.matchAll(/`+/g)].map((match) => match[0].length));
    return '`'.repeat(Math.max(3, longestRun + 1));
}

function directoryHasSvgFiles(directory) {
    if (!existsSync(directory)) {
        return false;
    }
    return readdirSync(directory, { withFileTypes: true }).some(
        (entry) => entry.isFile() && entry.name.endsWith('.svg'),
    );
}
