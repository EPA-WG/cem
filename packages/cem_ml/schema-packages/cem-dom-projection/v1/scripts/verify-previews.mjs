#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/cem-dom-projection/v1');
const previewRoot = join(packageRoot, 'examples/previews');
const generatedRoot = join(packageRoot, 'dist/previews');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const cliEnv = { ...process.env };
delete cliEnv.NO_COLOR;

const cases = [
    {
        id: 'basic-dom-binary-validate',
        preview: 'basic-dom-binary-validate.svg',
        title: 'CEM DOM binary validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic binary CEM DOM projection example.',
        terminalTitle: 'validate basic-dom.cem-bin',
        args: [
            'validate',
            '--format',
            'json',
            '--content-type',
            'application/vnd.cem.dom+cem-bin',
            '--schema',
            'https://cem.dev/ns/projection/dom/1',
            'packages/cem_ml/schema-packages/cem-dom-projection/v1/examples/basic-dom.cem-bin',
        ],
    },
    {
        id: 'basic-dom-json-validate',
        preview: 'basic-dom-json-validate.svg',
        title: 'CEM DOM JSON validation command preview',
        description:
            'Terminal-style preview of the JSON validation report for the basic CEM DOM JSON debug view example.',
        terminalTitle: 'validate basic-dom.dom.json',
        args: [
            'validate',
            '--format',
            'json',
            '--content-type',
            'application/vnd.cem.dom+json',
            '--schema',
            'https://cem.dev/ns/projection/dom/1',
            'packages/cem_ml/schema-packages/cem-dom-projection/v1/examples/basic-dom.dom.json',
        ],
    },
];

mkdirSync(previewRoot, { recursive: true });
mkdirSync(generatedRoot, { recursive: true });

const failures = [];
for (const testCase of cases) {
    const stdout = normalizeJson(runCli(testCase));
    const svg = renderJsonPreview(testCase, stdout);
    const expectedPath = join(previewRoot, testCase.preview);
    const generatedPath = join(generatedRoot, testCase.preview);
    writeFileSync(generatedPath, svg, 'utf8');
    if (update) {
        writeFileSync(expectedPath, svg, 'utf8');
        continue;
    }
    const expected = readFileSync(expectedPath, 'utf8');
    if (expected !== svg) {
        failures.push({
            id: testCase.id,
            expectedPath,
            generatedPath,
            message: firstDifference(expected, svg),
        });
    }
}

if (failures.length > 0) {
    for (const failure of failures) {
        console.error(`CEM DOM projection preview drift: ${failure.id}`);
        console.error(`  expected:  ${relative(workspaceRoot, failure.expectedPath)}`);
        console.error(`  generated: ${relative(workspaceRoot, failure.generatedPath)}`);
        console.error(`  ${failure.message}`);
    }
    console.error(
        'Refresh previews with: node packages/cem_ml/schema-packages/cem-dom-projection/v1/scripts/verify-previews.mjs --update',
    );
    process.exit(1);
}

console.log(
    update
        ? 'Updated CEM DOM projection README SVG previews.'
        : 'CEM DOM projection README SVG previews verified.',
);

function runCli(testCase) {
    const result = spawnSync(cli, testCase.args, {
        cwd: workspaceRoot,
        encoding: 'utf8',
        env: cliEnv,
        maxBuffer: 1024 * 1024,
    });
    if (result.status !== 0) {
        throw new Error(
            [
                `CEM DOM projection preview command failed for ${testCase.id}: ${cli} ${testCase.args.join(' ')}`,
                result.stdout,
                result.stderr,
            ]
                .filter(Boolean)
                .join('\n'),
        );
    }
    return result.stdout;
}

function normalizeJson(stdout) {
    return `${JSON.stringify(JSON.parse(stdout), null, 2)}\n`;
}

function renderJsonPreview(testCase, stdout) {
    const lines = stdout.trimEnd().split('\n');
    const height = Math.max(570, 88 + lines.length * 19);
    return [
        `<svg xmlns="http://www.w3.org/2000/svg" width="920" height="${height}" viewBox="0 0 920 ${height}" role="img" aria-labelledby="title desc">`,
        `  <title id="title">${escapeXml(testCase.title)}</title>`,
        `  <desc id="desc">${escapeXml(testCase.description)}</desc>`,
        `  <rect width="920" height="${height}" rx="8" fill="#101316"/>`,
        '  <rect x="0" y="0" width="920" height="36" rx="8" fill="#1b2127"/>',
        '  <circle cx="20" cy="18" r="5" fill="#ff5f56"/>',
        '  <circle cx="38" cy="18" r="5" fill="#ffbd2e"/>',
        '  <circle cx="56" cy="18" r="5" fill="#27c93f"/>',
        `  <text x="78" y="23" fill="#c9d1d9" font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" font-size="13">${escapeXml(testCase.terminalTitle)}</text>`,
        '  <text x="24" y="64" fill="#d6dee6" font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" font-size="15" xml:space="preserve">',
        ...lines.map((line, index) => `    <tspan x="24" dy="${index === 0 ? 0 : 19}">${escapeXml(line)}</tspan>`),
        '  </text>',
        '</svg>',
        '',
    ].join('\n');
}

function escapeXml(value) {
    return value
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;');
}

function firstDifference(expected, actual) {
    const max = Math.max(expected.length, actual.length);
    for (let index = 0; index < max; index += 1) {
        if (expected[index] !== actual[index]) {
            return `first difference at byte ${index}: expected ${JSON.stringify(expected.slice(index, index + 80))}, got ${JSON.stringify(actual.slice(index, index + 80))}`;
        }
    }
    return 'content differs';
}
