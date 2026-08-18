import type { Meta, StoryObj } from '@storybook/web-components-vite';
import actionCemFixture from '../../tests/parity/material/action.cem.html?raw';
import actionLegacyFixture from '../../tests/parity/material/action.legacy.html?raw';
import autocompleteCemFixture from '../../tests/parity/material/autocomplete.cem.html?raw';
import autocompleteLegacyFixture from '../../tests/parity/material/autocomplete.legacy.html?raw';
import badgeCemFixture from '../../tests/parity/material/badge.cem.html?raw';
import badgeLegacyFixture from '../../tests/parity/material/badge.legacy.html?raw';
import dropdownCemFixture from '../../tests/parity/material/dropdown.cem.html?raw';
import dropdownLegacyFixture from '../../tests/parity/material/dropdown.legacy.html?raw';
import iconCemFixture from '../../tests/parity/material/icon.cem.html?raw';
import iconLegacyFixture from '../../tests/parity/material/icon.legacy.html?raw';
import iconLinkCemFixture from '../../tests/parity/material/icon-link.cem.html?raw';
import iconLinkLegacyFixture from '../../tests/parity/material/icon-link.legacy.html?raw';
import inputCemFixture from '../../tests/parity/material/input.cem.html?raw';
import inputLegacyFixture from '../../tests/parity/material/input.legacy.html?raw';
import menuCemFixture from '../../tests/parity/material/menu.cem.html?raw';
import menuLegacyFixture from '../../tests/parity/material/menu.legacy.html?raw';
import {
    accessibleName,
    assertPhase3Accessibility,
} from '../../.storybook/accessibility-contract.js';
import { CemElementRuntime } from './cem-elements.js';

/**
 * Direct, file-backed material migration evidence.
 *
 * Both sides intentionally retain the production `cem-*` names, so each pair runs in two
 * same-origin iframe documents with independent CustomElementRegistry instances. The legacy
 * side applies the documented thin-adapter rule at ingestion: `<custom-element>` declarations
 * are handed to `<cem-element>` and their templates opt in with `lang="custom-element-v0"`.
 * Fixture bytes stay unchanged on disk.
 */

const meta: Meta = {
    title: 'CEM Elements/Material File Parity',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;
type MaterialId = 'icon' | 'icon-link' | 'menu' | 'badge' | 'action' | 'dropdown' | 'input' | 'autocomplete';
type MaterialMode = 'legacy' | 'cem-ml';

interface MaterialFixtureDefinition {
    id: MaterialId;
    tag: string;
    imports: MaterialId[];
    legacySource: string;
    cemMlSource: string;
    attributes: Record<string, string>;
    payload: string;
    assertShared(instance: HTMLElement, mode: MaterialMode): Promise<void>;
    assertMigration?(instance: HTMLElement, mode: MaterialMode): Promise<void>;
}

interface MaterialFrameState {
    frame: HTMLIFrameElement;
    loaded: Promise<void>;
}

interface MaterialStoryState {
    legacy: MaterialFrameState;
    cemMl: MaterialFrameState;
}

interface MaterialRuntimeState {
    document: Document;
    runtime: CemElementRuntime;
    declarations: HTMLElement[];
    instance: HTMLElement;
}

const materialOrder: MaterialId[] = ['icon', 'icon-link', 'menu', 'badge', 'action', 'dropdown', 'input', 'autocomplete'];
const storyStates = new WeakMap<HTMLElement, MaterialStoryState>();

const fixtureSources: Record<MaterialId, Pick<MaterialFixtureDefinition, 'tag' | 'imports' | 'legacySource' | 'cemMlSource'>> = {
    icon: {
        tag: 'cem-icon',
        imports: ['icon-link'],
        legacySource: iconLegacyFixture,
        cemMlSource: iconCemFixture,
    },
    'icon-link': {
        tag: 'cem-icon-link',
        imports: [],
        legacySource: iconLinkLegacyFixture,
        cemMlSource: iconLinkCemFixture,
    },
    menu: {
        tag: 'cem-menu',
        imports: ['icon-link'],
        legacySource: menuLegacyFixture,
        cemMlSource: menuCemFixture,
    },
    badge: {
        tag: 'cem-badge',
        imports: ['icon-link', 'icon'],
        legacySource: badgeLegacyFixture,
        cemMlSource: badgeCemFixture,
    },
    action: {
        tag: 'cem-action',
        imports: ['icon-link', 'icon'],
        legacySource: actionLegacyFixture,
        cemMlSource: actionCemFixture,
    },
    dropdown: {
        tag: 'cem-dropdown',
        imports: ['icon-link', 'menu'],
        legacySource: dropdownLegacyFixture,
        cemMlSource: dropdownCemFixture,
    },
    input: {
        tag: 'cem-input',
        imports: ['icon-link', 'icon'],
        legacySource: inputLegacyFixture,
        cemMlSource: inputCemFixture,
    },
    autocomplete: {
        tag: 'cem-autocomplete',
        imports: ['icon-link', 'input', 'menu'],
        legacySource: autocompleteLegacyFixture,
        cemMlSource: autocompleteCemFixture,
    },
};

const materialFixtures: Record<MaterialId, MaterialFixtureDefinition> = {
    icon: {
        id: 'icon',
        ...fixtureSources.icon,
        attributes: { image: 'home', direction: 'row' },
        payload: 'Icon label',
        async assertShared(instance) {
            const icon = await waitForElement(instance, '.material-icons');
            assertEqual(icon.textContent?.trim(), 'home', 'icon image renders as a material icon');
            assert(instance.textContent?.includes('Icon label'), 'icon projects its default-slot label');
        },
    },
    'icon-link': {
        id: 'icon-link',
        ...fixtureSources['icon-link'],
        attributes: {
            href: 'https://cdn.example.test/@epa-wg/material',
            icon: 'home',
        },
        payload: 'Home',
        async assertShared(instance) {
            const anchor = await waitForElement(instance, 'a');
            assertEqual(
                anchor.getAttribute('href'),
                'https://cdn.example.test/@epa-wg/material',
                'icon-link resolves its navigation URL'
            );
            assert(anchor.textContent?.includes('Home'), 'icon-link projects its accessible label');
            const icon = await waitForElement(instance, '.material-icons');
            assertEqual(icon.textContent?.trim(), 'home', 'icon-link renders its material icon');
        },
        async assertMigration(instance, mode) {
            if (mode !== 'cem-ml') return;
            const logo = await waitForElement(instance, 'img.resolved-logo');
            assertEqual(
                logo.getAttribute('src'),
                'https://cdn.example.test/@epa-wg/custom-element/demo/wc-square.svg',
                'CEM-ML module-url migration resolves the logo resource'
            );
            assertEqual(logo.getAttribute('alt'), '', 'the migrated resource logo stays decorative');
        },
    },
    menu: {
        id: 'menu',
        ...fixtureSources.menu,
        attributes: { direction: 'column', justify: 'start' },
        payload: '<a href="#one">One</a><a href="#two">Two</a>',
        async assertShared(instance) {
            const nav = await waitForElement(instance, 'nav.cem-menu');
            assert(nav.classList.contains('column'), 'menu applies its direction attribute');
            assertEqual(nav.querySelectorAll('a').length, 2, 'menu projects both navigation links');
            assertEqual(nav.querySelector('a')?.textContent, 'One', 'menu preserves projected link text');
        },
    },
    badge: {
        id: 'badge',
        ...fixtureSources.badge,
        attributes: { text: '5', color: 'alert' },
        payload: '<button type="button">Inbox</button>',
        async assertShared(instance) {
            const badge = await waitForElement(instance, '.badge-dd');
            assertEqual(badge.textContent?.trim(), '5', 'badge renders its text value');
            assertEqual(
                requiredElement(instance, 'button').textContent?.trim(),
                'Inbox',
                'badge projects its labelled control'
            );
        },
        async assertMigration(instance, mode) {
            if (mode !== 'cem-ml') return;
            assert(
                requiredElement(instance, '.cem-badge').classList.contains('alert'),
                'CEM-ML badge migration applies the host color'
            );
        },
    },
    action: {
        id: 'action',
        ...fixtureSources.action,
        attributes: {},
        payload: 'Submit',
        async assertShared(instance) {
            const button = await waitForElement(instance, 'button');
            assertEqual(button.getAttribute('type') ?? 'button', 'button', 'action renders a native button');
            assert(button.textContent?.includes('Submit'), 'action projects its button label');
        },
        async assertMigration(instance, mode) {
            if (mode !== 'cem-ml') return;
            (requiredElement(instance, 'button') as HTMLButtonElement).click();
            await waitForCondition(
                () => instance.querySelector('.state')?.textContent?.trim() === 'on',
                'CEM-ML action migration updates its pressed slice'
            );
        },
    },
    dropdown: {
        id: 'dropdown',
        ...fixtureSources.dropdown,
        attributes: { label: 'File' },
        payload: '<a href="#new">New</a>',
        async assertShared(instance) {
            await waitForCondition(
                () => instance.textContent?.includes('File') ?? false,
                'dropdown renders its host label'
            );
        },
        async assertMigration(instance, mode) {
            if (mode === 'legacy') {
                const menu = await waitForElement(instance, 'cem-menu nav.cem-menu');
                assertEqual(menu.querySelector('a')?.textContent?.trim(), 'New', 'legacy dropdown composes its menu');
                return;
            }
            const button = await waitForElement(instance, 'button');
            assertEqual(button.getAttribute('aria-expanded'), 'false', 'migrated dropdown starts closed');
            (button as HTMLButtonElement).click();
            const panel = await waitForElement(instance, '.panel');
            assertEqual(panel.querySelector('a')?.textContent?.trim(), 'New', 'migrated dropdown opens its items');
            assertEqual(
                requiredElement(instance, 'button').getAttribute('aria-expanded'),
                'true',
                'migrated dropdown reflects its open state'
            );
        },
    },
    input: {
        id: 'input',
        ...fixtureSources.input,
        attributes: { type: 'email', value: 'a@b.com' },
        payload:
            '<span slot="label">Email</span>' +
            '<input slot="input" type="email" aria-label="Email" value="a@b.com" />',
        async assertShared(instance) {
            const input = await waitForElement(instance, 'input');
            assertEqual(input.getAttribute('type'), 'email', 'input forwards its type');
            assertEqual(input.getAttribute('value'), 'a@b.com', 'input forwards its value');
            assertEqual(accessibleName(input), 'Email', 'input exposes its accessible name');
            assert(instance.textContent?.includes('Email'), 'input projects its named label slot');
        },
    },
    autocomplete: {
        id: 'autocomplete',
        ...fixtureSources.autocomplete,
        attributes: { value: 'a', label: 'Search fruit' },
        payload:
            '<cem-input slot="input" label="Search fruit" value="a">' +
            '<span slot="label">Search fruit</span>' +
            '<input slot="input" type="text" aria-label="Search fruit" value="a" />' +
            '</cem-input>' +
            '<data value="apple">Apple</data><data value="banana">Banana</data>',
        async assertShared(instance) {
            const input = await waitForElement(instance, 'cem-input input');
            assertEqual(input.getAttribute('value'), 'a', 'autocomplete projects its authored input value');
            assertEqual(accessibleName(input), 'Search fruit', 'autocomplete exposes its input name');
        },
        async assertMigration(instance, mode) {
            if (mode !== 'cem-ml') return;
            const input = await waitForElement(instance, 'input');
            assertEqual(input.getAttribute('value'), 'a', 'CEM-ML autocomplete forwards its value');
            const options = Array.from(instance.querySelectorAll('.options .opt'), (option) =>
                option.textContent?.trim()
            );
            assertEqual(options.join('|'), 'Apple|Banana', 'CEM-ML autocomplete renders its data payload');
        },
    },
};

export const FileMaterialIconParity: Story = materialFileStory(materialFixtures.icon);
export const FileMaterialIconLinkParity: Story = materialFileStory(materialFixtures['icon-link']);
export const FileMaterialMenuParity: Story = materialFileStory(materialFixtures.menu);
export const FileMaterialBadgeParity: Story = materialFileStory(materialFixtures.badge);
export const FileMaterialActionParity: Story = materialFileStory(materialFixtures.action);
export const FileMaterialDropdownParity: Story = materialFileStory(materialFixtures.dropdown);
export const FileMaterialInputParity: Story = materialFileStory(materialFixtures.input);
export const FileMaterialAutocompleteParity: Story = materialFileStory(materialFixtures.autocomplete);

function materialFileStory(fixture: MaterialFixtureDefinition): Story {
    return {
        render: () => {
            const root = document.createElement('section');
            root.dataset.materialFileParity = fixture.id;
            root.setAttribute('aria-label', `file-backed material parity: ${fixture.id}`);
            const legacy = createFixtureFrame(fixture.id, 'legacy');
            const cemMl = createFixtureFrame(fixture.id, 'cem-ml');
            root.append(legacy.frame, cemMl.frame);
            storyStates.set(root, { legacy, cemMl });
            return root;
        },
        play: async ({ canvasElement }) => {
            const root = requiredElement(
                canvasElement,
                `section[data-material-file-parity="${fixture.id}"]`
            ) as HTMLElement;
            const state = storyStates.get(root);
            assert(state, `${fixture.id} isolated story state exists`);
            await Promise.all([state.legacy.loaded, state.cemMl.loaded]);

            const legacy = await registerMaterialFixture(state.legacy.frame, fixture, 'legacy');
            const cemMl = await registerMaterialFixture(state.cemMl.frame, fixture, 'cem-ml');
            await fixture.assertShared(legacy.instance, 'legacy');
            await fixture.assertShared(cemMl.instance, 'cem-ml');
            await fixture.assertMigration?.(legacy.instance, 'legacy');
            await fixture.assertMigration?.(cemMl.instance, 'cem-ml');
            assertPhase3Accessibility([legacy.instance], `${fixture.id} legacy material parity`);
            assertPhase3Accessibility([cemMl.instance], `${fixture.id} CEM-ML material parity`);

            assert(
                legacy.document.defaultView?.customElements.get(fixture.tag),
                `legacy ${fixture.tag} is registered in its document`
            );
            assert(
                cemMl.document.defaultView?.customElements.get(fixture.tag),
                `CEM-ML ${fixture.tag} is registered in its document`
            );
            assert(
                legacy.document.defaultView?.customElements !== cemMl.document.defaultView?.customElements,
                `${fixture.id} parity sides use isolated custom-element registries`
            );
        },
    };
}

function createFixtureFrame(id: MaterialId, mode: MaterialMode): MaterialFrameState {
    const frame = document.createElement('iframe');
    frame.title = `${id} ${mode} material fixture`;
    frame.dataset.materialSide = mode;
    frame.style.width = '100%';
    frame.style.minHeight = '10rem';
    const loaded = new Promise<void>((resolve) => frame.addEventListener('load', () => resolve(), { once: true }));
    frame.srcdoc = [
        '<!doctype html><html><head>',
        '<base href="https://fixtures.example.test/material/">',
        '<style>body { font-family: sans-serif; margin: 0.5rem; }</style>',
        '</head><body></body></html>',
    ].join('');
    return { frame, loaded };
}

async function registerMaterialFixture(
    frame: HTMLIFrameElement,
    fixture: MaterialFixtureDefinition,
    mode: MaterialMode
): Promise<MaterialRuntimeState> {
    const document = frame.contentDocument;
    assert(document?.defaultView, `${fixture.id} ${mode} frame exposes a same-origin document`);
    const runtime = new CemElementRuntime({
        declarationTag: `cem-material-file-${mode}-${fixture.id}`,
        loadSrcDocument: async (path) => adaptLegacySource(legacySourceForPath(path), document),
        resolveModuleUrl: resolveFixtureModuleUrl,
    });

    const selectedIds = materialOrder.filter((id) => fixture.imports.includes(id) || id === fixture.id);
    const declarations: HTMLElement[] = [];
    for (const id of selectedIds) {
        const declaration = materialDeclaration(document, fixture, id, mode);
        document.body.appendChild(declaration);
        assert(runtime.registerDeclaration(declaration), `${fixture.id} ${mode} registers ${fixtureSources[id].tag}`);
        await runtime.whenDeclarationSettled(declaration);
        assert(
            document.defaultView.customElements.get(fixtureSources[id].tag),
            `${fixture.id} ${mode} defines ${fixtureSources[id].tag}`
        );
        declarations.push(declaration);
    }

    const instance = document.createElement(fixture.tag);
    for (const [name, value] of Object.entries(fixture.attributes)) {
        instance.setAttribute(name, value);
    }
    instance.innerHTML = fixture.payload;
    document.body.appendChild(instance);
    await runtime.whenRenderSettled(instance);
    return { document, runtime, declarations, instance };
}

function materialDeclaration(
    document: Document,
    fixture: MaterialFixtureDefinition,
    id: MaterialId,
    mode: MaterialMode
): HTMLElement {
    const sourceDefinition = fixtureSources[id];
    const source = mode === 'legacy'
        ? fixture.legacySource
        : (id === fixture.id ? fixture.cemMlSource : sourceDefinition.cemMlSource);
    const parsed = parseFixtureSource(document, source);
    const sourceTag = mode === 'legacy' ? 'custom-element' : 'cem-element';
    const sourceDeclaration = Array.from(parsed.querySelectorAll(sourceTag)).find(
        (candidate) => candidate.getAttribute('tag') === sourceDefinition.tag
    );
    assert(sourceDeclaration, `${fixture.id} ${mode} source contains ${sourceDefinition.tag}`);

    const declaration = document.createElement('div');
    for (const attribute of Array.from(sourceDeclaration.attributes)) {
        declaration.setAttribute(attribute.name, attribute.value);
    }
    for (const child of Array.from(sourceDeclaration.childNodes)) {
        declaration.appendChild(child.cloneNode(true));
    }
    if (mode === 'legacy') {
        annotateLegacyTemplates(declaration);
    }

    const src = declaration.getAttribute('src');
    if (src?.startsWith('#')) {
        const localTemplate = parsed.querySelector(src);
        assert(localTemplate, `${fixture.id} ${mode} resolves local template ${src}`);
        const support = localTemplate.cloneNode(true) as HTMLTemplateElement;
        if (mode === 'legacy') support.setAttribute('lang', 'custom-element-v0');
        document.body.appendChild(support);
    }
    return declaration;
}

function parseFixtureSource(document: Document, source: string): DocumentFragment {
    const container = document.createElement('template');
    container.innerHTML = source;
    return container.content;
}

function annotateLegacyTemplates(root: ParentNode): void {
    root.querySelectorAll('template').forEach((template) => {
        template.removeAttribute('type');
        template.setAttribute('lang', 'custom-element-v0');
    });
}

function adaptLegacySource(source: string, document: Document): string {
    const parsed = parseFixtureSource(document, source);
    annotateLegacyTemplates(parsed);
    const container = document.createElement('div');
    container.appendChild(parsed);
    return container.innerHTML;
}

function legacySourceForPath(path: string): string {
    const id = path.replace(/^\.\//, '').replace(/\.html$/, '') as MaterialId;
    const source = fixtureSources[id]?.legacySource;
    assert(source, `material legacy adapter resolves ${path}`);
    return source;
}

function resolveFixtureModuleUrl(specifier: string): string {
    const resolved: Record<string, string> = {
        '@epa-wg/material': 'https://cdn.example.test/@epa-wg/material',
        '@epa-wg/custom-element/demo/wc-square.svg':
            'https://cdn.example.test/@epa-wg/custom-element/demo/wc-square.svg',
    };
    return resolved[specifier] ?? specifier;
}

async function waitForElement(root: ParentNode, selector: string, timeout = 3000): Promise<Element> {
    const start = Date.now();
    for (;;) {
        const found = root.querySelector(selector);
        if (found) return found;
        if (Date.now() - start > timeout) {
            throw new Error(`timed out waiting for ${selector}`);
        }
        await new Promise((resolve) => setTimeout(resolve, 16));
    }
}

async function waitForCondition(predicate: () => boolean, message: string, timeout = 3000): Promise<void> {
    const start = Date.now();
    for (;;) {
        if (predicate()) return;
        if (Date.now() - start > timeout) throw new Error(`timed out waiting for ${message}`);
        await new Promise((resolve) => setTimeout(resolve, 16));
    }
}

function requiredElement(root: ParentNode, selector: string): Element {
    const found = root.querySelector(selector);
    assert(found, `expected ${selector}`);
    return found;
}

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) throw new Error(message);
}

function assertEqual(actual: unknown, expected: unknown, message: string): void {
    if (actual !== expected) {
        throw new Error(`${message}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    }
}
