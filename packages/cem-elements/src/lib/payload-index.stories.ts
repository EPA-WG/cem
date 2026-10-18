import { expect } from 'storybook/test';
import preview from '../../.storybook/preview.js';
import { CemElementRuntime } from './cem-elements.js';
import { createCemDeclarationScope } from './declaration-scope.js';

const meta = preview.meta({ title: 'Runtime/Payload indexes' });
let sequence = 0;

export const LargePayload = meta.story({
    render: () => '<p>Large inert payload indexes preserve order and snapshot independence.</p>',
    play: async () => {
        const scope = createCemDeclarationScope({ document });
        const runtime = new CemElementRuntime({ declarationTag: 'payload-index-definition', declarationScope: scope });
        const tag = `payload-index-${++sequence}`;
        const declaration = document.createElement('payload-index-definition');
        declaration.setAttribute('tag', tag);
        declaration.innerHTML = '<template><p>Payload index</p></template>';
        runtime.registerDeclaration(declaration);
        const instance = document.createElement(tag);
        const count = 4096;
        instance.innerHTML = Array.from({ length: count }, (_, index) =>
            `<span data-group="item" slot="items">Item ${index}</span>`).join('');
        try {
            const first = runtime.snapshotInstance(instance).payload;
            expect(first.slots.items).toHaveLength(count);
            expect(first.elementsByAttribute['data-group'].map(item => item.text))
                .toEqual(Array.from({ length: count }, (_, index) => `Item ${index}`));
            const payload = instance.querySelector('template')?.content.querySelector('[data-group]');
            if (!payload) throw new Error('Expected the captured payload item');
            payload.remove();
            const second = runtime.snapshotInstance(instance).payload;
            expect(second.slots.items).toHaveLength(count - 1);
            expect(second.elementsByAttribute['data-group'][0].text).toBe('Item 1');
            expect(first.elementsByAttribute['data-group'][0].text).toBe('Item 0');
            expect(first.slots.items).toHaveLength(count);
        } finally { scope.dispose(); }
    },
});

export const AuthoredIndexNames = meta.story({
    render: () => '<p>Authored slot and attribute names are ordinary index keys.</p>',
    play: async () => {
        const scope = createCemDeclarationScope({ document });
        const runtime = new CemElementRuntime({ declarationTag: 'payload-index-definition', declarationScope: scope });
        const tag = `payload-index-${++sequence}`;
        const declaration = document.createElement('payload-index-definition');
        declaration.setAttribute('tag', tag);
        declaration.innerHTML = '<template><p>Payload index</p></template>';
        runtime.registerDeclaration(declaration);
        const instance = document.createElement(tag);
        instance.innerHTML = '<span slot="__proto__" constructor="authored" __proto__="authored">First</span><span slot="constructor" constructor="authored">Second</span>';
        try {
            const payload = runtime.snapshotInstance(instance).payload;
            for (const snapshot of [payload, structuredClone(payload)]) {
                expect(Object.keys(snapshot.slots)).toEqual(['__proto__', 'constructor']);
                for (const name of ['__proto__', 'constructor']) {
                    expect(Object.hasOwn(snapshot.slots, name)).toBe(true);
                    expect(snapshot.slots[name]).toHaveLength(1);
                    expect(snapshot.elementsByAttribute[name].map(item => item.text))
                        .toEqual(name === 'constructor' ? ['First', 'Second'] : ['First']);
                }
            }
        } finally { scope.dispose(); }
    },
});
