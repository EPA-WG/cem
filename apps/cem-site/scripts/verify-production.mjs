import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { mkdir, readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { extname, join, normalize, relative, resolve, sep } from 'node:path';
import { chromium } from 'playwright';

const workspaceRoot = resolve(import.meta.dirname, '../../..');
const projectRoot = resolve(workspaceRoot, 'apps/cem-site');
const outputRoot = resolve(workspaceRoot, 'dist/apps/cem-site');
const reportRoot = resolve(workspaceRoot, 'dist/reports/cem-site');
const manifest = JSON.parse(await readFile(resolve(projectRoot, 'site.routes.json'), 'utf8'));
const determinism = JSON.parse(await readFile(resolve(reportRoot, 'determinism.json'), 'utf8'));
const pageEntries = manifest.entries.filter(({ kind }) => kind === 'page');

async function filesUnder(directory) {
    const entries = await readdir(directory, { withFileTypes: true });
    const files = [];
    for (const entry of entries) {
        const path = join(directory, entry.name);
        if (entry.isDirectory()) {
            files.push(...(await filesUnder(path)));
        } else {
            files.push(relative(outputRoot, path).replaceAll('\\', '/'));
        }
    }
    return files.sort();
}

const productionFiles = await filesUnder(outputRoot);
const deterministicFiles = determinism.files.map(({ path }) => path);
if (JSON.stringify(productionFiles) !== JSON.stringify(deterministicFiles)) {
    throw new Error('production output drifted from the clean deterministic build inventory');
}
for (const file of determinism.files) {
    const bytes = await readFile(join(outputRoot, file.path));
    const digest = createHash('sha256').update(bytes).digest('hex');
    if (bytes.byteLength !== file.bytes || digest !== file.sha256) {
        throw new Error(`production output drifted from the deterministic build: ${file.path}`);
    }
}
for (const entry of manifest.entries) {
    if (!productionFiles.includes(entry.output) || !productionFiles.includes(`${entry.output}.map`)) {
        throw new Error(`production output is missing ${entry.route}`);
    }
}

const server = createServer(async (request, response) => {
    try {
        const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');
        const pathname = decodeURIComponent(requestUrl.pathname);
        const relativePath = pathname.endsWith('/') ? `${pathname}index.html` : pathname;
        const filePath = normalize(join(outputRoot, relativePath));
        if (!filePath.startsWith(outputRoot + sep)) {
            response.writeHead(403);
            response.end('Forbidden');
            return;
        }
        const fileStat = await stat(filePath);
        if (!fileStat.isFile()) {
            throw new Error('not a file');
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
const origin = `http://127.0.0.1:${port}`;
let browser;

try {
    browser = await chromium.launch({ headless: true });
    const context = await browser.newContext();
    await context.route('**/*', async (route) => {
        const url = new URL(route.request().url());
        if (['http:', 'https:'].includes(url.protocol) && url.origin !== origin) {
            await route.fulfill({ status: 204, contentType: 'text/plain', body: '' });
            return;
        }
        await route.continue();
    });

    const routeReports = [];
    const idsByRoute = new Map();
    const fragmentLinks = [];
    for (const entry of pageEntries) {
        const page = await context.newPage();
        const runtimeErrors = [];
        page.on('pageerror', (error) => runtimeErrors.push(`pageerror: ${error.message}`));
        page.on('console', (message) => {
            if (message.type() === 'error') {
                runtimeErrors.push(`console: ${message.text()}`);
            }
        });
        page.on('requestfailed', (request) => {
            if (new URL(request.url()).origin === origin) {
                runtimeErrors.push(`${request.url()}: ${request.failure()?.errorText ?? 'request failed'}`);
            }
        });
        page.on('response', (response) => {
            if (new URL(response.url()).origin === origin && response.status() >= 400) {
                runtimeErrors.push(`${response.url()}: HTTP ${response.status()}`);
            }
        });

        const response = await page.goto(`${origin}${entry.route}`, {
            waitUntil: 'networkidle',
            timeout: 120_000,
        });
        if (!response || response.status() !== 200) {
            throw new Error(`${entry.route} returned ${response?.status() ?? 'no response'}`);
        }
        if (entry.contentRole === 'interactive-example') {
            await page.waitForFunction(() => globalThis.__cemSiteInteractive?.done === true, null, {
                timeout: 120_000,
            });
            const errors = await page.evaluate(() => globalThis.__cemSiteInteractive.errors ?? []);
            runtimeErrors.push(...errors.map((error) => `interactive: ${error}`));
        }
        if (entry.contentRole === 'search') {
            await page.waitForFunction(() => globalThis.__cemSiteSearch?.done === true, null, {
                timeout: 120_000,
            });
            const errors = await page.evaluate(() => globalThis.__cemSiteSearch.errors ?? []);
            runtimeErrors.push(...errors.map((error) => `search: ${error}`));
        }

        const structure = await page.evaluate(() => {
            const visible = (element) => {
                const style = getComputedStyle(element);
                return (
                    !element.closest('[hidden], [inert]') &&
                    style.display !== 'none' &&
                    style.visibility !== 'hidden' &&
                    element.getClientRects().length > 0
                );
            };
            const focusSelector = [
                'a[href]',
                'area[href]',
                'button',
                'input:not([type="hidden"])',
                'select',
                'textarea',
                'iframe',
                'summary',
                'audio[controls]',
                'video[controls]',
                '[contenteditable]:not([contenteditable="false"])',
                '[tabindex]:not([tabindex="-1"])',
            ].join(',');
            const accessibleName = (element) => {
                const labelledBy = element.getAttribute('aria-labelledby');
                if (labelledBy) {
                    return labelledBy
                        .split(/\s+/)
                        .map((id) => document.getElementById(id)?.textContent?.trim() ?? '')
                        .filter(Boolean)
                        .join(' ');
                }
                const explicitLabel = element.getAttribute('aria-label');
                if (explicitLabel?.trim()) {
                    return explicitLabel.trim();
                }
                if (element.id) {
                    const label = [...document.querySelectorAll('label[for]')].find(
                        (candidate) => candidate.htmlFor === element.id,
                    );
                    if (label?.textContent?.trim()) {
                        return label.textContent.trim();
                    }
                }
                const wrappingLabel = element.closest('label')?.textContent?.trim();
                if (wrappingLabel) {
                    return wrappingLabel;
                }
                const imageNames = [...element.querySelectorAll('img[alt]')]
                    .map((image) => image.alt.trim())
                    .filter(Boolean)
                    .join(' ');
                return (
                    element.textContent?.trim() ||
                    imageNames ||
                    element.getAttribute('alt')?.trim() ||
                    element.getAttribute('title')?.trim() ||
                    element.getAttribute('value')?.trim() ||
                    ''
                );
            };

            const headings = [...document.querySelectorAll('h1, h2, h3, h4, h5, h6')].map((heading) => ({
                id: heading.id,
                level: Number(heading.localName.slice(1)),
                text: heading.textContent?.replace(/\s+/g, ' ').trim() ?? '',
            }));
            const ids = [...document.querySelectorAll('[id]')].map(({ id }) => id);
            const invalidAriaReferences = [];
            for (const element of document.querySelectorAll(
                '[aria-labelledby], [aria-describedby], [aria-controls], [aria-activedescendant]',
            )) {
                for (const attribute of [
                    'aria-labelledby',
                    'aria-describedby',
                    'aria-controls',
                    'aria-activedescendant',
                ]) {
                    for (const id of (element.getAttribute(attribute) ?? '').split(/\s+/).filter(Boolean)) {
                        if (!document.getElementById(id)) {
                            invalidAriaReferences.push(`${attribute}:${id}`);
                        }
                    }
                }
            }
            const focusable = [...document.querySelectorAll(focusSelector)].filter(
                (element) => !element.matches(':disabled, [aria-disabled="true"]') && visible(element),
            );
            focusable.forEach((element, index) => {
                element.dataset.productionFocusId = String(index);
            });
            const unnamedFocusables = focusable
                .filter((element) => !accessibleName(element))
                .map(
                    (element) =>
                        `${element.localName}[data-production-focus-id="${element.dataset.productionFocusId}"]`,
                );
            const fragments = [...document.querySelectorAll('a[href]')]
                .map((anchor) => new URL(anchor.href))
                .filter((url) => url.origin === location.origin && url.hash)
                .map((url) => ({ pathname: url.pathname, fragment: decodeURIComponent(url.hash.slice(1)) }));

            return {
                lang: document.documentElement.lang,
                title: document.title.trim(),
                landmarkCounts: {
                    header: document.querySelectorAll('header').length,
                    primaryNav: document.querySelectorAll('nav[aria-label="Primary"]').length,
                    main: document.querySelectorAll('main#content').length,
                    footer: document.querySelectorAll('footer').length,
                },
                headings,
                ids,
                missingImageAlt: [...document.querySelectorAll('img:not([alt])')].length,
                invalidAriaReferences,
                focusableCount: focusable.length,
                unnamedFocusables,
                fragments,
            };
        });

        if (structure.lang !== 'en' || !structure.title) {
            throw new Error(`${entry.route} has an invalid language or empty title`);
        }
        if (Object.values(structure.landmarkCounts).some((count) => count !== 1)) {
            throw new Error(`${entry.route} landmark contract drifted: ${JSON.stringify(structure.landmarkCounts)}`);
        }
        if (
            structure.headings.length === 0 ||
            structure.headings[0].level > 2 ||
            structure.headings.filter(({ level }) => level === 1).length > 1 ||
            structure.headings.some(({ id, text }) => !id || !text)
        ) {
            throw new Error(`${entry.route} must start at h1/h2 and retain stable named headings`);
        }
        for (let index = 1; index < structure.headings.length; index += 1) {
            if (structure.headings[index].level > structure.headings[index - 1].level + 1) {
                throw new Error(`${entry.route} skips a heading level at ${structure.headings[index].id}`);
            }
        }
        if (new Set(structure.ids).size !== structure.ids.length) {
            throw new Error(`${entry.route} has duplicate DOM identifiers`);
        }
        if (
            structure.missingImageAlt > 0 ||
            structure.invalidAriaReferences.length > 0 ||
            structure.unnamedFocusables.length > 0
        ) {
            throw new Error(
                `${entry.route} accessible-name contract failed: ${JSON.stringify({
                    missingImageAlt: structure.missingImageAlt,
                    invalidAriaReferences: structure.invalidAriaReferences,
                    unnamedFocusables: structure.unnamedFocusables,
                })}`,
            );
        }

        const reachedFocus = new Set();
        await page.evaluate(() => {
            if (document.activeElement instanceof HTMLElement) {
                document.activeElement.blur();
            }
        });
        for (let index = 0; index < structure.focusableCount; index += 1) {
            await page.keyboard.press('Tab');
            const focus = await page.evaluate(() => {
                const element = document.activeElement;
                if (!(element instanceof HTMLElement)) {
                    return null;
                }
                const style = getComputedStyle(element);
                const outlineVisible =
                    style.outlineStyle !== 'none' &&
                    Number.parseFloat(style.outlineWidth) > 0 &&
                    style.outlineColor !== 'rgba(0, 0, 0, 0)';
                const boxShadowVisible = style.boxShadow !== 'none';
                return {
                    id: element.dataset.productionFocusId ?? null,
                    focusVisible: element.matches(':focus-visible'),
                    indicatorVisible: outlineVisible || boxShadowVisible,
                };
            });
            if (!focus?.id || !focus.focusVisible || !focus.indicatorVisible) {
                throw new Error(`${entry.route} has no visible keyboard focus at tab stop ${index + 1}`);
            }
            reachedFocus.add(focus.id);
        }
        if (reachedFocus.size !== structure.focusableCount) {
            throw new Error(
                `${entry.route} keyboard traversal reached ${reachedFocus.size}/${structure.focusableCount}`,
            );
        }
        if (runtimeErrors.length > 0) {
            throw new Error(`${entry.route} runtime failed:\n${runtimeErrors.map((error) => `- ${error}`).join('\n')}`);
        }

        idsByRoute.set(entry.route, new Set(structure.ids));
        fragmentLinks.push(...structure.fragments.map((fragment) => ({ source: entry.route, ...fragment })));
        routeReports.push({
            route: entry.route,
            status: response.status(),
            title: structure.title,
            headingCount: structure.headings.length,
            focusableCount: structure.focusableCount,
            fragmentLinkCount: structure.fragments.length,
            runtimeErrors: [],
        });
        await page.close();
    }

    for (const link of fragmentLinks) {
        if (!idsByRoute.get(link.pathname)?.has(link.fragment)) {
            throw new Error(`${link.source} links to missing production fragment ${link.pathname}#${link.fragment}`);
        }
    }

    const report = {
        version: 1,
        routeCount: routeReports.length,
        responseCount: routeReports.length,
        headingCount: routeReports.reduce((total, route) => total + route.headingCount, 0),
        focusableCount: routeReports.reduce((total, route) => total + route.focusableCount, 0),
        fragmentLinkCount: fragmentLinks.length,
        cleanOutputFiles: productionFiles.length,
        aggregateSha256: determinism.aggregateSha256,
        routes: routeReports,
        runtimeErrors: [],
    };
    await mkdir(reportRoot, { recursive: true });
    const reportPath = join(reportRoot, 'production.json');
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
    console.log(`CEM Site production verification passed: ${relative(workspaceRoot, reportPath)}`);
} finally {
    await browser?.close();
    await new Promise((resolveClose) => server.close(resolveClose));
}

function contentType(filePath) {
    switch (extname(filePath)) {
        case '.html':
            return 'text/html; charset=utf-8';
        case '.js':
            return 'text/javascript; charset=utf-8';
        case '.css':
            return 'text/css; charset=utf-8';
        case '.wasm':
            return 'application/wasm';
        case '.json':
        case '.map':
            return 'application/json; charset=utf-8';
        default:
            return 'application/octet-stream';
    }
}
