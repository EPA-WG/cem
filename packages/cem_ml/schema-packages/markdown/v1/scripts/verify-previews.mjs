#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/markdown/v1');
const readmePath = join(packageRoot, 'README.md');
const generatedRoot = join(
    workspaceRoot,
    'dist/cem_ml/schema-packages/markdown/v1/examples',
);
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const packageLabel = 'Markdown';

const examples = [
    {
        id: 'basic-document',
        file: 'basic-document.md',
        contentType: 'text/markdown; charset=utf-8; variant=CommonMark',
        expectedResult: 'pass',
    },
    {
        id: 'gfm-worklog',
        file: 'gfm-worklog.md',
        contentType: 'text/markdown; charset=utf-8; variant=GFM',
        expectedResult: 'pass',
    },
    {
        id: 'markdown-html-svg',
        file: 'markdown1.md',
        contentType: 'text/markdown; charset=utf-8; variant=CommonMark',
        expectedResult: 'pass',
    },
    {
        id: 'invalid-embedded-html',
        file: 'invalid-embedded-html.md',
        contentType: 'text/markdown; charset=utf-8; variant=CommonMark',
        expectedResult: 'fail',
    },
    {
        id: 'unknown-variant',
        file: 'unknown-variant.md',
        contentType: 'text/markdown; charset=utf-8; variant=CustomWiki',
        expectedResult: 'pass',
    },
];
const expectedHtmlFiles = new Set(
    examples
        .filter((example) => example.expectedResult === 'pass')
        .map((example) => `${example.file}.html`),
);

mkdirSync(generatedRoot, { recursive: true });

const cliEnv = { ...process.env };
delete cliEnv.NO_COLOR;

const readme = readFileSync(readmePath, 'utf8');
const failures = [];

if (readme.includes('examples/previews/') || readme.includes('.svg)')) {
    failures.push('README must not reference Markdown SVG previews.');
}

const previewRoot = join(packageRoot, 'examples/previews');
if (directoryHasSvgFiles(previewRoot)) {
    failures.push('Markdown examples/previews must not contain generated SVG files.');
}

for (const entry of readdirSync(generatedRoot, { withFileTypes: true })) {
    if (!entry.isFile()) {
        continue;
    }
    if (entry.name.endsWith('.svg')) {
        failures.push(`dist Markdown README preview must not generate SVG: ${entry.name}`);
        continue;
    }
    if (entry.name.endsWith('.html') && !expectedHtmlFiles.has(entry.name)) {
        failures.push(`unexpected dist Markdown README HTML preview: ${entry.name}`);
    }
}

for (const example of examples) {
    if (example.expectedResult !== 'pass') {
        continue;
    }
    const outputRelativePath = `dist/cem_ml/schema-packages/markdown/v1/examples/${example.file}.html`;
    const outputPath = join(workspaceRoot, outputRelativePath);
    runCli([
        'convert',
        '--input-spec',
        markdownInputSpec(example.file, example.contentType),
        '--to-content-type',
        'text/html',
        '--to-schema',
        'https://cem.dev/ns/data/html/1',
        '--cemt-formatter-profile',
        'tabular',
        '--cemt-color-profile',
        'none',
        '--out',
        outputRelativePath,
    ]);
    const html = readFileSync(outputPath, 'utf8').trimEnd();
    const expectedFence = `\`\`\`html\n${html}\n\`\`\``;
    if (!readme.includes(expectedFence)) {
        failures.push(
            `${example.id} README html snippet is missing or drifted from ${outputRelativePath}.`,
        );
    }
}

if (failures.length > 0) {
    for (const failure of failures) {
        console.error(`${packageLabel} README preview drift: ${failure}`);
    }
    console.error(
        'Refresh previews with: yarn nx run cem_ml_schema_package_markdown_v1:samples2readme',
    );
    process.exit(1);
}

console.log('Markdown README HTML snippets verified.');

function markdownInputSpec(file, contentType) {
    const path = `packages/cem_ml/schema-packages/markdown/v1/examples/${file}`;
    return `uri=${path},contentType=${contentType},schema=https://cem.dev/ns/data/markdown/1`;
}

function runCli(args) {
    const result = spawnSync(cli, args, {
        cwd: workspaceRoot,
        encoding: 'utf8',
        env: cliEnv,
        maxBuffer: 1024 * 1024,
    });
    if (result.status !== 0) {
        throw new Error(
            [
                `${packageLabel} README preview command failed: ${cli} ${args.join(' ')}`,
                `exit status: ${result.status}`,
                result.stdout,
                result.stderr,
            ]
                .filter(Boolean)
                .join('\n'),
        );
    }
}

function directoryHasSvgFiles(directory) {
    if (!existsSync(directory)) {
        return false;
    }
    return readdirSync(directory, { withFileTypes: true }).some(
        (entry) => entry.isFile() && entry.name.endsWith('.svg'),
    );
}
