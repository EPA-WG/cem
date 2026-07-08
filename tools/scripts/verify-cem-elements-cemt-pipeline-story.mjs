#!/usr/bin/env node

import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const storybookRoot = join(repoRoot, 'packages/cem-elements/storybook-static');
const timeout = 30_000;
const stories = [
    {
        id: 'cem-elements-cemt-output-pipeline--formatter-coloring-writer-stages',
        sourceIncludes: [],
    },
    {
        id: 'cem-elements-cemt-output-pipeline--schema-package-formatter-colorizer-assets',
        sourceIncludes: [
            'schema-packages/cem-ml/v1/formatters/formatter-coloring-pipeline.cemt',
            'schema-packages/cem-ml/v1/colorizers/formatter-coloring-pipeline.cemt',
        ],
    },
];

await requireStaticStorybook();

const server = createServer(async (request, response) => {
    try {
        const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');
        const pathname = decodeURIComponent(requestUrl.pathname === '/' ? '/index.html' : requestUrl.pathname);
        const filePath = normalize(join(storybookRoot, pathname));
        if (filePath !== storybookRoot && !filePath.startsWith(storybookRoot + sep)) {
            response.writeHead(403);
            response.end('Forbidden');
            return;
        }
        const fileStat = await stat(filePath);
        if (!fileStat.isFile()) {
            response.writeHead(404);
            response.end('Not found');
            return;
        }
        response.writeHead(200, { 'content-type': contentType(filePath) });
        createReadStream(filePath).pipe(response);
    } catch {
        response.writeHead(404);
        response.end('Not found');
    }
});

await new Promise((resolveListen) => server.listen(0, '127.0.0.1', resolveListen));

const address = server.address();
const port = typeof address === 'object' && address ? address.port : 0;
const browser = await chromium.launch({ headless: true });

try {
    const pageErrors = [];
    const page = await browser.newPage({
        viewport: { width: 1280, height: 900 },
        deviceScaleFactor: 1,
    });
    page.on('pageerror', (error) => pageErrors.push(error.message));
    page.on('console', (message) => {
        if (message.type() === 'error') {
            pageErrors.push(message.text());
        }
    });

    for (const story of stories) {
        const storyUrl = `http://127.0.0.1:${port}/iframe.html?id=${story.id}&viewMode=story`;
        await page.goto(storyUrl, { waitUntil: 'networkidle' });
        await page.waitForSelector('.cemt-pipeline-showcase', { timeout });
        await page.waitForSelector('[data-stage="writer"] article.cem-color-syntax-name', { timeout });

        const stageReport = await collectStageReport(page);
        assertStageReport(stageReport, story);

        const screenshot = await page.locator('.cemt-pipeline-showcase').screenshot({
            animations: 'disabled',
            type: 'png',
        });
        const screenshotStats = await screenshotPixelStats(page, screenshot);
        assertScreenshotStats(screenshotStats, story.id);
    }

    if (pageErrors.length > 0) {
        throw new Error(`CEMT output pipeline story emitted browser errors:\n${pageErrors.map((error) => `- ${error}`).join('\n')}`);
    }
} finally {
    await browser.close();
    await new Promise((resolveClose) => server.close(resolveClose));
}

console.log('cem-elements CEMT output pipeline Storybook stories verified with screenshot pixels.');

async function requireStaticStorybook() {
    try {
        const iframe = await stat(join(storybookRoot, 'iframe.html'));
        if (iframe.isFile()) {
            return;
        }
    } catch {
        // Fall through to the actionable error below.
    }
    throw new Error(
        `Storybook static output is missing at ${storybookRoot}. Run cem-elements:verify-cemt-pipeline-story or build Storybook first.`
    );
}

async function collectStageReport(page) {
    return page.evaluate(() => {
        const stageNames = ['cemt-source', 'source', 'formatted', 'colored', 'writer'];
        const stages = Object.fromEntries(
            stageNames.map((stage) => {
                const element = document.querySelector(`[data-stage="${stage}"]`);
                const rect = element?.getBoundingClientRect();
                return [
                    stage,
                    {
                        text: element?.textContent ?? '',
                        visible:
                            !!rect &&
                            rect.width >= 120 &&
                            rect.height >= 80 &&
                            getComputedStyle(element).visibility !== 'hidden' &&
                            getComputedStyle(element).display !== 'none',
                        width: rect?.width ?? 0,
                        height: rect?.height ?? 0,
                    },
                ];
            })
        );
        const writer = document.querySelector('[data-stage="writer"]');
        const writerArticle = writer?.querySelector('article.cem-color-syntax-name');
        const writerSpan = writer?.querySelector('span.cem-color-syntax-string');
        const writerStrong = writer?.querySelector('strong.cem-color-syntax-keyword');
        return {
            stages,
            writer: {
                articleText: writerArticle?.textContent ?? '',
                articleClass: writerArticle?.getAttribute('class') ?? '',
                spanClass: writerSpan?.getAttribute('class') ?? '',
                strongClass: writerStrong?.getAttribute('class') ?? '',
                elementColor: writerArticle ? getComputedStyle(writerArticle).color : '',
                textColor: writerSpan ? getComputedStyle(writerSpan).color : '',
                keywordColor: writerStrong ? getComputedStyle(writerStrong).color : '',
            },
        };
    });
}

function assertStageReport(report, story) {
    for (const [stage, state] of Object.entries(report.stages)) {
        assert(state.visible, `${story.id}: stage ${stage} is not visibly rendered (${state.width}x${state.height})`);
    }

    const cemtSource = report.stages['cemt-source'].text;
    assert(cemtSource.includes('@name="acme.showcase.format-tree"'), `${story.id}: CEMT source stage omits formatter declaration`);
    assert(cemtSource.includes('@name="acme.showcase.color-tree"'), `${story.id}: CEMT source stage omits colorizer declaration`);
    for (const expectedSource of story.sourceIncludes) {
        assert(cemtSource.includes(expectedSource), `${story.id}: CEMT source stage omits ${expectedSource}`);
    }

    const formatted = report.stages.formatted.text;
    assert(formatted.includes('formatted tree before writer'), `${story.id}: formatted stage omits formatter metadata`);
    assert(formatted.includes('@formatter-profile="acme.showcase.format-tree"'), `${story.id}: formatted stage omits formatter profile`);
    assert(!formatted.includes('colored tree before writer'), `${story.id}: formatted stage already includes coloring metadata`);
    assert(!formatted.includes('"kind":'), `${story.id}: formatted stage rendered JSON instead of CEM-native text`);

    const colored = report.stages.colored.text;
    assert(colored.includes('colored tree before writer'), `${story.id}: colored stage omits colorizer metadata`);
    assert(colored.includes('@colored=true'), `${story.id}: colored stage omits colored tree marker`);
    assert(colored.includes('@color-role="syntax.keyword"'), `${story.id}: colored stage omits keyword color role`);

    assert(report.writer.articleText.includes('Ready now.'), `${story.id}: writer output does not render the formatted content`);
    assert(report.writer.articleClass.includes('cem-color-syntax-name'), `${story.id}: writer output omits element color class`);
    assert(report.writer.spanClass.includes('cem-color-syntax-string'), `${story.id}: writer output omits text color class`);
    assert(report.writer.strongClass.includes('cem-color-syntax-keyword'), `${story.id}: writer output omits keyword color class`);
    assert(report.writer.elementColor !== report.writer.textColor, `${story.id}: writer element and text colors are not visually distinct`);
    assert(report.writer.keywordColor !== report.writer.textColor, `${story.id}: writer keyword and text colors are not visually distinct`);
}

async function screenshotPixelStats(page, screenshot) {
    assert(
        screenshot[0] === 0x89 &&
            screenshot[1] === 0x50 &&
            screenshot[2] === 0x4e &&
            screenshot[3] === 0x47,
        'Storybook screenshot is not a PNG image'
    );

    return page.evaluate(async (base64) => {
        const image = new Image();
        image.src = `data:image/png;base64,${base64}`;
        await image.decode();
        const canvas = document.createElement('canvas');
        canvas.width = image.naturalWidth;
        canvas.height = image.naturalHeight;
        const context = canvas.getContext('2d', { willReadFrequently: true });
        if (!context) {
            throw new Error('could not create screenshot canvas context');
        }
        context.drawImage(image, 0, 0);
        const data = context.getImageData(0, 0, canvas.width, canvas.height).data;
        const buckets = new Set();
        let samples = 0;
        let nonWhite = 0;
        let dark = 0;
        let saturated = 0;
        for (let y = 0; y < canvas.height; y += 6) {
            for (let x = 0; x < canvas.width; x += 6) {
                const offset = (y * canvas.width + x) * 4;
                const r = data[offset];
                const g = data[offset + 1];
                const b = data[offset + 2];
                const a = data[offset + 3];
                if (a === 0) {
                    continue;
                }
                samples += 1;
                if (!(r > 246 && g > 246 && b > 246)) {
                    nonWhite += 1;
                }
                if (r < 120 && g < 120 && b < 120) {
                    dark += 1;
                }
                if (Math.max(r, g, b) - Math.min(r, g, b) > 35) {
                    saturated += 1;
                }
                buckets.add(`${r >> 5}:${g >> 5}:${b >> 5}`);
            }
        }
        return {
            width: canvas.width,
            height: canvas.height,
            samples,
            nonWhite,
            dark,
            saturated,
            colorBuckets: buckets.size,
        };
    }, screenshot.toString('base64'));
}

function assertScreenshotStats(stats, storyId) {
    assert(stats.width >= 900, `${storyId}: Storybook screenshot is too narrow: ${stats.width}px`);
    assert(stats.height >= 600, `${storyId}: Storybook screenshot is too short: ${stats.height}px`);
    assert(
        stats.nonWhite > stats.samples * 0.02,
        `${storyId}: Storybook screenshot is visually blank or nearly blank: ${JSON.stringify(stats)}`
    );
    assert(stats.dark > 80, `${storyId}: Storybook screenshot does not contain enough rendered text pixels: ${JSON.stringify(stats)}`);
    assert(stats.saturated > 20, `${storyId}: Storybook screenshot does not contain enough colored output pixels: ${JSON.stringify(stats)}`);
    assert(
        stats.colorBuckets >= 12,
        `${storyId}: Storybook screenshot has too little color variation: ${JSON.stringify(stats)}`
    );
}

function contentType(filePath) {
    switch (extname(filePath)) {
        case '.html':
            return 'text/html; charset=utf-8';
        case '.js':
        case '.mjs':
            return 'text/javascript; charset=utf-8';
        case '.css':
            return 'text/css; charset=utf-8';
        case '.json':
        case '.map':
            return 'application/json; charset=utf-8';
        case '.wasm':
            return 'application/wasm';
        case '.svg':
            return 'image/svg+xml; charset=utf-8';
        case '.png':
            return 'image/png';
        case '.ico':
            return 'image/x-icon';
        case '.woff2':
            return 'font/woff2';
        default:
            return 'application/octet-stream';
    }
}

function assert(condition, message) {
    if (!condition) {
        throw new Error(message);
    }
}
