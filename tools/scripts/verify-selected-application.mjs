// Integration driver. Application data crosses into the selected runtime as bytes.
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

const root = resolve(import.meta.dirname, '../..');
const fixture = resolve(root, 'packages/cem_ql/fixtures/component-application/shopping-cart');
const bundle = resolve(root, 'dist/artifacts/cem_ql/selected-application');
const wasm = resolve(root, 'packages/cem_ql/dist/wasm-selected-application');
const profile = JSON.parse(readFileSync(resolve(fixture, 'profile.json'), 'utf8'));
const run = (command, args, env = process.env) => {
    const result = spawnSync(command, args, { cwd: root, env, stdio: 'inherit' });
    if (result.status !== 0) throw result.error ?? Error(`${command} failed (${result.status})`);
};
if (!process.argv.includes('--verify-built')) {
    run('cargo', ['run', '--locked', '--offline', '-p', 'cem-ql', '--features', 'application-contract-build',
        '--example', 'compile_declaration_closure', '--', 'packages/cem-components/src/components',
        'https://app.example/components/', bundle, 'prod', '--compiled-css',
        `--entry=${resolve(fixture, profile.entryTemplate)}`, `--entry-uri=https://app.example/${profile.entryTemplate}`,
        ...profile.hostBindings.map(binding => `--entry-binding=${binding}`),
        `--graphql-schema=${resolve(fixture, profile.graphql.schema)}`,
        `--graphql-operations=${resolve(fixture, profile.graphql.operations)}`, ...profile.components]);
    run(process.execPath, ['tools/scripts/build-cem-ql-wasm.mjs', '--component-runtime'],
        { ...process.env, CEM_SELECTED_APPLICATION_DIR: bundle });
}

const compiled = await WebAssembly.compile(readFileSync(resolve(wasm, 'cem_ql_bg.wasm')));
const names = WebAssembly.Module.exports(compiled).map(entry => entry.name)
    .filter(name => name !== 'memory' && !name.startsWith('__')).sort();
assert.deepEqual(names, [
    'compiledDeclarationInventory', 'compileddeclarationowner_describe', 'compileddeclarationowner_dispose',
    'compileddeclarationowner_new', 'compileddeclarationowner_releaseConsumer', 'compileddeclarationowner_renderJson',
    'compileddeclarationowner_renderInputs', 'compileddeclarationowner_renderInstanceInputs',
    'compileddeclarationowner_renderIsland', 'compileddeclarationowner_projectIsland',
    'compileddeclarationowner_stylesheet', 'renderSelectedApplicationResponse',
    'selectedApplicationDocument', 'validateSelectedApplicationQueryVariables', 'validateSelectedApplicationVariables',
    'captureSelectedApplicationResponse', 'claimSelectedApplicationMutation',
    'confirmSelectedApplicationResponseRetirement', 'disposeSelectedApplicationResponse',
    'restoreSelectedApplicationResponse', 'retainSelectedApplicationResponse',
    'sealSelectedApplicationCheckpoint', 'selectedApplicationIdentity', 'selectedApplicationResponseRevision',
].sort());

const { createCemComponentWasmBindings } = await import(pathToFileURL(resolve(wasm, 'cem_ql_factory.js')).href);
const document = readFileSync(resolve(fixture, profile.graphql.operations), 'utf8');
const responses = [
    ['Cart', 'cart', /Tea.*value="2".*USD 18\.00/s],
    ['SetCartLineQuantity', 'updated', /value="3".*Coffee &lt;img.*USD 32\.00/s],
    ['RemoveCartLine', 'removed', /Coffee.*USD 5\.00/s],
    ['Cart', 'empty', /Your cart is empty.*USD 0\.00/s],
    ['Cart', 'partial-error', /The shipping estimate is unavailable\..*Tea.*USD 9\.00/s],
    ['Cart', 'failed', /Please sign in again\..*Cart unavailable/s],
];
const requirements = JSON.parse(readFileSync(resolve(bundle, 'requirements.json'), 'utf8'));
const card = requirements.declarations['cem-card'];
const cardBytes = readFileSync(resolve(bundle, 'cem-card.bin'));
const cardMetadata = readFileSync(resolve(bundle, 'cem-card.json'), 'utf8');
const styleContext = JSON.stringify({
    identity: 'selected-cart', resolverIdentity: 'cart-build', resourcePolicyStamp: 'compiled-only',
    frames: [{ frameId: 'card', baseUrl: card.sourceUri, scopes: [], specifiers: { imports: {}, resources: {} } }],
});
const memories = [];
for (let index = 0; index < 2; index++) {
    const app = createCemComponentWasmBindings();
    assert.throws(() => app.applicationDocument(), /inactive/);
    await app.initialize(compiled);
    assert.ok(!memories.includes(app.memory()));
    memories.push(app.memory());
    assert.equal(app.applicationDocument(), document);
    const uri = 'app://shopping-cart/variables.json';
    const variables = (operation, value) => app.validateVariables(operation, Buffer.from(value), uri);
    variables('Cart', '{"id":"cart-1"}');
    variables('SetCartLineQuantity', '{"cartId":"cart-1","lineId":"tea","quantity":3}');
    variables('RemoveCartLine', '{"cartId":"cart-1","lineId":"tea"}');
    app.validateQueryVariables('Cart', Buffer.from('{"id":"cart-1"}'), uri);
    assert.throws(() => app.validateQueryVariables('RemoveCartLine', Buffer.from('{"cartId":"cart-1","lineId":"tea"}'), uri), /cem\.graphql\.operation\.kind/);
    assert.throws(() => variables('SetCartLineQuantity', '{"cartId":"cart-1","lineId":"tea","quantity":"3"}'), /cem\.graphql/);
    assert.throws(() => variables('Cart', '{"id":"a","id":"b"}'), /cem\.graphql/);
    assert.throws(() => variables('Foreign', '{}'), /cem\.graphql/);
    const render = (operation, bytes) => app.renderResponse(operation, bytes, 'app://shopping-cart/response.json');
    assert.throws(() => render('Cart', Buffer.from('{"data":{"cart":{"id":"bad"}}}')), /cem\.graphql/);
    assert.throws(() => render('Foreign', Buffer.from('{"errors":[{"message":"no"}]}')), /cem\.graphql/);
    for (const [operation, name, expected] of responses) {
        const html = render(operation, readFileSync(resolve(fixture, `${name}.json`)));
        assert.match(html, expected, name);
        assert.ok(!html.includes('<img'), name);
        if (name === 'cart') {
            assert.match(html, /<cem-text-field [^>]*host-command-change="SetCartLineQuantity"[^>]*host-command-key="tea"/);
            assert.match(html, /<cem-action [^>]*host-command="RemoveCartLine"[^>]*host-command-key="tea"/);
        }
        if (name === 'removed') assert.ok(!html.includes('Tea'));
        if (name === 'empty' || name === 'failed') assert.ok(!html.includes('<li '));
    }
    const declaration = app.open(cardBytes, cardMetadata, card.metadataHash);
    try {
        const receipt = JSON.parse(declaration.stylesheet('selected-cart', 0, styleContext));
        assert.equal(receipt.status, 'ready');
        assert.match(receipt.css, /@scope/);
        assert.match(receipt.css, /--cem-palette-comfort/);
        assert.equal(JSON.parse(app.inventory()).stylesheetConsumerLeases, 1);
        assert.equal(declaration.releaseConsumer('selected-cart'), 1);
    } finally {
        declaration.free();
    }
    const census = JSON.parse(app.inventory());
    assert.equal(census.liveDeclarationOwners, 0);
    assert.equal(census.stylesheetConsumerLeases, 0);
    app.release();
    assert.throws(() => render('Cart', Buffer.from('{}')), /inactive/);
}
console.log('Selected application WASM: guarded variables, six original responses and retained CSS passed on two independent heaps.');

// No service exists in this driver. Recovery must use the exact original
// native capsule and terminal write journal on a different WebAssembly.Memory.
const original = createCemComponentWasmBindings();
await original.initialize(compiled);
const raw = readFileSync(resolve(fixture, 'cart.json'));
const uri = 'app://shopping-cart/response.json';
const expected = original.retainResponse('Cart', raw, uri);
const firstRevision = original.responseRevision();
assert.throws(() => original.release(), /generation.owned/);
assert.equal(JSON.parse(original.inventory()).liveApplicationResponses, 1);
assert.equal(JSON.parse(original.inventory()).applicationResponseBytes, raw.length);
assert.equal(original.claimMutation('write-1'), 1);
assert.throws(() => original.claimMutation('write-1'), /response_checkpoint.refused/);
const capsule = original.captureResponse();
assert.equal(Buffer.from(capsule).subarray(-raw.length).compare(raw), 0);
assert.notEqual(original.responseRevision(), firstRevision);
assert.throws(() => original.retainResponse('Cart', Buffer.from('{}'), uri), /cem.graphql/);
assert.deepEqual(original.captureResponse(), capsule);
const receipt = {
    runtimeId: 'original', generation: 1, commitSequence: 1, policyStamp: 'selected-test',
    owners: [{ instanceId: 'application', declarationIdentity: original.applicationIdentity(),
        dataRevision: original.responseRevision(), acceptedEventSequence: 1 }],
    operations: [{ instanceId: 'application', operationId: 'write-1', outcome: 'acknowledged' }],
};
const seal = value => Buffer.from(original.sealCheckpoint(JSON.stringify({ version: 'cem-committed-checkpoint/1', receipt: value,
    owners: [{ instanceId: 'application', island: 'native-response' }] }), JSON.stringify(value)));
const unknown = seal({ ...receipt, operations: [{ ...receipt.operations[0], outcome: 'unknown' }] });
assert.throws(() => original.confirmResponseRetirement(capsule, unknown, 'original', 'application'), /response_checkpoint.refused/);
for (const mismatch of [{ ...receipt, operations: [] }, { ...receipt, owners: [{ ...receipt.owners[0], acceptedEventSequence: 0 }] }]) {
    assert.throws(() => original.confirmResponseRetirement(capsule, seal(mismatch), 'original', 'application'), /response_checkpoint.refused/);
}
const journal = seal(receipt);
const pin = original.confirmResponseRetirement(capsule, journal, 'original', 'application');
assert.equal(JSON.parse(original.inventory()).liveApplicationResponses, 0);
assert.throws(() => original.retainResponse('Cart', raw, uri), /response_checkpoint.refused/);
assert.throws(() => original.claimMutation('write-2'), /response_checkpoint.refused/);
const oldMemory = original.memory();
original.release();
const fresh = createCemComponentWasmBindings();
await fresh.initialize(compiled);
assert.notEqual(fresh.memory(), oldMemory);
const changed = capsule.slice();
changed[changed.length - 1] ^= 1;
assert.throws(() => fresh.restoreResponse(changed, pin, journal, 'original'), /response_checkpoint.refused/);
assert.throws(() => fresh.restoreResponse(capsule, pin, unknown, 'original'), /response_checkpoint.refused/);
assert.throws(() => fresh.restoreResponse(capsule, pin, journal, 'foreign'), /response_checkpoint.refused/);
assert.throws(() => fresh.restoreResponse(new Uint8Array(1_048_577), pin, journal, 'original'), /input.limit/);
assert.throws(() => fresh.restoreResponse(capsule, ' '.repeat(16 * 1024 + 1), journal, 'original'), /input.limit/);
assert.equal(fresh.restoreResponse(capsule, pin, journal, 'original'), expected);
assert.deepEqual(fresh.captureResponse(), capsule);
assert.throws(() => fresh.restoreResponse(capsule, pin, journal, 'original'), /response_checkpoint.refused/);
assert.throws(() => fresh.claimMutation('write-1'), /response_checkpoint.refused/);
assert.equal(fresh.claimMutation('write-2'), 2);
fresh.disposeResponse();
assert.equal(JSON.parse(fresh.inventory()).liveApplicationResponses, 0);
fresh.release();
console.log('Selected response owner: original-byte capsule, terminal-journal retirement, independent-heap reimport and native replay fence passed with no transport.');
