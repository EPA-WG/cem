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
