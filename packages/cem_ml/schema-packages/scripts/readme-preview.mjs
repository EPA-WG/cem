import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, relative, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { chromium } from 'playwright';

const DEFAULT_TEXT = '#d6dee6';
const DEFAULT_TAB_SIZE = 8;
const SVG_NS = 'http://www.w3.org/2000/svg';
const XML_NS = 'http://www.w3.org/XML/1998/namespace';

export async function verifyReadmePreviews({
    workspaceRoot,
    packageRoot,
    cli,
    cases,
    update,
    packageLabel,
    refreshCommand,
}) {
    const { activeCases, unmatchedPreviews } = update
        ? { activeCases: cases, unmatchedPreviews: [] }
        : readmeReferencedPreviewCases(packageRoot, cases);
    if (unmatchedPreviews.length > 0) {
        throw new Error(
            `${packageLabel} README references SVG previews without verifier cases: ${unmatchedPreviews.join(', ')}`,
        );
    }
    if (activeCases.length === 0) {
        console.log(`${packageLabel} has no README SVG fallback previews to verify.`);
        return;
    }
    ensureReadmePreviewDependencies();

    const previewRoot = join(packageRoot, 'examples/previews');
    const generatedRoot = previewDistExampleRoot(workspaceRoot, packageRoot);
    mkdirSync(previewRoot, { recursive: true });
    mkdirSync(generatedRoot, { recursive: true });

    const cliEnv = { ...process.env };
    delete cliEnv.NO_COLOR;

    const browser = await launchPreviewBrowser();
    const failures = [];
    try {
        for (const testCase of activeCases) {
            const previewInput = previewInputForCase({
                cli,
                cliEnv,
                workspaceRoot,
                packageLabel,
                testCase,
            });
            const preview = buildPreviewDocument(
                { ...testCase, renderer: previewInput.renderer ?? testCase.renderer },
                previewInput.content,
            );
            const htmlPath = join(
                generatedRoot,
                testCase.html ?? testCase.preview.replace(/\.svg$/i, '.html'),
            );
            const expectedPath = join(previewRoot, testCase.preview);
            const generatedPath = update ? expectedPath : join(generatedRoot, testCase.preview);
            writeFileSync(htmlPath, preview.html, 'utf8');
            await renderPreviewDocumentToSvg(browser, preview, generatedPath, htmlPath);
            addSvgAccessibilityMetadata(generatedPath, testCase);
            if (update) {
                continue;
            }
            let expected;
            try {
                expected = readFileSync(expectedPath, 'utf8');
            } catch {
                failures.push({
                    id: testCase.id,
                    expectedPath,
                    generatedPath,
                    message: 'expected preview is missing',
                });
                continue;
            }
            const actual = readFileSync(generatedPath, 'utf8');
            if (expected !== actual) {
                failures.push({
                    id: testCase.id,
                    expectedPath,
                    generatedPath,
                    message: firstDifference(expected, actual),
                });
            }
        }
    } finally {
        await browser.close();
    }

    if (failures.length > 0) {
        for (const failure of failures) {
            console.error(`${packageLabel} preview drift: ${failure.id}`);
            console.error(`  expected:  ${relative(workspaceRoot, failure.expectedPath)}`);
            console.error(`  generated: ${relative(workspaceRoot, failure.generatedPath)}`);
            console.error(`  ${failure.message}`);
        }
        console.error(`Refresh previews with: ${refreshCommand}`);
        process.exit(1);
    }

    console.log(
        update
            ? `Updated ${packageLabel} README SVG previews.`
            : `${packageLabel} README SVG previews verified.`,
    );
}

function readmeReferencedPreviewCases(packageRoot, cases) {
    let readme;
    try {
        readme = readFileSync(join(packageRoot, 'README.md'), 'utf8');
    } catch {
        return { activeCases: cases, unmatchedPreviews: [] };
    }
    const referencedPreviews = new Set(
        [...readme.matchAll(/\]\(examples\/previews\/([^\s)]+\.svg)\)/g)].map(
            (match) => match[1],
        ),
    );
    const activeCases = cases.filter((testCase) => referencedPreviews.has(testCase.preview));
    const casePreviews = new Set(cases.map((testCase) => testCase.preview));
    const unmatchedPreviews = [...referencedPreviews].filter(
        (preview) => !casePreviews.has(preview),
    );
    return { activeCases, unmatchedPreviews };
}

export function ensureReadmePreviewDependencies() {
    // Playwright is imported at module load and Chromium launch is validated by launchPreviewBrowser.
}

export async function launchPreviewBrowser() {
    try {
        return await chromium.launch({ headless: true });
    } catch (error) {
        throw new Error(
            [
                'Playwright Chromium is required for README SVG preview generation.',
                'Install browser dependencies with the project setup command:',
                'PLAYWRIGHT_HOST_PLATFORM_OVERRIDE=ubuntu24.04-x64 yarn playwright install',
                String(error),
            ].join('\n'),
        );
    }
}

export function runPreviewCli({ cli, cliEnv, workspaceRoot, packageLabel, testCase }) {
    const result = spawnSync(cli, testCase.args, {
        cwd: workspaceRoot,
        encoding: 'utf8',
        env: cliEnv,
        maxBuffer: 1024 * 1024,
    });
    if (!isExpectedExitStatus(result.status, testCase.expectedStatus ?? 'success')) {
        throw new Error(
            [
                `${packageLabel} preview command failed for ${testCase.id}: ${cli} ${testCase.args.join(' ')}`,
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

function previewInputForCase({ cli, cliEnv, workspaceRoot, packageLabel, testCase }) {
    if (testCase.sourcePath) {
        return readSourcePreviewInput(workspaceRoot, testCase.sourcePath);
    }

    const result = spawnSync(cli, testCase.args, {
        cwd: workspaceRoot,
        encoding: 'utf8',
        env: cliEnv,
        maxBuffer: 1024 * 1024,
    });
    if (!isExpectedExitStatus(result.status, testCase.expectedStatus ?? 'success')) {
        throw new Error(
            [
                `${packageLabel} preview command failed for ${testCase.id}: ${cli} ${testCase.args.join(' ')}`,
                `exit status: ${result.status}`,
                result.stdout,
                result.stderr,
            ]
                .filter(Boolean)
                .join('\n'),
        );
    }

    if (result.stdout.length > 0) {
        return { content: result.stdout };
    }

    if (testCase.fallbackSourcePath) {
        return readSourcePreviewInput(workspaceRoot, testCase.fallbackSourcePath);
    }

    return { content: result.stderr };
}

function readSourcePreviewInput(workspaceRoot, sourcePath) {
    const path = resolve(workspaceRoot, sourcePath);
    const bytes = readFileSync(path);
    return {
        renderer: 'text',
        content: sourcePreviewContent(bytes),
    };
}

function sourcePreviewContent(bytes) {
    const text = utf8SourceText(bytes);
    if (text !== null) {
        return text;
    }
    return binarySourcePreview(bytes);
}

function utf8SourceText(bytes) {
    try {
        const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
        if (/[\u0000-\u0008\u000b\u000c\u000e-\u001f]/.test(text)) {
            return null;
        }
        return text;
    } catch {
        return null;
    }
}

function binarySourcePreview(bytes) {
    const maxBytes = Math.min(bytes.length, 1024);
    const lines = [];
    for (let offset = 0; offset < maxBytes; offset += 16) {
        const chunk = bytes.subarray(offset, Math.min(offset + 16, maxBytes));
        const hex = [...chunk].map((byte) => byte.toString(16).padStart(2, '0')).join(' ');
        const ascii = [...chunk]
            .map((byte) => (byte >= 32 && byte <= 126 ? String.fromCharCode(byte) : '.'))
            .join('');
        lines.push(`${offset.toString(16).padStart(8, '0')}  ${hex.padEnd(47)}  ${ascii}`);
    }
    if (bytes.length > maxBytes) {
        lines.push(`... ${bytes.length - maxBytes} more bytes`);
    }
    return `${lines.join('\n')}\n`;
}

function isExpectedExitStatus(status, expectedStatus) {
    if (expectedStatus === 'any') {
        return true;
    }
    if (expectedStatus === 'failure') {
        return status !== 0;
    }
    return status === 0;
}

export function buildPreviewDocument(testCase, stdout) {
    switch (testCase.renderer ?? 'json') {
        case 'ansi': {
            const lines = parseAnsi(stdout);
            return buildPrePreviewDocument(testCase, styledLinesToPreHtml(lines));
        }
        case 'html': {
            const htmlPreview = extractHtmlPreview(stdout);
            return buildPrePreviewDocument(testCase, htmlPreview.html);
        }
        case 'document-html':
            return buildPrePreviewDocument(testCase, extractHtmlPreview(stdout).html);
        case 'text': {
            const lines = stdout.replace(/\r\n/g, '\n').replace(/\r/g, '\n').trimEnd().split('\n');
            return buildPrePreviewDocument(testCase, plainLinesToPreHtml(lines));
        }
        case 'json':
        default: {
            const lines = normalizeJson(stdout).trimEnd().split('\n');
            return buildPrePreviewDocument(testCase, plainLinesToPreHtml(lines));
        }
    }
}

function buildPrePreviewDocument(testCase, preHtml) {
    return {
        html: [
            '<!doctype html>',
            '<html lang="en">',
            '<head>',
            '<meta charset="utf-8">',
            `<title>${escapeHtml(testCase.title)}</title>`,
            '<style>',
            'html, body { margin: 0; padding: 0; background: transparent; }',
            'body { color: #d6dee6; }',
            `.cem-output { display: inline-block; margin: 0; white-space: pre; tab-size: ${DEFAULT_TAB_SIZE}; color: ${DEFAULT_TEXT}; font: 13px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; line-height: 15px; }`,
            '.cem-output span { white-space: pre; }',
            '</style>',
            '</head>',
            '<body>',
            ensurePreContainer(preHtml),
            '</body>',
            '</html>',
            '',
        ].join('\n'),
    };
}

export async function renderPreviewDocumentToSvg(browser, _preview, svgPath, htmlPath) {
    mkdirSync(dirname(svgPath), { recursive: true });
    const page = await browser.newPage({
        viewport: { width: 1600, height: 900 },
        deviceScaleFactor: 1,
    });
    try {
        await page.goto(pathToFileURL(htmlPath).href, { waitUntil: 'load' });
        const svg = await page.evaluate(async ({ svgNs, xmlNs }) => {
            const pre = document.querySelector('pre');
            if (!pre) {
                throw new Error('README preview HTML must contain a <pre> element');
            }
            if (document.fonts) {
                await document.fonts.ready;
            }
            splitMultilineSpans(pre);
            await new Promise((resolve) => requestAnimationFrame(resolve));

            const preRect = pre.getBoundingClientRect();
            const width = Math.ceil(Math.max(preRect.width, pre.scrollWidth));
            const height = Math.ceil(Math.max(preRect.height, pre.scrollHeight));
            const svg = document.createElementNS(svgNs, 'svg');

            svg.setAttribute('xmlns', svgNs);
            svg.setAttribute('width', String(width));
            svg.setAttribute('height', String(height));
            svg.setAttribute('viewBox', `0 0 ${width} ${height}`);
            svg.setAttributeNS(xmlNs, 'xml:space', 'preserve');

            for (const span of pre.querySelectorAll('span')) {
                if (!span.textContent) {
                    continue;
                }
                const rect = span.getBoundingClientRect();
                const style = window.getComputedStyle(span);
                const text = document.createElementNS(svgNs, 'text');

                text.setAttribute('x', String(Math.round((rect.left - preRect.left) * 100) / 100));
                text.setAttribute('y', String(Math.round((rect.top - preRect.top) * 100) / 100));
                text.setAttribute('fill', style.color);
                text.setAttribute('font-family', style.fontFamily);
                text.setAttribute('font-size', style.fontSize);
                text.setAttribute('font-style', style.fontStyle);
                text.setAttribute('font-weight', style.fontWeight);
                text.setAttribute('letter-spacing', style.letterSpacing);
                text.setAttribute('dominant-baseline', 'text-before-edge');
                text.setAttributeNS(xmlNs, 'xml:space', 'preserve');
                text.textContent = span.textContent;

                svg.append(text);
            }

            return new XMLSerializer().serializeToString(svg);

            function splitMultilineSpans(root) {
                for (const span of [...root.querySelectorAll('span')]) {
                    if (!span.textContent.includes('\n')) {
                        continue;
                    }
                    const parent = span.parentNode;
                    const parts = span.textContent.replace(/\r\n?/g, '\n').split('\n');
                    for (const [index, part] of parts.entries()) {
                        if (index > 0) {
                            parent.insertBefore(document.createTextNode('\n'), span);
                        }
                        if (part.length === 0) {
                            continue;
                        }
                        const clone = span.cloneNode(false);
                        clone.textContent = part;
                        parent.insertBefore(clone, span);
                    }
                    span.remove();
                }
            }
        }, { svgNs: SVG_NS, xmlNs: XML_NS });
        writeFileSync(svgPath, `${svg}\n`, 'utf8');
    } finally {
        await page.close();
    }
}

export function addSvgAccessibilityMetadata(svgPath, testCase) {
    const svg = readFileSync(svgPath, 'utf8');
    const title = `<title id="title">${escapeXml(testCase.title)}</title>`;
    const desc = `<desc id="desc">${escapeXml(testCase.description)}</desc>`;
    const withRole = svg.replace(
        /<svg\b(?![^>]*\brole=)/,
        '<svg role="img" aria-labelledby="title desc"',
    );
    const withMetadata = withRole.replace(/(<svg[^>]*>)/, `$1\n${title}\n${desc}`);
    writeFileSync(svgPath, withMetadata, 'utf8');
}

function normalizeJson(stdout) {
    return `${JSON.stringify(JSON.parse(stdout), null, 2)}\n`;
}

function plainLinesToPreHtml(lines) {
    return lines
        .map((line) => `<span>${escapeHtml(line)}</span>`)
        .join('\n');
}

function styledLinesToPreHtml(lines) {
    const trimmed = [...lines];
    while (trimmed.length > 0 && !hasVisibleText(trimmed[trimmed.length - 1])) {
        trimmed.pop();
    }
    return trimmed
        .map((line) => line.map(styledSpanToHtml).join(''))
        .join('\n');
}

function styledSpanToHtml(span) {
    const weight = span.fontWeight ? `; font-weight: ${span.fontWeight}` : '';
    return `<span style="color: ${span.fill}${weight}">${escapeHtml(span.text)}</span>`;
}

function hasVisibleText(line) {
    return line.some((span) => span.text.length > 0);
}

function parseAnsi(input) {
    const lines = [[]];
    let fill = DEFAULT_TEXT;
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
        return { fill: DEFAULT_TEXT, fontWeight: null };
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
            return '#ff8700';
        case 244:
            return '#8b949e';
        default:
            return DEFAULT_TEXT;
    }
}

function extractHtmlPreview(input) {
    const match = input.match(/<pre\b[\s\S]*?<\/pre>/i);
    return {
        html: match
            ? match[0]
            : `<pre class="cem-output">${plainLinesToPreHtml(
                  input.replace(/\r\n/g, '\n').replace(/\r/g, '\n').trimEnd().split('\n'),
              )}</pre>`,
    };
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

function escapeHtml(value) {
    return value
        .replaceAll('&', '&amp;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
        .replaceAll('"', '&quot;');
}

function escapeXml(value) {
    return escapeHtml(value);
}

function ensurePreContainer(html) {
    if (/<pre\b/i.test(html)) {
        return html;
    }
    return `<pre class="cem-output">${html}</pre>`;
}

function previewDistExampleRoot(workspaceRoot, packageRoot) {
    const cemMlRoot = join(workspaceRoot, 'packages/cem_ml');
    const relativePackageRoot = relative(cemMlRoot, packageRoot);
    if (!relativePackageRoot.startsWith('..')) {
        return join(workspaceRoot, 'dist/cem_ml', relativePackageRoot, 'examples');
    }
    return join(packageRoot, 'dist/examples');
}
