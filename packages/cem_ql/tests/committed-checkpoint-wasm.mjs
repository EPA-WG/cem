import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import ts from 'typescript';

// Exercise the actual shared host store with independently generated WASM
// glue modules/memories. No mock sealing or WASM-memory snapshots.
const source = await readFile(new URL('../../cem-elements/src/lib/committed-checkpoint.ts', import.meta.url), 'utf8');
const js = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } }).outputText;
const { CemCommittedCheckpointStore } = await import(`data:text/javascript;base64,${Buffer.from(js).toString('base64')}`);
const wasmBytes = await readFile(new URL('../dist/wasm/cem_ql_bg.wasm', import.meta.url));
const load = async generation => {
    const url = new URL('../dist/wasm/cem_ql.js', import.meta.url); url.searchParams.set('checkpoint-generation', generation);
    const wasm = await import(url.href);
    const native = await wasm.default({ module_or_path: wasmBytes });
    return { wasm, native };
};
const receipt = { runtimeId: 'runtime-a', generation: 1, commitSequence: 3, policyStamp: 'session-only',
    owners: [{ instanceId: 'editor', declarationIdentity: 'source:1', dataRevision: 'draft:2', acceptedEventSequence: 2 }],
    operations: [{ instanceId: 'editor', operationId: 'save:2', outcome: 'unknown' }] };
const candidate = { version: 'cem-committed-checkpoint/1', receipt,
    owners: [{ instanceId: 'editor', island: '<template data-cem-data-island>unsaved é text</template>' }] };
const store = new CemCommittedCheckpointStore('runtime-a');
store.acknowledge(receipt);
const runtimeState = { version: 'cem-checkpoint-runtime/1', capability: null,
    resources: [{ name: 'remote', kind: 'http', disposition: 'reimport-and-read-again', revision: 1, cursor: 0, live: false, state: 'loaded',
        url: 'https://example.test/source.xml', method: 'GET', document: { base64: Buffer.from('<data>é😀</data>').toString('base64'),
            contentType: 'application/xml', sourceUri: 'https://example.test/source.xml', digest: null } }],
    writers: [{ index: 0, trigger: null, implicit: true }], writeDisposition: 'suppress-until-new-input' };
let portableRuntime;
let previousMemory;
{
    const { wasm, native } = await load(1);
    previousMemory = native.memory;
    const sample = JSON.parse(wasm.runtimeMemoryStatus()).sample;
    const observed = Math.max(sample.allocatorLiveBytes, sample.linearMemoryBytes);
    // Ordinary admission is stopped; a bounded transport reservation is allowed.
    wasm.configureRuntimeMemoryBudget(JSON.stringify({ softLimitBytes: 1, hardLimitBytes: observed + 32 * 1024 * 1024,
        recoveryHeadroomBytes: 32 * 1024 * 1024, maxConcurrentOperations: 2 }));
    assert.ok(JSON.parse(wasm.compileTemplate('{p | refused}', '[]')).diagnostics.some(d => d.code === 'cem.runtime.memory.admission'));
    const controls = { version: 'cem-checkpoint-controls/1', controls: [{ id: 'editor:1', kind: 'text', inputType: 'plaintext',
        value: 'Draft é😀', checked: false, indeterminate: false, multiple: false, options: [],
        selection: { start: 1, end: 3, direction: 'backward' }, focused: true, disabled: false, readOnly: true, customValidity: '' }] };
    assert.deepEqual(JSON.parse(wasm.validateCheckpointControls(JSON.stringify(controls))), controls);
    const secret = structuredClone(controls); secret.controls[0].inputType = 'password';
    assert.throws(() => wasm.validateCheckpointControls(JSON.stringify(secret)));
    const invalidSelection = structuredClone(controls); invalidSelection.controls[0].selection.end = 1000;
    assert.throws(() => wasm.validateCheckpointControls(JSON.stringify(invalidSelection)));
    assert.equal(JSON.parse(wasm.runtimeMemoryStatus()).activeOperations, 0);

    portableRuntime = wasm.validateCheckpointRuntime(JSON.stringify(runtimeState));
    assert.match(JSON.parse(portableRuntime).resources[0].document.digest, /^[a-f0-9]{64}$/);
    const badRuntime = structuredClone(runtimeState); badRuntime.resources[0].url = 'https://user:secret@example.test/source.xml';
    assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(badRuntime)));
    assert.equal(JSON.parse(wasm.runtimeMemoryStatus()).activeOperations, 0);
    await store.capture(JSON.stringify(candidate), wasm.sealCommittedCheckpoint);
    assert.notEqual(store.current().bytes.buffer, native.memory.buffer);
    const changed = structuredClone(candidate); changed.receipt.owners[0].dataRevision = 'not-committed';
    await assert.rejects(store.capture(JSON.stringify(changed), wasm.sealCommittedCheckpoint));
    assert.equal(JSON.parse(wasm.runtimeMemoryStatus()).activeOperations, 0);
    // Even recovery refuses if there is no physical room; last-good host state survives.
    wasm.configureRuntimeMemoryBudget(JSON.stringify({ softLimitBytes: 1, hardLimitBytes: 65536,
        recoveryHeadroomBytes: 32768, maxConcurrentOperations: 1 }));
    await assert.rejects(store.capture(JSON.stringify(candidate), wasm.sealCommittedCheckpoint));
}
const saved = store.current();
assert.equal(JSON.parse(new TextDecoder().decode(saved.bytes)).owners[0].island, candidate.owners[0].island);
const { wasm, native } = await load(2);
assert.notEqual(native.memory, previousMemory);
const resealed = wasm.sealCommittedCheckpoint(new TextDecoder().decode(saved.bytes), JSON.stringify(saved.receipt));
assert.deepEqual(JSON.parse(resealed), candidate);
assert.equal(JSON.parse(resealed).receipt.operations[0].outcome, 'unknown');
assert.equal(wasm.validateCheckpointRuntime(portableRuntime), portableRuntime);
const body = JSON.parse(portableRuntime).resources[0].document;
const document = wasm.retainCemDocument(Buffer.from(body.base64, 'base64'), body.contentType, body.sourceUri);
assert.ok(document > 0); assert.ok(wasm.disposeCemDocument(document));
const terminal = structuredClone(candidate); terminal.receipt.operations[0].outcome = 'not-applied';
assert.equal(JSON.parse(wasm.sealCommittedCheckpoint(JSON.stringify(terminal), JSON.stringify(terminal.receipt))).receipt.operations[0].outcome, 'not-applied');
const widget = { ...JSON.parse(portableRuntime), capability: { kind: 'tree-view', focusValue: 'docs', typeahead: 'd', typeaheadAt: 1 },
    focus: { target: 'node', nodeId: 'tree.1' } };
assert.deepEqual(JSON.parse(wasm.validateCheckpointRuntime(JSON.stringify(widget))), widget);
const dependent = structuredClone(candidate);
dependent.receipt.relationships = [{ id: 'control', kind: 'placement', disposition: 'reacquire', property: 'aria-controls',
    requester: { instanceId: 'editor', dataRevision: 'draft:2', nodeId: 'button.1' },
    provider: { instanceId: 'editor', dataRevision: 'draft:2', nodeId: 'panel.1' } }];
assert.deepEqual(JSON.parse(wasm.sealCommittedCheckpoint(JSON.stringify(dependent), JSON.stringify(dependent.receipt))), dependent);
dependent.receipt.relationships[0].provider.dataRevision = 'stale';
assert.throws(() => wasm.sealCommittedCheckpoint(JSON.stringify(dependent), JSON.stringify(dependent.receipt)));
const active = { ...runtimeState, resources: [], capability: { kind: 'suggestions', state: {
    source: { version: 'cem-native-source/1', revision: 'options:1', rowCount: 2, select: 'input.children', resolve: false, requesting: 0,
        sources: [{ codec: 'xml', base64: Buffer.from('<options><option value="same">First</option><option value="same">Second</option></options>').toString('base64'), sourceUri: 'https://example.test/options.xml', primarySourceId: 1, context: true }],
        bindings: [], crossings: [], labels: { option: null, group: null }, disposition: 'reacquire' },
    publication: { filter: 'none', filterBy: null, expanded: null, active: null, committed: null },
    providerRevision: 0, preparedRevision: 0, queryRevision: 1, query: 'same', intent: false, feedbackIntent: false, feedbackState: 'ready', activeIndex: -1,
    committedIndex: 1, committedValue: 'same', visible: false, scrollTop: 0, scrollLeft: 0,
    rows: [{ index: 0, value: 'same', eligible: true, available: true, nodeId: 'row.0' }, { index: 1, value: 'same', eligible: true, available: true, nodeId: 'row.1' }] } } };
const activeSaved = wasm.validateCheckpointRuntime(JSON.stringify(active));
assert.equal(JSON.parse(activeSaved).capability.state.committedIndex, 1);
assert.equal(wasm.validateCheckpointRuntime(activeSaved), activeSaved);
const externalPublication = { ...runtimeState, resources: [], capability: null, nativePublication: { profile: 'listbox',
    source: active.capability.state.source, query: 'same', queryRevision: 1, publication: active.capability.state.publication,
    rows: active.capability.state.rows, disposition: 'reacquire' } };
const externalSaved = wasm.validateCheckpointRuntime(JSON.stringify(externalPublication));
assert.equal(wasm.validateCheckpointRuntime(externalSaved), externalSaved);
assert.equal(JSON.parse(externalSaved).nativePublication.rows[1].index, 1);
externalPublication.nativePublication.rows[1].nodeId = 'row.0';
assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(externalPublication)));
for (const feedbackState of ['idle', 'failed', 'unavailable']) {
    const inactive = { ...active, capability: { kind: 'suggestions', state: { phase: 'inactive', profile: 'listbox',
        source: { ...active.capability.state.source, rowCount: 0 }, providerRevision: 2, queryRevision: 3, query: 'draft',
        feedbackState, feedbackIntent: false, editable: feedbackState !== 'unavailable' } } };
    const saved = wasm.validateCheckpointRuntime(JSON.stringify(inactive));
    assert.equal(JSON.parse(saved).capability.state.feedbackState, feedbackState);
    assert.equal(wasm.validateCheckpointRuntime(saved), saved);
    inactive.capability.state.feedbackState = 'pending';
    assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(inactive)));
}
const retainedProof = { ...active, capability: { kind: 'suggestions', state: { phase: 'inactive', profile: 'listbox',
    source: { ...active.capability.state.source, rowCount: 0 }, providerRevision: 4, queryRevision: 5, query: 'same', feedbackState: 'unavailable', feedbackIntent: false, editable: false,
    proof: { source: active.capability.state.source, index: 1, value: 'same', disposition: 'revalidate' } } } };
const retainedSaved = wasm.validateCheckpointRuntime(JSON.stringify(retainedProof));
assert.equal(wasm.validateCheckpointRuntime(retainedSaved), retainedSaved);
assert.equal(JSON.parse(wasm.validateCheckpointRuntime(retainedSaved)).capability.state.proof.index, 1);
retainedProof.capability.state.proof.value = 'wrong';
assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(retainedProof)));
const deferredDelegate = { ...runtimeState, resources: [], capability: { kind: 'native-surface', phase: 'closed', defaultsInitialized: true,
    state: { returnValue: '', modal: false, focusWithin: false, closeReason: 'native', invocation: null, disposition: 'reacquire',
        body: { templateNode: 'template.1', mode: 'retain', materialized: true, roots: ['input.1'], disposition: 'rematerialize' } } },
    manualListbox: { panel: { instanceId: 'listbox', dataRevision: '1', nodeId: 'panel.1' }, editor: { instanceId: 'field', dataRevision: '2', nodeId: 'host' },
        providerRevision: 2, visible: false, presentation: 'reset-on-resume', disposition: 'reacquire' } };
const deferredSaved = wasm.validateCheckpointRuntime(JSON.stringify(deferredDelegate));
assert.equal(wasm.validateCheckpointRuntime(deferredSaved), deferredSaved);
assert.equal(JSON.parse(deferredSaved).capability.state.body.roots[0], 'input.1');
deferredDelegate.capability.state.body.roots.push('input.1');
assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(deferredDelegate)));
const originalBody = structuredClone(deferredDelegate);
originalBody.capability.state.body.roots = ['field'];
originalBody.capability.state.body.reuseOnOpen = true;
originalBody.capability.state.body.projection = { version: 'cem-render-projection/1', origin: { instanceId: 'parent', templateArtifactId: 'artifact', dataRevision: '1', scopePolicyStamp: 'policy' }, entries: [
    { kind: 'element', nodeId: 'field', tag: 'cem-field', namespace: null, attributes: [{name:'value', value:'Original'}] }, {kind:'end'} ] };
originalBody.capability.state.body.owners = [{ hostNode: 'field', owner: { instanceId: 'child', dataRevision: '3', nodeId: 'host' } }];
const originalSaved = wasm.validateCheckpointRuntime(JSON.stringify(originalBody));
assert.equal(wasm.validateCheckpointRuntime(originalSaved), originalSaved);
assert.equal(JSON.parse(originalSaved).capability.state.body.projection.entries[0].attributes[0].value, 'Original');
originalBody.capability.state.body.projection.entries[0].nativeValue = 'handle';
assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(originalBody)));
delete originalBody.capability.state.body.projection.entries[0].nativeValue;
originalBody.capability.state.body.projection.entries[0].attributes.push({name:'type',value:'password'});
assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(originalBody)));
const menuEndpoint = { instanceId: 'parent', dataRevision: '4', nodeId: 'menu.1' };
const submenu = { ...runtimeState, resources: [], capability: { kind: 'composite-menu', activeNode: 'item.1', typeahead: '', typeaheadAt: 0, open: true,
    state: { parent: menuEndpoint, launcher: menuEndpoint, panel: menuEndpoint, disposition: 'reacquire' } } };
const submenuSaved = wasm.validateCheckpointRuntime(JSON.stringify(submenu));
assert.equal(wasm.validateCheckpointRuntime(submenuSaved), submenuSaved);
assert.equal(JSON.parse(wasm.validateCheckpointRuntime(submenuSaved)).capability.open, true);
submenu.capability.state.panel = null;
assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(submenu)));
const explicitSubmenu = structuredClone(submenu);
explicitSubmenu.capability.state.panel = menuEndpoint;
explicitSubmenu.capability.state.policies = { compositeNode: 'composite.1',
    anchor: {mode: 'target', target: menuEndpoint}, boundary: {mode: 'viewport', target: null}, boundaryOwner: null,
    focusTarget: {mode: 'none', target: null}, returnFocus: {mode: 'target', target: menuEndpoint}, anchorLost: 'freeze', geometry: 'recompute' };
const explicitSaved = wasm.validateCheckpointRuntime(JSON.stringify(explicitSubmenu));
const { wasm: menuGeneration } = await load('explicit-submenu');
assert.equal(menuGeneration.validateCheckpointRuntime(explicitSaved), explicitSaved);
explicitSubmenu.capability.state.policies.geometry = 'restore-pixels';
assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(explicitSubmenu)));
explicitSubmenu.capability.state.policies.geometry = 'recompute';
explicitSubmenu.capability.state.policies.anchor.lease = 'stale authority';
assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(explicitSubmenu)));
const datalist = { ...active, capability: { kind: 'suggestions', state: { phase: 'native-datalist', source: active.capability.state.source,
    providerRevision: 2, targetNode: 'datalist.1', claim: 'attached', presentation: 'reset-on-resume' } } };
const datalistSaved = wasm.validateCheckpointRuntime(JSON.stringify(datalist));
assert.equal(wasm.validateCheckpointRuntime(datalistSaved), datalistSaved);
datalist.capability.state.presentation = 'restore-popup';
assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(datalist)));
const standaloneDatalist = { ...runtimeState, resources: [], capability: null, standaloneDatalist: {
    editor: { instanceId: 'field', dataRevision: '2', nodeId: 'host' }, target: { instanceId: 'frame', dataRevision: '3', nodeId: 'datalist.1' },
    providerRevision: 2, claim: 'attached', source: active.capability.state.source, presentation: 'reset-on-resume', disposition: 'reacquire' } };
const standaloneSaved = wasm.validateCheckpointRuntime(JSON.stringify(standaloneDatalist));
assert.equal(wasm.validateCheckpointRuntime(standaloneSaved), standaloneSaved);
const { wasm: standaloneGeneration } = await load('standalone-datalist');
assert.equal(standaloneGeneration.validateCheckpointRuntime(standaloneSaved), standaloneSaved);
standaloneDatalist.standaloneDatalist.lease = 'obsolete authority';
assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(standaloneDatalist)));
const completedCommit = { ...runtimeState, resources: [], capability: { kind: 'form-control', authoredValue: null, formDisabled: false, customValidity: '', selectionRequest: null,
    provider: { revision: 4, consumer: 'suggestions', validity: '' }, changeSuppression: { revision: 4, value: 'same', disposition: 'suppress-once' } } };
assert.equal(JSON.parse(wasm.validateCheckpointRuntime(JSON.stringify(completedCommit))).capability.changeSuppression.disposition, 'suppress-once');
completedCommit.capability.changeSuppression.revision = 3; assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(completedCommit)));
const invalidActive = structuredClone(active); invalidActive.capability.state.source.sources[0].digest = 'obsolete';
assert.throws(() => wasm.validateCheckpointRuntime(JSON.stringify(invalidActive)));
// Native renderer capsules remain binary; a fresh generation imports them independently.
const capsuleTemplate = JSON.parse(wasm.compileTemplate('{div @native-count={42} @native-flag={false}}', '[]'));
try {
    const plan = JSON.parse(wasm.renderTemplateWithNativeValues(capsuleTemplate.artifactId, 0, '{}', '[]', '[]', '', 'capsule-owner', undefined, undefined));
    assert.deepEqual(plan.diagnostics, []);
    const artifact = wasm.takeRenderValueArtifact(plan.nativeValueArtifactId).slice();
    const capsule = { ...runtimeState, resources: [], capability: {kind:'native-surface',phase:'closed',defaultsInitialized:true,
        state:{returnValue:'',modal:false,focusWithin:false,closeReason:'native',invocation:null,disposition:'reacquire',body:{
            templateNode:'template',mode:'retain',materialized:true,roots:['body'],disposition:'rematerialize',reuseOnOpen:false,
            projection:{version:'cem-render-projection/1',origin:{instanceId:'editor',templateArtifactId:'original',dataRevision:'1',scopePolicyStamp:'scope'},
                nativeArtifacts:[{contentHash:plan.nativeValueContentHash,base64:Buffer.from(artifact).toString('base64')}],
                entries:[{kind:'element',nodeId:'body',tag:'div',namespace:null,attributes:plan.nodes[0].attributes.map(a=>({name:a.name,value:a.value,nativeValue:{artifact:0,index:a.nativeValueIndex}}))},{kind:'end'}]}}}}};
    const saved = wasm.validateCheckpointRuntime(JSON.stringify(capsule));
    const { wasm: fresh } = await load('retained-native-capsules');
    assert.equal(fresh.validateCheckpointRuntime(saved), saved);
    const id = fresh.importNativeValueArtifact(artifact, JSON.stringify({maxBytes:262144,maxValues:4096,maxDepth:64}));
    assert.equal(fresh.exportCemJsonValue(artifact, plan.nodes[0].attributes.find(a=>a.name==='native-count').nativeValueIndex, 'native-count', ''), '42');
    assert.equal(fresh.exportCemJsonValue(artifact, plan.nodes[0].attributes.find(a=>a.name==='native-flag').nativeValueIndex, 'native-flag', ''), 'false');
    assert.equal(fresh.disposeNativeValueArtifact(id), true);
    const countIndex = plan.nodes[0].attributes.find(a=>a.name==='native-count').nativeValueIndex;
    const nativeArtifact = {contentHash:plan.nativeValueContentHash,base64:Buffer.from(artifact).toString('base64')};
    const inputs = { ...runtimeState, resources: [], references: { ...active.capability.state.source,
        preparation:{dataJson:'{"datadom":{"slices":{"caption":"é😀","enabled":true}}}',limits:{maxBytes:262144,maxValues:4096,maxDepth:64},
            nativeBindings:{version:'cem-native-bindings/1',artifacts:[nativeArtifact],bindings:[
                {target:'attribute',name:'native-count',artifact:0,index:countIndex},
                {target:'slice',name:'count',attribute:'native-count',artifact:0,index:countIndex},
                {target:'slice',name:'raw',artifact:0,index:countIndex}]}}},
        capability:{kind:'native-surface',phase:'open',defaultsInitialized:true,state:{returnValue:'',modal:false,focusWithin:true,closeReason:'native',disposition:'reacquire',
            invocation:{source:null,returnDestination:null,logicalParent:null,command:'edit',contextKey:'row:42',inputKind:'programmatic',pointer:null,selection:null,
                context:{artifact:nativeArtifact,index:countIndex}}}}};
    const inputSaved = wasm.validateCheckpointRuntime(JSON.stringify(inputs));
    const {wasm: inputGeneration} = await load('native-context-and-preparation');
    assert.equal(inputGeneration.validateCheckpointRuntime(inputSaved),inputSaved);
    const opaque = JSON.parse(inputSaved).capability.state.invocation.context;
    assert.equal(inputGeneration.exportCemJsonValue(Buffer.from(opaque.artifact.base64,'base64'),opaque.index,'native-count',''),'42');
    inputs.references.preparation.limits.maxBytes=artifact.byteLength-1;
    assert.throws(()=>inputGeneration.validateCheckpointRuntime(JSON.stringify(inputs)));
    inputs.references.preparation.limits.maxBytes=262144;
    inputs.references.preparation.nativeBindings.bindings[1].attribute='wrong';
    assert.throws(()=>inputGeneration.validateCheckpointRuntime(JSON.stringify(inputs)));
    capsule.capability.state.body.projection.nativeArtifacts[0].contentHash = 'forged';
    assert.throws(()=>fresh.validateCheckpointRuntime(JSON.stringify(capsule)));
} finally { wasm.disposeTemplate(capsuleTemplate.artifactId); }
// Execution/resume is deliberately not performed by the transport store.
store.invalidate(); assert.equal(store.current(), undefined);
assert.ok(store.lastCommitted()); store.dispose(); assert.equal(store.lastCommitted(), undefined);
console.log('Committed checkpoint real WASM: headroom, receipt/control/runtime validation, source reimport, terminal outcomes and independent host bytes passed');
