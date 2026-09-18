// XPath-owned executable bytes must cross native/WASM without source reparsing.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import init, {
    compileXPathArtifact, importXPathArtifact, disposeXPathArtifact,
    compileTemplate, renderTemplate, disposeTemplate,
} from '../../packages/cem_ql/dist/wasm/cem_ql.js';

const root = fileURLToPath(new URL('../../', import.meta.url));
const fixtureDirectory = mkdtempSync(join(tmpdir(), 'cem-xpath-artifacts-'));
try {
    execFileSync('cargo', [
        'test', '-p', 'cem-ml', '--test', 'xpath_compiled_artifact',
        'deterministic_binary_reload_retains_typed_host_without_xpath_source', '--', '--exact',
    ], {
        cwd: root, stdio: 'inherit',
        env: { ...process.env, CEM_XPATH_ARTIFACT_FIXTURE_DIR: fixtureDirectory },
    });
    await init({ module_or_path: readFileSync(join(root, 'packages/cem_ql/dist/wasm/cem_ql_bg.wasm')) });
    const fixtures = JSON.parse(readFileSync(join(fixtureDirectory, 'manifest.json'), 'utf8'));
    const load = (fixture, bytes = readFileSync(join(fixtureDirectory, `${fixture.name}.bin`))) =>
        JSON.parse(importXPathArtifact(bytes, fixture.contentHash, fixture.sourceHash, fixture.host));
    const [xslt, standalone, containers, inline] = fixtures;
    let checks = 0;
    for (const fixture of fixtures) {
        const imported = load(fixture);
        assert.equal(imported.contentType, 'application/vnd.cem.xpath-artifact+cem-bin');
        assert.equal(imported.schemaUri, 'https://cem.dev/ns/query/xpath/1');
        assert.equal(imported.invocationHost, fixture.host);
        assert.equal(disposeXPathArtifact(imported.artifactId), true);
        assert.equal(disposeXPathArtifact(imported.artifactId), false);
        checks++;
    }
    const wasmBytes = compileXPathArtifact('1 + 2', 'memory:expression.xpath');
    assert.deepEqual(Buffer.from(wasmBytes), readFileSync(join(fixtureDirectory, 'standalone.bin')));
    assert.equal(disposeXPathArtifact(load(standalone, wasmBytes).artifactId), true);
    checks++;
    const containerBytes = compileXPathArtifact("let $m := map {'rows': [(),(1,2)]} return ($m?rows?(2), $m ! ?rows?*)", 'memory:expression.xpath');
    assert.deepEqual(Buffer.from(containerBytes), readFileSync(join(fixtureDirectory, 'containers.bin')));
    assert.equal(disposeXPathArtifact(load(containers, containerBytes).artifactId), true);
    checks++;
    const inlineBytes = compileXPathArtifact('sort((3,1,2), (), function($x as xs:integer) as xs:integer {-$x})', 'memory:expression.xpath');
    assert.deepEqual(Buffer.from(inlineBytes), readFileSync(join(fixtureDirectory, 'inline.bin')));
    assert.equal(disposeXPathArtifact(load(inline, inlineBytes).artifactId), true);
    checks++;
    for (const fixture of [
        { ...xslt, host: 'cem-ql' }, { ...xslt, host: 'unknown' },
        { ...xslt, sourceHash: standalone.sourceHash }, { ...xslt, contentHash: standalone.contentHash },
        { ...xslt, contentHash: 'not-a-hash' },
    ]) {
        assert.throws(() => load(fixture), /cem\.xpath\.artifact_identity_mismatch/);
        checks++;
    }
    assert.throws(() => compileXPathArtifact('1 +', 'memory:invalid.xpath'), /cem\.xpath\.artifact_invalid/);
    checks++;
    const corrupt = Buffer.from(wasmBytes);
    corrupt[corrupt.length - 1] ^= 1;
    assert.throws(() => load(standalone, corrupt), /cem\.xpath\.artifact_identity_mismatch/);
    checks++;
    assert.throws(() => load(standalone, new Uint8Array(2 * 1024 * 1024 + 1)), /cem\.xpath\.artifact_limit/);
    checks++;
    const retained = Array.from({ length: 64 }, () => load(standalone).artifactId);
    assert.throws(() => load(standalone), /cem\.xpath\.artifact_limit/);
    for (const id of retained) assert.equal(disposeXPathArtifact(id), true);
    const next = load(standalone).artifactId;
    assert.ok(next > retained[retained.length - 1], 'stale XPath handles must never be reused');
    assert.equal(disposeXPathArtifact(next), true);
    checks++;
    // Loading a program does not register a callable native function in CEMT.
    const program = load(xslt);
    const template = JSON.parse(compileTemplate('{$native:call("https://cem.dev/ns/query/xpath/1#select")}', '[]'));
    assert.deepEqual(template.diagnostics, []);
    try {
        const rendered = JSON.parse(renderTemplate(template.artifactId, JSON.stringify({
            native_functions: { 'https://cem.dev/ns/query/xpath/1#select': program.artifactId },
        })));
        assert.ok(rendered.nodes.every(node => node.kind === 'text' && node.text === ''));
        assert.ok(rendered.diagnostics.some(d => d.code === 'cem.ql.native_function_unavailable'));
        checks++;
    } finally {
        assert.equal(disposeTemplate(template.artifactId), true);
        assert.equal(disposeXPathArtifact(program.artifactId), true);
    }
    console.log(`XPath artifact checks passed: ${checks} (native/WASM bytes, identities, bounds, disposal, capability isolation).`);
} finally {
    // Only remove the exact directory this process created for these fixtures.
    rmSync(fixtureDirectory, { recursive: true, force: true });
}
