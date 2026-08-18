#!/usr/bin/env node

import { createReadStream } from 'node:fs';
import { stat } from 'node:fs/promises';
import { createServer } from 'node:http';
import { dirname, extname, join, normalize, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { chromium } from 'playwright';

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const timeout = 45_000;

const htmlDemoElementModule = `
class HtmlDemoElement extends HTMLElement {
    connectedCallback() {
        if (this.__cemDemoMounted) return;
        this.__cemDemoMounted = true;
        const template = Array.from(this.children).find((child) => child.localName === 'template');
        if (!template) return;
        const demo = document.createElement('div');
        demo.setAttribute('slot', 'demo');
        demo.append(template.content.cloneNode(true));
        this.append(demo);
    }
}
customElements.define('html-demo-element', HtmlDemoElement);
`;

const fixtureSpecs = [
    {
        path: '/packages/cem-elements/index.html',
        checks: [
            text('dce-link a', 'link'),
            text('dce-1-slot', '\u{1f955}'),
            normalizedText('dce-3-slot', '#1 \u{1f955} and \u{1f955}'),
            normalizedText('dce-4-slot', '#2 \u{1f955} and \u{1f955}'),
            text('pokemon-tile h3', 'bulbasaur'),
            text('pokemon-tile', 'Smile as:'),
            attributeContains('pokemon-tile img[alt="bulbasaur image"]', 'src', '/1.svg'),
            countAtLeast('pokemon-tile button', 3),
            text('pokemon-tile button', 'ivysaur'),
            text('pokemon-tile button', 'venusaur'),
            text('pokemon-tile button', 'vulpix'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/attributes.html',
        checks: [
            text('#defaults-1 article.demo-card h2', 'attributes definition'),
            text('#defaults-2 article.demo-card', 'p1: 123'),
            text('#live-attr h2', 'Before'),
            clickThenText('button[data-target="#live-attr"][data-attr="label"]', '#live-attr h2', 'After'),
            clickThenText('button[data-target="#live-attr"][data-attr="tone"]', '#live-attr article.demo-card', 'tone: alert'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/data-slices.html',
        checks: [
            text('cem-slice-field section.demo-card', 'query:'),
            fillThenText('cem-slice-field input[type="text"]', 'demo query', 'cem-slice-field output', 'demo query'),
            clickThenText('cem-slice-field button[data-dispatch-select]', 'cem-slice-field output', 'cem-select'),
            clickThenText('cem-slice-field button[data-role="increment"]', 'cem-slice-field output[data-role="count"]', '1'),
            fillThenText('cem-slice-field input[data-role="fanout"]', 'mirrored', 'cem-slice-field output[data-role="s1"]', 'mirrored'),
            text('cem-slice-field output[data-role="s2"]', 'mirrored'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/dom-merge.html',
        checks: [
            text('cem-dom-merge-field article.demo-card h2', 'Word count in textarea'),
            fillThenText('cem-dom-merge-field input[type="text"]', 'two words', 'cem-dom-merge-field blockquote', 'two words'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/external-template.html',
        allowedPageErrors: ['Failed to load resource: the server responded with a status of 404 (Not Found)'],
        checks: [
            text('dce-internal', '👋'),
            text('dce-internal', 'World!'),
            countAtLeast('dce-construction', 2),
            text('dce-construction', 'construction'),
            countAtLeast('dce-external svg', 1),
            countAtLeast('dce-external-inline svg', 1),
            text('dce-external-missing', 'fallback for missing image'),
            text('dce-external-4', 'DCE with external CEM-ML template'),
            text('dce-external-4-inline', 'inline DCE loading from CEM-ML template'),
            text('dce-external-5', '👋'),
            text('dce-external-5', '👌'),
            countAtLeast('dce-external-5 svg', 1),
            countAtLeast('dce-external-5 math', 1),
            text('dce-html-wave', '👋'),
            countAtLeast('dce-html-logo svg', 1),
            countAtLeast('dce-html-formula math', 1),
            text('dce-cemt-tree', 'CEM-ML data island tree'),
            text('dce-cemt-tree', 'title='),
            text('dce-cemt-tree', 'Anonymous DCE data island'),
            text('dce-cemt-tree', 'data-demo='),
            text('dce-cemt-tree', 'data-root='),
            text('dce-cemt-tree', 'data-level='),
            text('dce-cemt-tree', 'cem-elements'),
            text('dce-cemt-tree', 'code='),
            text('dce-cemt-tree', 'a1'),
            text('dce-cemt-tree', 'Leaf text from cem-elements data island'),
            countAtLeast('dce-cemt-tree details', 5),
            text('dce-xslt-tree', 'XSLT data island tree'),
            text('dce-xslt-tree', 'title='),
            text('dce-xslt-tree', 'Anonymous DCE data island'),
            text('dce-xslt-tree', 'data-demo='),
            text('dce-xslt-tree', 'data-root='),
            text('dce-xslt-tree', 'data-level='),
            text('dce-xslt-tree', 'cem-elements-xslt'),
            text('dce-xslt-tree', 'code='),
            text('dce-xslt-tree', 'b1'),
            text('dce-xslt-tree', 'Leaf text from cem-elements XSLT data island'),
            countAtLeast('dce-xslt-tree details', 5),
            text('dce-missing-none', 'element with id=none is missing in template'),
            text('dce-embed-1', '🖖'),
            text('dce-embed-relative-hash', 'from embed-lib-component'),
            text('dce-embed-relative-file', '🖖'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/for-each.html',
        checks: [
            countAtLeast('cem-loop-list article.demo-card', 6),
            countAtLeast('cem-loop-list li', 3),
            text('cem-loop-list li', 'Apple'),
            countAtLeast('cem-loop-list tbody tr', 3),
            clickThenText('cem-loop-list input[type="checkbox"]', 'cem-loop-list span', '1 : First'),
            text('cem-loop-list .payload-feed li', 'payload-alpha : Payload Alpha'),
            text('cem-loop-list .payload-feed li', 'payload-beta : Payload Beta'),
            text('cem-loop-list .location-feed li', 'topic = feeds'),
            text('cem-loop-list .location-feed li', 'item = payload,resource'),
            text('cem-loop-list output[data-role="json-state"]', 'loaded'),
            text('cem-loop-list .http-json-feed li', 'alpha : ready'),
            text('cem-loop-list .http-json-feed li', 'beta : loaded'),
            text('cem-loop-list output[data-role="xml-state"]', 'loaded'),
            text('cem-loop-list .http-xml-feed li', 'gamma : xml-ready'),
            text('cem-loop-list .http-xml-feed li', 'delta : xml-loaded'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/form.html',
        checks: [
            text('cem-form-preview article.demo-card', 'password slice set:'),
            fillThenText('cem-form-preview input[name="username"]', 'ada', 'cem-form-preview output[data-role="form-username"]', 'ada'),
            text('cem-form-preview output[data-role="mirror-username"]', 'ada'),
            text('cem-form-preview output[data-role="form-valid"]', 'false'),
            text('cem-form-preview output[data-role="form-message"]', 'enter username and password'),
            text('cem-form-preview output[data-role="password-message"]', 'password is too short'),
            fillThenText('cem-form-preview input[name="password"]', 'secret', 'cem-form-preview output[data-role="password-valid"]', 'true'),
            text('cem-form-preview output[data-role="form-valid"]', 'true'),
            text('cem-form-preview p', 'yes'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/hex-grid.html',
        checks: [countAtLeast('cem-grid-tile .swatch', 6), text('cem-grid-tile .swatch', 'B3')],
    },
    {
        path: '/packages/cem-elements/demo/http-request.html',
        checks: [
            text('cem-resource-panel article.demo-card h2', 'HTTP request parity target'),
            text('cem-resource-panel article.demo-card', 'Requested URL: ./http-data.json'),
            text('cem-resource-panel article.demo-card', 'state: loaded'),
            text('cem-resource-panel li', 'alpha : ready'),
            text('cem-resource-panel li', 'beta : loaded'),
            text('cem-resource-panel article.demo-card', 'xml state: loaded'),
            text('cem-resource-panel ul.xml-results li', 'gamma : xml-ready'),
            text('cem-resource-panel ul.xml-results li', 'delta : xml-loaded'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/local-storage.html',
        checks: [
            text('cem-local-storage-panel article.demo-card', 'draft: stored initial'),
            text('cem-local-storage-panel article.demo-card', 'number: 3'),
            text('cem-local-storage-panel article.demo-card', 'basket fruit: apple'),
            fillThenText('cem-local-storage-panel input', 'stored draft', 'cem-local-storage-panel output[data-role="draft"]', 'stored draft'),
            clickThenText(
                'cem-local-storage-panel button[data-storage-write="draft"]',
                'cem-local-storage-panel output[data-role="draft"]',
                'external update'
            ),
        ],
    },
    {
        path: '/packages/cem-elements/demo/location-element.html',
        checks: [
            text('cem-location-panel article.demo-card', 'current href:'),
            text('cem-location-panel article.demo-card', 'sample host: example.test'),
            text('cem-location-panel article.demo-card', 'sample hash: #fixture'),
            text('cem-location-panel ul.sample-params', 'mode = demo'),
            text('cem-location-panel ul.sample-params', 'tag = one,two'),
            clickThenText(
                'cem-location-panel button[data-location-push="true"]',
                'cem-location-panel output[data-role="current-hash"]',
                '#checked'
            ),
        ],
    },
    {
        path: '/packages/cem-elements/demo/module-url.html',
        checks: [
            attributeContains('cem-module-link a', 'href', '/packages/custom-element/material/'),
            attributeContains('cem-module-link img.resolved-logo', 'src', '/packages/custom-element/demo/wc-square.svg'),
            text('cem-module-link p', 'wc-square.svg'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/npm-versions-demo.html',
        checks: [
            text('cem-version-row article.demo-card h2', '@epa-wg/cem-elements'),
            text('cem-version-row article.demo-card', 'selected version: workspace'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/scoped-css.html',
        checks: [
            text('cem-style-green button[data-case="case-1-green"]', 'Green dashed border'),
            computedStyle('cem-style-green button[data-case="case-1-green"]', 'borderTopStyle', 'dashed'),
            computedStyle('cem-style-green button[data-case="case-1-green"]', 'borderTopColor', 'rgb(0, 128, 0)'),
            computedStyleNot('button[data-case="case-1-default"]', 'borderTopStyle', 'dashed'),
            countAtLeast('cem-style-blue button[data-case="case-2-blue"]', 2),
            text('cem-style-blue', '1st'),
            text('cem-style-blue', '2nd'),
            computedStyle('cem-style-blue button[data-case="case-2-blue"]', 'borderTopStyle', 'dotted'),
            computedStyle('cem-style-blue button[data-case="case-2-blue"]', 'borderTopColor', 'rgb(0, 0, 255)'),
            computedStyleNot('button[data-case="case-2-default"]', 'borderTopStyle', 'dotted'),
            text('cem-style-override[data-case="case-3-red"] button', 'Red border'),
            computedStyle('cem-style-override[data-case="case-3-blue"] button', 'borderTopColor', 'rgb(0, 0, 255)'),
            computedStyle('cem-style-override[data-case="case-3-red"] button', 'borderTopColor', 'rgb(255, 0, 0)'),
            computedStyleNot('button[data-case="case-3-default"]', 'borderTopColor', 'rgb(255, 0, 0)'),
            computedStyle('cem-style-internal label[data-case="case-4-label"]', 'color', 'rgb(0, 128, 0)'),
            computedStyle('cem-style-internal b[data-case="case-4-blue"]', 'color', 'rgb(0, 0, 139)'),
            computedStyle('cem-style-inline-src p[data-case="case-5-p"]', 'color', 'rgb(0, 128, 0)'),
            computedStyle('cem-style-inline-src i[data-case="case-5-i"]', 'color', 'rgb(0, 0, 255)'),
            text('cem-style-external-src .external-scoped-item', 'External scoped template'),
            computedStyle('cem-style-external-src .external-scoped-item', 'backgroundColor', 'rgb(254, 243, 199)'),
            computedStyleNot('p[data-case="case-6-outside"]', 'backgroundColor', 'rgb(254, 243, 199)'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/set-url.html',
        checks: [
            text('cem-set-url-panel article.demo-card', 'pending set:'),
            fillThenText(
                'cem-set-url-panel label:nth-of-type(2) input',
                '#verified',
                'cem-set-url-panel p',
                '#verified'
            ),
            clickThenText(
                'cem-set-url-panel button',
                'cem-set-url-panel output[data-role="current-hash"]',
                '#verified'
            ),
        ],
    },
];

const server = createServer(async (request, response) => {
    try {
        const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');
        const pathname = decodeURIComponent(requestUrl.pathname === '/' ? '/packages/cem-elements/index.html' : requestUrl.pathname);
        const filePath = normalize(join(repoRoot, pathname));
        if (filePath !== repoRoot && !filePath.startsWith(repoRoot + sep)) {
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
    for (const fixture of fixtureSpecs) {
        const pageErrors = [];
        const page = await browser.newPage();
        page.on('pageerror', (error) => pageErrors.push(error.message));
        page.on('console', (message) => {
            if (message.type() === 'error') {
                pageErrors.push(message.text());
            }
        });
        await installOfflineRoutes(page);
        await installTextHelpers(page);

        try {
            await page.goto(`http://127.0.0.1:${port}${fixture.path}`, { waitUntil: 'networkidle' });
            await page.waitForTimeout(250);
            for (const check of fixture.checks) {
                await runCheck(page, check);
            }
        } catch (error) {
            const unexpectedErrors = unexpectedPageErrors(fixture, pageErrors);
            const diagnostics =
                unexpectedErrors.length > 0
                    ? `\nBrowser errors:\n${unexpectedErrors.map((item) => `- ${item}`).join('\n')}`
                    : '';
            const snapshot = await collectDebugSnapshot(page, error?.check);
            throw new Error(
                `${fixture.path} failed while running ${describeCheck(error?.check)}:\n${error.message}${diagnostics}${snapshot}`
            );
        } finally {
            await page.close();
        }

        const unexpectedErrors = unexpectedPageErrors(fixture, pageErrors);
        if (unexpectedErrors.length > 0) {
            throw new Error(
                `${fixture.path} emitted browser errors:\n${unexpectedErrors.map((error) => `- ${error}`).join('\n')}`
            );
        }
    }
} finally {
    await browser.close();
    await new Promise((resolveClose) => server.close(resolveClose));
}

console.log(`cem-elements demo fixtures verified (${fixtureSpecs.length} pages).`);

function unexpectedPageErrors(fixture, pageErrors) {
    const allowed = fixture.allowedPageErrors ?? [];
    return pageErrors.filter((error) => !allowed.some((allowedError) => error.includes(allowedError)));
}

async function installOfflineRoutes(page) {
    await page.route('https://unpkg.com/html-demo-element@*/html-demo-element.js', (route) =>
        route.fulfill({ contentType: 'text/javascript; charset=utf-8', body: htmlDemoElementModule })
    );
    await page.route(/^https:\/\/unpkg\.com\/pokeapi-sprites@.*\.svg$/, (route) =>
        route.fulfill({
            contentType: 'image/svg+xml; charset=utf-8',
            body: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><title>fixture sprite</title></svg>',
        })
    );
}

async function installTextHelpers(page) {
    await page.addInitScript(() => {
        globalThis.__cemFixtureVisibleText = (root) => {
            const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT, {
                acceptNode(node) {
                    const parent = node.parentElement;
                    if (!parent || parent.closest('template,script,style,[hidden]')) {
                        return NodeFilter.FILTER_REJECT;
                    }
                    return NodeFilter.FILTER_ACCEPT;
                },
            });
            const parts = [];
            for (let node = walker.nextNode(); node; node = walker.nextNode()) {
                parts.push(node.textContent ?? '');
            }
            return parts.join(' ');
        };
        globalThis.__cemFixtureNormalizeText = (value) => value.replace(/\s+/gu, ' ').trim();
    });
}

async function runCheck(page, check) {
    try {
        switch (check.kind) {
            case 'text':
                await waitForText(page, check.selector, check.expected);
                return;
            case 'normalizedText':
                await waitForNormalizedText(page, check.selector, check.expected);
                return;
            case 'countAtLeast':
                await waitForCount(page, check.selector, check.min);
                return;
            case 'attributeContains':
                await waitForAttribute(page, check.selector, check.name, check.expected);
                return;
            case 'computedStyle':
                await waitForComputedStyle(page, check.selector, check.property, check.expected);
                return;
            case 'computedStyleNot':
                await waitForComputedStyleNot(page, check.selector, check.property, check.unexpected);
                return;
            case 'clickThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.click(check.actionSelector);
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'fillThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.fill(check.actionSelector, check.value);
                await waitForText(page, check.resultSelector, check.expected);
                return;
            default:
                throw new Error(`unknown check kind ${check.kind}`);
        }
    } catch (error) {
        error.check = check;
        throw error;
    }
}

async function collectDebugSnapshot(page, check) {
    try {
        const snapshot = await page.evaluate((failedCheck) => {
            const customElementTags = [
                'dce-link',
                'dce-1-slot',
                'dce-2-slots',
                'dce-3-slot',
                'dce-4-slot',
                'pokemon-tile',
                'cem-attr-card',
                'cem-attr-defaults',
                'cem-attr-slice',
                'cem-slice-field',
                'cem-loop-list',
                'cem-style-green',
                'cem-style-blue',
                'cem-style-override',
                'cem-style-internal',
                'cem-style-inline-src',
                'cem-style-external-src',
                'cem-form-preview',
            ];
            const failedSelector =
                failedCheck?.selector ?? failedCheck?.resultSelector ?? failedCheck?.actionSelector ?? undefined;
            const failedTexts = failedSelector
                ? Array.from(document.querySelectorAll(failedSelector)).map((element) =>
                      globalThis.__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element))
                  )
                : [];
            return {
                bodyText: globalThis.__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(document.body)).slice(0, 1000),
                failedSelector,
                failedExpected: failedCheck?.expected,
                failedComputedValue:
                    failedCheck?.kind === 'computedStyle' && failedSelector
                        ? getComputedStyle(document.querySelector(failedSelector))?.[failedCheck.property]
                        : null,
                styles: Array.from(document.querySelectorAll('style')).map((style) => style.textContent?.trim().slice(0, 1000) ?? ''),
                failedCheckNow:
                    failedCheck?.kind === 'text' && typeof failedCheck.expected === 'string'
                        ? failedTexts.some((value) => value.includes(failedCheck.expected))
                        : null,
                failedSelectorMatches: failedSelector
                    ? Array.from(document.querySelectorAll(failedSelector)).map((element) => ({
                          tag: element.localName,
                          text: globalThis.__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element)).slice(0, 500),
                          html: element.outerHTML.slice(0, 1000),
                      }))
                    : [],
                elements: customElementTags
                    .flatMap((tag) => Array.from(document.querySelectorAll(tag)))
                    .map((element) => ({
                        tag: element.localName,
                        text: globalThis.__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element)).slice(0, 500),
                        html: element.outerHTML.slice(0, 1000),
                    })),
            };
        }, check);
        return `\nDebug snapshot:\n${JSON.stringify(snapshot, null, 2)}`;
    } catch (error) {
        return `\nDebug snapshot unavailable: ${error instanceof Error ? error.message : String(error)}`;
    }
}

function text(selector, expected) {
    return { kind: 'text', selector, expected };
}

function normalizedText(selector, expected) {
    return { kind: 'normalizedText', selector, expected };
}

function countAtLeast(selector, min) {
    return { kind: 'countAtLeast', selector, min };
}

function attributeContains(selector, name, expected) {
    return { kind: 'attributeContains', selector, name, expected };
}

function computedStyle(selector, property, expected) {
    return { kind: 'computedStyle', selector, property, expected };
}

function computedStyleNot(selector, property, unexpected) {
    return { kind: 'computedStyleNot', selector, property, unexpected };
}

function clickThenText(actionSelector, resultSelector, expected) {
    return { kind: 'clickThenText', actionSelector, resultSelector, expected };
}

function fillThenText(actionSelector, value, resultSelector, expected) {
    return { kind: 'fillThenText', actionSelector, value, resultSelector, expected };
}

async function waitForText(page, selector, expected) {
    await poll(
        page,
        ({ selector: checkSelector, expected: checkExpected }) => {
            const elements = Array.from(document.querySelectorAll(checkSelector));
            return elements.some((element) =>
                globalThis.__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element)).includes(checkExpected)
            );
        },
        { selector, expected }
    );
}

async function waitForNormalizedText(page, selector, expected) {
    await poll(
        page,
        ({ selector: checkSelector, expected: checkExpected }) => {
            const element = document.querySelector(checkSelector);
            return element
                ? globalThis.__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element)) === checkExpected
                : false;
        },
        { selector, expected }
    );
}

async function waitForCount(page, selector, min) {
    await poll(
        page,
        ({ selector: checkSelector, min: checkMin }) => document.querySelectorAll(checkSelector).length >= checkMin,
        { selector, min }
    );
}

async function waitForAttribute(page, selector, name, expected) {
    await poll(
        page,
        ({ selector: checkSelector, name: attributeName, expected: checkExpected }) => {
            const element = document.querySelector(checkSelector);
            return element ? (element.getAttribute(attributeName) ?? '').includes(checkExpected) : false;
        },
        { selector, name, expected }
    );
}

async function waitForComputedStyle(page, selector, property, expected) {
    await poll(
        page,
        ({ selector: checkSelector, property: styleProperty, expected: checkExpected }) => {
            const element = document.querySelector(checkSelector);
            return element ? getComputedStyle(element)[styleProperty] === checkExpected : false;
        },
        { selector, property, expected }
    );
}

async function waitForComputedStyleNot(page, selector, property, unexpected) {
    await poll(
        page,
        ({ selector: checkSelector, property: styleProperty, unexpected: checkUnexpected }) => {
            const element = document.querySelector(checkSelector);
            return element ? getComputedStyle(element)[styleProperty] !== checkUnexpected : false;
        },
        { selector, property, unexpected }
    );
}

async function poll(page, predicate, arg) {
    const startedAt = Date.now();
    let lastError;
    while (Date.now() - startedAt <= timeout) {
        try {
            if (await page.evaluate(predicate, arg)) {
                return;
            }
        } catch (error) {
            lastError = error;
        }
        await page.waitForTimeout(500);
    }
    try {
        if (await page.evaluate(predicate, arg)) {
            return;
        }
    } catch (error) {
        lastError = error;
    }
    await page.waitForTimeout(500);
    try {
        if (await page.evaluate(predicate, arg)) {
            return;
        }
    } catch (error) {
        lastError = error;
    }
    if (lastError) {
        throw lastError;
    }
    throw new Error(`poll timed out after ${timeout}ms`);
}

function describeCheck(check) {
    if (!check) return 'unknown check';
    switch (check.kind) {
        case 'text':
        case 'normalizedText':
            return `${check.kind}(${check.selector}, ${JSON.stringify(check.expected)})`;
        case 'countAtLeast':
            return `countAtLeast(${check.selector}, ${check.min})`;
        case 'attributeContains':
            return `attributeContains(${check.selector}, ${check.name}, ${JSON.stringify(check.expected)})`;
        case 'computedStyle':
            return `computedStyle(${check.selector}, ${check.property}, ${JSON.stringify(check.expected)})`;
        case 'computedStyleNot':
            return `computedStyleNot(${check.selector}, ${check.property}, ${JSON.stringify(check.unexpected)})`;
        case 'clickThenText':
            return `clickThenText(${check.actionSelector}, ${check.resultSelector}, ${JSON.stringify(check.expected)})`;
        case 'fillThenText':
            return `fillThenText(${check.actionSelector}, ${check.resultSelector}, ${JSON.stringify(check.expected)})`;
        default:
            return check.kind;
    }
}

function contentType(filePath) {
    switch (extname(filePath)) {
        case '.html':
            return 'text/html; charset=utf-8';
        case '.cemt':
            return 'text/cem-ml; charset=utf-8';
        case '.xhtml':
            return 'application/xhtml+xml; charset=utf-8';
        case '.xsl':
            return 'application/xslt+xml; charset=utf-8';
        case '.js':
        case '.mjs':
            return 'text/javascript; charset=utf-8';
        case '.json':
            return 'application/json; charset=utf-8';
        case '.xml':
            return 'application/xml; charset=utf-8';
        case '.wasm':
            return 'application/wasm';
        case '.css':
            return 'text/css; charset=utf-8';
        case '.svg':
            return 'image/svg+xml; charset=utf-8';
        case '.map':
            return 'application/json; charset=utf-8';
        default:
            return 'application/octet-stream';
    }
}
