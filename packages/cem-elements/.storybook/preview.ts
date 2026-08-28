/// <reference types="vite/client" />

import { installCemElementRuntime, type CemElementRuntime } from '../src/index.js';
import '@epa-wg/cem-theme/styles.css';
import { definePreview } from '@storybook/web-components-vite';

let runtime: CemElementRuntime | undefined;
const declarationLoads = new Map<string, Promise<void>>();

const preview = definePreview({
    parameters: {
        controls: {
            disable: true,
        },
    },
    beforeAll: async () => {
        runtime = installCemElementRuntime(window);
    },
});

export function loadCemDeclaration(path: string, source: string): Promise<void> {
    const existing = declarationLoads.get(path);
    if (existing) return existing;

    const load = registerCemDeclaration(path, source);
    declarationLoads.set(path, load);
    return load;
}

async function registerCemDeclaration(path: string, source: string): Promise<void> {
    runtime ??= installCemElementRuntime(window);
    const parsed = document.createElement('template');
    parsed.innerHTML = source.trim();
    const declaration = parsed.content.firstElementChild;
    if (!(declaration instanceof HTMLElement) || declaration.localName !== 'cem-element') {
        throw new Error(`${path} must contain one root <cem-element> declaration`);
    }

    declaration.hidden = true;
    declaration.dataset.cemStorybookDeclaration = path;
    document.body.prepend(declaration);
    await runtime.whenDeclarationSettled(declaration);

    const errors = runtime
        .diagnosticsFor(declaration)
        .filter(({ severity }) => severity === 'error' || severity === 'fatal');
    if (errors.length > 0) {
        throw new Error(errors.map(({ code, message }) => `${code}: ${message}`).join('\n'));
    }
}

export async function whenCemRendered(instance: HTMLElement): Promise<void> {
    await customElements.whenDefined(instance.localName);
    await runtime?.whenRenderSettled(instance);
}

export default preview;
