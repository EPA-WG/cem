#!/usr/bin/env node

import { createReadStream } from 'node:fs';
import { readdir, stat } from 'node:fs/promises';
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
            attributeEquals(
                'html-demo-element[legend^="2a."] dce-2-slots:first-of-type input',
                'placeholder',
                '\u{1f407}\u{2764}\u{fe0f}\u{1f955}',
            ),
            attributeEquals(
                'html-demo-element[legend^="2a."] dce-2-slots:last-of-type input',
                'placeholder',
                '\u{1f407}\u{2764}\u{fe0f}\u{1f407}',
            ),
            normalizedText('html-demo-element[legend^="2b."] dce-3-slot:first-of-type', '1 \u{1f603} 2 \u{1f603}'),
            normalizedText('html-demo-element[legend^="2c."] dce-4-slot', '1 \u{1f955} 2 \u{1f955}'),
            text('pokemon-tile h3', 'bulbasaur'),
            text('pokemon-tile', 'Smile as:'),
            attributeEquals(
                'pokemon-tile img[alt="bulbasaur image"]',
                'src',
                'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/1.svg',
            ),
            attributeEquals(
                'pokemon-tile button img[alt="ivysaur"]',
                'src',
                'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/2.svg',
            ),
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
            text('#defaults-2 article.demo-card', 'p2: always_p2'),
            attributeEquals('#defaults-1', 'p1', 'default_P1'),
            attributeEquals('#defaults-1', 'p2', 'always_p2'),
            attributeEquals('#defaults-1', 'p3', 'def_P3'),
            clickThenText('button[data-target="#defaults-2"][data-attr="p2"]', '#defaults-2', 'p2: always_p2'),
            attributeEquals('#defaults-2', 'p2', 'always_p2'),
            attributeEquals('#title-from-slice', 'title', '😃'),
            fillThenText('#title-from-slice input', 'Typed title', '#title-from-slice', 'title attribute: Typed title'),
            attributeEquals('#title-from-slice', 'title', 'Typed title'),
            attributeEquals('#value-default', 'v', 'def'),
            attributeEquals('#value-default', 'is-changed', 'false'),
            fillThenText('#value-default input', 'From input', '#value-default', 'v: From input'),
            attributeEquals('#value-default', 'v', 'From input'),
            attributeEquals('#value-default', 'is-changed', 'true'),
            text('#precedence-default article.demo-card', '/datadom/attributes/v: def'),
            typeThenText(
                '#precedence-default input',
                'qqq',
                '#precedence-default article.demo-card',
                '/datadom/attributes/v: qqq',
            ),
            text('#precedence-default article.demo-card', 'effective value: qqq'),
            text('#precedence-default article.demo-card', 'has-input: true'),
            attributeEquals('#precedence-default', 'v', 'qqq'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/data-slices.html',
        checks: [
            attributeEquals('cem-slice-counter-input-default input', 'value', '0'),
            text('cem-slice-counter-input-default article.demo-card', '0'),
            clickThenText(
                'cem-slice-counter-input-default button:first-of-type',
                'cem-slice-counter-input-default article.demo-card',
                '1',
            ),
            text('cem-slice-counter output', '0'),
            clickThenText(
                'cem-slice-counter button:first-of-type',
                'cem-slice-counter output',
                '1',
            ),
            mouseThenText(
                'cem-slice-event-data textarea',
                { x: 42, y: 17 },
                'cem-slice-event-data p:nth-of-type(1) output',
                'x:',
            ),
            computedStyleNot('cem-slice-event-data textarea', 'boxShadow', 'none'),
            computedStyleNot('cem-slice-event-data textarea', 'boxShadow', ''),
            text('cem-slice-event-data p:nth-of-type(2) output', 'mousemove'),
            fillBlurThenText('cem-slice-basic input', 'basic', 'cem-slice-basic output', 'basic'),
            text('cem-slice-initial-change output', 'B'),
            fillBlurThenText('cem-slice-initial-change input', 'changed', 'cem-slice-initial-change output', 'changed'),
            fillThenText(
                'cem-slice-initial-input input',
                'input event',
                'cem-slice-initial-input output',
                'input event',
            ),
            text('cem-slice-attribute-initial:first-of-type p:nth-of-type(1) output', '😁'),
            text('cem-slice-attribute-initial:last-of-type p:nth-of-type(1) output', '🤗'),
            fillDispatchThenText(
                'cem-slice-attribute-initial:first-of-type input',
                'qqq',
                'keyup',
                'cem-slice-attribute-initial:first-of-type p:nth-of-type(2) output',
                'qqq',
            ),
            text('cem-slice-transform output', 'xB'),
            fillBlurThenText('cem-slice-transform input', 'C', 'cem-slice-transform output', 'xC'),
            text('cem-slice-broccoli output', 'anonymous'),
            clickThenText('cem-slice-broccoli button', 'cem-slice-broccoli output', 'broccoli'),
            clickThenText('cem-slice-declared-counter button', 'cem-slice-declared-counter output', '1'),
            focusThenText(
                'cem-slice-multi-directive button',
                'cem-slice-multi-directive p:nth-of-type(2) output',
                '1',
            ),
            clickThenText(
                'cem-slice-multi-directive button',
                'cem-slice-multi-directive p:nth-of-type(1) output',
                '1',
            ),
            blurThenText(
                'cem-slice-multi-directive button',
                'cem-slice-multi-directive p:nth-of-type(2) output',
                '0',
            ),
            text('cem-slice-emotion-attribute:first-of-type output', ':)'),
            text('cem-slice-emotion-attribute:last-of-type output', '😃'),
            fillBlurThenText('cem-slice-emotion-attribute:last-of-type input', 'joyful', 'cem-slice-emotion-attribute:last-of-type output', 'joyful'),
            attributeEquals('cem-slice-emotion-attribute:last-of-type', 'emotion', 'joyful'),
            fillThenText('cem-slice-fanout input', 'mirrored', 'cem-slice-fanout p:nth-of-type(2) output', 'mirrored'),
            text('cem-slice-fanout p:nth-of-type(3) output', 'mirrored'),
            fillBlurThenText(
                'cem-slice-attribute-fanout input',
                'grinning',
                'cem-slice-attribute-fanout p:nth-of-type(1) output',
                'grinning',
            ),
            text('cem-slice-attribute-fanout p:nth-of-type(2) output', 'grinning'),
            attributeEquals('cem-slice-attribute-fanout', 'emotion', 'grinning'),
            text('cem-slice-checkboxes p:nth-of-type(1) output', 'V0'),
            uncheckThenNormalizedText(
                'cem-slice-checkboxes label:nth-of-type(1) input',
                'cem-slice-checkboxes p:nth-of-type(1) output',
                '',
            ),
            checkThenText(
                'cem-slice-checkboxes label:nth-of-type(2) input',
                'cem-slice-checkboxes p:nth-of-type(3) output',
                'V1',
            ),
            checkThenText(
                'cem-slice-checkboxes label:nth-of-type(3) input',
                'cem-slice-checkboxes p:nth-of-type(4) output',
                'V1',
            ),
            text('cem-slice-radios output', 'V1'),
            checkThenText('cem-slice-radios label:first-of-type input', 'cem-slice-radios output', 'V0'),
        ],
    },
    {
        path: '/packages/cem-elements/demo/dom-merge.html',
        checks: [
            text('cem-dom-merge-field article.demo-card h2', 'Word count in textarea'),
            fillThenText(
                'cem-dom-merge-field input[type="text"]',
                'two words',
                'cem-dom-merge-field blockquote',
                'two words',
            ),
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
            svgUseReferences('dce-external svg', ['#h', '#j']),
            countAtLeast('dce-external-inline svg', 1),
            svgUseReferences('dce-external-inline svg', ['#h', '#j']),
            text('dce-external-missing', 'fallback for missing image'),
            text('dce-external-4', 'External CEMT data-island transformation'),
            text('dce-external-4', 'template[data-cem-island="instance"]'),
            text('dce-external-4', 'cem-island:context-root'),
            text('dce-external-4', 'cem-hydration:data'),
            text('dce-external-4', 'cem-attributes:attributes'),
            text('dce-external-4', 'cem-dataset:dataset'),
            text('dce-external-4', 'cem-payload:payload'),
            text('dce-external-4', 'cem-slices:slices'),
            text('dce-external-4', 'cem-resources:resources'),
            text('dce-external-4', 'cem-form:form-state'),
            text('dce-external-4', 'cem-validation:validation-state'),
            text('dce-external-4', 'cem-events:event-state'),
            text('dce-external-4', 'Payload comment: explicit inert envelope follows'),
            text('dce-external-4', 'DCE with complete external CEMT island'),
            text('dce-external-4', 'wrapped-payload'),
            text('dce-external-4', 'slot="heading"'),
            text('dce-external-4', 'slot=""'),
            text('dce-external-4', 'data-fruit="🍌"'),
            text('dce-external-4', 'aria-label="Fruit choice"'),
            text('dce-external-4', 'Every element, attribute, dataset entry, and text node is data.'),
            countAtLeast('dce-external-4 details', 20),
            text('dce-external-4-inline', 'A second external-CEMT data island'),
            text('dce-external-4-inline', 'DCE with live payload capture'),
            text('dce-external-4-inline', 'name="data-smile"'),
            text('dce-external-4-inline', 'name="data-basket"'),
            text('dce-external-4-inline', 'data-kind="live-payload"'),
            text('dce-external-4-cem-ml', 'content-type="text/cem-ml"'),
            text('dce-external-4-cem-ml', 'Banana from CEM-ML payload source'),
            text('dce-external-5', '👋'),
            text('dce-external-5', '👌'),
            countAtLeast('dce-external-5 svg', 1),
            countAtLeast('dce-external-5 math', 1),
            attributeEquals('#dce-external-5-inline', 'data-cem-anonymous-declaration', ''),
            text('#dce-external-5-inline > [data-cem-anonymous-instance]', '👋'),
            text('#dce-external-5-inline > [data-cem-anonymous-instance]', '👌'),
            countAtLeast('#dce-external-5-inline > [data-cem-anonymous-instance] svg', 1),
            countAtLeast('#dce-external-5-inline > [data-cem-anonymous-instance] math', 1),
            text('dce-html-wave', '👋'),
            countAtLeast('dce-html-logo svg', 1),
            countAtLeast('dce-html-formula math', 1),
            text('dce-cemt-tree', 'CEM-ML data island tree'),
            text('dce-cemt-tree article.demo-card > details > summary > b', 'catalog'),
            text('dce-cemt-tree', 'data-root='),
            text('dce-cemt-tree', 'data-level='),
            text('dce-cemt-tree', 'cem-elements'),
            text('dce-cemt-tree', 'code='),
            text('dce-cemt-tree', 'a1'),
            text('dce-cemt-tree', 'Leaf text from cem-elements data island'),
            countAtLeast('dce-cemt-tree details', 4),
            text('dce-xslt-tree', 'XSLT data island tree'),
            text('dce-xslt-tree article.demo-card > details > summary > b', 'catalog'),
            text('dce-xslt-tree', 'data-root='),
            text('dce-xslt-tree', 'data-level='),
            text('dce-xslt-tree', 'cem-elements-xslt'),
            text('dce-xslt-tree', 'code='),
            text('dce-xslt-tree', 'b1'),
            text('dce-xslt-tree', 'Leaf text from cem-elements XSLT data island'),
            countAtLeast('dce-xslt-tree details', 4),
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
            fillThenText(
                'cem-form-preview input[name="username"]',
                'ada',
                'cem-form-preview output[data-role="form-username"]',
                'ada',
            ),
            text('cem-form-preview output[data-role="mirror-username"]', 'ada'),
            text('cem-form-preview output[data-role="form-valid"]', 'false'),
            text('cem-form-preview output[data-role="form-message"]', 'enter username and password'),
            text('cem-form-preview output[data-role="password-message"]', 'password is too short'),
            fillThenText(
                'cem-form-preview input[name="password"]',
                'secret',
                'cem-form-preview output[data-role="password-valid"]',
                'true',
            ),
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
            fillThenText(
                'cem-local-storage-panel input',
                'stored draft',
                'cem-local-storage-panel output[data-role="draft"]',
                'stored draft',
            ),
            clickThenText(
                'cem-local-storage-panel button[data-storage-write="draft"]',
                'cem-local-storage-panel output[data-role="draft"]',
                'external update',
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
                '#checked',
            ),
        ],
    },
    {
        path: '/packages/cem-elements/demo/module-url.html',
        checks: [
            attributeContains('cem-module-link a', 'href', '/packages/custom-element/material/'),
            attributeContains(
                'cem-module-link img.resolved-logo',
                'src',
                '/packages/custom-element/demo/wc-square.svg',
            ),
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
            // Private declaration CSS: one managed style, native tag scope,
            // no style cloned into either instance, the classless outside button
            // keeps browser/global styling, and ordinary outer cascade remains open.
            countExactly('cem-css-private button', 2),
            text('cem-css-private button', 'First DCE dashed border'),
            text('cem-css-private button', 'Second DCE dashed border'),
            text('button', 'Browser default border'),
            computedStyleByText('button', 'First DCE dashed border', 'borderTopStyle', 'dashed'),
            computedStyleByText('button', 'First DCE dashed border', 'borderTopColor', 'rgb(0, 128, 0)'),
            computedStyleByText('button', 'First DCE dashed border', 'color', 'rgb(148, 0, 211)'),
            computedStyleByText('button', 'Second DCE dashed border', 'borderTopStyle', 'dashed'),
            computedStyleByText('button', 'Second DCE dashed border', 'borderTopColor', 'rgb(0, 128, 0)'),
            computedStyleByText('button', 'Second DCE dashed border', 'color', 'rgb(148, 0, 211)'),
            computedStyleNotByText('button', 'Browser default border', 'borderTopStyle', 'dashed'),
            computedStyleNotByText('button', 'Browser default border', 'borderTopColor', 'rgb(0, 128, 0)'),
            countExactly('cem-element[tag="cem-css-private"] > style[data-cem-declaration-style="private"]', 1),
            countExactly('cem-css-private style', 0),
            styleTextContains(
                'cem-element[tag="cem-css-private"] > style[data-cem-declaration-style="private"]',
                '@scope (\n    cem-css-private',
            ),
            countExactly('cem-css-private[data-cem-render-scope*="cem-scope-"]', 2),
            countExactly(
                'cem-css-private[data-cem-scope], cem-css-private[data-cem-instance-scope], cem-css-private[scope]',
                0,
            ),

            // Bare-only and explicit-only shared declarations both target the public
            // group boundary and apply to separately declared group peers.
            computedStyle('cem-css-shared-bare .sample-shared-bare', 'color', 'rgb(0, 128, 0)'),
            computedStyle('cem-css-shared-peer .sample-shared-bare', 'color', 'rgb(0, 128, 0)'),
            attributeEquals('cem-css-shared-bare', 'scope', 'css-samples'),
            attributeEquals('cem-css-shared-peer', 'scope', 'css-samples'),
            countExactly('cem-element[tag="cem-css-shared-bare"] > style[data-cem-declaration-style="shared"]', 1),
            styleTextContains(
                'cem-element[tag="cem-css-shared-bare"] > style[data-cem-declaration-style="shared"]',
                '[scope="css-samples"]:has(> template[data-cem-island="instance"])',
            ),
            computedStyle('cem-css-shared-explicit .shared-bg', 'backgroundColor', 'rgb(219, 234, 254)'),
            computedStyle('cem-css-explicit-peer .shared-bg', 'backgroundColor', 'rgb(219, 234, 254)'),
            countExactly('cem-element[tag="cem-css-shared-explicit"] > style[data-cem-declaration-style="shared"]', 1),

            // Matching explicit scope separates the mixed declaration into independent
            // private and shared styles; no combined tag-or-group selector is emitted.
            computedStyle('cem-css-mixed .sample-mixed', 'borderTopColor', 'rgb(0, 0, 255)'),
            computedStyleNot('cem-css-mixed-peer .sample-mixed-shared', 'borderTopColor', 'rgb(0, 0, 255)'),
            computedStyle('cem-css-mixed .sample-mixed-shared', 'color', 'rgb(255, 0, 0)'),
            computedStyle('cem-css-mixed-peer .sample-mixed-shared', 'color', 'rgb(255, 0, 0)'),
            countExactly('cem-element[tag="cem-css-mixed"] > style[data-cem-declaration-style]', 2),
            countExactly('cem-element[tag="cem-css-mixed"] > style[data-cem-declaration-style="private"]', 1),
            countExactly('cem-element[tag="cem-css-mixed"] > style[data-cem-declaration-style="shared"]', 1),
            styleTextContains(
                'cem-element[tag="cem-css-mixed"] > style[data-cem-declaration-style="private"]',
                '@scope (\n    cem-css-mixed',
            ),
            styleTextContains(
                'cem-element[tag="cem-css-mixed"] > style[data-cem-declaration-style="shared"]',
                '[scope="css-samples"]:has(> template[data-cem-island="instance"])',
            ),
            styleTextNotContains(
                'cem-element[tag="cem-css-mixed"] > style[data-cem-declaration-style]',
                'cem-css-mixed, [scope=',
            ),

            // Invalid explicit scopes fail closed. A mismatch does not suppress a valid
            // bare shared shorthand; an invalid declaration scope falls back to private.
            countExactly('cem-element[tag="cem-css-unscoped-explicit"] > style[data-cem-declaration-style]', 0),
            countExactly('cem-element[tag="cem-css-mismatch"] > style[data-cem-declaration-style]', 0),
            computedStyleNot('cem-css-unscoped-explicit .must-not-apply', 'color', 'rgb(255, 0, 0)'),
            computedStyleNot('cem-css-mismatch .must-not-apply', 'color', 'rgb(255, 0, 0)'),
            countExactly('cem-element[tag="cem-css-mismatch-bare"] > style[data-cem-declaration-style="shared"]', 1),
            computedStyle('cem-css-mismatch-bare .sample-valid-bare', 'color', 'rgb(0, 128, 0)'),
            countExactly(
                'cem-element[tag="cem-css-invalid-declaration"] > style[data-cem-declaration-style="private"]',
                1,
            ),
            attributeAbsent('cem-css-invalid-declaration', 'scope'),
            computedStyle('cem-css-invalid-declaration .sample-invalid-declaration', 'color', 'rgb(0, 0, 255)'),

            // An inert instance payload style becomes a managed direct child under an
            // implicit parent-rooted scope; it overrides only that instance.
            computedStyle('cem-css-instance[data-testid="instance-blue"] button', 'borderTopColor', 'rgb(0, 0, 255)'),
            computedStyle('cem-css-instance[data-testid="instance-red"] button', 'borderTopColor', 'rgb(255, 0, 0)'),
            countExactly('cem-element[tag="cem-css-instance"] > style[data-cem-declaration-style="private"]', 1),
            countExactly('cem-css-instance[data-testid="instance-red"] style[data-cem-render-node-id^="payload-"]', 1),
            countExactly('cem-css-instance[data-testid="instance-blue"] style[data-cem-render-node-id^="payload-"]', 0),
            styleTextContains(
                'cem-css-instance[data-testid="instance-red"] style[data-cem-render-node-id^="payload-"]',
                '@scope to (',
            ),
            styleTextNotContains(
                'cem-css-instance[data-testid="instance-red"] style[data-cem-render-node-id^="payload-"]',
                'data-cem-render-scope',
            ),
            attributeAbsent('cem-css-instance[data-testid="instance-red"]', 'data-cem-instance-scope'),

            // Dynamic declaration styles are rejected, while fragment and anonymous
            // declarations scope static CSS to their effective produced tags.
            countExactly('cem-element[tag="cem-css-dynamic"] > style[data-cem-declaration-style]', 0),
            countExactly('cem-css-dynamic style', 0),
            computedStyleNot('cem-css-dynamic .must-not-apply', 'color', 'rgb(255, 0, 0)'),
            computedStyle('cem-css-fragment .sample-fragment', 'backgroundColor', 'rgb(254, 243, 199)'),
            countExactly('cem-element[tag="cem-css-fragment"] > style[data-cem-declaration-style="private"]', 1),
            styleTextContains(
                'cem-element[tag="cem-css-fragment"] > style[data-cem-declaration-style="private"]',
                '@scope (\n    cem-css-fragment',
            ),
            text('.sample-anonymous', 'anonymous'),
            computedStyle('.sample-anonymous', 'color', 'rgb(238, 130, 238)'),
            countExactly(
                'cem-element[data-testid="anonymous-declaration"] > style[data-cem-declaration-style="private"]',
                1,
            ),
            attributeContains('cem-element[data-testid="anonymous-declaration"]', 'tag', 'cem-'),

            // uid-seed is absent from ordinary scope samples. The focused keyframe
            // sample proves its internal identity purpose by matching the host render
            // identity to both the rewritten declaration and computed animation name.
            countExactly('cem-element[uid-seed]', 2),
            attributeEquals('cem-element[tag="cem-css-keyframes"]', 'uid-seed', 'demo/css/keyframes'),
            keyframeIdentity(
                'cem-element[tag="cem-css-keyframes"] > style[data-cem-declaration-style="private"]',
                'cem-css-keyframes[data-testid="keyframes"]',
                '[part~="indicator"]',
                'seeded-pulse',
                'udemoz2fcssz2fkeyframes',
            ),
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
                '#verified',
            ),
            clickThenText(
                'cem-set-url-panel button',
                'cem-set-url-panel output[data-role="current-hash"]',
                '#verified',
            ),
        ],
    },
];

const sourceDocumentSpecs = [
    {
        path: '/packages/cem-elements/index.html',
        samples: [
            sampleContract('1. simple payload', [text('dce-link a', 'link')]),
            sampleContract('2. payload with slot definition and slot value', [text('dce-1-slot', '🐇❤️'), text('dce-1-slot', '🥕')]),
            sampleContract('2a. payload with slot definition and slot value', [
                countExactly('dce-2-slots', 2),
                attributeEquals('dce-2-slots:first-of-type input', 'placeholder', '🐇❤️🥕'),
                attributeEquals('dce-2-slots:last-of-type input', 'placeholder', '🐇❤️🐇'),
            ]),
            sampleContract('2b. named default slot', [
                normalizedText('dce-3-slot:nth-of-type(1)', '1 😃 2 😃'),
                normalizedText('dce-3-slot:nth-of-type(2)', '1 🥕 2 🥕'),
                normalizedText('dce-3-slot:nth-of-type(3)', '1 ✌️ 2 ✌️'),
            ]),
            sampleContract('2c. named default slot', [normalizedText('dce-4-slot', '1 🥕 2 🥕')]),
            sampleContract('2d. default slot', [
                normalizedText('greet-element:first-of-type', 'Hello World!'),
                normalizedText('greet-element:last-of-type', '👋 World!'),
            ]),
            sampleContract('3. 💪 DCE template', [
                text('pokemon-tile[title="bulbasaur"] h3', 'bulbasaur'),
                text('pokemon-tile[title="bulbasaur"]', 'Smile as: 👼'),
                attributeEquals(
                    'pokemon-tile img[alt="bulbasaur image"]',
                    'src',
                    'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/1.svg',
                ),
                attributeEquals(
                    'pokemon-tile button img[alt="ivysaur"]',
                    'src',
                    'https://unpkg.com/pokeapi-sprites@2.0.2/sprites/pokemon/other/dream-world/2.svg',
                ),
                text('pokemon-tile button', 'venusaur'),
                text('pokemon-tile button', 'vulpix'),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/attributes.html',
        samples: [
            sampleContract('1. attributes definition', [
                text('#defaults-1 article.demo-card', 'p1: default_P1'),
                text('#defaults-1 article.demo-card', 'p2: always_p2'),
                text('#defaults-1 article.demo-card', 'p3: def_P3'),
                attributeEquals('#defaults-1', 'p1', 'default_P1'),
                attributeEquals('#defaults-1', 'p2', 'always_p2'),
                attributeEquals('#defaults-1', 'p3', 'def_P3'),
                text('#defaults-2 article.demo-card', 'p1: 123'),
                clickThenText('button[data-attr="p2"]', '#defaults-2 article.demo-card', 'p2: always_p2'),
                attributeEquals('#defaults-2', 'p2', 'always_p2'),
                text('#defaults-3 article.demo-card', 'p3: qwe'),
            ]),
            sampleContract('2. attribute from slice', [
                text('#title-from-slice article.demo-card', 'title attribute: 😃'),
                attributeEquals('#title-from-slice', 'title', '😃'),
                fillThenText('#title-from-slice input', 'Typed title', '#title-from-slice article.demo-card', 'title attribute: Typed title'),
                attributeEquals('#title-from-slice', 'title', 'Typed title'),
            ]),
            sampleContract('3. V attribute matches input value', [
                text('#value-default article.demo-card', 'v: def'),
                text('#value-container article.demo-card', 'v: V1'),
                attributeEquals('#value-default', 'is-changed', 'false'),
                fillThenText('#value-default input', 'From input', '#value-default article.demo-card', 'v: From input'),
                attributeEquals('#value-default', 'v', 'From input'),
                attributeEquals('#value-default', 'is-changed', 'true'),
            ]),
            sampleContract('4. attribute defaults, from container, and from slice', [
                text('#precedence-default article.demo-card', '/datadom/attributes/v: def'),
                text('#precedence-default article.demo-card', 'effective value: def'),
                text('#precedence-container article.demo-card', 'effective value: From Container'),
                attributeEquals('#precedence-default', 'v', 'def'),
                typeThenText(
                    '#precedence-default input',
                    'qqq',
                    '#precedence-default article.demo-card',
                    '/datadom/attributes/v: qqq',
                ),
                text('#precedence-default article.demo-card', 'effective value: qqq'),
                text('#precedence-default article.demo-card', 'has-input: true'),
                attributeEquals('#precedence-default', 'v', 'qqq'),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/data-slices.html',
        samples: [
            sampleContract('A1. inline slice initialization, change on event', [
                attributeEquals('cem-slice-counter-input-default input', 'value', '0'),
                text('cem-slice-counter-input-default article.demo-card', '0'),
                clickThenText('cem-slice-counter-input-default button:first-of-type', 'cem-slice-counter-input-default article.demo-card', '1'),
                clickThenText('cem-slice-counter-input-default button:nth-of-type(2)', 'cem-slice-counter-input-default article.demo-card', '0'),
            ]),
            sampleContract('A2. slice initialization, change on event', [
                text('cem-slice-counter output', '0'),
                clickThenText('cem-slice-counter button:first-of-type', 'cem-slice-counter output', '1'),
                clickThenText('cem-slice-counter button:nth-of-type(2)', 'cem-slice-counter output', '0'),
            ]),
            sampleContract('B. slice event data.', [
                mouseThenText('cem-slice-event-data textarea', { x: 42, y: 17 }, 'cem-slice-event-data p:nth-of-type(1) output', 'x:'),
                computedStyleNot('cem-slice-event-data textarea', 'boxShadow', 'none'),
                computedStyleNot('cem-slice-event-data textarea', 'boxShadow', ''),
                text('cem-slice-event-data p:nth-of-type(3) output', '17'),
                text('cem-slice-event-data p:nth-of-type(2) output', 'mousemove'),
            ]),
            sampleContract('1. slice change on event. 1:1 slice⮂value', [
                normalizedText('cem-slice-basic output', ''),
                fillBlurThenText('cem-slice-basic input', 'basic', 'cem-slice-basic output', 'basic'),
            ]),
            sampleContract('2. initial slice value, slice change on event. slice⮂value', [
                text('cem-slice-initial-change output', 'B'),
                fillBlurThenText('cem-slice-initial-change input', 'changed', 'cem-slice-initial-change output', 'changed'),
            ]),
            sampleContract('3. on input event. slice⮂value', [
                text('cem-slice-initial-input output', 'B'),
                fillThenText('cem-slice-initial-input input', 'input event', 'cem-slice-initial-input output', 'input event'),
            ]),
            sampleContract('4. initial slice value from attribute', [
                text('cem-slice-attribute-initial:first-of-type p:nth-of-type(1) output', '😁'),
                text('cem-slice-attribute-initial:last-of-type p:nth-of-type(1) output', '🤗'),
                fillDispatchThenText('cem-slice-attribute-initial:first-of-type input', 'qqq', 'keyup', 'cem-slice-attribute-initial:first-of-type p:nth-of-type(2) output', 'qqq'),
            ]),
            sampleContract('5. slice value computed from event', [
                text('cem-slice-transform output', 'xB'),
                attributeEquals('cem-slice-transform input', 'value', 'B'),
                fillBlurThenText('cem-slice-transform input', 'C', 'cem-slice-transform output', 'xC'),
            ]),
            sampleContract('6. button ignored till change on click.', [
                text('cem-slice-broccoli output', 'anonymous'),
                clickThenText('cem-slice-broccoli button', 'cem-slice-broccoli output', 'broccoli'),
            ]),
            sampleContract('7. initial slice value from SLICE element', [
                text('cem-slice-declared-counter output', '0'),
                clickThenText('cem-slice-declared-counter button', 'cem-slice-declared-counter output', '1'),
            ]),
            sampleContract('8. multiple slices by SLICE element', [
                text('cem-slice-multi-directive p:nth-of-type(1) output', '0'),
                focusThenText('cem-slice-multi-directive button', 'cem-slice-multi-directive p:nth-of-type(2) output', '1'),
                clickThenText('cem-slice-multi-directive button', 'cem-slice-multi-directive p:nth-of-type(1) output', '1'),
                blurThenText('cem-slice-multi-directive button', 'cem-slice-multi-directive p:nth-of-type(2) output', '0'),
            ]),
            sampleContract('9. slice in attribute', [
                text('cem-slice-emotion-attribute:first-of-type output', ':)'),
                text('cem-slice-emotion-attribute:last-of-type output', '😃'),
                fillBlurThenText('cem-slice-emotion-attribute:last-of-type input', 'joyful', 'cem-slice-emotion-attribute:last-of-type output', 'joyful'),
                attributeEquals('cem-slice-emotion-attribute:last-of-type', 'emotion', 'joyful'),
            ]),
            sampleContract('10. multiple slices by same field', [
                fillThenText('cem-slice-fanout input', 'mirrored', 'cem-slice-fanout p:nth-of-type(2) output', 'mirrored'),
                text('cem-slice-fanout p:nth-of-type(3) output', 'mirrored'),
            ]),
            sampleContract('11. slices and attribute', [
                text('cem-slice-attribute-fanout p:nth-of-type(1) output', '😃'),
                fillBlurThenText('cem-slice-attribute-fanout input', 'grinning', 'cem-slice-attribute-fanout p:nth-of-type(1) output', 'grinning'),
                text('cem-slice-attribute-fanout p:nth-of-type(2) output', 'grinning'),
                attributeEquals('cem-slice-attribute-fanout', 'emotion', 'grinning'),
            ]),
            sampleContract('12. checkbox use', [
                text('cem-slice-checkboxes p:nth-of-type(1) output', 'V0'),
                uncheckThenNormalizedText('cem-slice-checkboxes label:nth-of-type(1) input', 'cem-slice-checkboxes p:nth-of-type(1) output', ''),
                checkThenText('cem-slice-checkboxes label:nth-of-type(2) input', 'cem-slice-checkboxes p:nth-of-type(3) output', 'V1'),
                checkThenText('cem-slice-checkboxes label:nth-of-type(3) input', 'cem-slice-checkboxes p:nth-of-type(4) output', 'V1'),
            ]),
            sampleContract('13. Radio group', [
                text('cem-slice-radios output', 'V1'),
                checkThenText('cem-slice-radios label:first-of-type input', 'cem-slice-radios output', 'V0'),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/dom-merge.html',
        samples: [
            sampleContract('1. Word count in textarea', [
                text('cem-dom-merge-field article.demo-card h2', 'Word count in textarea'),
                fillThenText('cem-dom-merge-field input[type="text"]', 'two words', 'cem-dom-merge-field blockquote', 'two words'),
            ]),
        ],
    },
    { path: '/packages/cem-elements/demo/embed-1.html', checks: [text('h4', 'embed-1.html'), text(':scope', '🖖')] },
    {
        path: '/packages/cem-elements/demo/embed-lib.html#embed-lib-component',
        checks: [text(':scope', '👋 from embed-lib-component')],
    },
    {
        path: '/packages/cem-elements/demo/external-template-document.html',
        checks: [text('h2', 'External document'), text('p', 'External document fallback')],
    },
    {
        path: '/packages/cem-elements/demo/external-template-templates.html#external-card-template',
        attributes: { title: 'Source-loaded card' },
        content: 'Projected source content',
        checks: [text('h2', 'Source-loaded card'), text('p', 'Projected source content')],
    },
    {
        path: '/packages/cem-elements/demo/external-template.html',
        allowedPageErrors: ['Failed to load resource: the server responded with a status of 404 (Not Found)'],
        samples: [
            sampleContract('1. reference the template in page DOM', [text('dce-internal:first-of-type', '👋 World!'), text('dce-internal:last-of-type', 'Hello World!')]),
            sampleContract('2. without TAG, inline instantiation', [countExactly('dce-construction', 2), text('dce-construction', 'construction')]),
            sampleContract('3. external SVG file', [countAtLeast('dce-external svg', 1), svgUseReferences('dce-external svg', ['#h', '#j']), countAtLeast('dce-external-inline svg', 1), svgUseReferences('dce-external-inline svg', ['#h', '#j']), text('dce-external-missing', 'fallback for missing image')]),
            sampleContract('4. external CEM-ML template file', [
                text('dce-external-4', 'External CEMT data-island transformation'),
                text('dce-external-4', 'template[data-cem-island="instance"]'),
                text('dce-external-4', 'cem-island:context-root'),
                text('dce-external-4', 'cem-hydration:data'),
                text('dce-external-4', 'cem-attributes:attributes'),
                text('dce-external-4', 'cem-dataset:dataset'),
                text('dce-external-4', 'cem-payload:payload'),
                text('dce-external-4', 'cem-slices:slices'),
                text('dce-external-4', 'cem-resources:resources'),
                text('dce-external-4', 'cem-form:form-state'),
                text('dce-external-4', 'cem-validation:validation-state'),
                text('dce-external-4', 'cem-events:event-state'),
                text('dce-external-4', 'Payload comment: explicit inert envelope follows'),
                text('dce-external-4', 'DCE with complete external CEMT island'),
                text('dce-external-4', 'wrapped-payload'),
                text('dce-external-4', 'slot="heading"'),
                text('dce-external-4', 'slot=""'),
                text('dce-external-4', 'data-fruit="🍌"'),
                text('dce-external-4', 'aria-label="Fruit choice"'),
                text('dce-external-4', 'Every element, attribute, dataset entry, and text node is data.'),
                countAtLeast('dce-external-4 details', 20),
                text('dce-external-4-inline', 'A second external-CEMT data island'),
                text('dce-external-4-inline', 'DCE with live payload capture'),
                text('dce-external-4-inline', 'name="data-smile"'),
                text('dce-external-4-inline', 'name="data-basket"'),
                text('dce-external-4-inline', 'data-kind="live-payload"'),
                text('dce-external-4-cem-ml', 'content-type="text/cem-ml"'),
                text('dce-external-4-cem-ml', 'Banana from CEM-ML payload source'),
            ]),
            sampleContract('5. external HTML template', [
                text('dce-external-5', '👋'),
                text('dce-external-5', '👌'),
                countAtLeast('dce-external-5 svg', 1),
                countAtLeast('dce-external-5 math', 1),
                attributeEquals('#dce-external-5-inline', 'data-cem-anonymous-declaration', ''),
                text('#dce-external-5-inline > [data-cem-anonymous-instance]', '👋'),
                text('#dce-external-5-inline > [data-cem-anonymous-instance]', '👌'),
                countAtLeast('#dce-external-5-inline > [data-cem-anonymous-instance] svg', 1),
                countAtLeast('#dce-external-5-inline > [data-cem-anonymous-instance] math', 1),
            ]),
            sampleContract('6. HTML, SVG by ID within external file', [text('dce-html-wave', '👋'), countAtLeast('dce-html-logo svg', 1), countAtLeast('dce-html-formula math', 1)]),
            sampleContract('7a. external CEM-ML data-island tree template', [
                text('dce-cemt-tree', 'CEM-ML data island tree'),
                text('dce-cemt-tree article.demo-card > details > summary > b', 'catalog'),
                text('dce-cemt-tree', 'Leaf text from cem-elements data island'),
                countAtLeast('dce-cemt-tree details', 4),
            ]),
            sampleContract('7b. external XSLT data-island tree template', [
                text('dce-xslt-tree', 'XSLT data island tree'),
                text('dce-xslt-tree article.demo-card > details > summary > b', 'catalog'),
                text('dce-xslt-tree', 'Leaf text from cem-elements XSLT data island'),
                countAtLeast('dce-xslt-tree details', 4),
                text('dce-missing-none', 'element with id=none is missing in template'),
            ]),
            sampleContract('8. external file with embedding of another external DCE', [text('dce-embed-1', '🖖')]),
            sampleContract('9. external file with invoking of relative template as hash by enclosed custom-element', [
                text('dce-embed-relative-hash', 'from embed-lib-component'),
                attributeContains(
                    'dce-embed-relative-hash img',
                    'src',
                    '/packages/cem-elements/demo/lib-dir/Smiley.svg',
                ),
            ]),
            sampleContract('10. external file with invoking of template in another relative path file by enclosed custom-element', [text('dce-embed-relative-file', '🖖')]),
            sampleContract('embed-1.html external file', [attributeEquals(':scope', 'src', 'embed-1.html')]),
            sampleContract('embed-lib.html with multiple templates', [attributeEquals(':scope', 'src', 'embed-lib.html')]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/for-each.html',
        samples: [
            sampleContract('1. Simple for-each', [
                countAtLeast('cem-loop-list article.demo-card', 6),
                text('cem-loop-list li', 'Apple'),
                countAtLeast('cem-loop-list tbody tr', 3),
                text('cem-loop-list .payload-feed', 'payload-beta : Payload Beta'),
                text('cem-loop-list .http-json-feed', 'beta : loaded'),
                text('cem-loop-list .http-xml-feed', 'delta : xml-loaded'),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/form.html',
        samples: [
            sampleContract('1. Simple validation', [
                text('cem-form-preview article.demo-card', 'password slice set:'),
                fillThenText('cem-form-preview input[name="username"]', 'ada', 'cem-form-preview output[data-role="form-username"]', 'ada'),
                fillThenText('cem-form-preview input[name="password"]', 'secret', 'cem-form-preview output[data-role="password-valid"]', 'true'),
                text('cem-form-preview output[data-role="form-valid"]', 'true'),
            ]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/hex-grid.html',
        samples: [sampleContract('1. external file with invoking of relative template as hash by enclosed custom-element', [countExactly('cem-grid-tile .swatch', 6), text('cem-grid-tile .swatch', 'B3')])],
    },
    { path: '/packages/cem-elements/demo/html-template.html', checks: [text('#wave', '👋'), text('#ok', '👌'), countExactly('#dwc-logo', 1), countExactly('#sophomores-dream', 1)] },
    {
        path: '/packages/cem-elements/demo/http-request.html',
        samples: [sampleContract('0. url from text to http-request', [text('cem-resource-panel article.demo-card', 'state: loaded'), text('cem-resource-panel li', 'beta : loaded'), text('cem-resource-panel article.demo-card', 'xml state: loaded'), text('cem-resource-panel .xml-results', 'delta : xml-loaded')])],
    },
    {
        path: '/packages/cem-elements/demo/lib-dir/embed-lib.html#embed-lib-component',
        checks: [text(':scope', '👋 from embed-lib-component')],
    },
    {
        path: '/packages/cem-elements/demo/local-storage.html',
        samples: [sampleContract('3. localStorage type', [text('cem-local-storage-panel article.demo-card', 'draft: stored initial'), text('cem-local-storage-panel article.demo-card', 'number: 3'), fillThenText('cem-local-storage-panel input', 'stored draft', 'cem-local-storage-panel output[data-role="draft"]', 'stored draft'), clickThenText('cem-local-storage-panel button', 'cem-local-storage-panel output[data-role="draft"]', 'external update')])],
    },
    {
        path: '/packages/cem-elements/demo/location-element.html',
        samples: [sampleContract('1. window.location live update', [text('cem-location-panel article.demo-card', 'sample host: example.test'), text('cem-location-panel .sample-params', 'tag = one,two'), clickThenText('cem-location-panel button', 'cem-location-panel output[data-role="current-hash"]', '#checked')])],
    },
    {
        path: '/packages/cem-elements/demo/module-url.html',
        samples: [
            sampleContract('this page import maps', [text(':scope', '"lib-root"'), text(':scope', '"embed-lib"')]),
            sampleContract('4. module path by symbolic name', [attributeContains('cem-module-link a', 'href', '/packages/custom-element/material/'), attributeContains('cem-module-link img', 'src', '/packages/custom-element/demo/wc-square.svg')]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/npm-versions-demo.html',
        samples: [sampleContract('1. NPM package version picker', [text('cem-version-row h2', '@epa-wg/cem-elements'), text('cem-version-row', 'selected version: workspace')])],
    },
    {
        path: '/packages/cem-elements/demo/scoped-css.html',
        samples: [
            sampleContract('1. Private declaration CSS and ordinary outer cascade', [countExactly('cem-css-private button', 2), computedStyleByText('cem-css-private button', 'First DCE dashed border', 'borderTopStyle', 'dashed'), computedStyleByText('cem-css-private button', 'First DCE dashed border', 'color', 'rgb(148, 0, 211)'), computedStyleNotByText('button', 'Browser default border', 'borderTopStyle', 'dashed')]),
            sampleContract('2. Component in a named scope shares default declaration styles with peers in the same scope', [computedStyle('cem-css-shared-bare .sample-shared-bare', 'color', 'rgb(0, 128, 0)'), computedStyle('cem-css-shared-peer .sample-shared-bare', 'color', 'rgb(0, 128, 0)')]),
            sampleContract('3. Style can be scoped explicitly', [computedStyle('cem-css-shared-explicit .shared-bg', 'backgroundColor', 'rgb(219, 234, 254)'), computedStyle('cem-css-explicit-peer .shared-bg', 'backgroundColor', 'rgb(219, 234, 254)')]),
            sampleContract('4. Mixed private and shared styles', [computedStyle('cem-css-mixed .sample-mixed', 'borderTopColor', 'rgb(0, 0, 255)'), computedStyle('cem-css-mixed-peer .sample-mixed-shared', 'color', 'rgb(255, 0, 0)')]),
            sampleContract('5. Invalid and mismatched scopes fail closed', [computedStyleNot('cem-css-unscoped-explicit .must-not-apply', 'color', 'rgb(255, 0, 0)'), computedStyle('cem-css-mismatch-bare .sample-valid-bare', 'color', 'rgb(0, 128, 0)'), computedStyle('cem-css-invalid-declaration .sample-invalid-declaration', 'color', 'rgb(0, 0, 255)')]),
            sampleContract('6. Payload style belongs to one instance', [computedStyle('cem-css-instance:first-of-type button', 'borderTopColor', 'rgb(0, 0, 255)'), computedStyle('cem-css-instance:last-of-type button', 'borderTopColor', 'rgb(255, 0, 0)')]),
            sampleContract('7. Declaration styles must be static', [countExactly('cem-css-dynamic style', 0), computedStyleNot('cem-css-dynamic .must-not-apply', 'color', 'rgb(255, 0, 0)')]),
            sampleContract('8. Fragment template CSS uses the effective produced tag', [computedStyle('cem-css-fragment .sample-fragment', 'backgroundColor', 'rgb(254, 243, 199)')]),
            sampleContract('9. Anonymous declaration CSS uses its generated tag', [text('.sample-anonymous', 'anonymous violet'), computedStyle('.sample-anonymous', 'color', 'rgb(238, 130, 238)')]),
            sampleContract('10. uid-seed stabilizes keyframe names', [keyframeIdentity('cem-element[tag="cem-css-keyframes"] > style[data-cem-declaration-style="private"]', 'cem-css-keyframes', '[part~="indicator"]', 'seeded-pulse', 'udemoz2fcssz2fkeyframes')]),
        ],
    },
    {
        path: '/packages/cem-elements/demo/set-url.html',
        samples: [sampleContract('4. Set page URL methods', [text('cem-set-url-panel article.demo-card', 'pending set:'), fillThenText('cem-set-url-panel label:nth-of-type(2) input', '#verified', 'cem-set-url-panel p', '#verified'), clickThenText('cem-set-url-panel button', 'cem-set-url-panel output[data-role="current-hash"]', '#verified')])],
    },
];

const sourceHarnessHtml = `<!doctype html>
<html lang="en">
<head>
    <meta charset="utf-8">
    <script>${htmlDemoElementModule}</script>
    <script>
        localStorage.setItem('cemDemoDraft', 'stored initial');
        localStorage.setItem('cemDemoCount', '3');
        localStorage.setItem('cemDemoBasket', JSON.stringify({ fruit: 'apple' }));
        document.addEventListener('click', (event) => {
            const target = event.target instanceof Element ? event.target : null;
            const attributeButton = target?.closest('[data-set-attr]');
            if (attributeButton) {
                const instance = document.querySelector(attributeButton.dataset.target ?? '');
                const valueSource = document.querySelector(attributeButton.dataset.valueFrom ?? '');
                instance?.setAttribute(attributeButton.dataset.attr ?? '', valueSource?.value ?? attributeButton.dataset.value ?? '');
            }
            const selectButton = target?.closest('[data-dispatch-select]');
            selectButton?.dispatchEvent(new CustomEvent('cem-select', { bubbles: true, detail: { id: 'demo' } }));
            if (target?.matches('[data-storage-write="draft"]')) localStorage.setItem('cemDemoDraft', 'external update');
            if (target?.matches('[data-location-push="true"]')) history.pushState({}, '', './location-element.html?mode=live#checked');
        });
    </script>
    <script type="module">
        import { installCemElementRuntime } from '/packages/cem-elements/dist/index.js';
        installCemElementRuntime(window, {
            resolveModuleUrl(specifier, document, resourceBaseUrl) {
                if (specifier === '@epa-wg/custom-element/demo/wc-square.svg') {
                    return new URL('/packages/custom-element/demo/wc-square.svg', document.baseURI).href;
                }
                if (specifier === '@epa-wg/material') {
                    return new URL('/packages/custom-element/material/', document.baseURI).href;
                }
                return new URL(specifier, resourceBaseUrl).href;
            },
        });
    </script>
</head>
<body></body>
</html>`;

const server = createServer(async (request, response) => {
    try {
        const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');
        const pathname = decodeURIComponent(
            requestUrl.pathname === '/' ? '/packages/cem-elements/index.html' : requestUrl.pathname,
        );
        if (pathname === '/__cem-source-harness.html') {
            response.writeHead(200, { 'content-type': 'text/html; charset=utf-8' });
            response.end(sourceHarnessHtml);
            return;
        }
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
    await verifySourceDocumentInventory();
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
                `${fixture.path} failed while running ${describeCheck(error?.check)}:\n${error.message}${diagnostics}${snapshot}`,
                { cause: error },
            );
        } finally {
            await page.close();
        }

        const unexpectedErrors = unexpectedPageErrors(fixture, pageErrors);
        if (unexpectedErrors.length > 0) {
            throw new Error(
                `${fixture.path} emitted browser errors:\n${unexpectedErrors.map((error) => `- ${error}`).join('\n')}`,
            );
        }
    }
    for (const [index, fixture] of sourceDocumentSpecs.entries()) {
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

        const tag = `cem-demo-source-${index + 1}`;
        try {
            await page.goto(`http://127.0.0.1:${port}/__cem-source-harness.html`, { waitUntil: 'networkidle' });
            await mountSourceDocument(page, fixture, tag);
            await verifySampleContractInventory(page, tag, fixture);
            if (fixture.samples) {
                for (const [sampleIndex, sample] of fixture.samples.entries()) {
                    const rootSelector = await markSampleRoot(page, tag, sample.legend, sampleIndex);
                    for (const check of sample.checks) {
                        await runCheck(page, scopeCheck(check, rootSelector));
                    }
                }
            }
            for (const check of fixture.checks ?? []) {
                await runCheck(page, scopeCheck(check, tag));
            }
        } catch (error) {
            const unexpectedErrors = unexpectedPageErrors(fixture, pageErrors);
            const diagnostics =
                unexpectedErrors.length > 0
                    ? `\nBrowser errors:\n${unexpectedErrors.map((item) => `- ${item}`).join('\n')}`
                    : '';
            const snapshot = await collectDebugSnapshot(page, error?.check);
            throw new Error(
                `${fixture.path} source contract failed while running ${describeCheck(error?.check)}:\n${error.message}${diagnostics}${snapshot}`,
                { cause: error },
            );
        } finally {
            await page.close();
        }

        const unexpectedErrors = unexpectedPageErrors(fixture, pageErrors);
        if (unexpectedErrors.length > 0) {
            throw new Error(
                `${fixture.path} source contract emitted browser errors:\n${unexpectedErrors.map((error) => `- ${error}`).join('\n')}`,
            );
        }
    }
} finally {
    await browser.close();
    await new Promise((resolveClose) => server.close(resolveClose));
}

console.log(
    `cem-elements demo fixtures verified (${fixtureSpecs.length} standalone pages, ${sourceDocumentSpecs.length} source-loaded documents).`,
);

async function verifySourceDocumentInventory() {
    const discovered = await demoHtmlPaths(resolve(repoRoot, 'packages/cem-elements/demo'));
    const declared = Array.from(
        new Set(
            sourceDocumentSpecs
                .map(({ path }) => path.split('#', 1)[0])
                .filter((path) => path.startsWith('/packages/cem-elements/demo/')),
        ),
    ).sort();
    if (JSON.stringify(discovered) !== JSON.stringify(declared)) {
        const missing = discovered.filter((path) => !declared.includes(path));
        const stale = declared.filter((path) => !discovered.includes(path));
        throw new Error(
            `source-loaded demo HTML inventory mismatch; missing contracts: ${missing.join(', ') || 'none'}; stale contracts: ${stale.join(', ') || 'none'}`,
        );
    }
}

async function demoHtmlPaths(directory, relativeDirectory = '') {
    const paths = [];
    for (const entry of await readdir(directory, { withFileTypes: true })) {
        const relativePath = relativeDirectory ? `${relativeDirectory}/${entry.name}` : entry.name;
        if (entry.isDirectory()) {
            paths.push(...(await demoHtmlPaths(join(directory, entry.name), relativePath)));
        } else if (entry.isFile() && entry.name.endsWith('.html')) {
            paths.push(`/packages/cem-elements/demo/${relativePath}`);
        }
    }
    return paths.sort();
}

async function mountSourceDocument(page, fixture, tag) {
    await page.evaluate(
        async ({ path, producedTag, attributes, content }) => {
            await customElements.whenDefined('cem-element');
            const declaration = document.createElement('cem-element');
            declaration.hidden = true;
            declaration.setAttribute('tag', producedTag);
            declaration.setAttribute('src', path);
            const instance = document.createElement(producedTag);
            for (const [name, value] of Object.entries(attributes ?? {})) {
                instance.setAttribute(name, value);
            }
            if (content) instance.textContent = content;
            document.body.append(declaration, instance);
        },
        { path: fixture.path, producedTag: tag, attributes: fixture.attributes, content: fixture.content },
    );
}

async function verifySampleContractInventory(page, tag, fixture) {
    const expected = (fixture.samples ?? []).map((sample) => sample.legend.replace(/\s+/gu, ' ').trim());
    await poll(
        page,
        ({ hostSelector, expectedCount }) => {
            const host = document.querySelector(hostSelector);
            if (!host) return false;
            const renderComplete = Array.from(host.childNodes).some(
                (node) => node.nodeType === Node.COMMENT_NODE && node.nodeValue === 'cem-render-end',
            );
            return renderComplete && host.querySelectorAll('html-demo-element[legend]').length >= expectedCount;
        },
        { hostSelector: tag, expectedCount: expected.length },
    );
    const actual = await page.evaluate(
        (hostSelector) =>
            Array.from(document.querySelectorAll(`${hostSelector} html-demo-element[legend]`)).map((element) =>
                (element.getAttribute('legend') ?? '').replace(/\s+/gu, ' ').trim(),
            ),
        tag,
    );
    if (JSON.stringify(actual) !== JSON.stringify(expected)) {
        const missing = actual.filter((legend) => !expected.includes(legend));
        const stale = expected.filter((legend) => !actual.includes(legend));
        throw new Error(
            `${fixture.path} sample inventory mismatch; missing legend contracts: ${missing.join(', ') || 'none'}; stale legend contracts: ${stale.join(', ') || 'none'}`,
        );
    }
}

async function markSampleRoot(page, tag, legend, index) {
    const marker = String(index + 1);
    await poll(
        page,
        ({ hostSelector, expectedLegend, markerValue }) => {
            const host = document.querySelector(hostSelector);
            const normalizeText = (value) => value.replace(/\s+/gu, ' ').trim();
            const sample = host
                ? Array.from(host.querySelectorAll('html-demo-element')).find(
                      (element) => normalizeText(element.getAttribute('legend') ?? '') === normalizeText(expectedLegend),
                  )
                : null;
            if (!sample) return false;
            sample.setAttribute('data-cem-fixture-sample', markerValue);
            return true;
        },
        { hostSelector: tag, expectedLegend: legend, markerValue: marker },
    );
    return `${tag} html-demo-element[data-cem-fixture-sample="${marker}"]`;
}

function scopeCheck(check, rootSelector) {
    const scoped = { ...check };
    for (const key of ['selector', 'actionSelector', 'resultSelector', 'styleSelector', 'hostSelector']) {
        if (typeof scoped[key] === 'string') {
            scoped[key] = scoped[key] === ':scope' ? rootSelector : `${rootSelector} ${scoped[key]}`;
        }
    }
    return scoped;
}

function unexpectedPageErrors(fixture, pageErrors) {
    const allowed = fixture.allowedPageErrors ?? [];
    return pageErrors.filter((error) => !allowed.some((allowedError) => error.includes(allowedError)));
}

async function installOfflineRoutes(page) {
    await page.route('https://unpkg.com/html-demo-element@*/html-demo-element.js', (route) =>
        route.fulfill({ contentType: 'text/javascript; charset=utf-8', body: htmlDemoElementModule }),
    );
    await page.route(/^https:\/\/unpkg\.com\/pokeapi-sprites@.*\.svg$/, (route) =>
        route.fulfill({
            contentType: 'image/svg+xml; charset=utf-8',
            body: '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1 1"><title>fixture sprite</title></svg>',
        }),
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
            case 'countExactly':
                await waitForExactCount(page, check.selector, check.count);
                return;
            case 'attributeContains':
                await waitForAttribute(page, check.selector, check.name, check.expected);
                return;
            case 'attributeEquals':
                await waitForExactAttribute(page, check.selector, check.name, check.expected);
                return;
            case 'attributeAbsent':
                await waitForAbsentAttribute(page, check.selector, check.name);
                return;
            case 'svgUseReferences':
                await verifySvgUseReferences(page, check.selector, check.expected);
                return;
            case 'styleTextContains':
                await waitForStyleText(page, check.selector, check.expected, true);
                return;
            case 'styleTextNotContains':
                await waitForStyleText(page, check.selector, check.unexpected, false);
                return;
            case 'computedStyle':
                await waitForComputedStyle(page, check.selector, check.property, check.expected);
                return;
            case 'computedStyleNot':
                await waitForComputedStyleNot(page, check.selector, check.property, check.unexpected);
                return;
            case 'computedStyleByText':
                await waitForComputedStyleByText(page, check.selector, check.text, check.property, check.expected, true);
                return;
            case 'computedStyleNotByText':
                await waitForComputedStyleByText(page, check.selector, check.text, check.property, check.unexpected, false);
                return;
            case 'keyframeIdentity':
                await waitForKeyframeIdentity(page, check);
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
            case 'typeThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.locator(check.actionSelector).pressSequentially(check.value);
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'fillBlurThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.fill(check.actionSelector, check.value);
                await page.locator(check.actionSelector).blur();
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'fillDispatchThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.fill(check.actionSelector, check.value);
                await page.dispatchEvent(check.actionSelector, check.eventName);
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'mouseThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.locator(check.actionSelector).hover({ position: check.eventInit });
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'focusThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.locator(check.actionSelector).focus();
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'blurThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.locator(check.actionSelector).blur();
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'checkThenText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.locator(check.actionSelector).check();
                await waitForText(page, check.resultSelector, check.expected);
                return;
            case 'uncheckThenNormalizedText':
                await page.waitForSelector(check.actionSelector, { timeout });
                await page.locator(check.actionSelector).uncheck();
                await waitForNormalizedText(page, check.resultSelector, check.expected);
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
                'cem-css-private',
                'cem-css-shared-bare',
                'cem-css-shared-peer',
                'cem-css-shared-explicit',
                'cem-css-explicit-peer',
                'cem-css-mixed',
                'cem-css-mixed-peer',
                'cem-css-unscoped-explicit',
                'cem-css-mismatch',
                'cem-css-mismatch-bare',
                'cem-css-invalid-declaration',
                'cem-css-instance',
                'cem-css-dynamic',
                'cem-css-fragment',
                'cem-css-keyframes',
                'cem-form-preview',
            ];
            const failedSelector =
                failedCheck?.selector ??
                failedCheck?.targetSelector ??
                failedCheck?.hostSelector ??
                failedCheck?.resultSelector ??
                failedCheck?.actionSelector ??
                undefined;
            const failedTexts = failedSelector
                ? Array.from(document.querySelectorAll(failedSelector)).map((element) =>
                      globalThis.__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element)),
                  )
                : [];
            const failedElement = failedSelector ? document.querySelector(failedSelector) : null;
            return {
                bodyText: globalThis
                    .__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(document.body))
                    .slice(0, 1000),
                failedSelector,
                failedExpected: failedCheck?.expected,
                failedComputedValue:
                    failedCheck?.kind === 'computedStyle' && failedElement
                        ? getComputedStyle(failedElement)[failedCheck.property]
                        : null,
                styles: Array.from(document.querySelectorAll('style')).map(
                    (style) => style.textContent?.trim().slice(0, 1000) ?? '',
                ),
                failedCheckNow:
                    failedCheck?.kind === 'text' && typeof failedCheck.expected === 'string'
                        ? failedTexts.some((value) => value.includes(failedCheck.expected))
                        : null,
                failedSelectorMatches: failedSelector
                    ? Array.from(document.querySelectorAll(failedSelector)).map((element) => ({
                          tag: element.localName,
                          text: globalThis
                              .__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element))
                              .slice(0, 500),
                          html: element.outerHTML.slice(0, 1000),
                      }))
                    : [],
                elements: customElementTags
                    .flatMap((tag) => Array.from(document.querySelectorAll(tag)))
                    .map((element) => ({
                        tag: element.localName,
                        text: globalThis
                            .__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element))
                            .slice(0, 500),
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

function sampleContract(legend, checks) {
    return { legend, checks };
}

function normalizedText(selector, expected) {
    return { kind: 'normalizedText', selector, expected };
}

function countAtLeast(selector, min) {
    return { kind: 'countAtLeast', selector, min };
}

function countExactly(selector, count) {
    return { kind: 'countExactly', selector, count };
}

function attributeContains(selector, name, expected) {
    return { kind: 'attributeContains', selector, name, expected };
}

function attributeEquals(selector, name, expected) {
    return { kind: 'attributeEquals', selector, name, expected };
}

function attributeAbsent(selector, name) {
    return { kind: 'attributeAbsent', selector, name };
}

function svgUseReferences(selector, expected) {
    return { kind: 'svgUseReferences', selector, expected };
}

function styleTextContains(selector, expected) {
    return { kind: 'styleTextContains', selector, expected };
}

function styleTextNotContains(selector, unexpected) {
    return { kind: 'styleTextNotContains', selector, unexpected };
}

function computedStyle(selector, property, expected) {
    return { kind: 'computedStyle', selector, property, expected };
}

function computedStyleNot(selector, property, unexpected) {
    return { kind: 'computedStyleNot', selector, property, unexpected };
}

function computedStyleByText(selector, text, property, expected) {
    return { kind: 'computedStyleByText', selector, text, property, expected };
}

function computedStyleNotByText(selector, text, property, unexpected) {
    return { kind: 'computedStyleNotByText', selector, text, property, unexpected };
}

function keyframeIdentity(styleSelector, hostSelector, targetSelector, authoredName, encodedSeed) {
    return { kind: 'keyframeIdentity', styleSelector, hostSelector, targetSelector, authoredName, encodedSeed };
}

function clickThenText(actionSelector, resultSelector, expected) {
    return { kind: 'clickThenText', actionSelector, resultSelector, expected };
}

function fillThenText(actionSelector, value, resultSelector, expected) {
    return { kind: 'fillThenText', actionSelector, value, resultSelector, expected };
}

function typeThenText(actionSelector, value, resultSelector, expected) {
    return { kind: 'typeThenText', actionSelector, value, resultSelector, expected };
}

function fillBlurThenText(actionSelector, value, resultSelector, expected) {
    return { kind: 'fillBlurThenText', actionSelector, value, resultSelector, expected };
}

function fillDispatchThenText(actionSelector, value, eventName, resultSelector, expected) {
    return { kind: 'fillDispatchThenText', actionSelector, value, eventName, resultSelector, expected };
}

function mouseThenText(actionSelector, eventInit, resultSelector, expected) {
    return { kind: 'mouseThenText', actionSelector, eventInit, resultSelector, expected };
}

function focusThenText(actionSelector, resultSelector, expected) {
    return { kind: 'focusThenText', actionSelector, resultSelector, expected };
}

function blurThenText(actionSelector, resultSelector, expected) {
    return { kind: 'blurThenText', actionSelector, resultSelector, expected };
}

function checkThenText(actionSelector, resultSelector, expected) {
    return { kind: 'checkThenText', actionSelector, resultSelector, expected };
}

function uncheckThenNormalizedText(actionSelector, resultSelector, expected) {
    return { kind: 'uncheckThenNormalizedText', actionSelector, resultSelector, expected };
}

async function waitForText(page, selector, expected) {
    await poll(
        page,
        ({ selector: checkSelector, expected: checkExpected }) => {
            const elements = Array.from(document.querySelectorAll(checkSelector));
            return elements.some((element) =>
                globalThis
                    .__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(element))
                    .includes(checkExpected),
            );
        },
        { selector, expected },
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
        { selector, expected },
    );
}

async function waitForCount(page, selector, min) {
    await poll(
        page,
        ({ selector: checkSelector, min: checkMin }) => document.querySelectorAll(checkSelector).length >= checkMin,
        { selector, min },
    );
}

async function waitForExactCount(page, selector, count) {
    await poll(
        page,
        ({ selector: checkSelector, count: checkCount }) =>
            document.querySelectorAll(checkSelector).length === checkCount,
        { selector, count },
    );
}

async function waitForAttribute(page, selector, name, expected) {
    await poll(
        page,
        ({ selector: checkSelector, name: attributeName, expected: checkExpected }) => {
            const element = document.querySelector(checkSelector);
            return element ? (element.getAttribute(attributeName) ?? '').includes(checkExpected) : false;
        },
        { selector, name, expected },
    );
}

async function waitForExactAttribute(page, selector, name, expected) {
    await poll(
        page,
        ({ selector: checkSelector, name: attributeName, expected: checkExpected }) =>
            document.querySelector(checkSelector)?.getAttribute(attributeName) === checkExpected,
        { selector, name, expected },
    );
}

async function waitForAbsentAttribute(page, selector, name) {
    await poll(
        page,
        ({ selector: checkSelector, name: attributeName }) => {
            const element = document.querySelector(checkSelector);
            return element ? !element.hasAttribute(attributeName) : false;
        },
        { selector, name },
    );
}

async function verifySvgUseReferences(page, selector, expected) {
    await page.waitForSelector(selector, { timeout });
    const actual = await page.evaluate(
        ({ selector: checkSelector, xlinkNamespace }) => {
            const svg = document.querySelector(checkSelector);
            return svg
                ? Array.from(svg.querySelectorAll('use')).map((element) => {
                      const bounds = element.getBBox();
                      return {
                          href: element.getAttributeNS(xlinkNamespace, 'href'),
                          width: bounds.width,
                          height: bounds.height,
                      };
                  })
                : [];
        },
        { selector, xlinkNamespace: 'http://www.w3.org/1999/xlink' },
    );
    const renderedReferences = actual.map(({ href }) => href);
    if (
        JSON.stringify(renderedReferences) !== JSON.stringify(expected) ||
        actual.some(({ width, height }) => width <= 0 || height <= 0)
    ) {
        throw new Error(
            `expected rendered SVG use references ${JSON.stringify(expected)}, received ${JSON.stringify(actual)}`,
        );
    }
}

async function waitForStyleText(page, selector, text, contains) {
    await poll(
        page,
        ({ selector: checkSelector, text: checkText, contains: shouldContain }) => {
            const styles = Array.from(document.querySelectorAll(checkSelector));
            if (styles.length === 0) return false;
            return shouldContain
                ? styles.some((style) => (style.textContent ?? '').includes(checkText))
                : styles.every((style) => !(style.textContent ?? '').includes(checkText));
        },
        { selector, text, contains },
    );
}

async function waitForComputedStyle(page, selector, property, expected) {
    await poll(
        page,
        ({ selector: checkSelector, property: styleProperty, expected: checkExpected }) => {
            const element = document.querySelector(checkSelector);
            return element ? getComputedStyle(element)[styleProperty] === checkExpected : false;
        },
        { selector, property, expected },
    );
}

async function waitForComputedStyleNot(page, selector, property, unexpected) {
    await poll(
        page,
        ({ selector: checkSelector, property: styleProperty, unexpected: checkUnexpected }) => {
            const element = document.querySelector(checkSelector);
            return element ? getComputedStyle(element)[styleProperty] !== checkUnexpected : false;
        },
        { selector, property, unexpected },
    );
}

async function waitForComputedStyleByText(page, selector, text, property, value, equals) {
    await poll(
        page,
        ({ selector: checkSelector, text: checkText, property: styleProperty, value: checkValue, equals: shouldEqual }) => {
            const element = Array.from(document.querySelectorAll(checkSelector)).find(
                (candidate) =>
                    globalThis.__cemFixtureNormalizeText(globalThis.__cemFixtureVisibleText(candidate)) === checkText,
            );
            return element ? (getComputedStyle(element)[styleProperty] === checkValue) === shouldEqual : false;
        },
        { selector, text, property, value, equals },
    );
}

async function waitForKeyframeIdentity(page, check) {
    await poll(
        page,
        ({ styleSelector, hostSelector, targetSelector, authoredName, encodedSeed }) => {
            const style = document.querySelector(styleSelector);
            const host = document.querySelector(hostSelector);
            const target = document.querySelector(`${hostSelector} ${targetSelector}`);
            const renderScope = host?.getAttribute('data-cem-render-scope') ?? '';
            if (!style || !target || !renderScope.includes(encodedSeed)) return false;
            const rewrittenName = `${authoredName}-${renderScope}-s1`;
            const css = style.textContent ?? '';
            return (
                css.includes(`@keyframes ${rewrittenName}`) &&
                css.includes(`animation: ${rewrittenName} `) &&
                getComputedStyle(target).animationName === rewrittenName
            );
        },
        check,
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
        case 'countExactly':
            return `countExactly(${check.selector}, ${check.count})`;
        case 'attributeContains':
            return `attributeContains(${check.selector}, ${check.name}, ${JSON.stringify(check.expected)})`;
        case 'attributeEquals':
            return `attributeEquals(${check.selector}, ${check.name}, ${JSON.stringify(check.expected)})`;
        case 'attributeAbsent':
            return `attributeAbsent(${check.selector}, ${check.name})`;
        case 'svgUseReferences':
            return `svgUseReferences(${check.selector}, ${JSON.stringify(check.expected)})`;
        case 'styleTextContains':
            return `styleTextContains(${check.selector}, ${JSON.stringify(check.expected)})`;
        case 'styleTextNotContains':
            return `styleTextNotContains(${check.selector}, ${JSON.stringify(check.unexpected)})`;
        case 'computedStyle':
            return `computedStyle(${check.selector}, ${check.property}, ${JSON.stringify(check.expected)})`;
        case 'computedStyleNot':
            return `computedStyleNot(${check.selector}, ${check.property}, ${JSON.stringify(check.unexpected)})`;
        case 'computedStyleByText':
            return `computedStyleByText(${check.selector}, ${JSON.stringify(check.text)}, ${check.property}, ${JSON.stringify(check.expected)})`;
        case 'computedStyleNotByText':
            return `computedStyleNotByText(${check.selector}, ${JSON.stringify(check.text)}, ${check.property}, ${JSON.stringify(check.unexpected)})`;
        case 'keyframeIdentity':
            return `keyframeIdentity(${check.hostSelector}, ${check.authoredName}, ${check.encodedSeed})`;
        case 'clickThenText':
            return `clickThenText(${check.actionSelector}, ${check.resultSelector}, ${JSON.stringify(check.expected)})`;
        case 'fillThenText':
            return `fillThenText(${check.actionSelector}, ${check.resultSelector}, ${JSON.stringify(check.expected)})`;
        case 'typeThenText':
            return `typeThenText(${check.actionSelector}, ${check.resultSelector}, ${JSON.stringify(check.expected)})`;
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
