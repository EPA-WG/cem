// CEMT-COMPACT-TEMPLATES: native preflight and the WASM renderer agree on module bodies.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import * as wasm from '../dist/wasm/cem_ql.js';

await wasm.default({ module_or_path: await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url)) });
const page = await readFile(new URL('../../cem-elements/demo/cell-overrides.html', import.meta.url), 'utf8');
const pokemon = page.split('<template type="text/cem-ml">')[1].split('</template>')[0];
const inspected = JSON.parse(wasm.templateModuleImports(pokemon, 'https://example.test/cell.cemt'));
assert.deepEqual(inspected.diagnostics, []);
assert.equal(inspected.imports[0].uri, './data-table-view.cemt');

const base = '{module | {template @name=label @visibility=public | {param @name=title}{strong | {$title}}}}';
const source = '{module | {import @as=base @src="./base.cemt"}{body | {call @from=base @template=label @with:title=ivy}}}';
for (const text of [source, base]) {
    assert.deepEqual(JSON.parse(wasm.templateModuleImports(text, 'https://example.test/module.cemt')).diagnostics, []);
}
const hash = source => JSON.parse(wasm.templateArtifactPayloadKey(source, 'dev')).sourceHash;
const artifact = JSON.parse(wasm.compileTemplateModuleClosure(source, JSON.stringify({
    rootUri: 'https://example.test/entry.cemt', rootContentHash: hash(source),
    modules: [{ alias: 'base', uri: 'https://example.test/base.cemt', contentHash: hash(base), source: base }],
}), '[]'));
try {
    assert.deepEqual(artifact.diagnostics, []);
    const plan = JSON.parse(wasm.renderTemplate(artifact.artifactId, '{}'));
    assert.deepEqual(plan.diagnostics, []);
    const element = plan.nodes.find(node => node.kind === 'element');
    assert.equal(element.tag, 'strong');
    assert.equal(element.children[0].text, 'ivy');
} finally { wasm.disposeTemplate(artifact.artifactId); }

for (const body of ['{body | first}{b | second}', 'text{body | wrapped}', '{body | first}{body | second}']) {
    const source = `{module | {template @name=label | ${body}}{body | {call @template=label}}}`;
    const preflight = JSON.parse(wasm.templateModuleImports(source, 'https://example.test/invalid.cemt'));
    assert.ok(preflight.diagnostics.some(d => d.code === 'cem.transform_template.declaration_invalid'));
    const plan = JSON.parse(wasm.renderTemplateSource(source, '{}'));
    assert.ok(plan.diagnostics.some(d => d.code === 'cem.transform_template.declaration_invalid'));
    assert.ok(plan.nodes.every(node => node.kind === 'text' && node.text.trim() === ''));
}
console.log('Compact CEMT bodies: demo preflight, imported calls and invalid-body rejection passed.');
