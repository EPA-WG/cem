#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';

const workspaceRoot = resolve(import.meta.dirname, '../..');
const cli = join(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const options = parseArgs(process.argv.slice(2));
const inputPath =
    options.input ??
    'packages/cem_ml/schema-packages/cem-ml/v1/examples/formatter-coloring-pipeline.package-artifacts.fixture.cem';
const outputRoot = join(workspaceRoot, 'dist/svg');
const outputStem = basename(inputPath).replace(/[^A-Za-z0-9._-]+/g, '-');
const htmlPath = options.htmlOut
    ? resolve(workspaceRoot, options.htmlOut)
    : join(outputRoot, `${outputStem}.html`);
const svgPath = options.svgOut
    ? resolve(workspaceRoot, options.svgOut)
    : join(outputRoot, `${outputStem}.svg`);

const preview = runCliPreview(inputPath);
const html = buildSourceHtml(preview, inputPath);
mkdirSync(dirname(htmlPath), { recursive: true });
writeFileSync(htmlPath, html, 'utf8');
if (options.htmlOnly) {
    console.log(`HTML: ${htmlPath}`);
    process.exit(0);
}

const browser = await chromium.launch({
    headless: false,
    args: ['--no-sandbox'],
});
const page = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
await page.goto(pathToFileURL(htmlPath).href);
await page.waitForSelector('pre[data-preview]');
const svg = await page.evaluate(createSvgFromPreviewDom);
writeFileSync(svgPath, svg, 'utf8');

const svgPage = await browser.newPage({ viewport: { width: 1440, height: 1100 } });
await svgPage.setContent(
    [
        '<!doctype html>',
        '<html>',
        '<head>',
        '<meta charset="utf-8">',
        `<title>${escapeHtml(outputStem)} SVG</title>`,
        '<style>html,body{margin:0;background:#0b0d10;} svg{display:block;}</style>',
        '</head>',
        '<body>',
        svg,
        '</body>',
        '</html>',
        '',
    ].join('\n'),
);
await svgPage.bringToFront();

console.log(`HTML: ${htmlPath}`);
console.log(`SVG:  ${svgPath}`);
console.log('Browser is open with the generated SVG tab focused. Press Ctrl+C here to close it.');
await new Promise(() => {});

function parseArgs(argv) {
    const result = {
        input: null,
        htmlOut: null,
        svgOut: null,
        htmlOnly: false,
    };
    for (let index = 0; index < argv.length; index += 1) {
        const arg = argv[index];
        if (arg === '--html-only') {
            result.htmlOnly = true;
            continue;
        }
        if (arg === '--html-out') {
            result.htmlOut = requireValue(argv, (index += 1), arg);
            continue;
        }
        if (arg === '--svg-out') {
            result.svgOut = requireValue(argv, (index += 1), arg);
            continue;
        }
        if (arg.startsWith('--')) {
            throw new Error(`Unknown option: ${arg}`);
        }
        if (result.input) {
            throw new Error(`Unexpected extra input path: ${arg}`);
        }
        result.input = arg;
    }
    return result;
}

function requireValue(argv, index, option) {
    const value = argv[index];
    if (!value || value.startsWith('--')) {
        throw new Error(`${option} requires a value`);
    }
    return value;
}

function runCliPreview(path) {
    const args = [
        'convert',
        '--input-spec',
        `uri=${path},contentType=application/cem,schema=https://cem.dev/ns/cem-ml/1`,
        '--to-content-type',
        'application/cem',
        '--to-schema',
        'https://cem.dev/ns/cem-ml/1',
        '--cemt-formatter-profile',
        'tabular',
        '--cemt-color-profile',
        'terminal',
        '--output-color-type',
        'ansi-256',
    ];
    const env = { ...process.env };
    delete env.NO_COLOR;
    const result = spawnSync(cli, args, {
        cwd: workspaceRoot,
        encoding: 'utf8',
        env,
        maxBuffer: 1024 * 1024 * 4,
    });
    if (result.status !== 0 && result.stdout.length === 0) {
        throw new Error(
            [
                `preview command failed: ${cli} ${args.join(' ')}`,
                `exit status: ${result.status}`,
                result.stdout,
                result.stderr,
            ]
                .filter(Boolean)
                .join('\n'),
        );
    }
    return result.stdout || result.stderr;
}

function buildSourceHtml(ansi, title) {
    const lines = parseAnsi(ansi);
    const contentHtml = lines
        .filter((line) => line.some((part) => part.text.length > 0))
        .map((line) =>
            `<span class="line">${line
                .map(
                    (part) =>
                        `<span class="token" style="color:${part.fill};${
                            part.fontWeight ? `font-weight:${part.fontWeight};` : ''
                        }">${escapeHtml(part.text)}</span>`,
                )
                .join('')}</span>`,
        )
        .join('');
    return [
        '<!doctype html>',
        '<html>',
        '<head>',
        '<meta charset="utf-8">',
        `<title>${escapeHtml(title)}</title>`,
        '<style>',
        ':root { color-scheme: dark; }',
        'html, body { margin: 0; min-height: 100%; background: #0b0d10; }',
        'body { padding: 28px; }',
        '.terminal { display: inline-block; overflow: visible; border-radius: 8px; background: #101316; color: #d6dee6; }',
        '.header { position: relative; height: 36px; border-radius: 8px 8px 0 0; background: #1b2127; }',
        '.dot { position: absolute; top: 13px; width: 10px; height: 10px; border-radius: 50%; }',
        '.dot.red { left: 15px; background: #ff5f56; }',
        '.dot.yellow { left: 33px; background: #ffbd2e; }',
        '.dot.green { left: 51px; background: #27c93f; }',
        '.terminal-title { position: absolute; left: 78px; top: 8px; color: #c9d1d9; font: 13px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; line-height: 18px; }',
        'pre { margin: 0; padding: 22px 28px 26px; white-space: pre; tab-size: 8; font: 18px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; line-height: 28px; }',
        '.line { display: block; min-height: 28px; white-space: pre; }',
        '.token { white-space: pre; }',
        '</style>',
        '</head>',
        '<body>',
        '<div class="terminal" data-terminal>',
        '<div class="header" data-header><span class="dot red"></span><span class="dot yellow"></span><span class="dot green"></span><span class="terminal-title">',
        escapeHtml(basename(title)),
        '</span></div>',
        `<pre data-preview>${contentHtml}</pre>`,
        '</div>',
        '</body>',
        '</html>',
        '',
    ].join('\n');
}

function createSvgFromPreviewDom() {
    const terminal = document.querySelector('[data-terminal]');
    const header = document.querySelector('[data-header]');
    const pre = document.querySelector('pre[data-preview]');
    const terminalRect = terminal.getBoundingClientRect();
    const preRect = pre.getBoundingClientRect();
    const preStyle = getComputedStyle(pre);
    const width = Math.ceil(Math.max(terminalRect.width, preRect.left - terminalRect.left + pre.scrollWidth + 28));
    const height = Math.ceil(Math.max(terminalRect.height, preRect.top - terminalRect.top + pre.scrollHeight + 26));
    const rows = [];
    for (const line of pre.querySelectorAll('.line')) {
        const lineRect = line.getBoundingClientRect();
        for (const token of line.querySelectorAll('.token')) {
            if (token.textContent.length === 0) {
                continue;
            }
            const rect = token.getBoundingClientRect();
            const style = getComputedStyle(token);
            rows.push({
                text: token.textContent,
                x: round(rect.left - terminalRect.left),
                y: round(lineRect.top - terminalRect.top),
                fill: style.color,
                fontWeight: style.fontWeight,
            });
        }
    }

    const title = document.title;
    const svg = document.implementation.createDocument('http://www.w3.org/2000/svg', 'svg');
    const root = svg.documentElement;
    root.setAttribute('xmlns', 'http://www.w3.org/2000/svg');
    root.setAttribute('width', String(width));
    root.setAttribute('height', String(height));
    root.setAttribute('viewBox', `0 0 ${width} ${height}`);
    root.setAttribute('role', 'img');
    root.setAttribute('aria-labelledby', 'title desc');

    append('title', { id: 'title' }, title);
    append('desc', { id: 'desc' }, `Browser-measured SVG preview for ${title}`);
    append('rect', { x: '0', y: '0', width, height, rx: '8', fill: '#101316' });
    append('rect', { x: '0', y: '0', width, height: Math.ceil(header.getBoundingClientRect().height), rx: '8', fill: '#1b2127' });
    append('rect', { x: '0', y: '28', width, height: '12', fill: '#1b2127' });
    append('circle', { cx: '20', cy: '18', r: '5', fill: '#ff5f56' });
    append('circle', { cx: '38', cy: '18', r: '5', fill: '#ffbd2e' });
    append('circle', { cx: '56', cy: '18', r: '5', fill: '#27c93f' });
    append('text', {
        x: '78',
        y: '8',
        fill: '#c9d1d9',
        'font-family': 'ui-monospace, SFMono-Regular, Menlo, Consolas, monospace',
        'font-size': '13',
        'dominant-baseline': 'text-before-edge',
    }, title.split('/').at(-1) ?? title);

    for (const row of rows) {
        append('text', {
            x: row.x,
            y: row.y,
            fill: row.fill,
            'font-family': preStyle.fontFamily,
            'font-size': preStyle.fontSize,
            'font-weight': row.fontWeight,
            'dominant-baseline': 'text-before-edge',
            'xml:space': 'preserve',
        }, row.text);
    }

    return new XMLSerializer().serializeToString(svg);

    function append(name, attrs, text = null) {
        const node = svg.createElementNS('http://www.w3.org/2000/svg', name);
        for (const [key, value] of Object.entries(attrs)) {
            node.setAttribute(key, String(value));
        }
        if (text !== null) {
            node.textContent = text;
        }
        root.append(node);
    }

    function round(value) {
        return Math.round(value * 100) / 100;
    }
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
        const code = codes[index];
        if (code === 1) {
            fontWeight = '600';
        }
        if (code === 22) {
            fontWeight = null;
        }
        if (code === 38 && codes[index + 1] === 5) {
            fill = ansi256Color(codes[index + 2]);
            index += 2;
        }
    }
    return { fill, fontWeight };
}

function ansi256Color(code) {
    switch (code) {
        case 75:
            return '#5fafff';
        case 76:
            return '#5fd75f';
        case 81:
            return '#5fd7ff';
        case 141:
            return '#af87ff';
        case 208:
        case 214:
            return '#ff8700';
        case 244:
            return '#8b949e';
        case 250:
            return '#bcbcbc';
        default:
            return '#d6dee6';
    }
}

function escapeHtml(value) {
    return value
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;');
}
