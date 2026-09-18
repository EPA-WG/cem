// XSLT-BUNDLE-WASM: native-produced executable members, separate WASM process.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import init, {
    importXsltBundle, renderXsltBundle, disposeXsltBundle,
    retainCemDocument, disposeCemDocument,
    compileTemplate, renderTemplate, disposeTemplate,
} from '../../packages/cem_ql/dist/wasm/cem_ql.js';

const root = fileURLToPath(new URL('../../', import.meta.url));
const directory = mkdtempSync(join(tmpdir(), 'cem-xslt-bundles-'));
try {
    execFileSync('cargo', ['test', '-p', 'cem-ql', '--test', 'xslt_bundle'], {
        cwd: root, stdio: 'inherit', env: { ...process.env, CEM_XSLT_BUNDLE_FIXTURE_DIR: directory },
    });
    await init({ module_or_path: readFileSync(join(root, 'packages/cem_ql/dist/wasm/cem_ql_bg.wasm')) });
    // Only deployment/control manifests are decoded in JS. All document bytes
    // go straight to the common CEM import/retention boundary below.
    const fixture = JSON.parse(readFileSync(join(directory, 'fixture.json'), 'utf8'));
    const bytes = readFileSync(join(directory, 'bundle.bin'));
    const load = (value = bytes, hash = fixture.contentHash, source = fixture.rootSourceHash) =>
        JSON.parse(importXsltBundle(value, hash, source));
    const bundle = load();
    assert.equal(bundle.contentType, 'application/vnd.cem.xslt-bundle+cem-bin');
    assert.equal(bundle.formatVersion, 'cem-xslt-bundle/1');
    assert.equal(bundle.sourceClosure[1].source.uri, 'memory:library.xslt');
    assert.equal(bundle.sourceClosure[0].dependencies[0].kind, 'import');
    assert.deepEqual(bundle.diagnostics, []);
    let checks = 1;
    const text = nodes => nodes.map(node => node.kind === 'text' ? node.text : text(node.children ?? [])).join('');
    const render = (documentId, scalars = '{"prefix":"hello ","position":2,"size":3}', id = bundle.bundleId) =>
        JSON.parse(renderXsltBundle(id, scalars, JSON.stringify([{ name: 'document', documentId }])));
    for (const [source, type, value] of [
        ['<r><item>A</item></r>', 'application/xml', 'A'],
        ['<r><item>B</item></r>', 'application/xml', 'B'],
        ['{"label":"from JSON"}', 'application/json', 'from JSON'],
        ['label: from YAML\n', 'application/yaml', 'from YAML'],
        ['label\nfrom CSV\n', 'text/csv', 'from CSV'],
    ]) {
        const document = retainCemDocument(new TextEncoder().encode(source), type, 'memory:input');
        try {
            const output = render(document);
            assert.deepEqual(output.diagnostics, []);
            assert.equal(text(output.nodes), `hello ${value}:2/3`);
            checks++;
        } finally { assert.equal(disposeCemDocument(document), true); }
        assert.ok(render(document).diagnostics.some(d => d.code === 'cem.xslt.unknown_document'));
        checks++;
    }
    const document = retainCemDocument(new TextEncoder().encode('<r/>'), 'application/xml', 'memory:focus');
    try {
        const output = render(document, '{"prefix":"hello ","position":4,"size":3}');
        const diagnostic = output.diagnostics.find(d => d.code === 'cem.xpath.focus_invalid');
        assert.equal(diagnostic.uri, 'memory:library.xslt');
        assert.ok(diagnostic.byteOffset > 0);
        assert.ok(diagnostic.sourceMap.frames.length > 0);
        checks++;
        for (const scalars of [
            '{"native_functions":{"xslt.program.0":1}}',
            '{"document":{"root":"fake"}}', '{"prefix":["fake"]}', '{"unknown":1}',
        ]) {
            assert.ok(render(document, scalars).diagnostics.some(d => d.code === 'cem.xslt.invalid_binding'));
            checks++;
        }
        for (const controls of [
            [{ name: 'document', documentId: document }, { name: 'document', documentId: document }],
            [{ name: 'unknown', documentId: document }], [{ name: 'document', documentId: document, extra: true }],
        ]) {
            const output = JSON.parse(renderXsltBundle(bundle.bundleId, '{}', JSON.stringify(controls)));
            assert.ok(output.diagnostics.some(d => d.code === 'cem.xslt.invalid_binding'));
            checks++;
        }
    } finally { disposeCemDocument(document); }
    // A bundle import does not change the ordinary template registry.
    const ordinary = JSON.parse(compileTemplate(fixture.generated, '["document","position","size","prefix"]'));
    try {
        const output = JSON.parse(renderTemplate(ordinary.artifactId, '{"document":null,"position":1,"size":1,"prefix":"ordinary","native_functions":{"xslt.program.0":1}}'));
        assert.ok(output.diagnostics.some(d => d.code === 'cem.ql.native_function_unavailable'));
        checks++;
    } finally { disposeTemplate(ordinary.artifactId); }
    for (const rejected of JSON.parse(readFileSync(join(directory, 'rejections.json'), 'utf8'))) {
        assert.throws(() => load(readFileSync(join(directory, rejected.name)), rejected.hash), /cem\.xslt\.bundle_/);
        checks++;
    }
    assert.throws(() => load(bytes, fixture.rootSourceHash), /bundle_identity_mismatch/);
    assert.throws(() => load(bytes, fixture.contentHash, fixture.contentHash), /bundle_identity_mismatch/);
    assert.throws(() => load(bytes, 'invalid'), /bundle_identity_mismatch/);
    assert.throws(() => load(new Uint8Array(8 * 1024 * 1024 + 1)), /bundle_limit/);
    checks += 4;
    assert.equal(disposeXsltBundle(bundle.bundleId), true);
    assert.equal(disposeXsltBundle(bundle.bundleId), false);
    assert.ok(JSON.parse(renderXsltBundle(bundle.bundleId, '{}', '[]')).diagnostics.some(d => d.code === 'cem.xslt.unknown_bundle'));
    const retained = Array.from({ length: 16 }, () => load().bundleId);
    assert.ok(retained[0] > bundle.bundleId);
    assert.throws(() => load(), /bundle_limit/);
    for (const id of retained) assert.equal(disposeXsltBundle(id), true);
    const next = load().bundleId;
    assert.ok(next > retained.at(-1));
    assert.equal(disposeXsltBundle(next), true);
    checks++;
    console.log(`XSLT bundle checks passed: ${checks} (native/WASM, shared CEM documents, ownership, focus, bounds and isolation).`);
} finally {
    rmSync(directory, { recursive: true, force: true });
}
