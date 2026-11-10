import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { Worker, isMainThread, parentPort, workerData } from 'node:worker_threads';

const output = resolve(isMainThread ? process.argv[2] : workerData.output);
const binary = await readFile(resolve(output, 'application_source_probe_bg.wasm'));
const source = await readFile(resolve(output, 'application.cemah'));
const pins = await readFile(resolve(output, 'application.pin.json'), 'utf8');
const [typedSource, typedPins, typedInput] = await Promise.all([
    readFile(resolve(output, 'typed.cemah')),
    readFile(resolve(output, 'typed.pin.json'), 'utf8'),
    readFile(resolve(output, 'typed.inputs')),
]);
const [nestedSource, nestedPins, nestedInput, badNestedInput] = await Promise.all([
    readFile(resolve(output, 'nested.cemah')),
    readFile(resolve(output, 'nested.pin.json'), 'utf8'),
    readFile(resolve(output, 'nested.inputs')),
    readFile(resolve(output, 'nested.bad.inputs')),
]);
const [styledSource, styledPins, styledInput] = await Promise.all([
    readFile(resolve(output, 'styled.cemah')),
    readFile(resolve(output, 'styled.pin.json'), 'utf8'),
    readFile(resolve(output, 'styled.inputs')),
]);
const [navigationSource, navigationPins, navigationInput, island] = await Promise.all([
    readFile(resolve(output, 'navigation.cemah')),
    readFile(resolve(output, 'navigation.pin.json'), 'utf8'),
    readFile(resolve(output, 'navigation.inputs')),
    readFile(resolve(output, 'frame.island')),
]);
const [providerSource, providerPins, providerInput] = await Promise.all([
    readFile(resolve(output, 'provider.cemah')),
    readFile(resolve(output, 'provider.pin.json'), 'utf8'),
    readFile(resolve(output, 'provider.inputs')),
]);

async function verify(heap) {
    const url = pathToFileURL(resolve(output, 'application_source_probe.js'));
    url.searchParams.set('heap', heap);
    const reader = await import(url.href);
    const wasm = await reader.default({ module_or_path: binary });
    const read = (bytes = source, pin = pins, cancelled = false) => reader.probeApplicationSources(bytes, pin, cancelled);
    assert.equal(read(), '3:5:1:2');
    assert.throws(() => read(source, pins, true));
    assert.throws(() => read(source.subarray(0, 10)));
    assert.throws(() => read(new Uint8Array(4 * 1024 * 1024 + 1)));
    assert.throws(() => read(source, '{'));
    const changed = JSON.parse(pins);
    changed.application.registry_hash = changed.hash;
    assert.throws(() => read(source, JSON.stringify(changed)));
    changed.hash = 'foreign';
    assert.throws(() => read(source, JSON.stringify(changed)));
    assert.equal(read(), '3:5:1:2', 'refusal must release temporary sources and leave reader usable');
    assert.equal(typeof reader.applicationSourceOpenV1, 'function');
    assert.throws(() => reader.applicationSourceOpenV1(source, pins), /application.endpoint.refused/);
    assert.throws(() => reader.applicationSourceCaptureV1(Object.freeze({})), /application.endpoint.refused/);
    assert.throws(() => reader.applicationSourceReleaseV1(Object.freeze({})), /application.endpoint.refused/);

    reader.probeApplicationProvision(typedPins, typedInput);
    const owner = reader.applicationSourceOpenV1(typedSource, typedPins);
    const ordinary = reader.probeApplicationReceiverInput(owner);
    const render = input => reader.applicationSourceRenderReceiverV1(owner, input, 'typed');
    assert.throws(() => render(ordinary), /application.endpoint.refused/);
    reader.applicationSourceReleaseReceiverInputV1(owner, ordinary);
    const typed = reader.probeApplicationTypedReceiverInput(owner);
    assert.equal(render(typed), '<p>Tea/3/typed/typed</p>');
    assert.throws(() => render(Object.freeze({ ...typed })), /application.endpoint.refused/);
    assert.ok(reader.applicationSourceProjectReceiverV1(owner, typed, 'typed').length);
    assert.throws(() => reader.applicationSourceCaptureV1(owner), /application.endpoint.refused/);
    reader.probeApplicationDropTypedProducer();
    assert.throws(() => render(typed), /application.endpoint.refused/);
    assert.throws(() => reader.applicationSourceProjectReceiverV1(owner, typed, 'typed'), /application.endpoint.refused/);
    reader.applicationSourceReleaseReceiverInputV1(owner, typed);
    assert.deepEqual(Buffer.from(reader.applicationSourceCaptureV1(owner)), typedSource);
    reader.applicationSourceReleaseV1(owner);
    reader.probeApplicationUnprovision();

    for (const input of [nestedInput, badNestedInput]) {
        reader.probeApplicationProvision(nestedPins, input);
        const owner = reader.applicationSourceOpenV1(nestedSource, nestedPins);
        const receiver = reader.probeApplicationReceiverInput(owner);
        const render = () => reader.applicationSourceRenderReceiverV1(owner, receiver, 'nested');
        const project = () => reader.applicationSourceProjectReceiverV1(owner, receiver, 'nested');
        if (input === nestedInput) {
            assert.equal(render(), '<p>Tea/3/ok/4/leaf/Tea/detail/nested</p>');
            assert.ok(project().length);
        } else {
            assert.throws(render, /application.endpoint.refused/);
            assert.throws(project, /application.endpoint.refused/);
        }
        reader.applicationSourceReleaseV1(owner);
        reader.probeApplicationUnprovision();
    }

    reader.probeApplicationProvision(styledPins, styledInput);
    assert.throws(() => reader.applicationSourceOpenV1(styledSource, styledPins), /application.endpoint.refused/);
    reader.probeApplicationUnprovision();
    reader.probeApplicationProvisionStyled(styledPins, styledInput, true);
    assert.throws(() => reader.applicationSourceOpenV1(styledSource, styledPins), /application.endpoint.refused/);
    reader.probeApplicationUnprovision();
    reader.probeApplicationProvisionStyled(styledPins, styledInput, false);
    const styled = reader.applicationSourceOpenV1(styledSource, styledPins);
    const styleTag = 'cem-receiver-frame';
    const [style] = JSON.parse(reader.applicationSourceStylesheetsV1(styled, styleTag));
    assert.equal(style.index, 0);
    assert.match(style.stylesheet.css, /--imported:ready/);
    assert.match(style.stylesheet.css, /--original:ready/);
    assert.ok(style.stylesheet.identity.cacheKey);
    assert.throws(() => reader.applicationSourceStylesheetsV1({ ...styled }, styleTag), /application.endpoint.refused/);
    const styledReceiver = reader.probeApplicationReceiverInput(styled);
    assert.equal(reader.applicationSourceRenderReceiverV1(styled, styledReceiver, 'styled'), '<p>Tea/3/ok/styled</p>');
    reader.probeApplicationRetire();
    assert.throws(() => reader.applicationSourceStylesheetsV1(styled, styleTag), /application.endpoint.refused/);
    reader.applicationSourceReleaseV1(styled);
    reader.probeApplicationUnprovision();

    reader.probeApplicationProvision(navigationPins, navigationInput);
    assert.throws(() => reader.applicationSourceOpenV1(navigationSource, navigationPins), /application.endpoint.refused/);
    reader.probeApplicationUnprovision();
    reader.probeApplicationProvisionNavigation(navigationPins, navigationInput);
    const navigation = reader.applicationSourceOpenV1(navigationSource, navigationPins);
    const renderNavigation = (owner, instance = 'island-one') =>
        reader.applicationSourceRenderBrowserIslandV1(owner, 'cem-count', island, instance);
    const frame = JSON.parse(renderNavigation(navigation));
    assert.equal(frame.behaviorIdentity, 'cem-elements-navigation-landmark-v1');
    assert.match(frame.html, /<nav.*<button[^>]+>3<\/button><\/nav>/);
    assert.throws(() => renderNavigation({ ...navigation }), /application.endpoint.refused/);
    assert.throws(() => renderNavigation(navigation, 'foreign'), /application.endpoint.refused/);
    assert.throws(() => reader.applicationSourceRenderIslandV1(navigation, 'cem-count', island, 'island-one'));
    reader.applicationSourceReleaseV1(navigation);
    assert.throws(() => renderNavigation(navigation), /application.endpoint.refused/);
    reader.probeApplicationUnprovision();

    reader.probeApplicationProvisionProvider(providerPins, providerInput);
    const provider = reader.applicationSourceOpenV1(providerSource, providerPins);
    const providerFrame = (owner = provider, instance = 'source-route') =>
        reader.applicationSourceRenderBrowserReceiverV1(owner, 'cem-receiver-frame', instance);
    assert.throws(() => providerFrame(), /application.endpoint.refused/);
    const originalInput = reader.probeApplicationReceiverInput(provider);
    const text = JSON.parse(providerFrame());
    assert.equal(text.value, 'Tea');
    assert.equal(text.behaviorIdentity, 'cem-elements-form-control-v4');
    assert.throws(() => providerFrame({ ...provider }), /application.endpoint.refused/);
    assert.throws(() => providerFrame(provider, 'foreign'), /application.endpoint.refused/);
    assert.throws(() => reader.applicationSourceRenderBrowserIslandV1(provider, 'cem-receiver-frame', island, 'source-route'), /application.endpoint.refused/);
    reader.applicationSourceReleaseReceiverInputV1(provider, originalInput);
    assert.throws(() => providerFrame(), /application.endpoint.refused/);
    reader.applicationSourceReleaseV1(provider);
    reader.probeApplicationUnprovision();

    return wasm.memory;
}

if (isMainThread) {
    const first = await verify('first');
    const fresh = await verify('fresh');
    assert.notEqual(first, fresh, 'fresh native source trees belong to separate heaps');
    const worker = new Worker(new URL(import.meta.url), { workerData: { output } });
    await new Promise((resolve, reject) => {
        worker.once('error', reject);
        worker.once('exit', code => code === 0 ? resolve() : reject(Error(`reader worker exited ${code}`)));
    });
    console.log(`Application sources, receivers, styles and browser frames passed in two fresh heaps and a real worker; ${source.length} source bytes.`);
} else {
    await verify('worker');
    parentPort.close();
}
