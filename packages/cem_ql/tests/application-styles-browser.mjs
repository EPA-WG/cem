import { createCemQlWasmGeneration } from '../../../dist/fixtures/application-source-wasm/application_factory.js';
import { createCemDeclarationScope } from '../../cem-elements/src/lib/declaration-scope.ts';
import { DeclarationStyleOwnership } from '../../cem-elements/src/lib/declaration-style-ownership.ts';
import { prepareApplicationStylesheets } from '../../cem-elements/src/lib/internal/runtime-support/application-stylesheets.ts';
import { prepareApplicationBrowserFrame } from '../../cem-elements/src/lib/internal/runtime-support/application-browser-frame.ts';
import { prepareApplicationTextProvider } from '../../cem-elements/src/lib/internal/runtime-support/application-text-provider.ts';
import { getCemEditorProvider, isCemCredentialControl } from '../../cem-elements/src/lib/form-control-capability.ts';
import { CemCommittedCheckpointStore } from '../../cem-elements/src/lib/committed-checkpoint.ts';
import { issueCheckpointResumeTicket, resumeCemCheckpointOwners } from '../../cem-elements/src/lib/checkpoint-resume.ts';
import {
    CemNativeGenerationGroup, completeNativeRetirement, registerNativeGeneration, releaseNativeGenerationResourcesAsync,
} from '../runtime/native-generation.mjs';
import {
    createApplicationSourceWasmBackend, createQuerySourceEndpoint, createQuerySourceWorkerEndpoint,
    installQuerySourceWorkerTransport, QuerySourceResource,
} from '../runtime/query-source-resource.mjs';

const assets = '/dist/fixtures/application-source-wasm/';
const tag = 'cem-receiver-frame';
const marker = 'data-cem-css-context';
let compiled;
const check = (condition, message) => { if (!condition) throw Error(message); };
async function refusal(work, code) {
    try { await work(); }
    catch (error) {
        check(code.test(String(error)), `Expected ${code}, received ${error}`);
        return;
    }
    throw Error(`Expected refusal ${code}`);
}

async function nativeBackend(module, pins, input, variant, runtimeId) {
    const runtime = createCemQlWasmGeneration();
    await runtime.initialize(module);
    if (variant === 'installed') runtime.call('probeApplicationProvision', [pins, input]);
    else if (variant === 'navigation') runtime.call('probeApplicationProvisionNavigation', [pins, input]);
    else if (variant === 'provider') runtime.call('probeApplicationProvisionProvider', [pins, input]);
    else runtime.call('probeApplicationProvisionStyled', [pins, input, variant === 'denied']);
    const native = createApplicationSourceWasmBackend(runtime);
    let owner, receiver;
    const call = (name, args) => {
        if (name === 'dispose') return runtime.dispose();
        if (name === 'edit') {
            receiver = runtime.call('probeApplicationEditReceiver', [owner, receiver, ...args]);
            return;
        }
        return runtime.call(name, args);
    };
    if (variant !== 'provider') return { runtime, native, call };
    return { runtime, call, native: {
        ...native,
        open(bytes, pins) {
            owner = native.open(bytes, pins);
            try { receiver = runtime.call('probeApplicationReceiverInput', [owner]); return owner; }
            catch (error) { native.release(owner); throw error; }
        },
        receiver: {
            capture(owner) {
                const [bytes, revision] = runtime.call('probeApplicationCaptureReceiver', [owner, receiver]);
                return { bytes, revision };
            },
            retire(owner, bytes, journal) {
                return runtime.call('probeApplicationConfirmReceiver', [owner, bytes, journal, runtimeId]);
            },
            open(bytes, pins, recovery) {
                [owner, receiver] = runtime.call('probeApplicationRestoreReceiver',
                    [bytes, pins, recovery.bytes, recovery.pin, recovery.journal, recovery.runtimeId]);
                return owner;
            },
        },
    } };
}

if (typeof document === 'undefined') {
    globalThis.addEventListener('message', async ({ data: { module, port, pins, input, variant, runtimeId } }) => {
        const { native, call } = await nativeBackend(module, pins, input, variant, runtimeId);
        installQuerySourceWorkerTransport(port, native);
        globalThis.addEventListener('message', ({ data: { id, name, args } }) => {
            try { globalThis.postMessage({ id, value: call(name, args) }); }
            catch (error) { globalThis.postMessage({ id, error: String(error) }); }
        });
        globalThis.postMessage('ready');
    }, { once: true });
}

async function deployment(variant) {
    const file = variant === 'denied' ? 'styled' : variant;
    const [bytes, pins, input, module] = await Promise.all([
        fetch(`${assets}${file}.cemah`).then(response => response.bytes()),
        fetch(`${assets}${file}.pin.json`).then(response => response.text()),
        fetch(`${assets}${file}.inputs`).then(response => response.bytes()),
        compiled ??= WebAssembly.compileStreaming(fetch(`${assets}application_source_probe_bg.wasm`)),
    ]);
    return { bytes, pins, input, module };
}

async function physical(mode, variant, deployment) {
    const { pins, input, module } = deployment;
    const runtimeId = crypto.randomUUID();
    let endpoint, worker, port, fallback, live = true, sequence = 0;
    const register = identity => registerNativeGeneration(identity, runtimeId, async (_current, signal) => {
        const fresh = await physical(mode, variant, deployment);
        return { identity: fresh.identity, runtimeId: fresh.runtimeId, runtime: fresh,
            current: () => !signal.aborted && fresh.current(), discard: () => fresh.discard() };
    });
    if (mode === 'worker') {
        const channel = new MessageChannel();
        port = channel.port1;
        worker = new Worker('/packages/cem_ql/tests/application-styles-browser.mjs', { type: 'module' });
        try {
            await new Promise((resolve, reject) => {
                const timer = setTimeout(() => reject(Error('Application stylesheet worker did not start')), 5000);
                worker.addEventListener('message', () => { clearTimeout(timer); resolve(); }, { once: true });
                worker.addEventListener('error', event => { clearTimeout(timer); reject(Error(event.message)); }, { once: true });
                worker.postMessage({ module, port: channel.port2, pins, input, variant, runtimeId }, [channel.port2]);
            });
        } catch (error) { channel.port1.close(); worker.terminate(); throw error; }
    } else {
        fallback = await nativeBackend(module, pins, input, variant, runtimeId);
    }
    const identity = () => live ? (worker ?? fallback.runtime.memory()) : undefined;
    const call = (name, ...args) => {
        if (!worker) return Promise.resolve().then(() => fallback.call(name, args));
        return new Promise((resolve, reject) => {
            const id = ++sequence;
            const receive = ({ data }) => {
                if (data?.id !== id) return;
                clearTimeout(timer); worker.removeEventListener('message', receive);
                if (data.error) reject(Error(data.error)); else resolve(data.value);
            };
            const timer = setTimeout(() => {
                worker.removeEventListener('message', receive); reject(Error('Application fixture control timed out'));
            }, 5000);
            worker.addEventListener('message', receive);
            worker.postMessage({ id, name, args });
        });
    };
    const close = () => {
        live = false;
        endpoint?.dispose();
        if (!endpoint) port?.close();
        worker?.terminate();
    };
    const dispose = () => {
        if (!live) return;
        close();
        fallback?.call('probeApplicationUnprovision', []);
        fallback?.call('dispose', []);
    };
    return { get identity() { return identity(); }, runtimeId, call, current: () => live, close, dispose,
        get endpoint() {
            if (endpoint) return endpoint;
            const actual = identity();
            const options = { identity: actual, participant: register(actual), current: () => live };
            endpoint = worker ? createQuerySourceWorkerEndpoint({ ...options, port, maxSourceBytes: 4 * 1024 * 1024,
                applicationStylesheets: true, applicationBrowserFrames: true, receiverRecovery: variant === 'provider' })
                : createQuerySourceEndpoint({ ...options, native: fallback.native });
            return endpoint;
        },
        discard: () => { if (live) setTimeout(dispose, 0); } };
}

async function binding(mode, variant) {
    const config = await deployment(variant);
    const host = await physical(mode, variant, config);
    let allowed = true;
    try {
        const resource = await QuerySourceResource.open({ endpoint: host.endpoint, bytes: config.bytes, pins: config.pins,
            current: () => allowed, target: fresh => fresh.endpoint });
        return {
            resource, refuse: () => { allowed = false; },
            async dispose() { await resource.dispose(); host.dispose(); },
        };
    } catch (error) { host.dispose(); throw error; }
}

export async function verifyApplicationStylesheets(mode) {
    const declaration = document.createElement('div');
    const first = document.createElement(tag);
    const second = document.createElement(tag);
    const outside = document.createElement(tag);
    document.body.append(declaration, first, second, outside);
    const scope = createCemDeclarationScope({ document });
    const ownership = new DeclarationStyleOwnership(document, scope);
    ownership.add(declaration, scope);
    const bindings = [];
    const open = async variant => {
        const active = await binding(mode, variant);
        bindings.push(active);
        return active;
    };
    const stage = (active, host) => prepareApplicationStylesheets(active.resource, ownership, host, scope);
    const commit = entry => check(DeclarationStyleOwnership.commitGroup([entry]), 'Stylesheet publication refused');
    const imported = element => getComputedStyle(element).getPropertyValue('--imported').trim();
    try {
        const original = await open('styled');
        const initial = await stage(original, first);
        check(!declaration.querySelector('style') && !first.hasAttribute(marker), 'Styles were published during preparation');
        commit(initial);
        commit(await stage(original, second));
        const node = declaration.querySelector('style');
        check(node && declaration.querySelectorAll('style').length === 1, 'Consumers failed to share one stylesheet');
        check(imported(first) === 'ready' && imported(second) === 'ready' && imported(outside) === '', 'Native CSS escaped its hosts or lost the closed import');
        check(getComputedStyle(first).getPropertyValue('--original').trim() === 'ready', 'Local CSS was lost');
        check(first.getAttribute(marker) && first.getAttribute(marker) === second.getAttribute(marker), 'Context markers did not match');
        await refusal(() => original.resource.stylesheets('é'.repeat(129)), /resource.stylesheet/);
        await refusal(() => original.resource.stylesheets('cem-unknown'), /native-refusal|endpoint.refused/);
        await refusal(() => open('denied'), /native-refusal|endpoint.refused/);

        const denied = await open('installed');
        const deniedEntry = await stage(denied, first);
        denied.refuse();
        check(!DeclarationStyleOwnership.commitGroup([deniedEntry]), 'A stale candidate published');
        check(declaration.firstElementChild === node && imported(first) === 'ready', 'Rejection lost the committed stylesheet');
        await denied.dispose();

        const replacement = await open('styled');
        const [stale, next] = await Promise.allSettled([stage(replacement, first), stage(replacement, first)]);
        check(stale.status === 'rejected' && next.status === 'fulfilled', 'Supersession retained a stale candidate');
        commit(next.value);
        await original.dispose();
        check(declaration.firstElementChild === node && imported(first) === 'ready' && first.hasAttribute(marker),
            'Retirement removed the equivalent stylesheet still held by the replacement');
        check(imported(second) === '' && !second.hasAttribute(marker), 'Retirement retained the original consumer');

        const unstyled = await open('installed');
        commit(await stage(unstyled, first));
        check(!declaration.querySelector('style') && imported(first) === '' && !first.hasAttribute(marker),
            'An empty replacement retained the previous styles');
        commit(await stage(replacement, first));
        first.remove();
        await new Promise(resolve => setTimeout(resolve, 0));
        check(!declaration.querySelector('style') && !first.hasAttribute(marker), 'Disconnect retained a stylesheet lease');
    } finally {
        scope.dispose();
        for (const active of bindings.reverse()) await active.dispose();
        for (const element of [declaration, first, second, outside]) element.remove();
    }
}

export async function verifyApplicationBrowserFrame(mode) {
    const island = await fetch(`${assets}frame.island`).then(response => response.bytes());
    const host = document.createElement('cem-count');
    document.body.append(host);
    const original = await binding(mode, 'navigation');
    const hostLifetime = new AbortController();
    let replacement;
    try {
        const prepare = (active, instance = 'island-one') =>
            prepareApplicationBrowserFrame(active.resource, host, island, instance, hostLifetime.signal);
        await refusal(() => original.resource.renderBrowserIsland({
            tag: host.localName, instance: 'island-one', bytes: new Uint8Array(1024 * 1024 + 1),
        }), /resource.limit/);
        await refusal(() => prepare(original, 'foreign'), /native-refusal|endpoint.refused/);
        const first = await prepare(original);
        // Fixture-only mounting of the admitted HTML before the shared commit.
        host.innerHTML = first.html;
        const button = host.querySelector('nav[part=root] > button');
        const permits = () => button.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true }));
        check(button?.textContent === '3' && permits(), 'The native output or staged behavior was wrong');
        check(first.commit() && !permits(), 'The shared navigation behavior did not suppress the disabled button');
        const key = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true });
        check(!button.dispatchEvent(key), 'The shared navigation behavior did not suppress the Enter key');
        button.removeAttribute('aria-disabled');
        check(permits(), 'The shared navigation behavior suppressed an enabled button');
        button.setAttribute('aria-disabled', 'true');

        replacement = await binding(mode, 'navigation');
        const obsolete = await prepare(original);
        const next = await prepare(replacement);
        check(next.html === first.html && next.declarationIdentity === first.declarationIdentity,
            'The replacement did not bind the original compiled declaration');
        check(!obsolete.commit() && next.commit() && !permits(), 'A superseded frame changed the current behavior');
        await original.dispose();
        check(!permits(), 'Releasing the first session removed the replacement behavior');
        hostLifetime.abort();
        check(permits(), 'The host lifetime did not remove the shared listener');
        const last = await prepareApplicationBrowserFrame(replacement.resource, host, island, 'island-one', new AbortController().signal);
        check(last.commit() && !permits(), 'The behavior could not rebind on the same host');
        await replacement.dispose();
        check(permits(), 'The original session did not release its browser listener');
    } finally {
        hostLifetime.abort();
        await original.dispose();
        await replacement?.dispose();
        host.remove();
    }
}

export async function verifyApplicationProviderRecovery(mode) {
    const config = await deployment('provider');
    const original = await physical(mode, 'provider', config);
    const host = document.createElement(tag);
    const instance = 'source-route';
    const lifetime = new AbortController();
    const store = new CemCommittedCheckpointStore(original.runtimeId);
    document.body.append(host);
    let input, handoff, admission, resource;
    const current = () => host.isConnected && (!input || !isCemCredentialControl(input));
    const prepare = () => prepareApplicationTextProvider(resource, host, instance, lifetime.signal);
    let changes = 0;
    host.addEventListener('change', () => changes++);
    try {
        resource = await QuerySourceResource.open({ endpoint: original.endpoint, bytes: config.bytes, pins: config.pins,
            current, target: fresh => fresh.endpoint, receiverCheckpoint: () => handoff.retain() });
        await refusal(() => resource.renderBrowserReceiver({ tag, instance: 'foreign' }), /native-refusal|endpoint.refused/);
        const initial = await prepare();
        // Fixture-only mounting. During recovery the original control stays in place.
        host.innerHTML = initial.html;
        input = host.querySelector('input[part=control]');
        const initialProvider = initial.commit();
        check(input?.value === 'Tea' && initialProvider === getCemEditorProvider(host), 'Original source did not bind the text provider');
        const initialLease = initialProvider.lease({});
        input.setAttribute('autocomplete', 'one-time-code');
        input.value = 'private draft';
        await refusal(() => resource.prepareRecovery(), /resource.policy/);
        check(input.value === 'private draft' && !initialLease.current, 'Credential refusal changed the live input or retained its lease');
        input.removeAttribute('autocomplete');
        input.value = 'Tea';
        check(initialLease.commit('Coffee', { revision: initialProvider.revision }) && changes === 1, 'The actual editor provider did not accept the edit once');
        const edited = await fetch(`${assets}installed.edited.inputs`).then(response => response.bytes());
        initial.release();
        await original.call('edit', config.bytes, config.pins, edited);
        const unconfirmed = await prepare();
        input.value = 'unconfirmed draft';
        check(!unconfirmed.commit() && input.value === 'unconfirmed draft', 'An unconfirmed browser edit bound the native provider');
        input.value = 'Coffee';
        const editedFrame = await prepare();
        const originalProvider = editedFrame.commit();
        check(originalProvider && !initialLease.current && input.value === 'Coffee', 'The edited source did not bind the original control');
        const oldLease = originalProvider.lease({});
        input.focus();
        input.setSelectionRange(1, 4, 'backward');
        const revision = await resource.prepareRecovery();
        const receipt = { runtimeId: original.runtimeId, generation: 1, commitSequence: 1, policyStamp: 'provider-fixture',
            owners: [{ instanceId: instance, declarationIdentity: JSON.parse(config.pins).hash, dataRevision: revision, acceptedEventSequence: 1 }],
            operations: [{ instanceId: instance, operationId: 'editor-change', outcome: 'acknowledged' }] };
        store.acknowledge(receipt);
        const candidate = JSON.stringify({ version: 'cem-committed-checkpoint/1', receipt,
            owners: [{ instanceId: instance, island: '<template>original text provider</template>' }] });
        await store.capture(candidate, (text, proof) => original.call('sealCommittedCheckpoint', text, proof));
        handoff = store.handoff();
        const witness = () => current() && host.firstElementChild === input && input.value === 'Coffee'
            && document.activeElement === input && input.selectionStart === 1 && input.selectionEnd === 4 && input.selectionDirection === 'backward';
        let freshProvider;
        const ticket = issueCheckpointResumeTicket(original.runtimeId, store.current(), handoff.current, new Map([[instance, {
            witness: () => witness,
            async prepare(context) {
                check(!context.active() && !oldLease.current && !getCemEditorProvider(host), 'The old provider survived source retirement');
                return {
                    owner: { resume: context.renewOwners },
                    current: witness,
                    async renew() {
                        const next = await prepare();
                        check(next.declarationIdentity === initial.declarationIdentity && next.html.includes('value="Coffee"'),
                            'Fresh native source did not preserve the edit');
                        freshProvider = next.commit();
                        check(freshProvider && freshProvider !== originalProvider && freshProvider.control === input, 'The text field did not acquire a fresh provider');
                    },
                    renewed: () => check(witness() && getCemEditorProvider(host) === freshProvider, 'Provider renewal changed the original input'),
                    discard: () => { lifetime.abort(); },
                };
            },
        }]]));
        const group = new CemNativeGenerationGroup([original.endpoint.participant]);
        await releaseNativeGenerationResourcesAsync(original.endpoint.participant, () => witness() && handoff.current());
        await original.call('probeApplicationUnprovision');
        await original.call('closeCommandNativeAdmission');
        const inventory = JSON.parse(await original.call('commandNativeResourceInventory'));
        check(inventory.blockingResources === 0, 'Original native provider input/source owners did not drain');
        const confirmation = await original.call('dispose');
        check(confirmation.frozen === true, 'Original provider heap did not confirm retirement');
        original.close();
        const retirement = completeNativeRetirement(original.endpoint.participant, handoff.retain());
        admission = await group.allocate([retirement]);
        const resumed = await resumeCemCheckpointOwners(admission, [ticket]);
        await resumed.owners[0].owner.resume();
        check(changes === 1 && witness(), 'Recovery replayed an editor change or lost the edit and selection');
        const lease = freshProvider.lease({});
        check(lease.valid && !oldLease.commit('old', { revision: originalProvider.revision }), 'The old lease gained access to the new provider');
        check(lease.commit('Cocoa', { revision: freshProvider.revision }) && input.value === 'Cocoa' && changes === 2,
            'The replacement provider could not accept a fresh edit');
        await resource.dispose();
        check(!lease.current && !getCemEditorProvider(host), 'Session disposal left a provider or lease live');
    } finally {
        lifetime.abort();
        await resource?.dispose();
        admission?.revoke();
        original.dispose();
        store.dispose();
        host.remove();
    }
}
