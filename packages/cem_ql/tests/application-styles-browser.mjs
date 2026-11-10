import { createCemQlWasmGeneration } from '../../../dist/fixtures/application-source-wasm/application_factory.js';
import { createCemDeclarationScope } from '../../cem-elements/src/lib/declaration-scope.ts';
import { DeclarationStyleOwnership } from '../../cem-elements/src/lib/declaration-style-ownership.ts';
import { prepareApplicationStylesheets } from '../../cem-elements/src/lib/internal/runtime-support/application-stylesheets.ts';
import { registerNativeGeneration } from '../runtime/native-generation.mjs';
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

async function nativeBackend(module, pins, input, variant) {
    const runtime = createCemQlWasmGeneration();
    await runtime.initialize(module);
    if (variant === 'installed') runtime.call('probeApplicationProvision', [pins, input]);
    else runtime.call('probeApplicationProvisionStyled', [pins, input, variant === 'denied']);
    return { runtime, native: createApplicationSourceWasmBackend(runtime) };
}

if (typeof document === 'undefined') {
    globalThis.addEventListener('message', async ({ data: { module, port, pins, input, variant } }) => {
        const { native } = await nativeBackend(module, pins, input, variant);
        installQuerySourceWorkerTransport(port, native);
        globalThis.postMessage('ready');
    }, { once: true });
}

async function binding(mode, variant) {
    const file = variant === 'denied' ? 'styled' : variant;
    const [bytes, pins, input, module] = await Promise.all([
        fetch(`${assets}${file}.cemah`).then(response => response.bytes()),
        fetch(`${assets}${file}.pin.json`).then(response => response.text()),
        fetch(`${assets}${file}.inputs`).then(response => response.bytes()),
        compiled ??= WebAssembly.compileStreaming(fetch(`${assets}application_source_probe_bg.wasm`)),
    ]);
    const register = identity => registerNativeGeneration(identity, crypto.randomUUID(), () => { throw Error('Fixture allocation is explicit'); });
    let endpoint, worker, fallback;
    if (mode === 'worker') {
        const channel = new MessageChannel();
        worker = new Worker('/packages/cem_ql/tests/application-styles-browser.mjs', { type: 'module' });
        try {
            await new Promise((resolve, reject) => {
                const timer = setTimeout(() => reject(Error('Application stylesheet worker did not start')), 5000);
                worker.addEventListener('message', () => { clearTimeout(timer); resolve(); }, { once: true });
                worker.addEventListener('error', event => { clearTimeout(timer); reject(Error(event.message)); }, { once: true });
                worker.postMessage({ module, port: channel.port2, pins, input, variant }, [channel.port2]);
            });
            endpoint = createQuerySourceWorkerEndpoint({ identity: worker, participant: register(worker), port: channel.port1,
                current: () => true, maxSourceBytes: 4 * 1024 * 1024, applicationStylesheets: true });
        } catch (error) { channel.port1.close(); worker.terminate(); throw error; }
    } else {
        fallback = await nativeBackend(module, pins, input, variant);
        const identity = fallback.runtime.memory();
        endpoint = createQuerySourceEndpoint({ identity, participant: register(identity), current: () => true, native: fallback.native });
    }
    const close = () => {
        endpoint.dispose();
        worker?.terminate();
        if (fallback) {
            const { runtime } = fallback;
            fallback = undefined;
            runtime.call('probeApplicationUnprovision', []);
            runtime.dispose();
        }
    };
    let allowed = true;
    try {
        const resource = await QuerySourceResource.open({ endpoint, bytes, pins, current: () => allowed, target: () => endpoint });
        return {
            resource, refuse: () => { allowed = false; },
            async dispose() { await resource.dispose(); close(); },
        };
    } catch (error) { close(); throw error; }
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
