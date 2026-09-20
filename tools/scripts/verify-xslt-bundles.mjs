// XSLT-BUNDLE-WASM: native-produced executable members, separate WASM process.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import init, {
    importXsltBundle, renderXsltBundle, disposeXsltBundle,
    compileXsltBundle, retainXsltStylesheet,
    retainXsltComponent, renderXsltComponent, disposeXsltComponent, xsltStylesheetImports,
    retainCemDocument, disposeCemDocument,
    compileTemplate, renderTemplate, disposeTemplate, evaluateQuerySource,
} from '../../packages/cem_ql/dist/wasm/cem_ql.js';

const root = fileURLToPath(new URL('../../', import.meta.url));
const directory = mkdtempSync(join(tmpdir(), 'cem-xslt-bundles-'));
try {
    execFileSync('cargo', ['test', '-p', 'cem-ql', '--test', 'xslt_bundle'], {
        cwd: root, stdio: 'inherit', env: { ...process.env, CEM_XSLT_BUNDLE_FIXTURE_DIR: directory },
    });
    execFileSync('cargo', ['test', '-p', 'cem-ql', '--test', 'xslt_lowering'], {
        cwd: root, stdio: 'inherit', env: { ...process.env, CEM_XSLT_LOWER_FIXTURE_DIR: directory },
    });
    execFileSync('cargo', ['test', '-p', 'cem-ql', '--test', 'xslt_matching'], {
        cwd: root, stdio: 'inherit', env: { ...process.env, CEM_XSLT_MATCH_FIXTURE_DIR: directory },
    });
    execFileSync('cargo', ['test', '-p', 'cem-ql', '--test', 'xslt_grouping'], {
        cwd: root, stdio: 'inherit', env: { ...process.env, CEM_XSLT_GROUP_FIXTURE_DIR: directory },
    });
    execFileSync('cargo', ['test', '-p', 'cem-ql', '--test', 'xslt_sorting'], {
        cwd: root, stdio: 'inherit', env: { ...process.env, CEM_XSLT_SORT_FIXTURE_DIR: directory },
    });
    execFileSync('cargo', ['test', '-p', 'cem-ql', '--test', 'xslt_data_recovery'], {
        cwd: root, stdio: 'inherit', env: { ...process.env, CEM_XSLT_DATA_FIXTURE_DIR: directory },
    });
    execFileSync('cargo', ['test', '-p', 'cem-ql', '--test', 'xslt_output'], {
        cwd: root, stdio: 'inherit', env: { ...process.env, CEM_XSLT_OUTPUT_FIXTURE_DIR: directory },
    });
    execFileSync('cargo', ['test', '-p', 'cem-ql', '--test', 'xslt_viewer_selection_boundary'], {
        cwd: root, stdio: 'inherit', env: { ...process.env, CEM_SOURCE_PROVENANCE_FIXTURE_DIR: directory },
    });
    execFileSync('cargo', ['test', '-p', 'cem-ql', '--test', 'xslt_data_view'], {
        cwd: root, stdio: 'inherit', env: { ...process.env, CEM_XSLT_VIEW_FIXTURE_DIR: directory },
    });
    execFileSync('cargo', ['test', '-p', 'cem-ql', '--test', 'xslt_component'], {
        cwd: root, stdio: 'inherit', env: { ...process.env, CEM_XSLT_COMPONENT_FIXTURE_DIR: directory },
    });
    execFileSync('cargo', ['test', '-p', 'cem-ql', '--test', 'xslt_optional_focus'], {
        cwd: root, stdio: 'inherit', env: { ...process.env, CEM_XSLT_OPTIONAL_FOCUS_FIXTURE_DIR: directory },
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
    const legacy = JSON.parse(compileTemplate('{p @title=legacy |text}', '[]'));
    try {
        const output = JSON.parse(renderTemplate(legacy.artifactId, '{}'));
        assert.deepEqual(output.diagnostics, []);
        assert.equal(output.nodes[0].attributes[0].name, 'title');
        assert.equal('namespaceUri' in output.nodes[0].attributes[0], false);
        checks++;
    } finally { disposeTemplate(legacy.artifactId); }
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
    // XSLT-LOWER-CORE: this bundle is produced by the stylesheet compiler,
    // with runtime loops and original typed XPath programs, not hand-written CEMT.
    const loweredBytes = readFileSync(join(directory, 'lowered.bin'));
    const loweredManifest = JSON.parse(readFileSync(join(directory, 'manifest.json'), 'utf8'));
    const stylesheet = readFileSync(join(directory, 'stylesheet.xslt'), 'utf8');
    assert.deepEqual(Buffer.from(compileXsltBundle(stylesheet, 'memory:lower.xslt')), loweredBytes);
    checks++;
    const lowered = load(loweredBytes, loweredManifest.contentHash, loweredManifest.sourceHash);
    const sourceLoaded = JSON.parse(retainXsltStylesheet(stylesheet, 'memory:lower.xslt'));
    assert.equal(sourceLoaded.contentHash, loweredManifest.contentHash);
    assert.equal(sourceLoaded.rootSourceHash, loweredManifest.sourceHash);
    assert.deepEqual(sourceLoaded.hostBindings, ['document']);
    checks++;
    for (const retained of [lowered, sourceLoaded]) {
        try {
            for (const [source, type, labels] of [
                ['<r><a>A</a><b>B</b></r>', 'application/xml', ['A', 'B']],
                ['{"a":"A","b":"B"}', 'application/json', ['A', 'B']],
                ['a: A\nb: B\n', 'application/yaml', ['A', 'B']],
                ['v\nA\nB\n', 'text/csv', ['A', 'B']],
                ['<r><a>C</a></r>', 'application/xml', ['C']],
                ['<r/>', 'application/xml', []],
            ]) {
                const document = retainCemDocument(new TextEncoder().encode(source), type, 'memory:lower-input');
                try {
                    const output = render(document, '{}', retained.bundleId);
                    assert.deepEqual(output.diagnostics, []);
                    assert.equal(text(output.nodes), labels.map((label, i) =>
                        `${label}:${i + 1}/${labels.length}10:1/220:2/2${i + 1}/${labels.length}`).join(''));
                    checks++;
                } finally { assert.equal(disposeCemDocument(document), true); }
            }
        } finally { assert.equal(disposeXsltBundle(retained.bundleId), true); }
        assert.ok(JSON.parse(renderXsltBundle(retained.bundleId, '{}', '[]')).diagnostics.some(d => d.code === 'cem.xslt.unknown_bundle'));
    }
    // XSLT-MATCH-RUNTIME: imported named overrides and match precedence are
    // already linked in this native-produced bundle; WASM performs no I/O.
    const matchedManifest = JSON.parse(readFileSync(join(directory, 'matched.json'), 'utf8'));
    const matched = load(readFileSync(join(directory, 'matched.bin')), matchedManifest.contentHash, matchedManifest.sourceHash);
    try {
        assert.equal(matched.sourceClosure.length, 2);
        for (const [source, expected] of [['<r><a>A</a></r>', 'overrideA'], ['<r><a>B</a><a>C</a></r>', 'overrideBC']]) {
            const document = retainCemDocument(new TextEncoder().encode(source), 'application/xml', 'memory:match-input');
            try {
                const output = render(document, '{}', matched.bundleId);
                assert.deepEqual(output.diagnostics, []);
                assert.equal(text(output.nodes), expected);
                checks++;
            } finally { disposeCemDocument(document); }
        }
    } finally { disposeXsltBundle(matched.bundleId); }
    const wrap = body => `<xsl:stylesheet xmlns:xsl="http://www.w3.org/1999/XSL/Transform" version="3.0"><xsl:template match="/">${body}</xsl:template></xsl:stylesheet>`;
    for (const [name, inputs] of [
        ['grouped', [
            ['<r><row id="a"><name>A</name></row><note/><row id="b"><age>2</age><name>B</name></row></r>', 'application/xml', 'idnameageab'],
            ['<r><entry id="c"><city>C</city></entry><entry id="d"><extra>D</extra></entry></r>', 'application/xml', 'idcityextracd'],
            ['<r/>', 'application/xml', ''],
        ]],
        ['grouped-formats', [
            ['<r><a>A</a><b>B</b></r>', 'application/xml', 'all|A|B'],
            ['{"a":"A","b":"B"}', 'application/json', 'all|A|B'],
            ['a: A\nb: B\n', 'application/yaml', 'all|A|B'],
            ['v\nA\nB\n', 'text/csv', 'all|A|B'],
        ]],
    ]) {
        const manifest = JSON.parse(readFileSync(join(directory, `${name}.json`), 'utf8'));
        const bytes = readFileSync(join(directory, `${name}.bin`));
        const source = readFileSync(join(directory, `${name}.xslt`), 'utf8');
        assert.deepEqual(Buffer.from(compileXsltBundle(source, 'memory:group.xslt')), bytes);
        checks++;
        for (const retained of [load(bytes, manifest.contentHash, manifest.sourceHash), JSON.parse(retainXsltStylesheet(source, 'memory:group.xslt'))]) {
            try {
                for (const [input, type, expected] of inputs) {
                    const document = retainCemDocument(new TextEncoder().encode(input), type, 'memory:group-input');
                    try {
                        const output = render(document, '{}', retained.bundleId);
                        assert.deepEqual(output.diagnostics, []);
                        assert.equal(text(output.nodes), expected);
                        checks++;
                    } finally { assert.equal(disposeCemDocument(document), true); }
                }
            } finally { assert.equal(disposeXsltBundle(retained.bundleId), true); }
        }
    }
    for (const [expression, code] of [['current-group()', 'XTDE1061'], ['current-grouping-key()', 'XTDE1071']]) {
        const retained = JSON.parse(retainXsltStylesheet(wrap(`<xsl:for-each-group select="/*/*" group-by="'one'"><p>partial<xsl:value-of select="(function() { ${expression} })()"/></p></xsl:for-each-group>`), 'memory:group-error.xslt'));
        const document = retainCemDocument(new TextEncoder().encode('<r><row/></r>'), 'application/xml', 'memory:group-input');
        try {
            const output = render(document, '{}', retained.bundleId);
            assert.deepEqual(output.nodes, []);
            assert.ok(output.diagnostics.some(d => d.message.includes(code) && d.uri === 'memory:group-error.xslt' && d.sourceMap.frames.length));
            checks++;
        } finally {
            disposeXsltBundle(retained.bundleId);
            disposeCemDocument(document);
        }
    }
    // XSLT-SORT-WASM: the same portable programs retain native values across
    // changed sort controls, group sorting, and each shared import format.
    for (const [name, inputs] of [
        ['sorted', [
            ['<r order="ascending"><row id="a" n="2" label="B"/><row id="b" n="10"/><row id="c" n="2" label="A"/><row id="d" n="02" label="A"/><row id="e" n="bad"/><row id="f"/></r>', 'application/xml', 'c|1|6|rd|2|6|ra|3|6|rb|4|6|re|5|6|rf|6|6|r'],
            ['<r order="descending"><row id="a" n="2"/><row id="b" n="10"/><row id="c" n="02"/><row id="d" n="bad"/></r>', 'application/xml', 'b|1|4|ra|2|4|rc|3|4|rd|4|4|r'],
            ['<r order="ascending"/>', 'application/xml', ''],
        ]],
        ['sorted-groups', [
            ['<r><row id="a" g="B"/><row id="b" g="A"/><row id="c" g="A"/></r>', 'application/xml', 'A|1|2|b|cA|c|1|2|b|cA|b|2|2|b|cB|2|2|aB|a|1|1|a'],
            ['<r/>', 'application/xml', ''],
        ]],
        ['sorted-conversions', [
            ['<r kind="number" order="descending"><row id="a" n="2"/><row id="b" n="bad"/><row id="c" n="10"/><row id="d" n="02"/></r>', 'application/xml', 'cadb'],
            ['<r kind="text" order="ascending"><row id="a" n="2"/><row id="b" n="10"/><row id="c"/></r>', 'application/xml', 'cba'],
        ]],
        ['sorted-formats', [
            ['<r><a>B</a><b>A</b></r>', 'application/xml', 'AB'],
            ['{"a":"B","b":"A"}', 'application/json', 'AB'],
            ['a: B\nb: A\n', 'application/yaml', 'AB'],
            ['v\nB\nA\n', 'text/csv', 'AB'],
        ]],
    ]) {
        const manifest = JSON.parse(readFileSync(join(directory, `${name}.json`), 'utf8'));
        const bytes = readFileSync(join(directory, `${name}.bin`));
        const source = readFileSync(join(directory, `${name}.xslt`), 'utf8');
        assert.deepEqual(Buffer.from(compileXsltBundle(source, 'memory:sort.xslt')), bytes);
        checks++;
        for (const retained of [load(bytes, manifest.contentHash, manifest.sourceHash), JSON.parse(retainXsltStylesheet(source, 'memory:sort.xslt'))]) {
            try {
                for (const [input, type, expected] of inputs) {
                    const document = retainCemDocument(new TextEncoder().encode(input), type, 'memory:sort-input');
                    try {
                        const output = render(document, '{}', retained.bundleId);
                        assert.deepEqual(output.diagnostics, []);
                        assert.equal(text(output.nodes), expected);
                        checks++;
                    } finally { assert.equal(disposeCemDocument(document), true); }
                }
            } finally { assert.equal(disposeXsltBundle(retained.bundleId), true); }
        }
    }
    // XSLT-DATA-WASM: changed XML/JSON text is imported only inside the native
    // evaluator. Both deployment routes retain trees and recover typed errors.
    {
        const name = 'parsed-recovered';
        const manifest = JSON.parse(readFileSync(join(directory, `${name}.json`), 'utf8'));
        const bytes = readFileSync(join(directory, `${name}.bin`));
        const source = readFileSync(join(directory, `${name}.xslt`), 'utf8');
        assert.deepEqual(Buffer.from(compileXsltBundle(source, 'memory:data-recovery.xslt')), bytes);
        checks++;
        for (const retained of [load(bytes, manifest.contentHash, manifest.sourceHash), JSON.parse(retainXsltStylesheet(source, 'memory:data-recovery.xslt'))]) {
            try {
                for (const [input, expected] of [
                    ['<input><![CDATA[<r><x>A</x><x>B</x></r>]]></input>', 'beforeA|1|2B|2|2'],
                    ['<input>bad XML</input>', 'err:FODC0006|true|input'],
                    ['<input format="json">{"A":1,"B":2}</input>', 'before1|1|22|2|2'],
                    ['<input format="json">[</input>', 'json error'],
                ]) {
                    const document = retainCemDocument(new TextEncoder().encode(input), 'application/xml', 'memory:data-input.xml');
                    try {
                        const output = render(document, '{}', retained.bundleId);
                        assert.deepEqual(output.diagnostics, []);
                        assert.equal(text(output.nodes), expected);
                        checks++;
                    } finally { assert.equal(disposeCemDocument(document), true); }
                }
            } finally { assert.equal(disposeXsltBundle(retained.bundleId), true); }
        }
    }
    // SOURCE-PROVENANCE-QUERY: compare native and WASM source keys, locations,
    // sorting/selection and CSV options. JSON here describes expected output;
    // document inputs remain source strings imported by CEM-ML.
    const selection = JSON.parse(readFileSync(join(directory, 'selection.json'), 'utf8'));
    const selectionBytes = readFileSync(join(directory, 'selection.bin'));
    const selectionSource = readFileSync(join(directory, 'selection.xslt'), 'utf8');
    assert.deepEqual(Buffer.from(compileXsltBundle(selectionSource, 'memory:selection.xslt')), selectionBytes);
    checks++;
    for (const retained of [load(selectionBytes, selection.contentHash, selection.sourceHash), JSON.parse(retainXsltStylesheet(selectionSource, 'memory:selection.xslt'))]) {
        try {
            for (const test of selection.cases) {
                const document = retainCemDocument(new TextEncoder().encode(test.input), 'application/xml', 'memory:selection-input');
                try {
                    const output = render(document, '{}', retained.bundleId);
                    assert.deepEqual(output.diagnostics, []);
                    assert.deepEqual(output.nodes.map(node => [
                        ...['value', 'aria-label', 'aria-pressed'].map(name => node.attributes.find(attribute => attribute.name === name).value),
                        text(node.children),
                    ]), test.rows);
                    checks++;
                } finally { disposeCemDocument(document); }
            }
            const document = retainCemDocument(new TextEncoder().encode(selection.invalid), 'application/xml', 'memory:selection-input');
            try {
                const output = render(document, '{}', retained.bundleId);
                assert.deepEqual(output.diagnostics, []);
                assert.equal(text(output.nodes), 'Invalid CSV options');
                checks++;
            } finally { disposeCemDocument(document); }
        } finally { disposeXsltBundle(retained.bundleId); }
    }
    // XSLT-VIEW-BASE-WASM: scalar controls and expected render protocol only;
    // external source strings are parsed exclusively inside CEM-ML import.
    const viewer = JSON.parse(readFileSync(join(directory, 'viewer.json'), 'utf8'));
    assert.deepEqual(Buffer.from(compileXsltBundle(readFileSync(join(directory, 'viewer.xslt'), 'utf8'), 'memory:data-table-view.xslt')),
        readFileSync(join(directory, 'viewer-default.bin')));
    checks++;
    const visible = nodes => {
        const result = [];
        let pending = '';
        const flush = () => { if (pending.trim()) result.push({ Text: pending.trim() }); pending = ''; };
        for (const node of nodes) {
            if (node.kind === 'text') { pending += node.text; continue; }
            assert.equal(node.kind, 'element');
            flush();
            if (node.tag === 'style') continue;
            const selection = node.tag === 'button' && node.attributes.some(a => a.name === 'slice' && a.value === 'selected');
            const attributes = node.attributes.filter(a => !(selection && a.name === 'value')
                && !((node.tag === 'select' || node.tag === 'input') && a.name === 'value' && !a.value))
                .map(a => [a.name, a.value]).sort((a, b) => a[0] < b[0] ? -1 : a[0] > b[0] ? 1 : 0);
            result.push({ Element: [node.tag, attributes, visible(node.children)] });
        }
        flush();
        return result;
    };
    const selections = nodes => nodes.flatMap(node => {
        if (node.kind !== 'element') return [];
        const attr = name => node.attributes.find(a => a.name === name)?.value ?? '';
        return [...(attr('slice') === 'selected' ? [[attr('value'), attr('aria-label'), attr('aria-pressed')]] : []), ...selections(node.children)];
    });
    const retainedViewer = load(readFileSync(join(directory, 'viewer.bin')), viewer.contentHash, viewer.sourceHash);
    try {
        const document = retainCemDocument(new TextEncoder().encode('<input/>'), 'application/xml', 'memory:viewer-input');
        try {
            for (const test of viewer.cases) {
                const output = render(document, JSON.stringify(test.controls), retainedViewer.bundleId);
                assert.deepEqual(output.diagnostics, []);
                assert.deepEqual(visible(output.nodes), test.nodes);
                assert.deepEqual(selections(output.nodes), test.selections);
                checks++;
            }
        } finally { assert.equal(disposeCemDocument(document), true); }
    } finally { assert.equal(disposeXsltBundle(retainedViewer.bundleId), true); }
    // XSLT-VIEW-BROWSER-SCALARS: metadata carries CEM-QL selectors and scalar
    // state, while the initial focus and resource data use native handles.
    const componentSource = readFileSync(join(directory, 'component.xslt'), 'utf8');
    const componentOptions = JSON.parse(readFileSync(join(directory, 'component-options.json'), 'utf8'));
    const retainComponent = (options = componentOptions, source = componentSource, uri = 'memory:component.xslt') =>
        JSON.parse(retainXsltComponent(source, uri, JSON.stringify(options), '["datadom"]'));
    // CONTROL-INPUT-SCOPE-POLICY: environment authority, constrain-only scopes,
    // UTF-8 bytes before decoding, and immutable component-local policy.
    const retainBounded = policy => JSON.parse(retainXsltComponent(componentSource, 'memory:bounded.xslt',
        JSON.stringify(componentOptions), '["datadom"]', JSON.stringify(policy)));
    const control = '{"datadom":{"slices":{"text":"🍒🍋"}}}';
    const controlBytes = new TextEncoder().encode(control).byteLength;
    const bounded = retainBounded({ environment: 16 * 1024 * 1024, scopes: [4096, controlBytes] });
    const roomy = retainBounded({ environment: 16 * 1024 * 1024, scopes: [] });
    try {
        assert.deepEqual(JSON.parse(renderXsltComponent(bounded.artifactId, control, '[]')).diagnostics, []);
        for (const rejected of [control + ' ', '!'.repeat(controlBytes + 1)]) {
            const result = JSON.parse(renderXsltComponent(bounded.artifactId, rejected, '[]'));
            assert.ok(result.diagnostics.some(d => d.code === 'cem.xslt.binding_limit'));
            assert.deepEqual(result.nodes, []);
        }
        assert.deepEqual(JSON.parse(renderXsltComponent(roomy.artifactId, control + ' '.repeat(8 * 1024 * 1024), '[]')).diagnostics, []);
        assert.ok(JSON.parse(renderXsltComponent(roomy.artifactId, control, ' '.repeat(128 * 1024 + 1)))
            .diagnostics.some(d => d.code === 'cem.xslt.binding_limit'));
        for (const policy of [{ environment: 0 }, { environment: 4096, scopes: [0] },
            { environment: 4096, scopes: [4097] }, { environment: 4096, scopes: [1024, 2048] }]) {
            assert.throws(() => retainBounded(policy));
        }
        checks += 9;
    } finally { disposeXsltComponent(bounded.artifactId); disposeXsltComponent(roomy.artifactId); }
    const aspects = JSON.parse(readFileSync(join(directory, 'aspects.json'), 'utf8'));
    const aspectBundle = load(readFileSync(join(directory, 'aspects.bin')), aspects.contentHash, aspects.sourceHash);
    const aspectDefaults = load(readFileSync(join(directory, 'aspects-defaults.bin')), aspects.defaultsContentHash, aspects.defaultsSourceHash);
    const aspectComponent = retainComponent({
        entrypoint: 'viewer-aspects',
        parameters: Object.keys(aspects.cases[0].controls).map(name => ({ name, select: `datadom.slices.${name}` })),
        modules: [{ parentUri: 'memory:data-table-aspects.xslt', href: './data-table-view.xslt',
            uri: 'memory:data-table-view.xslt', source: readFileSync(join(directory, 'viewer.xslt'), 'utf8'), contentHash: aspects.moduleHash }],
    }, readFileSync(join(directory, 'aspects.xslt'), 'utf8'), 'memory:data-table-aspects.xslt');
    try {
        for (const test of aspects.cases) {
            // Missing drafts are absent bindings; JSON null is not an XPath empty sequence.
            const controls = Object.fromEntries(Object.entries(test.controls).filter(([, value]) => value !== null));
            for (const output of [
                JSON.parse(renderXsltBundle(test.controls.ipAddress === null ? aspectDefaults.bundleId : aspectBundle.bundleId, JSON.stringify(controls), '[]')),
                JSON.parse(renderXsltComponent(aspectComponent.artifactId, JSON.stringify({ datadom: { slices: controls } }), '[]')),
            ]) {
                assert.deepEqual(output.diagnostics, []);
                assert.deepEqual(visible(output.nodes), test.nodes);
                checks++;
            }
        }
    } finally {
        disposeXsltComponent(aspectComponent.artifactId);
        disposeXsltBundle(aspectBundle.bundleId);
        disposeXsltBundle(aspectDefaults.bundleId);
    }
    const component = retainComponent();
    const focus = retainCemDocument(new TextEncoder().encode('<input/>'), 'application/xml', 'memory:component-input');
    try {
        assert.deepEqual(component.diagnostics, []);
        for (const value of ['one', 'two', null]) {
            const output = JSON.parse(renderXsltComponent(component.artifactId,
                JSON.stringify({ datadom: { slices: { text: value } } }), '[]', focus));
            assert.deepEqual(output.diagnostics, []);
            assert.equal(text(output.nodes), `${value ?? 'fallback'}:false`);
            checks++;
        }
        for (const value of [[], { value: 'structured control' }]) {
            const output = JSON.parse(renderXsltComponent(component.artifactId,
                JSON.stringify({ datadom: { slices: { text: value } } }), '[]', focus));
            assert.deepEqual(output.nodes, []);
            assert.ok(output.diagnostics.some(d => d.code === 'cem.xslt.scalar_mapping'));
            checks++;
        }
        const missingFocus = JSON.parse(renderXsltComponent(component.artifactId,
            '{"datadom":{"slices":{"text":"scalar"}}}', '[]'));
        assert.deepEqual(missingFocus.diagnostics, []);
        assert.equal(text(missingFocus.nodes), 'scalar:false');
        checks++;
        const importedSource = readFileSync(join(directory, 'component-import.xslt'), 'utf8');
        const importedOptions = JSON.parse(readFileSync(join(directory, 'component-import-options.json'), 'utf8'));
        assert.deepEqual(JSON.parse(xsltStylesheetImports(importedSource, 'memory:component.xslt')), ['./base.xslt']);
        const imported = retainComponent(importedOptions, importedSource);
        try {
            const output = JSON.parse(renderXsltComponent(imported.artifactId, '{}', '[]', focus));
            assert.deepEqual(output.diagnostics, []);
            assert.equal(text(output.nodes), 'imported:false');
            checks++;
        } finally { assert.equal(disposeXsltComponent(imported.artifactId), true); }
        importedOptions.modules[0].source += '<!-- drift -->';
        assert.throws(() => retainComponent(importedOptions, importedSource));
        checks++;
        for (const options of [
            { ...componentOptions, parameters: [{ name: 'missing', select: '1' }] },
            { ...componentOptions, parameters: [...componentOptions.parameters, ...componentOptions.parameters] },
            { ...componentOptions, parameters: [{ name: 'p:text', select: 'unknown +' }] },
            { ...componentOptions, unexpected: true },
        ]) {
            assert.throws(() => retainComponent(options));
            checks++;
        }
        const nativeControl = retainComponent({ ...componentOptions,
            parameters: [{ name: 'p:text', select: 'data:node_key(datadom.slices.response.data)' }] });
        try {
            const output = JSON.parse(renderXsltComponent(nativeControl.artifactId,
                '{"datadom":{"slices":{"response":{"state":"ready"}}}}',
                JSON.stringify([{ slice: 'response', documentId: focus }]), focus));
            assert.deepEqual(output.diagnostics, []);
            assert.ok(text(output.nodes).startsWith('cem-source:1:'));
            checks++;
        } finally { disposeXsltComponent(nativeControl.artifactId); }
        const viewerComponent = retainComponent({ entrypoint: 'viewer', parameters:
            Object.keys(viewer.cases[0].controls).map(name => ({ name, select: `datadom.slices.${name}` })),
        }, readFileSync(join(directory, 'viewer.xslt'), 'utf8'), 'memory:data-table-view.xslt');
        try {
            for (const test of viewer.cases) {
                const output = JSON.parse(renderXsltComponent(viewerComponent.artifactId,
                    JSON.stringify({ datadom: { slices: test.controls } }), '[]'));
                assert.deepEqual(output.diagnostics, []);
                assert.deepEqual(visible(output.nodes), test.nodes);
                assert.deepEqual(selections(output.nodes), test.selections);
                checks++;
            }
        } finally { disposeXsltComponent(viewerComponent.artifactId); }
    } finally { disposeCemDocument(focus); }
    assert.ok(JSON.parse(renderXsltComponent(component.artifactId, '{}', '[]', focus))
        .diagnostics.some(d => d.code === 'cem.xslt.unknown_document'));
    assert.equal(disposeXsltComponent(component.artifactId), true);
    assert.equal(disposeXsltComponent(component.artifactId), false);
    assert.ok(JSON.parse(renderXsltComponent(component.artifactId, '{}', '[]'))
        .diagnostics.some(d => d.code === 'cem.xslt.unknown_component'));
    checks++;
    const componentHandles = [];
    try {
        for (let index = 0; index < 16; index++) componentHandles.push(retainComponent().artifactId);
        assert.throws(() => retainComponent());
        checks++;
    } finally { for (const id of componentHandles) assert.equal(disposeXsltComponent(id), true); }
    const nextComponent = retainComponent();
    assert.ok(nextComponent.artifactId > componentHandles.at(-1));
    assert.equal(disposeXsltComponent(nextComponent.artifactId), true);
    checks++;
    const optionalManifest = JSON.parse(readFileSync(join(directory, 'optional-focus.json'), 'utf8'));
    const optional = load(readFileSync(join(directory, 'optional-focus.bin')), optionalManifest.contentHash, optionalManifest.sourceHash);
    try {
        const output = JSON.parse(renderXsltBundle(optional.bundleId, '{}', '[]'));
        assert.deepEqual(output.diagnostics, []);
        assert.equal(text(output.nodes), 'scalar:2/1/2:4/2/2:absent');
        checks++;
    } finally { disposeXsltBundle(optional.bundleId); }
    // XSLT-OUTPUT-WASM: the JSON below is the explicit render-plan protocol.
    // XML/JSON document bytes are still imported and queried entirely in CEM-ML.
    for (const name of ['native-output', 'avt-output']) {
        const manifest = JSON.parse(readFileSync(join(directory, `${name}.json`), 'utf8'));
        const bytes = readFileSync(join(directory, `${name}.bin`));
        const source = readFileSync(join(directory, `${name}.xslt`), 'utf8');
        assert.deepEqual(Buffer.from(compileXsltBundle(source, 'memory:output.xslt')), bytes);
        checks++;
        for (const retained of [load(bytes, manifest.contentHash, manifest.sourceHash), JSON.parse(retainXsltStylesheet(source, 'memory:output.xslt'))]) {
            try {
                for (const input of name === 'native-output'
                    ? ['<input>&lt;r&gt;&lt;b/&gt;&lt;!--end--&gt;&lt;?test data?&gt;&lt;/r&gt;</input>', '<input>{"a":null,"b":""}</input>']
                    : ['<r>A &amp; B</r>', '<r>changed</r>']) {
                    const document = retainCemDocument(new TextEncoder().encode(input), 'application/xml', 'memory:output-input.xml');
                    try {
                        const output = render(document, '{}', retained.bundleId);
                        assert.deepEqual(output.diagnostics, []);
                        const [root] = output.nodes;
                        if (name === 'native-output') {
                            assert.equal(root.tag, 'main');
                            const [copied] = root.children;
                            assert.ok(copied.sourceMap.frames.length);
                            if (copied.tag === 'r') {
                                assert.deepEqual(copied.children.map(n => n.kind), ['element', 'comment', 'processing-instruction']);
                                assert.equal(copied.children[2].target, 'test');
                                assert.equal(copied.children[2].data, 'data');
                            } else {
                                assert.equal(copied.tag, 'map');
                                assert.equal(copied.namespace, 'http://www.w3.org/2005/xpath-functions');
                                assert.deepEqual(copied.children.map(n => n.tag), ['null', 'string']);
                                assert.equal(copied.children[1].attributes.find(a => a.name === 'key').value, 'b');
                            }
                        } else {
                            assert.equal(root.attributes.find(a => a.name === 'title').value, input.includes('changed') ? '{changed}:1 2' : '{A & B}:1 2');
                            assert.equal(root.attributes.find(a => a.name === 'xml:space').namespaceUri, 'http://www.w3.org/XML/1998/namespace');
                            assert.equal(root.children[0].text, ' ');
                        }
                        checks++;
                    } finally { assert.equal(disposeCemDocument(document), true); }
                }
            } finally { assert.equal(disposeXsltBundle(retained.bundleId), true); }
        }
    }
    for (const [body, code] of [
        ['<p>body<xsl:attribute name="late">bad</xsl:attribute></p>', 'XTDE0410'],
        ['<xsl:sequence select="/*/@a"/>', 'XTDE0420'],
        ['<p><xsl:sequence select="map{}"/></p>', 'XTDE0450'],
    ]) {
        const retained = JSON.parse(retainXsltStylesheet(wrap(body), 'memory:output-error.xslt'));
        const document = retainCemDocument(new TextEncoder().encode('<r a="bad"/>'), 'application/xml', 'memory:input');
        try {
            const output = render(document, '{}', retained.bundleId);
            assert.deepEqual(output.nodes, []);
            assert.ok(output.diagnostics.some(d => d.details?.errorQName?.localName === code && d.uri === 'memory:output-error.xslt'));
            checks++;
        } finally { disposeXsltBundle(retained.bundleId); disposeCemDocument(document); }
    }
    for (const [attributes, code] of [
        ['select="(1, 2)"', 'XTTE1020'],
        ['select="if (position() = 1) then 1 else &quot;a&quot;"', 'XTDE1030'],
        ['order="{/*/@order}"', 'XTDE0030'],
    ]) {
        const retained = JSON.parse(retainXsltStylesheet(wrap(`<p>partial</p><xsl:for-each select="/*/*"><xsl:sort ${attributes}/><b>bad</b></xsl:for-each>`), 'memory:sort-error.xslt'));
        const document = retainCemDocument(new TextEncoder().encode('<r order="sideways"><row/><row/></r>'), 'application/xml', 'memory:sort-input');
        try {
            const output = render(document, '{}', retained.bundleId);
            assert.deepEqual(output.nodes, []);
            assert.ok(output.diagnostics.some(d => d.code === code && d.message.includes('memory:sort-error.xslt')));
            checks++;
        } finally {
            disposeXsltBundle(retained.bundleId);
            disposeCemDocument(document);
        }
    }
    for (const compile of [compileXsltBundle, retainXsltStylesheet]) {
        assert.throws(() => compile(wrap('<xsl:apply-imports/>'), 'memory:unsupported.xslt'), error => {
            const diagnostics = JSON.parse(String(error)).diagnostics;
            assert.ok(diagnostics.some(d => d.code === 'cem.xslt.compile_unsupported'
                && d.uri === 'memory:unsupported.xslt' && d.byteOffset > 0 && d.sourceMap.frames.length));
            return true;
        });
        checks++;
    }
    const failed = JSON.parse(retainXsltStylesheet(wrap('<p>partial<xsl:value-of select="map{1:2}"/></p>'), 'memory:failed.xslt'));
    const failedInput = retainCemDocument(new TextEncoder().encode('<r/>'), 'application/xml', 'memory:input');
    try {
        const output = render(failedInput, '{}', failed.bundleId);
        assert.deepEqual(output.nodes, []);
        assert.ok(output.diagnostics.some(d => d.uri === 'memory:failed.xslt' && d.byteOffset > 0));
        checks++;
    } finally {
        disposeXsltBundle(failed.bundleId);
        disposeCemDocument(failedInput);
    }
    // Native CEM-QL counterparts consume scalar source text; no JS document
    // objects or AST serialization are used to cross the import boundary.
    for (const [query, input, expected] of [
        ['data:parse(source, "xml").children.name', '<r/>', ['r']],
        ['data:parse(source, "json").children.children.value', '"\\uDEAD"', ['�']],
        ['data:parse(source, "json", {escape: true}).children.children.value', '"\\uDEAD"', ['\\udead']],
        ['seq:count(data:parse(source, "json", {duplicates: "use-first"}).children.children)', '{"x":1,"x":2}', [1]],
        ['seq:count(data:parse(source, "csv", {header: "present"}).children.children)', 'fruit\nCherry', [1]],
        ['data:parse(source, "yaml").children.name', 'fruit: Cherry', ['object']],
        ['data:base_uri(data:parse(source, "xml", {"base-uri": "https://example.test/base/"}))', '<r/>', ['https://example.test/base/']],
        ['data:document_uri(data:parse(source, "xml"))', '<r/>', []],
        ['try { data:parse(source, "xml") } catch (code, message) { code }', '<r>', ['cem.ql.import_malformed']],
        ['str:normalize_space(source, "xml")', ' \t Cherry \n ', [' Cherry ']],
    ]) {
        const result = JSON.parse(evaluateQuerySource(query, JSON.stringify({source: input})));
        assert.equal(result.error, null, query);
        assert.deepEqual(result.diagnostics, []);
        assert.deepEqual(result.items.map(item => item.value), expected);
        checks++;
    }
    const excessive = JSON.parse(evaluateQuerySource('try { data:parse(source, "xml") } catch (code, message) { "hidden" }', JSON.stringify({source: ' '.repeat(32769)})));
    assert.ok(excessive.error);
    assert.deepEqual(excessive.items, []);
    assert.ok(excessive.diagnostics.some(d => d.code === 'cem.ql.import_limit'));
    checks++;
    console.log(`XSLT bundle checks passed: ${checks} (native/WASM, shared CEM documents, native output, ownership, focus, grouping, sorting, parsing, recovery, bounds, CEM-QL parity and isolation).`);
} finally {
    rmSync(directory, { recursive: true, force: true });
}
