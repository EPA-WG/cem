#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, '../../../../../..');
const packageRoot = join(workspaceRoot, 'packages/cem_ml/schema-packages/cem-ql/v1');
const previewRoot = join(packageRoot, 'examples/previews');
const generatedRoot = join(packageRoot, 'dist/previews');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const update = process.argv.includes('--update');
const cliEnv = { ...process.env };
delete cliEnv.NO_COLOR;
const defaultFormatterTabSize = 8;

const commonInputArgs = [
    'packages/cem_ml/schema-packages/cem-ql/v1/examples/basic-query.cemql',
    '--content-type',
    'application/vnd.cem.query+cem-ql',
    '--schema',
    'https://cem.dev/ns/query/cem-ql/1',
];

const cases = [
    {
        id: 'basic-query-validate',
        preview: 'basic-query-validate.svg',
        title: 'CEM-QL validation command preview',
        description: 'Terminal-style preview of the JSON validation report for the basic CEM-QL query example.',
        terminalTitle: 'validate basic-query.cemql',
        renderer: 'json',
        args: [
            'validate',
            '--format',
            'json',
            '--content-type',
            'application/vnd.cem.query+cem-ql',
            '--schema',
            'https://cem.dev/ns/query/cem-ql/1',
            'packages/cem_ml/schema-packages/cem-ql/v1/examples/basic-query.cemql',
        ],
    },
    {
        id: 'basic-query-tabular-terminal',
        preview: 'basic-query-tabular-terminal.svg',
        title: 'CEM-QL tabular formatter terminal preview',
        description: 'Terminal-style preview of colored tabular CEM-QL output for the basic query example.',
        terminalTitle: 'tabular + terminal color',
        renderer: 'ansi',
        args: [
            'convert',
            ...commonInputArgs,
            '--to-content-type',
            'application/vnd.cem.query+cem-ql',
            '--to-schema',
            'https://cem.dev/ns/query/cem-ql/1',
            '--cemt-formatter',
            'cem-ql.format-tree',
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-colorizer',
            'cem-ql.color-tree',
            '--cemt-color-profile',
            'terminal',
            '--output-color-type',
            'ansi-256',
        ],
    },
    {
        id: 'basic-query-tabular-html',
        preview: 'basic-query-tabular-html.svg',
        title: 'CEM-QL tabular formatter HTML preview',
        description: 'Rendered preview of HTML color output for the basic CEM-QL query example.',
        terminalTitle: 'tabular + HTML color',
        renderer: 'html',
        args: [
            'convert',
            ...commonInputArgs,
            '--to-content-type',
            'text/html',
            '--to-schema',
            'https://cem.dev/ns/data/html/1',
            '--cemt-formatter',
            'cem-ql.format-tree',
            '--cemt-formatter-profile',
            'tabular',
            '--cemt-colorizer',
            'cem-ql.color-tree',
            '--cemt-color-profile',
            'html',
            '--output-color-type',
            'html-css-vars',
        ],
    },
];

mkdirSync(previewRoot, { recursive: true });
mkdirSync(generatedRoot, { recursive: true });

const failures = [];
for (const testCase of cases) {
    const stdout = runCli(testCase);
    const svg =
        testCase.renderer === 'json'
            ? renderJsonPreview(testCase, normalizeJson(stdout))
            : testCase.renderer === 'html'
              ? renderStyledPreview(testCase, parseHtmlPreview(stdout))
              : renderStyledPreview(testCase, parseAnsi(stdout));
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
        console.error(`CEM-QL preview drift: ${failure.id}`);
        console.error(`  expected:  ${relative(workspaceRoot, failure.expectedPath)}`);
        console.error(`  generated: ${relative(workspaceRoot, failure.generatedPath)}`);
        console.error(`  ${failure.message}`);
    }
    console.error('Refresh previews with: node packages/cem_ml/schema-packages/cem-ql/v1/scripts/verify-previews.mjs --update');
    process.exit(1);
}

console.log(update ? 'Updated CEM-QL README SVG previews.' : 'CEM-QL README SVG previews verified.');

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
                `CEM-QL preview command failed for ${testCase.id}: ${cli} ${testCase.args.join(' ')}`,
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

function renderStyledPreview(testCase, lines) {
    const visibleLines = lines.filter((line) => line.some((span) => span.text.length > 0));
    const height = Math.max(190, 58 + visibleLines.length * 26 + 24);
    return [
        `<svg xmlns="http://www.w3.org/2000/svg" width="760" height="${height}" viewBox="0 0 760 ${height}" role="img" aria-labelledby="title desc">`,
        `  <title id="title">${escapeXml(testCase.title)}</title>`,
        `  <desc id="desc">${escapeXml(testCase.description)}</desc>`,
        `  <rect width="760" height="${height}" rx="8" fill="#101316"/>`,
        '  <rect x="0" y="0" width="760" height="36" rx="8" fill="#1b2127"/>',
        '  <circle cx="20" cy="18" r="5" fill="#ff5f56"/>',
        '  <circle cx="38" cy="18" r="5" fill="#ffbd2e"/>',
        '  <circle cx="56" cy="18" r="5" fill="#27c93f"/>',
        `  <text x="78" y="23" fill="#c9d1d9" font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" font-size="13">${escapeXml(testCase.terminalTitle)}</text>`,
        '  <g font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" font-size="18" xml:space="preserve">',
        ...visibleLines.flatMap((line, index) => renderStyledLine(line, 70 + index * 28)),
        '  </g>',
        '</svg>',
        '',
    ].join('\n');
}

function renderStyledLine(line, y) {
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
            const weight = span.fontWeight ? ` font-weight="${span.fontWeight}"` : '';
            tspans.push(
                `<tspan x="${28 + bufferColumn * 11}" fill="${span.fill}"${weight}>${escapeXml(buffer)}</tspan>`,
            );
            buffer = '';
        }
    }
    return [`    <text y="${y}">`, `      ${tspans.join('')}`, '    </text>'];
}

function parseAnsi(input) {
    const lines = [[]];
    let fill = '#d6dee6';
    let fontWeight = null;
    let buffer = '';
    for (let index = 0; index < input.length; index += 1) {
        const char = input[index];
        if (char === '\u001b' && input[index + 1] === '[') {
            flush();
            const end = input.indexOf('m', index + 2);
            if (end === -1) {
                continue;
            }
            ({ fill, fontWeight } = applySgr(input.slice(index + 2, end), fill, fontWeight));
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
        lines[lines.length - 1].push({ fill, fontWeight, text: buffer });
        buffer = '';
    }
}

function applySgr(raw, currentFill, currentWeight) {
    const codes = raw
        .split(';')
        .filter((part) => part.length > 0)
        .map((part) => Number.parseInt(part, 10));
    if (codes.length === 0 || codes.includes(0)) {
        return { fill: '#d6dee6', fontWeight: null };
    }
    let fill = currentFill;
    let fontWeight = currentWeight;
    for (let index = 0; index < codes.length; index += 1) {
        if (codes[index] === 1) {
            fontWeight = '600';
        }
        if (codes[index] === 22) {
            fontWeight = null;
        }
        if (codes[index] === 38 && codes[index + 1] === 5) {
            fill = ansi256Color(codes[index + 2]);
            index += 2;
        }
    }
    return { fill, fontWeight };
}

function ansi256Color(code) {
    switch (code) {
        case 76:
            return '#5fd75f';
        case 81:
            return '#5fd7ff';
        case 141:
            return '#af87ff';
        case 244:
            return '#8b949e';
        default:
            return '#d6dee6';
    }
}

function parseHtmlPreview(input) {
    const preStart = input.indexOf('>');
    const preEnd = input.lastIndexOf('</pre>');
    const content = preStart === -1 || preEnd === -1 ? input : input.slice(preStart + 1, preEnd);
    const lines = [[]];
    let fill = '#d6dee6';
    let fontWeight = null;
    let buffer = '';

    for (let index = 0; index < content.length; index += 1) {
        if (content.startsWith('<span ', index)) {
            flush();
            const end = content.indexOf('>', index);
            if (end === -1) {
                break;
            }
            const tag = content.slice(index, end + 1);
            fill = htmlSpanColor(tag);
            fontWeight = tag.includes('font-weight: 600') ? '600' : null;
            index = end;
            continue;
        }
        if (content.startsWith('</span>', index)) {
            flush();
            fill = '#d6dee6';
            fontWeight = null;
            index += '</span>'.length - 1;
            continue;
        }
        if (content[index] === '<') {
            flush();
            const end = content.indexOf('>', index);
            if (end === -1) {
                break;
            }
            index = end;
            continue;
        }
        const char = content[index];
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
        lines[lines.length - 1].push({
            fill,
            fontWeight,
            text: decodeHtmlEntities(buffer),
        });
        buffer = '';
    }
}

function htmlSpanColor(tag) {
    const directColor = /color:\s*(#[0-9a-fA-F]{6})/.exec(tag);
    if (directColor) {
        return directColor[1].toLowerCase();
    }
    const fallbackColor = /color:\s*var\([^,]+,\s*(#[0-9a-fA-F]{6})\)/.exec(tag);
    if (fallbackColor) {
        return fallbackColor[1].toLowerCase();
    }
    return '#d6dee6';
}

function decodeHtmlEntities(value) {
    return value
        .replaceAll('&quot;', '"')
        .replaceAll('&lt;', '<')
        .replaceAll('&gt;', '>')
        .replaceAll('&#39;', "'")
        .replaceAll('&amp;', '&');
}

function nextTabColumn(column) {
    return (Math.floor(column / defaultFormatterTabSize) + 1) * defaultFormatterTabSize;
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
