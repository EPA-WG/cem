#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/csv/v1');
const previewRoot = join(packageRoot, 'examples/previews');
const generatedRoot = join(packageRoot, 'dist/previews');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const cliEnv = { ...process.env };
delete cliEnv.NO_COLOR;

const commonInputArgs = [
    'packages/cem_ml/schema-packages/csv/v1/examples/basic-table.csv',
    '--content-type',
    'text/csv',
    '--schema',
    'https://cem.dev/ns/data/csv/1',
];

const cases = [
    {
        id: 'basic-table-validate',
        preview: 'basic-table-validate.svg',
        title: 'CSV validation command preview',
        description: 'Terminal-style preview of the JSON validation report for the basic CSV table example.',
        terminalTitle: 'validate basic-table.csv',
        renderer: 'json',
        args: [
            'validate',
            '--format',
            'json',
            '--content-type',
            'text/csv',
            '--schema',
            'https://cem.dev/ns/data/csv/1',
            'packages/cem_ml/schema-packages/csv/v1/examples/basic-table.csv',
        ],
    },
    {
        id: 'basic-table-pretty-terminal',
        preview: 'basic-table-pretty-terminal.svg',
        title: 'CSV pretty formatter terminal preview',
        description: 'Terminal-style preview of colored pretty CSV output with tab-based near alignment.',
        terminalTitle: 'pretty + terminal color',
        renderer: 'ansi',
        args: [
            'convert',
            ...commonInputArgs,
            '--to-content-type',
            'text/csv',
            '--to-schema',
            'https://cem.dev/ns/data/csv/1',
            '--cemt-formatter-profile',
            'pretty',
            '--cemt-color-profile',
            'terminal',
            '--output-color-type',
            'ansi-256',
        ],
    },
    {
        id: 'basic-table-tabular-terminal',
        preview: 'basic-table-tabular-terminal.svg',
        title: 'CSV tabular formatter terminal preview',
        description: 'Terminal-style preview of colored tabular CSV output with vertically aligned delimiters.',
        terminalTitle: 'tabular + terminal color',
        renderer: 'ansi',
        args: [
            'convert',
            ...commonInputArgs,
            '--to-content-type',
            'text/csv',
            '--to-schema',
            'https://cem.dev/ns/data/csv/1',
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-formatter-option',
            'csv.maxFieldWidth=24',
            '--cemt-formatter-option',
            'csv.stringTrim=middle',
            '--cemt-color-profile',
            'terminal',
            '--output-color-type',
            'ansi-256',
        ],
    },
];

mkdirSync(generatedRoot, { recursive: true });

const failures = [];
for (const testCase of cases) {
    const stdout = runCli(testCase);
    const svg =
        testCase.renderer === 'json'
            ? renderJsonPreview(testCase, normalizeJson(stdout))
            : renderAnsiPreview(testCase, stdout);
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
        console.error(`CSV preview drift: ${failure.id}`);
        console.error(`  expected:  ${relative(workspaceRoot, failure.expectedPath)}`);
        console.error(`  generated: ${relative(workspaceRoot, failure.generatedPath)}`);
        console.error(`  ${failure.message}`);
    }
    console.error('Refresh previews with: node packages/cem_ml/schema-packages/csv/v1/scripts/verify-previews.mjs --update');
    process.exit(1);
}

console.log(update ? 'Updated CSV README SVG previews.' : 'CSV README SVG previews verified.');

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
                `CSV preview command failed for ${testCase.id}: ${cli} ${testCase.args.join(' ')}`,
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
    return [
        '<svg xmlns="http://www.w3.org/2000/svg" width="920" height="570" viewBox="0 0 920 570" role="img" aria-labelledby="title desc">',
        `  <title id="title">${escapeXml(testCase.title)}</title>`,
        `  <desc id="desc">${escapeXml(testCase.description)}</desc>`,
        '  <rect width="920" height="570" rx="8" fill="#101316"/>',
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

function renderAnsiPreview(testCase, stdout) {
    const lines = parseAnsi(stdout).filter((line) => line.some((span) => span.text.length > 0));
    return [
        '<svg xmlns="http://www.w3.org/2000/svg" width="520" height="160" viewBox="0 0 520 160" role="img" aria-labelledby="title desc">',
        `  <title id="title">${escapeXml(testCase.title)}</title>`,
        `  <desc id="desc">${escapeXml(testCase.description)}</desc>`,
        '  <rect width="520" height="160" rx="8" fill="#101316"/>',
        '  <rect x="0" y="0" width="520" height="36" rx="8" fill="#1b2127"/>',
        '  <circle cx="20" cy="18" r="5" fill="#ff5f56"/>',
        '  <circle cx="38" cy="18" r="5" fill="#ffbd2e"/>',
        '  <circle cx="56" cy="18" r="5" fill="#27c93f"/>',
        `  <text x="78" y="23" fill="#c9d1d9" font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" font-size="13">${escapeXml(testCase.terminalTitle)}</text>`,
        '  <g font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" font-size="18" xml:space="preserve">',
        ...lines.flatMap((line, index) => renderAnsiLine(line, 70 + index * 28)),
        '  </g>',
        '</svg>',
        '',
    ].join('\n');
}

function renderAnsiLine(line, y) {
    const tspans = [];
    let column = 0;
    for (const span of line) {
        let buffer = '';
        let bufferColumn = column;
        for (const char of span.text) {
            if (char === '\t') {
                flush();
                column = nextTabColumn(column);
                bufferColumn = column;
            } else {
                if (buffer.length === 0) {
                    bufferColumn = column;
                }
                buffer += char;
                column += displayWidth(char);
            }
        }
        flush();

        function flush() {
            if (buffer.length === 0) {
                return;
            }
            tspans.push(
                `<tspan x="${28 + bufferColumn * 11}" fill="${span.fill}">${escapeXml(buffer)}</tspan>`,
            );
            buffer = '';
        }
    }
    return [`    <text y="${y}">`, `      ${tspans.join('')}`, '    </text>'];
}

function parseAnsi(input) {
    const lines = [[]];
    let fill = '#d6dee6';
    let buffer = '';
    for (let index = 0; index < input.length; index += 1) {
        const char = input[index];
        if (char === '\u001b' && input[index + 1] === '[') {
            flush();
            const end = input.indexOf('m', index + 2);
            if (end === -1) {
                continue;
            }
            fill = applySgr(input.slice(index + 2, end), fill);
            index = end;
            continue;
        }
        if (char === '\r') {
            continue;
        }
        if (char === '\n') {
            flush();
            lines.push([]);
            continue;
        }
        buffer += char;
    }
    flush();
    return lines;

    function flush() {
        if (buffer.length === 0) {
            return;
        }
        lines[lines.length - 1].push({ fill, text: buffer });
        buffer = '';
    }
}

function applySgr(raw, currentFill) {
    const codes = raw
        .split(';')
        .filter((part) => part.length > 0)
        .map((part) => Number.parseInt(part, 10));
    if (codes.length === 0 || codes.includes(0)) {
        return '#d6dee6';
    }
    for (let index = 0; index < codes.length - 2; index += 1) {
        if (codes[index] === 38 && codes[index + 1] === 5) {
            return ansi256Color(codes[index + 2]);
        }
    }
    return currentFill;
}

function ansi256Color(code) {
    switch (code) {
        case 75:
            return '#5fafff';
        case 76:
            return '#5fd75f';
        case 208:
            return '#ff8700';
        case 244:
            return '#8b949e';
        default:
            return '#d6dee6';
    }
}

function nextTabColumn(column) {
    return (Math.floor(column / 8) + 1) * 8;
}

function displayWidth(char) {
    return char.codePointAt(0) < 0x20 ? 0 : 1;
}

function firstDifference(expected, actual) {
    const expectedLines = expected.split('\n');
    const actualLines = actual.split('\n');
    const max = Math.max(expectedLines.length, actualLines.length);
    for (let index = 0; index < max; index += 1) {
        if (expectedLines[index] !== actualLines[index]) {
            return `first differing line ${index + 1}`;
        }
    }
    return `length differs: expected ${expected.length} bytes, generated ${actual.length} bytes`;
}

function escapeXml(value) {
    return value
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;');
}
