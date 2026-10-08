// Real WASM sources, executable descendants and presentation-only CEMV.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import * as wasm from '../dist/wasm/cem_ql.js';
await wasm.default({ module_or_path: await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
const limits = JSON.stringify({ maxBytes: 16 * 1024 * 1024, maxValues: 100000, maxDepth: 128 });
const source = text => wasm.parseReferenceSource(new TextEncoder().encode(text), 'text/cem-ml', 'memory:options.cem', '');
const relation = source('{#datadom.slices.options}');
const options = source('{option @value=same | First {#datadom.slices.other}}{option @value=same | Second}');
const metadata = { requesting: 0, sources: [{ sourceId: relation, context: true }, { sourceId: options, context: true }], bindings: [
    { source: 0, name: 'relation', select: 'input.children' }, { source: 1, name: 'options', select: 'input.children' }], grants: [[0, 1]] };
const prepare = value => wasm.prepareNativeCapabilitySession('{}', JSON.stringify(value), 'relation', true, '[]', limits);
for (const bad of [{ ...metadata, grants: [] }, { ...metadata, sources: [{ sourceId: relation, context: false }, metadata.sources[1]] },
    { ...metadata, sources: [metadata.sources[0], { sourceId: options, context: true, maxWork: 1 }] }]) assert.throws(() => prepare(bad));
const session = prepare(metadata);
assert.equal(wasm.nativeCapabilitySessionLength(session), 2);
// The session keeps its owners after disposable ingress handles are released.
wasm.disposeReferenceSource(relation); wasm.disposeReferenceSource(options);
const label = JSON.parse(wasm.compileTemplate('{span | {$input.name}|{$input.children.kind}|{$input.children.expression}}', '["input"]'));
assert.deepEqual(label.diagnostics, []);
try {
    const plan = JSON.parse(wasm.renderNativeCapabilityTemplate(session, label.artifactId, 0));
    assert.deepEqual(plan.diagnostics, []);
    assert.ok(plan.nodes[0].children.some(n => n.text === '#datadom.slices.other'));
    assert.throws(() => wasm.exportNativeCapabilityView(session, 'input', 0), /executable source reference/);
    const presentation = JSON.parse(wasm.exportNativeCapabilityView(session, 'input.name', undefined));
    assert.equal(presentation.length, 2);
    const bytes = wasm.takeRenderValueArtifact(presentation.artifactId);
    assert.equal(wasm.exportCemJsonValue(bytes, 0, '', limits), '"option"');
    assert.equal(wasm.exportCemJsonValue(bytes, 1, '', limits), '"option"');
    const empty = JSON.parse(wasm.exportNativeCapabilityView(session, '()', undefined));
    assert.equal(empty.length, 0); assert.equal(empty.artifactId, null);
    assert.throws(() => wasm.renderNativeCapabilityTemplate(session, label.artifactId, 2));
    assert.equal(wasm.disposeNativeCapabilitySession(session), true);
    assert.equal(wasm.disposeNativeCapabilitySession(session), false);
    assert.throws(() => wasm.nativeCapabilitySessionLength(session), /Unknown native/);
} finally { wasm.disposeTemplate(label.artifactId); wasm.disposeNativeCapabilitySession(session); }
const suggestionSource = source('{optgroup @label=Group | {option @value=a | Apple}{option @value=same @label=Straße @selected=false | {#datadom.slices.later}}}{option @value=same | Second}');
const suggestions = wasm.prepareNativeCapabilitySession('{}', JSON.stringify({ requesting: 0, sources: [{ sourceId: suggestionSource, context: true }], bindings: [], grants: [] }), 'input.children', true, '[]', limits);
// 384 admits native source preparation but is below the 460-byte diagnostic envelope.
const bounded = wasm.prepareNativeCapabilitySession('{}', JSON.stringify({ requesting: 0, sources: [{ sourceId: suggestionSource, context: true }], bindings: [], grants: [] }), 'input.children', true, '[]', JSON.stringify({ maxBytes: 384, maxValues: 100000, maxDepth: 128 }));
try { assert.throws(() => wasm.prepareNativeSuggestions(bounded), /Suggestions control byte limit exceeded/); }
finally { wasm.disposeNativeCapabilitySession(bounded); }
wasm.disposeReferenceSource(suggestionSource);
const config = JSON.stringify({ query: 'STRASSE', queryRevision: 1 });
const scalar = (expression, index) => {
    const value = JSON.parse(wasm.exportNativeSuggestionsView(suggestions, config, expression, index));
    const bytes = wasm.takeRenderValueArtifact(value.artifactId);
    return Array.from({ length: value.length }, (_, i) => JSON.parse(wasm.exportCemJsonValue(bytes, i, '', limits)));
};
const suggestionLabel = JSON.parse(wasm.compileTemplate('{span | {$dom:attribute(suggestion, "label").value}|{$suggestion.content.expression}}', '["suggestion"]'));
const groupLabel = JSON.parse(wasm.compileTemplate('{span | {$dom:attribute(group, "label").value}}', '["group"]'));
try {
    const plan = JSON.parse(wasm.prepareNativeSuggestions(suggestions));
    assert.equal(plan.rows, 3); assert.equal(plan.groups, 1);
    assert.equal(plan.identity, 'cem-suggestions-v1-unicode-17.0.0');
    assert.equal(plan.diagnostics[0].code, 'cem.suggestions.selected_ignored');
    assert.ok(plan.diagnostics[0].sourceMap.frames.length);
    assert.deepEqual(scalar('input.children.children.dom:attribute("matched").value'), [false, true]);
    assert.deepEqual(scalar('input.children.children.dom:attribute("value").value'), ['a', 'same']);
    assert.deepEqual(scalar('input.content.expression', 1), ['#datadom.slices.later']);
    assert.throws(() => wasm.exportNativeSuggestionsView(suggestions, config, 'input', undefined), /live suggestions source\/content edges/);
    assert.throws(() => wasm.exportNativeSuggestionsView(suggestions, JSON.stringify({ query: '', filter: 'external', filterBy: 'label' }), 'input', undefined), /cem.suggestions.configuration/);
    assert.throws(() => wasm.exportNativeSuggestionsView(suggestions, config, 'input', 3), /row handle/);
    const label = JSON.parse(wasm.renderNativeSuggestionTemplate(suggestions, suggestionLabel.artifactId, config, 1, false));
    assert.ok(JSON.stringify(label.nodes).includes('Straße'));
    assert.ok(JSON.stringify(label.nodes).includes('#datadom.slices.later'));
    const group = JSON.parse(wasm.renderNativeSuggestionTemplate(suggestions, groupLabel.artifactId, config, 0, true));
    assert.ok(JSON.stringify(group.nodes).includes('Group'));
} finally { wasm.disposeTemplate(suggestionLabel.artifactId); wasm.disposeTemplate(groupLabel.artifactId); wasm.disposeNativeCapabilitySession(suggestions); }
console.log('native capability and suggestions session WASM checks passed');

// Native datalist publications retain their source owner, with no query or row proof.
const nativeSource = source('{option @value=1 @label=One}{option @value=1 @label=Duplicate}{option @value="" @label=Empty}');
const nativeSession = wasm.prepareNativeCapabilitySession('{}', JSON.stringify({ requesting: 0, sources: [{ sourceId: nativeSource, context: true }], bindings: [], grants: [] }), 'input.children', true, '[]', limits);
wasm.disposeReferenceSource(nativeSource);
const nativeTemplate = JSON.parse(wasm.compileTemplate('{datalist | {cem:for-each @select=suggestions.children @as=row | {option @value={row.dom:attribute("value").value} @label={row.dom:attribute("label").value}}}}', '["suggestions"]'));
try {
    const prepared = JSON.parse(wasm.prepareNativeDatalist(nativeSession));
    assert.equal(prepared.identity, 'cem-native-datalist-v1'); assert.equal(prepared.rows, 2);
    assert.equal(prepared.diagnostics[0].code, 'cem.suggestions.datalist_empty_value');
    assert.ok(prepared.diagnostics[0].sourceMap.frames.length);
    for (const controls of ['{"query":"x"}', '{"active":0}', '[]', 'null']) assert.throws(() => wasm.publishNativeDatalist(nativeSession, 'bad', controls));
    wasm.publishNativeDatalist(nativeSession, 'native', '{}');
    const rendered = JSON.parse(wasm.renderNativeSuggestionsFrame(nativeSession, 'native', nativeTemplate.artifactId, '{}'));
    assert.deepEqual(rendered.diagnostics, []); assert.equal(rendered.nodes[0].children.length, 2);
    assert.ok(JSON.stringify(rendered).includes('One')); assert.ok(!JSON.stringify(rendered).includes('Empty'));
    assert.throws(() => wasm.nativeSuggestionRowControls(nativeSession, 'native'));
    wasm.releaseNativeSuggestions(nativeSession, 'native');
    assert.throws(() => wasm.renderNativeSuggestionsFrame(nativeSession, 'native', nativeTemplate.artifactId, '{}'));
    assert.throws(() => wasm.publishNativeDatalist(nativeSession, 'native', '{}'));
} finally { wasm.disposeTemplate(nativeTemplate.artifactId); wasm.disposeNativeCapabilitySession(nativeSession); }
