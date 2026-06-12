/**
 * `cem-css-generator` — the CEM-ML/CEM-QL CSS-generator bootstrap (Phase 3.6, Option B of
 * [`docs/custom-element-template-migration-options.md`](../../../../../docs/custom-element-template-migration-options.md)).
 *
 * Replaces the legacy `@epa-wg/custom-element` XSLT/XPath runtime for the `cem-theme` CSS
 * generators. It drives the converted `<template type="cem-ml; version=0.0">` declarations directly through
 * the `@epa-wg/cem-elements` substrate's runtime-support render boundary (the same `cem_ql` WASM
 * engine the element lifecycle uses), with no live browser XSLT.
 *
 * Per generator page it:
 *   1. reads the `<template type="cem-ml; version=0.0">` config (`data-token-url` + `data-slices`);
 *   2. fetches the compiled token document and parses it into a DOM (BR-PH-3 — the browser is the
 *      parser; no XHTML parser is shipped);
 *   3. shapes the relevant token `<table>`s into cem-ql row records via the slice-3 DOM→datadom
 *      bridge (`tokenTableRows`), bound under `datadom.slices.<key>`;
 *   4. renders the CEM-ML template against that datadom through `renderCemMlTemplate`;
 *   5. materializes the resolved render plan into the mount, then feeds the generated
 *      `code[data-generated-css]` text to `<cem-css-loader>` so the page resolves live values.
 *
 * The substrate runtime is vendored next to the built generator (`compile-html.mjs`
 * `stageSubstrateRuntime`), so these imports resolve from the built page under `dist/`.
 */

import {
    ensureRuntimeReady,
    renderCemMlTemplate,
} from '../../vendor/@epa-wg/cem-elements/dist/lib/internal/runtime-support/cem-ql-render.js';
import { tokenTableRows } from '../../vendor/@epa-wg/cem-elements/dist/lib/data-document.js';

/** Parse `data-slices="key=anchor-id key2=anchor-id2"` into ordered `{ key, anchorId }` pairs. */
function parseSliceConfig(spec) {
    return (spec ?? '')
        .split(/\s+/)
        .map((pair) => pair.trim())
        .filter(Boolean)
        .map((pair) => {
            const [key, anchorId] = pair.split('=');
            return { key, anchorId };
        })
        .filter(({ key, anchorId }) => key && anchorId);
}

/** Read the CEM-ML source from a `<template>` (its body lives in `.content`, not as child text). */
function templateSource(template) {
    const fromContent = template.content?.textContent ?? '';
    return fromContent.trim() ? fromContent : template.textContent ?? '';
}

/** Materialize a runtime-support render-plan node into a real DOM node. */
function materialize(node, doc) {
    if (node.kind === 'text') {
        return doc.createTextNode(node.text);
    }
    if (node.kind === 'comment') {
        return doc.createComment(node.text);
    }
    const element = node.namespace
        ? doc.createElementNS(node.namespace, node.tag)
        : doc.createElement(node.tag);
    for (const attribute of node.attributes ?? []) {
        element.setAttribute(attribute.name, attribute.value);
    }
    for (const child of node.children ?? []) {
        element.appendChild(materialize(child, doc));
    }
    return element;
}

/** Fetch + parse the token document referenced by `url` (resolved against the page). */
async function loadTokenDocument(url) {
    const response = await fetch(new URL(url, document.baseURI));
    if (!response.ok) {
        throw new Error(`token document fetch failed (${response.status}) for ${url}`);
    }
    const text = await response.text();
    // The token doc is XHTML, but the lenient HTML parser handles it and yields the same
    // id-anchored `<table>` DOM the slice-3 bridge navigates with native queries.
    return new DOMParser().parseFromString(text, 'text/html');
}

async function runGenerator(template) {
    const tokenUrl = template.getAttribute('data-token-url');
    if (!tokenUrl) {
        throw new Error('cem-ml; version=0.0 template is missing data-token-url');
    }
    const sliceConfig = parseSliceConfig(template.getAttribute('data-slices'));
    const mountSelector = template.getAttribute('data-mount') ?? 'main';
    const mount = document.querySelector(mountSelector);
    if (!mount) {
        throw new Error(`cem-ml; version=0.0 mount not found: ${mountSelector}`);
    }

    const tokenDoc = await loadTokenDocument(tokenUrl);
    const slices = {};
    for (const { key, anchorId } of sliceConfig) {
        slices[key] = tokenTableRows(tokenDoc, anchorId);
    }

    await ensureRuntimeReady();
    const source = templateSource(template);
    const { nodes, diagnostics } = await renderCemMlTemplate(
        source,
        { datadom: { slices } },
        { renderNodeIdPrefix: template.id || 'cem-css' }
    );

    const errors = (diagnostics ?? []).filter(
        (diagnostic) => diagnostic.severity === 'error' || diagnostic.severity === 'fatal'
    );
    if (errors.length) {
        for (const diagnostic of errors) {
            console.error(`[cem-css-generator] ${diagnostic.code}: ${diagnostic.message}`);
        }
    }

    mount.replaceChildren(...nodes.map((node) => materialize(node, document)));

    const generated = mount.querySelector('code[data-generated-css]');
    const css = generated ? generated.textContent ?? '' : '';
    const loader = document.querySelector('cem-css-loader');
    if (loader) {
        loader.setAttribute('value', css);
    }
}

async function bootstrap() {
    const templates = document.querySelectorAll('template[type="cem-ml; version=0.0"]');
    for (const template of templates) {
        try {
            await runGenerator(template);
        } catch (error) {
            console.error(`[cem-css-generator] ${error.message}`, error);
        }
    }
}

if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', bootstrap, { once: true });
} else {
    bootstrap();
}
