import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime, type CemProducedElementBehaviorContext } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';
import { classifyRetainedPayloadElement, resolveRetainedPayloadContract, type CemRetainedPayloadContract } from './retained-payload-contract.js';

export default { title: 'CEM Elements/Retained Payload Declaration Contract', tags: ['test'] } satisfies Meta;
type Story = StoryObj;
const BODY = '{p | Declaration-owned payload vocabulary}';
const HEADER = { 'payload-item': 'entry option', 'payload-group': 'section optgroup', 'payload-topology': 'grouped' };
const ATTRS = Object.entries(HEADER).map(([name, value]) => `${name}="${value}"`).join(' ');
const EXPECTED = resolveRetainedPayloadContract(HEADER);
const assets = import.meta.glob<string>('../../../../dist/artifacts/cem_ql/declarations/*', { eager: true, query: '?url', import: 'default' });

function probe() {
    const constructed: (CemRetainedPayloadContract | null)[] = [], rendered: (CemRetainedPayloadContract | null)[] = [];
    const matches: ReturnType<typeof classifyRetainedPayloadElement>[][] = [];
    const inspect = (context: CemProducedElementBehaviorContext) => {
        rendered.push(context.payloadContract);
        const nodes = context.snapshot().payload.nodes;
        matches.push(nodes.filter(node => node.kind === 'element').map(node => classifyRetainedPayloadElement(context.payloadContract, node)));
        for (const node of nodes) if (node.kind === 'element') {
            const { key, children } = node;
            if (node.tag === 'entry') expect(classifyRetainedPayloadElement(context.payloadContract, node)).toEqual({ role: 'item', vocabulary: 0 });
            expect(node.key).toBe(key); expect(node.children).toBe(children);
        }
    };
    return { constructed, rendered, matches, options: { behaviorIdentity: 'test-retained-payload-observer-v1', behavior: {
        constructed(_host: HTMLElement, context: CemProducedElementBehaviorContext) { constructed.push(context.payloadContract); },
        beforeRender(_host: HTMLElement, context: CemProducedElementBehaviorContext) { inspect(context); },
    } } };
}
async function sourceContract(root: HTMLElement, fallback: boolean) {
    const scope = createCemDeclarationScope({ document }), suffix = crypto.randomUUID();
    let reads = 0;
    const runtime = new CemElementRuntime({ declarationTag: `payload-declaration-${suffix}`, declarationScope: scope,
        ...(fallback ? { processingWorkerFactory: () => { throw Error('fixture fallback'); } } : {}),
        loadSrcDocument: async () => { reads++; return `<main>
            <cem-element tag="source-payload" ${ATTRS}><template id="selected" type="text/cem-ml">${BODY}</template></cem-element>
            <cem-element tag="other-payload" payload-item="other"><template id="neighbor" type="text/cem-ml">${BODY}</template></cem-element>
            <cem-element tag="encoded-payload" payload-item="Q{urn:payload?a=1&amp;b=2}entr&#233;e"><template id="encoded" type="text/cem-ml">${BODY}</template></cem-element>
        </main>`; } });
    runtime.install(window);
    const declarations: HTMLElement[] = [], hosts: HTMLElement[] = [];
    const local = document.createElement('template');
    local.id = `payload-local-${suffix}`; local.setAttribute('type', 'text/cem-ml'); local.textContent = BODY;
    for (const [name, value] of Object.entries(HEADER)) local.setAttribute(name, value);
    root.append(local);
    try {
        for (const route of ['inline', 'local', 'remote', 'namespace', 'encoded']) {
            const tag = `payload-${route}-${suffix}`, declaration = document.createElement(runtime.declarationTag), observer = probe();
            declaration.setAttribute('tag', tag);
            const header: Record<string, string> = route === 'namespace' ? { 'payload-item': 'Q{urn:retained}entry', 'payload-topology': 'recursive' }
                : route === 'encoded' ? { 'payload-item': 'Q{urn:payload?a=1&b=2}entrée' } : HEADER;
            if (route === 'inline' || route === 'namespace') {
                for (const [name, value] of Object.entries(header)) declaration.setAttribute(name, value);
                const template = document.createElement('template'); template.setAttribute('type', 'text/cem-ml'); template.textContent = BODY; declaration.append(template);
            } else declaration.setAttribute('src', route === 'local' ? `#${local.id}` : `https://payload.example.test/bundle.xhtml#${route === 'encoded' ? 'encoded' : 'selected'}`);
            const host = document.createElement(tag); host.setAttribute('payload-item', 'forged');
            host.innerHTML = route === 'namespace' ? '<first:entry xmlns:first="urn:retained">Retained source</first:entry><wrong:entry xmlns:wrong="urn:other">Other source</wrong:entry>'
                : route === 'encoded' ? '<data:entrée xmlns:data="urn:payload?a=1&amp;b=2">Retained source</data:entrée>'
                : '<entry value="one"><strong>Retained source</strong></entry>'; root.append(host); hosts.push(host);
            declarations.push(declaration); runtime.registerDeclaration(declaration, observer.options); root.append(declaration);
            await runtime.whenDeclarationSettled(declaration);
            expect(runtime.diagnosticsFor(declaration)).toEqual([]);
            await runtime.whenRenderSettled(host);
            expect(observer.constructed).toEqual([resolveRetainedPayloadContract(header)]);
            if (route === 'namespace') expect(observer.matches[0]).toEqual([{ role: 'item', vocabulary: 0 }, null]);
            if (route === 'encoded') expect(observer.matches[0]).toEqual([{ role: 'item', vocabulary: 0 }]);
            const original = observer.constructed[0];
            expect(Object.isFrozen(original?.vocabularies[0].item)).toBe(true);
            expect(Reflect.set(original!.vocabularies[0].item, 'localName', 'forged')).toBe(false);
            expect(observer.rendered.length).toBeGreaterThan(0);
            expect(observer.rendered.every(contract => contract === original)).toBe(true);
            host.setAttribute('payload-topology', 'recursive'); host.remove(); root.append(host);
            await runtime.whenRenderSettled(host);
            expect(observer.rendered.every(contract => contract === original)).toBe(true);
        }
        // The cached remote document is shared, but each selected declaration owns its metadata.
        const neighbor = document.createElement(runtime.declarationTag), observer = probe();
        neighbor.setAttribute('tag', `payload-neighbor-${suffix}`); neighbor.setAttribute('src', 'https://payload.example.test/bundle.xhtml#neighbor');
        declarations.push(neighbor); runtime.registerDeclaration(neighbor, observer.options); root.append(neighbor);
        await runtime.whenDeclarationSettled(neighbor);
        const host = document.createElement(`payload-neighbor-${suffix}`); hosts.push(host); root.append(host); await runtime.whenRenderSettled(host);
        expect(observer.constructed[0]).toEqual(resolveRetainedPayloadContract({ 'payload-item': 'other' }));
        expect(reads).toBe(1);
        // An unrelated ancestor cannot supply declaration metadata.
        const wrapper = document.createElement('div'); wrapper.setAttribute('payload-item', 'unrelated');
        const plain = document.createElement('template'); plain.id = `plain-${suffix}`; plain.setAttribute('type', 'text/cem-ml'); plain.textContent = BODY; wrapper.append(plain); root.append(wrapper);
        const plainDeclaration = document.createElement(runtime.declarationTag), plainObserver = probe();
        const plainTag = `payload-plain-${suffix}`; plainDeclaration.setAttribute('tag', plainTag); plainDeclaration.setAttribute('src', `#${plain.id}`);
        declarations.push(plainDeclaration); runtime.registerDeclaration(plainDeclaration, plainObserver.options); root.append(plainDeclaration);
        await runtime.whenDeclarationSettled(plainDeclaration); const plainHost = document.createElement(plainTag); hosts.push(plainHost); root.append(plainHost);
        await runtime.whenRenderSettled(plainHost); expect(plainObserver.constructed).toEqual([null]); wrapper.remove();
        // Invalid inline blocks are refused before a constructor can be published.
        const invalid = document.createElement(runtime.declarationTag), badTag = `payload-invalid-${suffix}`;
        invalid.setAttribute('tag', badTag); invalid.setAttribute('payload-item', 'item');
        const badTemplate = document.createElement('template'); badTemplate.setAttribute('type', 'text/cem-ml'); badTemplate.setAttribute('payload-group', 'group'); badTemplate.textContent = BODY;
        invalid.append(badTemplate); declarations.push(invalid); root.append(invalid); await runtime.whenDeclarationSettled(invalid);
        expect(customElements.get(badTag)).toBeUndefined();
        expect(runtime.diagnosticsFor(invalid).some(item => item.code === 'cem-element.payload_contract_invalid')).toBe(true);
    } finally { hosts.forEach(host => host.remove()); declarations.forEach(declaration => declaration.remove()); local.remove(); scope.dispose(); }
}

async function compiledContract(root: HTMLElement, fallback: boolean) {
    const mode = fallback ? 'fallback' : 'worker', prefix = `../../../../dist/artifacts/cem_ql/declarations/${mode}-payload`;
    const load = async (extension: string) => { const url = assets[`${prefix}.${extension}`]; if (!url) throw Error('Build native payload declaration fixtures'); return fetch(url); };
    const input = { metadataJson: await (await load('json')).text(), metadataHash: (await (await load('hash')).text()).trim(), templateBytes: await (await load('bin')).arrayBuffer() };
    const scope = createCemDeclarationScope({ document }), runtime = new CemElementRuntime({ declarationTag: `pinned-payload-${crypto.randomUUID()}`, declarationScope: scope,
        ...(fallback ? { processingWorkerFactory: () => { throw Error('fixture fallback'); } } : {}) });
    runtime.install(window);
    const declaration = document.createElement(runtime.declarationTag), tag = `cem-aot-payload-${mode}`, observer = probe();
    declaration.setAttribute('tag', tag); declaration.setAttribute('payload-item', 'forged');
    let host: HTMLElement | undefined;
    try {
        await expect(runtime.registerCompiledDeclaration(declaration, input, observer.options)).rejects.toThrow('override payload contract');
        expect(customElements.get(tag)).toBeUndefined(); declaration.removeAttribute('payload-item');
        await runtime.registerCompiledDeclaration(declaration, input, observer.options); root.append(declaration);
        host = document.createElement(tag); root.append(host); await runtime.whenRenderSettled(host);
        expect(observer.constructed).toEqual([EXPECTED]);
        expect(observer.rendered.every(contract => contract === observer.constructed[0])).toBe(true);
    } finally { host?.remove(); declaration.remove(); scope.dispose(); }
}
export const SourceWorker: Story = { render: () => '<section></section>', play: ({ canvasElement }) => sourceContract(canvasElement, false) };
export const SourceFallback: Story = { render: () => '<section></section>', play: ({ canvasElement }) => sourceContract(canvasElement, true) };
export const CompiledWorker: Story = { render: () => '<section></section>', play: ({ canvasElement }) => compiledContract(canvasElement, false) };
export const CompiledFallback: Story = { render: () => '<section></section>', play: ({ canvasElement }) => compiledContract(canvasElement, true) };
