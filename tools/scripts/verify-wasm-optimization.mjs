#!/usr/bin/env node
// Compare isolated pre/post-optimization native instances through their matching web glue.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const [glue, baseline, ...candidates] = process.argv.slice(2);
if (!glue || !baseline || !candidates.length) throw new Error('Usage: node verify-wasm-optimization.mjs <web-glue.js> <baseline.wasm> <candidate.wasm> [...]');
const snapshots = [];
for (const [index, path] of [baseline, ...candidates].entries()) {
    const bytes = await readFile(path);
    const module = new WebAssembly.Module(bytes);
    const runtime = await import(`${pathToFileURL(resolve(glue))}?optimization-instance=${index}`);
    await runtime.default({ module_or_path: module });
    const request = JSON.stringify({ runtime: 'wasm-browser-worker', targetIdentity: 'optimization-comparison:web', abiIdentity: 'wasm-bindgen@0.2.122;profile=debug-control', debugControlActive: false });
    const capabilities = JSON.parse(runtime.capabilityManifest(request));
    const protocol = JSON.parse(runtime.workerProtocolDescriptor());
    const rendered = JSON.parse(runtime.renderCemMlToHtmlV1(JSON.stringify({ source: '{p @class=message | Offline preview}', sourceUrl: 'memory:comparison.cem' })));
    assert.equal(rendered.html, '<p class="message">Offline preview</p>');
    assert.equal(rendered.status, 'rendered');
    const highlighted = JSON.parse(runtime.highlightSourceToHtmlV1(JSON.stringify({ source: '<p class="message">Ready</p>', contentType: 'text/html' })));
    assert.equal(highlighted.status, 'highlighted');
    const query = JSON.parse(runtime.evaluateQuerySource('6 * 7', '{}'));
    assert.deepEqual(query, { diagnostics: [], error: null, items: [{ kind: 'atomic', type: 'integer', value: 42 }] });
    const template = JSON.parse(runtime.compileTemplate('{strong | Preserved}', '[]'));
    assert.deepEqual(template.diagnostics, []);
    const templateOutput = JSON.parse(runtime.renderTemplate(template.artifactId, '{}'));
    assert.deepEqual(templateOutput.diagnostics, []);
    runtime.disposeTemplate(template.artifactId);
    const sorted = values => values.sort((a, b) => JSON.stringify(a).localeCompare(JSON.stringify(b)));
    const snapshot = {
        imports: sorted(WebAssembly.Module.imports(module)), exports: sorted(WebAssembly.Module.exports(module)),
        capabilities, protocol, rendered, highlighted, query, templateOutput,
    };
    if (index) assert.deepEqual(snapshot, snapshots[0], `Native behavior or public capabilities changed: ${path}`);
    snapshots.push(snapshot);
    console.log(JSON.stringify({ path, bytes: bytes.length, sha256: createHash('sha256').update(bytes).digest('hex'),
        exports: snapshot.exports.length, imports: snapshot.imports.length,
        capabilitiesSha256: createHash('sha256').update(JSON.stringify(capabilities)).digest('hex'),
        protocolSha256: createHash('sha256').update(JSON.stringify(protocol)).digest('hex'),
        matchesBaseline: index ? true : undefined }));
}
