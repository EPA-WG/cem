import assert from 'node:assert/strict';
import { CemCheckpointPublicationCoordinator } from '../dist/lib/checkpoint-publication.js';
import { CemNativeCheckpointDomain } from '../dist/lib/internal/runtime-support/checkpoint-domain.js';
import { registerNativeGeneration, completeNativeRetirement, CemNativeGenerationGroup } from '../dist/lib/internal/runtime-support/native-generation.js';
import { resumeCemCheckpointOwners } from '../dist/lib/checkpoint-resume.js';

assert.equal(typeof global.gc, 'function', 'Run with node --expose-gc after cem-elements:build');
const references = [];
// Keep both the coordinator and its public registration handle alive. Neither
// may retain a removed/disposed owner's native sealing closure or memory.
function createSeal() {
    const memory = new WebAssembly.Memory({ initial: 1 });
    references.push(new WeakRef(memory));
    return text => { assert.equal(memory.buffer.byteLength, 65536); return text; };
}
async function register(coordinator, route) {
    if (route === 'registration') return coordinator.register(crypto.randomUUID(), () => '<template/>', () => true, 0, () => [], {
        host: {}, resolve: () => undefined, seal: createSeal(),
    });
    const old = new CemNativeCheckpointDomain('worker'), next = new CemNativeCheckpointDomain('worker');
    const oldClient = old.client(), freshClient = next.client(), id = crypto.randomUUID();
    let context;
    const owner = coordinator.register(id, () => '<template/>', () => true, 0, () => [], {
        host: {}, nativeClient: oldClient, resolve: () => undefined,
        resume: { witness: () => () => true, prepare: async value => {
            context = value; return { owner: {}, current: () => true, discard: () => undefined };
        } },
    });
    owner.published({ instanceId: id, declarationIdentity: 'source', dataRevision: '1' });
    await coordinator.capture(); const ticket = coordinator.prepareResume(), saved = coordinator.current();
    const participant = registerNativeGeneration({}, old.issuedIdentity(), async () => ({ identity: {}, runtimeId: next.issuedIdentity(),
        runtime: {}, current: () => !!next.identity(), discard: () => next.revoke() }));
    old.revoke();
    const admission = await new CemNativeGenerationGroup([participant]).allocate([completeNativeRetirement(participant, saved.bytes)]);
    const result = await resumeCemCheckpointOwners(admission, [ticket]);
    owner.resume(context, freshClient, createSeal());
    return { owner, release: result.revoke };
}
const coordinators = [], owners = [], releases = [];
for (const route of ['registration', 'resume']) for (const disposition of ['remove', 'dispose']) {
    const coordinator = new CemCheckpointPublicationCoordinator({ runtimeId: crypto.randomUUID(), generation: 1,
        policyStamp: 'private', allowOwner: () => true, seal: text => text });
    const registration = await register(coordinator, route), owner = registration.owner ?? registration;
    if (registration.release) releases.push(registration.release);
    coordinators.push(coordinator); owners.push(owner);
    if (disposition === 'remove') owner.remove(); else coordinator.dispose();
}
for (let n = 0; n < 20; n++) { await new Promise(resolve => setTimeout(resolve, 10)); global.gc(); }
for (const reference of references) assert.equal(reference.deref(), undefined, 'released owner retained its native sealer memory');
assert.ok(owners.every(owner => !owner.acknowledged));
for (const release of releases) release();
for (const coordinator of coordinators) coordinator.dispose();
console.log('Removed/disposed owner sealing routes release native memory with retained registration handles.');
