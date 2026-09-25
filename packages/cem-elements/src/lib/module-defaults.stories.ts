import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { expect } from 'storybook/test';
import { CemElementRuntime } from './cem-elements.js';

const meta: Meta = { title: 'CEM Elements/Module Declaration Defaults', tags: ['test'] };
export default meta;
type Story = StoryObj;

function moduleDefaults(mode: 'worker' | 'wasm'): Story {
    return {
        render: () => document.createElement('section'),
        play: async ({ canvasElement }) => {
            const declarationTag = `cem-module-defaults-${mode}`;
            const tag = `story-module-defaults-${mode}`;
            const runtime = new CemElementRuntime({ declarationTag });
            runtime.install(window);
            const declaration = document.createElement(declarationTag);
            declaration.setAttribute('tag', tag);
            const template = document.createElement('template');
            template.type = 'text/cem-ml';
            template.textContent = `{module |
    {slice @name=choice | first}{slice @name=enabled | false}
    {attribute @name=label | Default}
    {body |
        ${mode === 'wasm' ? '{location-element @slice=route @href="https://example.test/modules"}' : ''}
        {output | {$choice}|{$datadom.slices.choice}|{$enabled}|{$datadom.slices.enabled}|{$label}}
    }
}`;
            declaration.append(template);
            canvasElement.append(declaration);
            runtime.registerDeclaration(declaration);
            await runtime.whenDeclarationSettled(declaration);
            const first = document.createElement(tag);
            const second = document.createElement(tag);
            canvasElement.append(first, second);
            await Promise.all([runtime.whenRenderSettled(first), runtime.whenRenderSettled(second)]);
            const value = (instance: HTMLElement) => instance.querySelector('output')?.textContent?.trim();
            for (const instance of [first, second]) {
                expect(value(instance)).toBe('first|first|false|false|Default');
                expect(instance.getAttribute('label')).toBe('Default');
            }
            const output = first.querySelector('output');
            runtime.setInstanceSlices(first, { choice: '', enabled: true });
            await runtime.whenRenderSettled(first);
            expect(value(first)).toBe('||true|true|Default');
            first.setAttribute('label', '');
            await runtime.whenRenderSettled(first);
            expect(value(first)).toBe('||true|true|');
            expect(first.getAttribute('label')).toBe('');
            runtime.setInstanceSlices(first, { choice: 'next', enabled: false });
            await runtime.whenRenderSettled(first);
            expect(value(first)).toBe('next|next|false|false|');
            expect(first.querySelector('output')).toBe(output);
            expect(value(second)).toBe('first|first|false|false|Default');
            for (const target of [declaration, first, second]) expect(runtime.diagnosticsFor(target)).toEqual([]);
        },
    };
}

export const WorkerModuleDefaults = moduleDefaults('worker');
export const WasmModuleDefaults = moduleDefaults('wasm');
