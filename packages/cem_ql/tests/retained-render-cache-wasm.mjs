import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import * as wasm from '../dist/wasm/cem_ql.js';
await wasm.default({ module_or_path: await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
const ids = [];
const retain = source => { const value = JSON.parse(wasm.compileTemplate(source, '["value"]')); ids.push(value.artifactId); return value.artifactId; };
const render = (id, value, owner = 'engine/instance', extra = {}) => JSON.parse(wasm.renderTemplateWithNativeValuesAndAdmission(id, extra.companion ?? 0,
    JSON.stringify(value), JSON.stringify(extra.documents ?? []), extra.bindings ?? '[]', extra.limits ?? '', 'instance', extra.references, undefined,
    JSON.stringify({owner, scopePolicyStamp: 'scope/1'})));
try {
    const id = retain('{attribute @name=notice @select=\'"ready"\'}{p | {$value}}');
    const first = render(id, {value:'Ada'}), hit = render(id, {value:'Ada'});
    assert.equal(first.renderReuse.hit, false); assert.equal(hit.renderReuse.hit, true);
    assert.equal(hit.renderReuse.evaluations, first.renderReuse.evaluations);
    assert.equal(hit.renderReuse.admissions, first.renderReuse.admissions + 1);
    assert.deepEqual(hit.nodes, first.nodes); assert.deepEqual(hit.hostAttributeUpdates, first.hostAttributeUpdates);
    assert.equal(render(id, {value:'Lin'}).renderReuse.hit, false);
    assert.equal(render(id, {value:'Ada'}, 'other-owner').renderReuse.hit, false);
    const opaque = retain('{p | static}');
    for (let i=0;i<2;i++) assert.equal(render(opaque, {unused:{$resource:{id:'r',contentType:'application/xml'}}}).renderReuse.hit, false);
    const typed = retain('{input @value="{$value}"}');
    const scalar = render(typed, {value:42});
    const bytes = wasm.takeRenderValueArtifact(scalar.nativeValueArtifactId);
    const scalarHit = render(typed, {value:42});
    assert.equal(scalarHit.renderReuse.hit, true);
    assert.notEqual(scalar.nativeValueArtifactId, scalarHit.nativeValueArtifactId);
    wasm.takeRenderValueArtifact(scalarHit.nativeValueArtifactId);
    const nativeId = wasm.importNativeValueArtifact(bytes, JSON.stringify({maxBytes:16777216,maxValues:100000,maxDepth:128}));
    try {
        for (let i=0;i<2;i++) {
            const native = render(typed, {}, 'engine/instance', {bindings:JSON.stringify([{name:'value', artifactId:nativeId, index:scalar.nodes[0].attributes[0].nativeValueIndex}])});
            assert.equal(native.renderReuse, undefined);
            assert.deepEqual(native.diagnostics, []);
            if (native.nativeValueArtifactId) wasm.takeRenderValueArtifact(native.nativeValueArtifactId);
        }
    } finally { wasm.disposeNativeValueArtifact(nativeId); }
    const limitedHit = render(typed, {value:42}, 'engine/instance', {limits:JSON.stringify({maxBytes:1,maxValues:1,maxDepth:1})});
    assert.equal(limitedHit.renderReuse.hit, true);
    assert.deepEqual(limitedHit.nodes, []);
    assert.ok(limitedHit.diagnostics.some(d => d.severity === 'error'));
    const projection = retain('{p | projection}');
    render(projection, {}); const projectedHit = render(projection, {});
    assert.equal(projectedHit.renderReuse.hit, true); assert.equal(projectedHit.referenceProjectionComplete, undefined);
    // Unknown explicit capabilities must reject before a previously cached body.
    assert.ok(render(projection, {}, 'engine/instance', {documents:[{slice:'input', documentId:999999}]}).diagnostics.some(d=>d.code==='cem.ql.wasm.unknown_document'));
    assert.ok(render(projection, {}, 'engine/instance', {companion:999999}).diagnostics.some(d=>d.code==='cem.ql.wasm.unknown_xpath_companion'));
    assert.equal(JSON.parse(wasm.renderTemplateWithNativeValues(id, 0, '{"value":"Ada"}', '[]', '[]', '', 'instance', undefined, undefined)).renderReuse, undefined);
    const generation = JSON.parse(wasm.retainedRenderCacheStats()).runtimeGeneration;
    wasm.resetRetainedRenderCache();
    assert.equal(JSON.parse(wasm.retainedRenderCacheStats()).runtimeGeneration, generation + 1);
    assert.equal(render(id, {value:'Ada'}).renderReuse.hit, false);
    wasm.disposeTemplate(id);
    assert.equal(JSON.parse(wasm.retainedRenderCacheStats()).residentEntries, 0);
    assert.ok(render(id,{value:'Ada'}).diagnostics.some(d=>d.code==='cem.ql.wasm.unknown_artifact'));
    console.log('Retained WASM pure-render reuse: native freshness, admission, projection, opaque inputs and disposal passed.');
} finally { for (const id of ids) wasm.disposeTemplate(id); wasm.resetRetainedRenderCache(); }
