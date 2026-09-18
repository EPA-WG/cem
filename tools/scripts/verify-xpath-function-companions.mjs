// XPATH-DEMO-COMPANION-WASM: control metadata and opaque programs, not data ASTs.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import init, {
    compileCemtXPathFunctions, retainCemtXPathFunctions, importCemtXPathFunctions, disposeCemtXPathFunctions,
    compileTemplate, compileTemplateArtifact, importTemplateArtifact,
    renderTemplate, renderTemplateWithXPathFunctions, disposeTemplate,
} from '../../packages/cem_ql/dist/wasm/cem_ql.js';

const root = fileURLToPath(new URL('../../', import.meta.url));
const directory = mkdtempSync(join(tmpdir(), 'cem-xpath-companions-'));
try {
    execFileSync('cargo', ['test', '-p', 'cem-ql', '--test', 'xpath_function_companion',
        'deterministic_companion_reloads_without_source_and_rebinds_only_explicitly', '--', '--exact'], {
        cwd: root, stdio: 'inherit', env: { ...process.env, CEM_XPATH_COMPANION_FIXTURE_DIR: directory },
    });
    await init({ module_or_path: readFileSync(join(root, 'packages/cem_ql/dist/wasm/cem_ql_bg.wasm')) });
    const fixture = JSON.parse(readFileSync(join(directory, 'fixture.json'), 'utf8'));
    const nativeBytes = readFileSync(join(directory, 'companion.bin'));
    const wasmBytes = compileCemtXPathFunctions(fixture.source, fixture.sourceUri);
    assert.deepEqual(Buffer.from(wasmBytes), nativeBytes);
    let checks = 1;
    const load = (bytes = wasmBytes, hash = fixture.contentHash, sourceHash = fixture.sourceHash) =>
        JSON.parse(importCemtXPathFunctions(bytes, hash, sourceHash));
    const imported = load();
    assert.equal(imported.contentType, fixture.contentType);
    assert.equal(imported.formatVersion, 'cemt-xpath-functions/1');
    checks++;
    const fromSource = JSON.parse(retainCemtXPathFunctions(fixture.source, fixture.sourceUri));
    assert.equal(fromSource.contentHash, fixture.contentHash);
    assert.equal(fromSource.sourceHash, fixture.sourceHash);
    checks++;
    const templateSource = `{module |
        {template @mode=fruit @match='native:call("demo.accept", node)' |
            {body | {b | {$native:call("demo.label", text)}}}}
        {template @mode=fruit @match=true @priority=-10 | {body | {i | 🍋}}}
        {body | {apply-templates @mode=fruit @select=quantity}}
    }`;
    const bindings = '["text","quantity"]';
    const template = JSON.parse(compileTemplate(templateSource, bindings));
    assert.deepEqual(template.diagnostics, []);
    const portable = compileTemplateArtifact(templateSource, bindings, 'dev');
    const reloaded = JSON.parse(importTemplateArtifact(portable, '', templateSource, bindings, 'dev'));
    assert.deepEqual(reloaded.diagnostics, []);
    const html = nodes => nodes.map(node => node.kind === 'text' ? node.text :
        `<${node.tag}>${html(node.children)}</${node.tag}>`).join('');
    const hasCode = (result, code) => result.diagnostics.some(d => d.code === code);
    try {
        for (const artifactId of [template.artifactId, reloaded.artifactId]) {
            for (const [quantity, text, expected] of [[1, 'one', '<i>🍋</i>'], [2, 'two', '<b>two 🍒</b>'], [3, 'changed', '<b>changed 🍒</b>']]) {
                const data = JSON.stringify({ quantity, text });
                const result = JSON.parse(renderTemplateWithXPathFunctions(artifactId, imported.companionId, data));
                assert.deepEqual(result.diagnostics, []);
                assert.equal(html(result.nodes), expected);
                checks++;
            }
            // Explicit companion rendering must not mutate this template's default path.
            const defaultResult = JSON.parse(renderTemplate(artifactId, JSON.stringify({
                quantity: 2, text: 'x', companionId: imported.companionId,
                native_functions: { 'demo.label': imported.companionId },
            })));
            assert.ok(hasCode(defaultResult, 'cem.ql.native_function_unavailable'));
            checks++;
        }
        const sourceLoaded = JSON.parse(renderTemplateWithXPathFunctions(template.artifactId, fromSource.companionId,
            JSON.stringify({ quantity: 2, text: 'source' })));
        assert.deepEqual(sourceLoaded.diagnostics, []);
        assert.equal(html(sourceLoaded.nodes), '<b>source 🍒</b>');
        checks++;
        const wrongType = JSON.parse(renderTemplateWithXPathFunctions(template.artifactId, imported.companionId,
            JSON.stringify({ quantity: 2, text: { value: 'no implicit conversion' } })));
        assert.ok(hasCode(wrongType, 'cem.ql.xpath_function_argument'));
        checks++;
        const unknownTemplate = JSON.parse(renderTemplateWithXPathFunctions(0, imported.companionId, '{}'));
        assert.ok(hasCode(unknownTemplate, 'cem.ql.wasm.unknown_artifact'));
        checks++;
        const corrupt = Buffer.from(wasmBytes); corrupt[corrupt.length - 1] ^= 1;
        for (const attempt of [
            () => load(corrupt), () => load(wasmBytes, 'not-a-hash'),
            () => load(wasmBytes, fixture.sourceHash), () => load(wasmBytes, fixture.contentHash, fixture.contentHash),
            () => load(new Uint8Array(4 * 1024 * 1024 + 1)),
        ]) { assert.throws(attempt, /cem\.ql\.xpath_companion_(identity_mismatch|limit)/); checks++; }
        assert.throws(() => compileCemtXPathFunctions(fixture.source.replace('$text ||', '$text || ||'), fixture.sourceUri));
        checks++;
    } finally {
        assert.equal(disposeTemplate(template.artifactId), true);
        assert.equal(disposeTemplate(reloaded.artifactId), true);
        assert.equal(disposeCemtXPathFunctions(imported.companionId), true);
        assert.equal(disposeCemtXPathFunctions(fromSource.companionId), true);
    }
    const stale = JSON.parse(renderTemplateWithXPathFunctions(0, imported.companionId, '{}'));
    assert.ok(stale.diagnostics.some(d => d.code === 'cem.ql.wasm.unknown_xpath_companion'));
    assert.equal(disposeCemtXPathFunctions(imported.companionId), false);
    checks++;
    const retained = Array.from({ length: 64 }, () => load().companionId);
    assert.throws(() => load(), /cem\.ql\.xpath_companion_limit/);
    for (const id of retained) assert.equal(disposeCemtXPathFunctions(id), true);
    const next = load().companionId;
    assert.ok(next > retained[retained.length - 1]);
    assert.equal(disposeCemtXPathFunctions(next), true);
    checks++;
    // The explicit reader creates and retains XML owners inside WASM. JSON
    // carries source bytes only; records cannot masquerade as native nodes.
    const xmlLibrary = readFileSync(join(root, 'packages/cem-elements/demo/xpath-functions.cemt'), 'utf8');
    const xmlCompanion = JSON.parse(retainCemtXPathFunctions(xmlLibrary, 'memory:xml-functions.cemt'));
    const xmlTemplate = `{module |
        {template @mode=stock @match='native:call("demo.stocked", node)' | {body | {b | {$node}}}}
        {template @mode=stock @match=true @priority=-10 | {body | {i | {$node}}}}
        {body | {cem-data @name=document @select=source @type=xml @projection=xpath}
            {cem:choose |
                {cem:when @test='document.error != ""' | {p | Invalid XML}}
                {cem:otherwise | {apply-templates @mode=stock @select='native:call("demo.items", document.root)'}}
    }   }   }`;
    const xmlBindings = '["source"]';
    const xmlCompiled = JSON.parse(compileTemplate(xmlTemplate, xmlBindings));
    const xmlReloaded = JSON.parse(importTemplateArtifact(
        compileTemplateArtifact(xmlTemplate, xmlBindings, 'dev'), '', xmlTemplate, xmlBindings, 'dev'));
    try {
        for (const artifact of [xmlCompiled, xmlReloaded]) {
            assert.deepEqual(artifact.diagnostics, []);
            for (const [source, expected] of [
                ['<r><item qty="1">Lemon</item><item qty="2">Cherry</item></r>', '<i>Lemon</i><b>Cherry</b>'],
                ['<r><item qty="3">Grape</item></r>', '<b>Grape</b>'],
                ['<r>', '<p>Invalid XML</p>'],
                ['<r><item qty="3">Grape</item></r>', '<b>Grape</b>'],
                ['<r xmlns="urn:other"><item qty="3">Wrong namespace</item></r>', ''],
                [{ kind: 'document', children: [] }, '<p>Invalid XML</p>'],
            ]) {
                const result = JSON.parse(renderTemplateWithXPathFunctions(artifact.artifactId,
                    xmlCompanion.companionId, JSON.stringify({ source })));
                assert.deepEqual(result.diagnostics, []);
                assert.equal(html(result.nodes).trim(), expected);
                checks++;
            }
            const unbound = JSON.parse(renderTemplate(artifact.artifactId,
                JSON.stringify({ source: '<r><item>unbound</item></r>' })));
            assert.ok(hasCode(unbound, 'cem.ql.native_function_unavailable'));
            checks++;
        }
    } finally {
        assert.equal(disposeTemplate(xmlCompiled.artifactId), true);
        assert.equal(disposeTemplate(xmlReloaded.artifactId), true);
        assert.equal(disposeCemtXPathFunctions(xmlCompanion.companionId), true);
    }
    assert.ok(hasCode(JSON.parse(renderTemplate(xmlCompiled.artifactId, '{}')), 'cem.ql.wasm.unknown_artifact'));
    checks++;
    const nodeLibrary = readFileSync(join(root, 'packages/cem-elements/demo/xpath-nodes.cemt'), 'utf8');
    const nodeCompanion = JSON.parse(retainCemtXPathFunctions(nodeLibrary, 'memory:nodes.cemt'));
    const nodeTemplate = `{cem-data @name=document @select=source @type=xml @projection=xpath}
{cem:for-each @as=row @select='seq:sorted(native:call("node.rows", document.root), fn(row) => native:call("node.value", row), "ascending", "text")' |
{p | {$native:call("node.local", row)}|{$native:call("node.namespace", row)}|{$native:call("node.value", row)}|{$native:call("node.previous", row)}|{$native:call("node.parent", row)}}}`;
    const nodeCompiled = JSON.parse(compileTemplate(nodeTemplate, '["source"]'));
    const nodeReloaded = JSON.parse(importTemplateArtifact(
        compileTemplateArtifact(nodeTemplate, '["source"]', 'dev'), '', nodeTemplate, '["source"]', 'dev'));
    try {
        for (const artifact of [nodeCompiled, nodeReloaded]) {
            assert.deepEqual(artifact.diagnostics, []);
            for (const [source, expected] of [
                ['<r xmlns:a="urn:a" xmlns:b="urn:b"><a:item>Z<![CDATA[est]]></a:item><b:item>Apple</b:item></r>',
                    '<p>item|urn:b|Apple|Zest|r</p><p>item|urn:a|Zest||r</p>'],
                ['<r><item>Cherry</item></r>', '<p>item||Cherry||r</p>'],
            ]) {
                const result = JSON.parse(renderTemplateWithXPathFunctions(artifact.artifactId,
                    nodeCompanion.companionId, JSON.stringify({ source })));
                assert.deepEqual(result.diagnostics, []);
                assert.equal(html(result.nodes).replace(/>\s+</gu, '><').trim(), expected);
                checks++;
            }
        }
    } finally {
        disposeTemplate(nodeCompiled.artifactId);
        disposeTemplate(nodeReloaded.artifactId);
        disposeCemtXPathFunctions(nodeCompanion.companionId);
    }
    const sequenceLibrary = readFileSync(join(root, 'packages/cem-elements/demo/xpath-sequences.cemt'), 'utf8');
    const sequenceCompanion = JSON.parse(retainCemtXPathFunctions(sequenceLibrary, 'memory:sequences.cemt'));
    const sequenceTemplate = `{cem:variable @name=window @select='native:call("sequence.window", text, start, length, descending)'}
{p | {$native:call("sequence.unique", text)}|{$str:concat(window, " / ")}|{$native:call("sequence.head", window)}|{$native:call("sequence.tail", window)}}
{cem-data @name=document @select=source @type=xml @projection=xpath}
{cem:variable @name=columns @select='native:call("table.columns", document.root)'}
{h1 | {cem:for-each @as=column @select=columns | {b | {$native:call("table.heading", column)}}}}
{cem:for-each @as=row @select='native:call("table.rows", document.root)' |
{p | {cem:for-each @as=column @select=columns | {i | {$native:call("table.value", native:call("table.cells", row, column))}}}}}`;
    const sequenceBindings = '["text","start","length","descending","source"]';
    const sequenceCompiled = JSON.parse(compileTemplate(sequenceTemplate, sequenceBindings));
    const sequenceReloaded = JSON.parse(importTemplateArtifact(
        compileTemplateArtifact(sequenceTemplate, sequenceBindings, 'dev'), '', sequenceTemplate, sequenceBindings, 'dev'));
    try {
        for (const artifact of [sequenceCompiled, sequenceReloaded]) {
            assert.deepEqual(artifact.diagnostics, []);
            for (const [data, expected] of [
                [{ text: 'cherry apple cherry pear', start: '2', length: '2', descending: false,
                    source: '<r><row id="2"><fruit>Apple</fruit></row><row id="1" qty="3"><fruit>Cherry</fruit><note/></row></r>' },
                '<p>3|apple / cherry|apple|cherry</p><h1><b>@id</b><b>fruit</b><b>@qty</b><b>note</b></h1><p><i>2</i><i>Apple</i><i>∅</i><i>∅</i></p><p><i>1</i><i>Cherry</i><i>3</i><i>""</i></p>'],
                [{ text: '🍒 a a b', start: '2.5', length: '1.5', descending: true,
                    source: '<r xmlns:x="urn:x"><row id="1"><fruit>A</fruit><x:fruit>B</x:fruit><fruit>C</fruit></row><row extra="new"><fruit>D</fruit></row></r>' },
                '<p>3|b / a|b|a</p><h1><b>@id</b><b>fruit</b><b>fruit [urn:x]</b><b>@extra</b></h1><p><i>1</i><i>A / C</i><i>B</i><i>∅</i></p><p><i>∅</i><i>D</i><i>∅</i><i>new</i></p>'],
                [{ text: '', start: '1', length: 'INF', descending: true, source: '<r/>' }, '<p>0|||</p><h1></h1>'],
            ]) {
                const result = JSON.parse(renderTemplateWithXPathFunctions(artifact.artifactId,
                    sequenceCompanion.companionId, JSON.stringify(data)));
                assert.deepEqual(result.diagnostics, []);
                assert.equal(html(result.nodes).replace(/>\s+</gu, '><').trim(), expected);
                checks++;
            }
        }
    } finally {
        disposeTemplate(sequenceCompiled.artifactId);
        disposeTemplate(sequenceReloaded.artifactId);
        disposeCemtXPathFunctions(sequenceCompanion.companionId);
    }
    const sortLibrary = readFileSync(join(root, 'packages/cem-elements/demo/xpath-sort.cemt'), 'utf8');
    const sortCompanion = JSON.parse(retainCemtXPathFunctions(sortLibrary, 'memory:sort.cemt'));
    const sortTemplate = `{cem-data @name=document @select=source @type=xml @projection=xpath}
{p | {$str:concat(native:call("sort.words", text, true, descending), " / ")}}
{cem:for-each @as=row @select='native:call("sort.rows", document.root, descending)' |
    {b | {$native:call("row.label", row)}}}`;
    const sortBindings = '["source","text","descending"]';
    const sortCompiled = JSON.parse(compileTemplate(sortTemplate, sortBindings));
    const sortReloaded = JSON.parse(importTemplateArtifact(
        compileTemplateArtifact(sortTemplate, sortBindings, 'dev'), '', sortTemplate, sortBindings, 'dev'));
    try {
        for (const artifact of [sortCompiled, sortReloaded]) {
            assert.deepEqual(artifact.diagnostics, []);
            for (const descending of [false, true]) {
                const result = JSON.parse(renderTemplateWithXPathFunctions(artifact.artifactId,
                    sortCompanion.companionId, JSON.stringify({
                        source: '<r><row id="a" group="A" qty="10">A</row><row id="b" group="A" qty="2">B</row><row id="c" group="A" qty="2">C</row><row id="d">D</row></r>',
                        text: '10 2 02 bad 1', descending,
                    })));
                assert.deepEqual(result.diagnostics, []);
                assert.equal(html(result.nodes).replace(/>\s+</gu, '><').trim(), descending
                    ? '<p>10 / 2 / 02 / 1 / bad</p><b>A</b><b>B</b><b>C</b><b>D</b>'
                    : '<p>1 / 2 / 02 / 10 / bad</p><b>B</b><b>C</b><b>A</b><b>D</b>');
                checks++;
            }
        }
    } finally {
        disposeTemplate(sortCompiled.artifactId);
        disposeTemplate(sortReloaded.artifactId);
        disposeCemtXPathFunctions(sortCompanion.companionId);
    }
    const validationLibrary = readFileSync(join(root, 'packages/cem-elements/demo/xpath-validation.cemt'), 'utf8');
    const validationCompanion = JSON.parse(retainCemtXPathFunctions(validationLibrary, 'memory:validation.cemt'));
    const validationImported = JSON.parse(importCemtXPathFunctions(
        compileCemtXPathFunctions(validationLibrary, 'memory:validation.cemt'),
        validationCompanion.contentHash, validationCompanion.sourceHash));
    const validationTemplate = '{p | {$native:call("form.tags", text)}}{b | {$native:call("ip.preview", address, prefixes)}}';
    const validationBindings = '["text","address","prefixes"]';
    const validationCompiled = JSON.parse(compileTemplate(validationTemplate, validationBindings));
    const validationReloaded = JSON.parse(importTemplateArtifact(
        compileTemplateArtifact(validationTemplate, validationBindings, 'dev'), '', validationTemplate, validationBindings, 'dev'));
    try {
        for (const companion of [validationCompanion, validationImported]) {
            for (const artifact of [validationCompiled, validationReloaded]) {
                assert.deepEqual(artifact.diagnostics, []);
                for (const [text, address, prefixes, tags, verdict] of [
                    ['Blue, green; RED', '192.0.2.10/24', '24 32', 'Blue / green / RED', 'Allowed by the local prefix rule'],
                    ['Red; GOLD', '192.0.2.10/16', '24 32', 'Red / GOLD', 'Blocked by the local prefix rule'],
                    ['a,b', '256.0.2.10/24', '24', 'a / b', 'Octets must be 0–255 and prefix length 0–32'],
                    ['a,b', '192.00.2.10/24', '24', 'a / b', 'Enter IPv4 with an optional /prefix; no leading zeros'],
                ]) {
                    const result = JSON.parse(renderTemplateWithXPathFunctions(artifact.artifactId,
                        companion.companionId, JSON.stringify({ text, address, prefixes })));
                    assert.deepEqual(result.diagnostics, []);
                    assert.equal(html(result.nodes).replace(/>\s+</gu, '><').trim(),
                        '<p>' + tags + '</p><b>' + verdict + '</b>');
                    checks++;
                }
            }
        }
    } finally {
        disposeTemplate(validationCompiled.artifactId);
        disposeTemplate(validationReloaded.artifactId);
        disposeCemtXPathFunctions(validationCompanion.companionId);
        disposeCemtXPathFunctions(validationImported.companionId);
    }
    const regexSource = '@doc cem-ml 1\n@ns t = "https://cem.dev/ns/transform/cem/1"\n@default t\n' +
        '{module | {function @name=test.regex @visibility=public @returns=string |' +
        '{param @name=pattern @type=string @required=true}{param @name=flags @type=string @required=true}' +
        '{param @name=replacement @type=string @required=true}' +
        '{body | {xpath @sequence-type="xs:string" |' +
        '{variable @binding=pattern @local-name=pattern}{variable @binding=flags @local-name=flags}' +
        '{variable @binding=replacement @local-name=replacement}' +
        '{expression | replace("aa", $pattern, $replacement, $flags)} } } } }';
    const regexCompanion = JSON.parse(retainCemtXPathFunctions(regexSource, 'memory:regex.cemt'));
    const regexTemplate = JSON.parse(compileTemplate(
        '{p | {$native:call("test.regex", pattern, flags, replacement)}}', '["pattern","flags","replacement"]'));
    try {
        assert.deepEqual(regexTemplate.diagnostics, []);
        for (const [pattern, flags, replacement, code] of [
            ['a', 'z', 'x', 'cem.xpath.regex_flags'],
            ['(?i)a', '', 'x', 'cem.xpath.regex_pattern'],
            ['(a)\\1', '', 'x', 'cem.xpath.regex_unsupported'],
            ['a*', '', 'x', 'cem.xpath.regex_empty_match'],
            ['b', '', '$', 'cem.xpath.regex_replacement'],
            ['a{1025}', '', 'x', 'cem.xpath.regex_limit_exceeded'],
        ]) {
            const result = JSON.parse(renderTemplateWithXPathFunctions(regexTemplate.artifactId,
                regexCompanion.companionId, JSON.stringify({ pattern, flags, replacement })));
            assert.ok(hasCode(result, code), JSON.stringify(result));
            checks++;
        }
    } finally {
        disposeTemplate(regexTemplate.artifactId);
        disposeCemtXPathFunctions(regexCompanion.companionId);
    }
    const aggregateLibrary = readFileSync(join(root, 'packages/cem-elements/demo/xpath-aggregates.cemt'), 'utf8');
    const aggregateCompanion = JSON.parse(retainCemtXPathFunctions(aggregateLibrary, 'memory:aggregates.cemt'));
    const aggregateTemplate = `{cem-data @name=document @select=source @type=xml @projection=xpath}
{cem:choose |
{cem:when @test='document.error != ""' | {p | Invalid XML}}
{cem:when @test='native:call("basket.valid", document.root)' |
{cem:variable @name=values @select='native:call("basket.values", document.root)'}
{p | {$native:call("aggregate.sum", values)}|{$native:call("aggregate.min", values)}|{$native:call("aggregate.max", values)}|{$native:call("aggregate.avg", values)}}}
{cem:otherwise | {p | Invalid amount}}}`;
    const aggregateCompiled = JSON.parse(compileTemplate(aggregateTemplate, '["source"]'));
    const aggregateReloaded = JSON.parse(importTemplateArtifact(
        compileTemplateArtifact(aggregateTemplate, '["source"]', 'dev'), '', aggregateTemplate, '["source"]', 'dev'));
    try {
        for (const artifact of [aggregateCompiled, aggregateReloaded]) {
            assert.deepEqual(artifact.diagnostics, []);
            for (const [source, expected] of [
                ['<basket><apple>0.1</apple><cherry>0.2</cherry></basket>', '0.3|0.1|0.2|0.15'],
                ['<basket><apple>0.1</apple><cherry>0.2</cherry><pear>0.6</pear></basket>', '0.9|0.1|0.6|0.3'],
                ['<basket><apple>0</apple><cherry>0</cherry><pear>1</pear></basket>', '1|0|1|0.333333333333333333'],
                ['<basket/>', '0|∅|∅|∅'],
                ['<basket><pear>bad</pear></basket>', 'Invalid amount'],
                ['<basket><pear>-0.1</pear></basket>', 'Invalid amount'],
                ['<basket>', 'Invalid XML'],
                ['<basket><plum>7</plum></basket>', '7|7|7|7'],
            ]) {
                const result = JSON.parse(renderTemplateWithXPathFunctions(artifact.artifactId,
                    aggregateCompanion.companionId, JSON.stringify({ source })));
                assert.deepEqual(result.diagnostics, []);
                assert.equal(html(result.nodes).trim(), `<p>${expected}</p>`);
                checks++;
            }
        }
    } finally {
        disposeTemplate(aggregateCompiled.artifactId);
        disposeTemplate(aggregateReloaded.artifactId);
        disposeCemtXPathFunctions(aggregateCompanion.companionId);
    }
    const containerLibrary = readFileSync(join(root, 'packages/cem-elements/demo/xpath-maps-arrays.cemt'), 'utf8');
    const containerCompanion = JSON.parse(retainCemtXPathFunctions(containerLibrary, 'memory:containers.cemt'));
    const containerTemplate = `{cem-data @name=document @select=source @type=xml @projection=xpath}
{cem:choose |
{cem:when @test='document.error != ""' | {p | Invalid XML}}
{cem:otherwise |
{cem:variable @name=basket @select='native:call("basket.pack", document.root)'}
{cem:variable @name=filter @select='native:call("filter.make", "192.0.2.0/24", "allow", note)'}
{p | {$native:call("basket.size", basket)}|{$native:call("basket.label", native:call("basket.pick", basket, position))}|{$native:call("basket.note", basket)}|{$native:call("filter.note", filter)}|{$native:call("filter.count", filter)}}}}`;
    const containerBindings = '["source","position","note"]';
    const containerCompiled = JSON.parse(compileTemplate(containerTemplate, containerBindings));
    const containerReloaded = JSON.parse(importTemplateArtifact(
        compileTemplateArtifact(containerTemplate, containerBindings, 'dev'), '', containerTemplate, containerBindings, 'dev'));
    try {
        for (const artifact of [containerCompiled, containerReloaded]) {
            assert.deepEqual(artifact.diagnostics, []);
            for (const [source, position, note, expected] of [
                ['<basket><apple>2</apple><pear>3</pear></basket>', '1', 'absent', '2|apple: 2|Empty member (array size 1)|Absent entry|2'],
                ['<basket><apple>2</apple><pear>3</pear></basket>', '2', 'empty', '2|pear: 3|Empty member (array size 1)|Present, empty sequence|3'],
                ['<basket note="Fresh"><plum>4</plum></basket>', '1', 'value', '1|plum: 4|Note: Fresh|Local preview|3'],
                ['<basket note=""/>', '1', 'absent', '0|No member at this position|Note: |Absent entry|2'],
                ['<basket><pear>3</pear></basket>', '0', 'empty', '1|No member at this position|Empty member (array size 1)|Present, empty sequence|3'],
                ['<basket><pear>3</pear></basket>', 'bad', 'absent', '1|No member at this position|Empty member (array size 1)|Absent entry|2'],
                ['<basket>', '1', 'value', 'Invalid XML'],
                ['<basket><cherry>5</cherry></basket>', '1', 'absent', '1|cherry: 5|Empty member (array size 1)|Absent entry|2'],
            ]) {
                const result = JSON.parse(renderTemplateWithXPathFunctions(artifact.artifactId,
                    containerCompanion.companionId, JSON.stringify({ source, position, note })));
                assert.deepEqual(result.diagnostics, []);
                assert.equal(html(result.nodes).trim(), `<p>${expected}</p>`);
                checks++;
            }
        }
    } finally {
        disposeTemplate(containerCompiled.artifactId);
        disposeTemplate(containerReloaded.artifactId);
        disposeCemtXPathFunctions(containerCompanion.companionId);
    }
    const treeLibrary = `@doc cem-ml 1
@ns t = "https://cem.dev/ns/transform/cem/1"
@default t
{module | {function @name=tree.text @visibility=public @returns=string |
{param @name=document @type=any @required=true}
{body | {xpath @context=document @sequence-type="xs:string" |
{expression | string(.)}
}}}}`;
    const treeCompanion = JSON.parse(retainCemtXPathFunctions(treeLibrary, 'memory:tree.cemt'));
    const treeTemplate = `{cem-data @name=document @select=source @type="{$format}" @projection="{$projection}"}
{p | {$native:call("tree.text", document.root)}}`;
    const treeBindings = '["source","format","projection"]';
    const treeCompiled = JSON.parse(compileTemplate(treeTemplate, treeBindings));
    const treeReloaded = JSON.parse(importTemplateArtifact(
        compileTemplateArtifact(treeTemplate, treeBindings, 'dev'), '', treeTemplate, treeBindings, 'dev'));
    try {
        assert.ok(Number.isInteger(treeCompanion.companionId));
        for (const artifact of [treeCompiled, treeReloaded]) {
            assert.deepEqual(artifact.diagnostics, []);
            for (const [source, format, projection, expected] of [
                ['<r>3<![CDATA[ 🍒]]></r>', 'xml', 'cem', '3 🍒'],
                ['{"qty":3,"note":null}', 'json', 'cem', '3'],
                ['{"qty":3,"note":null}', 'json', 'json-to-xml', '3'],
                ['qty: 3\nnote: null\n', 'yaml', 'cem', '3'],
                ['qty,note\n3,hello', 'csv', 'cem', '3hello'],
                ['{"qty":4}', 'json', 'cem', '4'],
            ]) {
                const result = JSON.parse(renderTemplateWithXPathFunctions(artifact.artifactId,
                    treeCompanion.companionId, JSON.stringify({ source, format, projection })));
                assert.deepEqual(result.diagnostics, []);
                assert.equal(html(result.nodes).trim(), `<p>${expected}</p>`);
                checks++;
            }
        }
    } finally {
        disposeTemplate(treeCompiled.artifactId);
        disposeTemplate(treeReloaded.artifactId);
        disposeCemtXPathFunctions(treeCompanion.companionId);
    }
    const textLibrary = readFileSync(join(root, 'packages/cem-elements/demo/xpath-text.cemt'), 'utf8');
    const textCompanion = JSON.parse(retainCemtXPathFunctions(textLibrary, 'memory:text.cemt'));
    const textTemplate = JSON.parse(compileTemplate('{p |' + [
        '{$native:call("text.words", text)}',
        '/{$native:call("text.length", text)}',
        '/{$native:call("text.normalize", text)}',
        '/{$native:call("text.join", text, "/")}',
    ].join('') + '}', '["text"]'));
    assert.deepEqual(textTemplate.diagnostics, []);
    try {
        for (const [text, expected] of [
            ['a\ta\nb', '<p>3/5/a a b/a/a/b</p>'],
            ['', '<p>0/0//</p>'],
            ['a\u00a0b', '<p>1/3/a\u00a0b/a\u00a0b</p>'],
            ['🍒e\u0301', '<p>1/3/🍒e\u0301/🍒e\u0301</p>'],
        ]) {
            const result = JSON.parse(renderTemplateWithXPathFunctions(textTemplate.artifactId,
                textCompanion.companionId, JSON.stringify({ text })));
            assert.deepEqual(result.diagnostics, []);
            assert.equal(html(result.nodes), expected);
            checks++;
        }
    } finally {
        disposeTemplate(textTemplate.artifactId);
        disposeCemtXPathFunctions(textCompanion.companionId);
    }
    const boundedLibrary = textLibrary.replace('normalize-space($text)',
        'normalize-space('.repeat(8) + '$text' + ')'.repeat(8));
    const boundedCompanion = JSON.parse(retainCemtXPathFunctions(boundedLibrary, 'memory:bounded-text.cemt'));
    const boundedTemplate = JSON.parse(compileTemplate(
        '{p | {$try { native:call("text.normalize", text) } catch (code, message) { "caught" }}}', '["text"]'));
    assert.deepEqual(boundedTemplate.diagnostics, []);
    try {
        for (const [length, code] of [
            [1024 * 1024 + 1, 'cem.xpath.text_byte_limit_exceeded'],
            [750_000, 'cem.xpath.work_limit_exceeded'],
        ]) {
            const result = JSON.parse(renderTemplateWithXPathFunctions(boundedTemplate.artifactId,
                boundedCompanion.companionId, JSON.stringify({ text: 'a'.repeat(length) })));
            assert.ok(hasCode(result, code), JSON.stringify(result.diagnostics));
            assert.ok(!html(result.nodes).includes('caught'));
            checks++;
        }
    } finally {
        disposeTemplate(boundedTemplate.artifactId);
        disposeCemtXPathFunctions(boundedCompanion.companionId);
    }
    const fence = '```';
    const recursiveBody = `let $f := function($self) { ${'('.repeat(8)}$self($self)${')'.repeat(8)} } return $f($f)`;
    const recursiveLibrary = [
        '@doc cem-ml 1', '@ns t = "https://cem.dev/ns/transform/cem/1"', '@default t',
        '{module | {function @name=test.recurse @visibility=public @returns=any |',
        '{body | {xpath @sequence-type="item()*" |',
        `{expression | ${fence}${recursiveBody}${fence}}`, '} } }',
        '{function @name=test.emptyNamespace @visibility=public @returns=integer |',
        '{body | {xpath @sequence-type="xs:integer" |',
        `{expression | ${fence}(function($Q{}x) {$x})(3)${fence}}`, '} } } }',
    ].join('\n');
    const recursiveCompanion = JSON.parse(retainCemtXPathFunctions(recursiveLibrary, 'memory:recursive.cemt'));
    const recursiveTemplate = JSON.parse(compileTemplate('{$native:call("test.recurse")}', '[]'));
    const namespaceTemplate = JSON.parse(compileTemplate('{$native:call("test.emptyNamespace")}', '[]'));
    try {
        assert.deepEqual(recursiveTemplate.diagnostics, []);
        const result = JSON.parse(renderTemplateWithXPathFunctions(recursiveTemplate.artifactId,
            recursiveCompanion.companionId, '{}'));
        assert.ok(hasCode(result, 'cem.xpath.function_depth_exceeded'));
        const namespaceResult = JSON.parse(renderTemplateWithXPathFunctions(namespaceTemplate.artifactId,
            recursiveCompanion.companionId, '{}'));
        assert.deepEqual(namespaceResult.diagnostics, []);
        assert.equal(html(namespaceResult.nodes).trim(), '3');
        checks += 2;
    } finally {
        disposeTemplate(recursiveTemplate.artifactId);
        disposeTemplate(namespaceTemplate.artifactId);
        disposeCemtXPathFunctions(recursiveCompanion.companionId);
    }
    console.log(`XPath function companion checks passed: ${checks} (native/WASM parity, matching, reload, nodes, sequences, aggregates, maps/arrays, inline calls, sort, regex, text, limits and isolation).`);
} finally {
    // Only the exact temporary fixture directory created above is removed.
    rmSync(directory, { recursive: true, force: true });
}
