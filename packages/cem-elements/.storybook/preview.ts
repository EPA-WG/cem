/// <reference types="vite/client" />

import { installCemElementRuntime, type CemElementRuntime } from '../src/index.js';
import '@epa-wg/cem-theme/styles.css';
import { definePreview } from '@storybook/web-components-vite';

let runtime: CemElementRuntime | undefined;
const declarationLoads = new Map<string, Promise<void>>();
const moduleUrlDemoUrl = new URL('../demo/module-url.html', import.meta.url);
// Resolve demo assets from the external declaration URL at runtime. Keeping the
// second argument dynamic prevents Vite from inlining small SVGs as data URLs,
// which would change the import-map address semantics being demonstrated.
const moduleUrlDemoSmileyUrl = new URL('./lib-dir/Smiley.svg', moduleUrlDemoUrl);
const moduleUrlDemoConfusedUrl = new URL('./confused.svg', moduleUrlDemoUrl);
const moduleUrlDemoSquareUrl = new URL('./wc-square.svg', moduleUrlDemoUrl);
const moduleUrlDemoRelativeReferrerUrl = new URL('./relative-referrer/', moduleUrlDemoUrl);
const moduleUrlDemoMappedReferrerUrl = new URL('./module-referrer/', moduleUrlDemoUrl);

function installStorybookRuntime(): CemElementRuntime {
    return installCemElementRuntime(window, {
        moduleUrlRoot: {
            importMap: {
                imports: {
                    '@epa-wg/cem-elements/demo/lib-dir/Smiley.svg': moduleUrlDemoSmileyUrl.href,
                    '@epa-wg/material': 'https://storybook.example.test/material/',
                    'demo-src-image': withSearch(moduleUrlDemoSmileyUrl, 'src', 'module'),
                    'demo-referrer-image': withSearch(moduleUrlDemoSmileyUrl, 'referrer', 'default'),
                    'demo-module-referrer': new URL('./module-referrer/component.js', moduleUrlDemoUrl).href,
                },
                scopes: {
                    [moduleUrlDemoRelativeReferrerUrl.href]: {
                        'demo-referrer-image': withSearch(moduleUrlDemoSmileyUrl, 'referrer', 'relative'),
                    },
                    [moduleUrlDemoMappedReferrerUrl.href]: {
                        'demo-referrer-image': withSearch(moduleUrlDemoConfusedUrl, 'referrer', 'module'),
                    },
                    'https://referrer.example.test/absolute/': {
                        'demo-referrer-image': withSearch(moduleUrlDemoSquareUrl, 'referrer', 'absolute'),
                    },
                },
            },
        },
    });
}

function withSearch(input: URL, name: string, value: string): string {
    const url = new URL(input);
    url.searchParams.set(name, value);
    return url.href;
}

const preview = definePreview({
    parameters: {
        controls: {
            disable: true,
        },
    },
    beforeAll: async () => {
        runtime = installStorybookRuntime();
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
    runtime ??= installStorybookRuntime();
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
