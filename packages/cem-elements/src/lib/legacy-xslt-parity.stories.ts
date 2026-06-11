import type { Meta, StoryObj } from '@storybook/web-components-vite';
import { CemElementRuntime, type CemElementRuntimeOptions } from './cem-elements.js';

/**
 * Legacy HTML+XSLT ⇆ CEM-ML parity stories.
 *
 * Each story registers a **legacy** `<custom-element>` template authored as declarative HTML+XSLT
 * (bare `for-each`/`if`/`choose`/`when`, `{$x}` AVT, XPath functions) and its **hand-written CEM-ML
 * twin**, instantiates both, and asserts they render **identical** light DOM. This proves the
 * DOM→CEM-ML converter (`legacy-xslt/convert.ts`) + the runtime `legacy-xslt` mode lower legacy
 * markup onto the same cem_ql WASM engine as migrated templates — no browser XSLT engine involved.
 */

const meta: Meta = {
    title: 'CEM Elements/Legacy XSLT Parity',
    tags: ['test'],
};

export default meta;

type Story = StoryObj;

export const LegacyIconLinkXsltParity: Story = {
    render: () => {
        const root = section('legacy icon-link parity');
        // Legacy HTML+XSLT (verbatim material `icon-link` shape: choose/when + contains() + AVT + slot).
        defineLegacy(
            makeRuntime('legacy-iconlink'),
            'legacy-iconlink',
            '<a href="{$href}">' +
                '<choose>' +
                "<when test=\"contains($icon,'/')\"><img class=\"icon\" src=\"{$icon}\"/></when>" +
                '<when test="$icon"><span class="icon material-icons">{$icon}</span></when>' +
                '</choose>' +
                '<slot></slot>' +
                '</a>'
        );
        // Hand-written CEM-ML twin.
        defineCemMl(
            makeRuntime('cem-iconlink'),
            'cem-iconlink',
            '{a @href="{$href}" |' +
                ' {cem:choose |' +
                ' {cem:when @test=\'str:contains(icon, "/")\' | {img @class="icon" @src="{$icon}"}}' +
                ' {cem:when @test="icon" | {span @class="icon material-icons" | {$icon}}}}' +
                ' {slot}}'
        );
        root.append(
            instance('legacy-iconlink', { href: '#go', icon: 'recycling' }, 'Recycle'),
            instance('cem-iconlink', { href: '#go', icon: 'recycling' }, 'Recycle')
        );
        return root;
    },
    play: async ({ canvasElement }) => {
        const legacy = requiredElement(canvasElement, 'legacy-iconlink') as HTMLElement;
        const twin = requiredElement(canvasElement, 'cem-iconlink') as HTMLElement;

        // The legacy instance renders the expected structure through conversion → WASM.
        const link = (await waitForElement(legacy, 'a.icon, a')) as HTMLAnchorElement;
        assertEqual(link.getAttribute('href'), '#go', 'AVT href resolves');
        const icon = await waitForElement(legacy, 'span.material-icons');
        assertEqual(icon.textContent, 'recycling', 'cem:when ($icon truthy, not a path) renders the icon name');
        assert(legacy.textContent?.includes('Recycle'), 'the default slot projects the payload');

        // Legacy output equals its hand-written CEM-ML twin (ignoring cem-internal frame attrs).
        await waitForElement(twin, 'span.material-icons');
        assertEqual(renderedHtml(legacy), renderedHtml(twin), 'legacy HTML+XSLT renders identically to its CEM-ML twin');
    },
};

export const LegacyIfNotXsltParity: Story = {
    render: () => {
        const root = section('legacy if/not parity');
        defineLegacy(
            makeRuntime('legacy-field'),
            'legacy-field',
            '<attribute name="disabled"></attribute>' +
                '<label>{$label}' +
                '<if test="not($disabled)"><input type="text"/></if>' +
                '</label>'
        );
        defineCemMl(
            makeRuntime('cem-field'),
            'cem-field',
            '{attribute @name=disabled}' +
                '{label | {$label}{cem:if @test="not (disabled)" | {input @type="text"}}}'
        );
        root.append(
            instance('legacy-field', { label: 'Name' }, ''),
            instance('cem-field', { label: 'Name' }, '')
        );
        return root;
    },
    play: async ({ canvasElement }) => {
        const legacy = requiredElement(canvasElement, 'legacy-field') as HTMLElement;
        const twin = requiredElement(canvasElement, 'cem-field') as HTMLElement;
        await waitForElement(legacy, 'label');
        const input = await waitForElement(legacy, 'input');
        assertEqual(input.getAttribute('type'), 'text', 'cem:if with not() renders the input when not disabled');
        await waitForElement(twin, 'input');
        assertEqual(renderedHtml(legacy), renderedHtml(twin), 'legacy if/not renders identically to its CEM-ML twin');
    },
};

export const LegacyForEachXsltParity: Story = {
    render: () => {
        const root = section('legacy for-each parity');
        // Legacy: inline node-set variable + for-each with position() and the context item `.`.
        defineLegacy(
            makeRuntime('legacy-fruits'),
            'legacy-fruits',
            '<xsl:variable name="fruits">' +
                '<item>Apple</item><item>Banana</item><item>Cherry</item>' +
                '</xsl:variable>' +
                '<ul><for-each select="exsl:node-set($fruits)/*"><li>{position()}. {.}</li></for-each></ul>'
        );
        // Twin: the unrolled CEM-ML the converter produces for a static node-set.
        defineCemMl(
            makeRuntime('cem-fruits'),
            'cem-fruits',
            '{ul | {li | 1. Apple}{li | 2. Banana}{li | 3. Cherry}}'
        );
        root.append(instance('legacy-fruits', {}, ''), instance('cem-fruits', {}, ''));
        return root;
    },
    play: async ({ canvasElement }) => {
        const legacy = requiredElement(canvasElement, 'legacy-fruits') as HTMLElement;
        const twin = requiredElement(canvasElement, 'cem-fruits') as HTMLElement;
        await waitForElement(legacy, 'ul li');
        const items = Array.from(legacy.querySelectorAll('li')).map((li) => li.textContent?.trim());
        assertEqual(items.join('|'), '1. Apple|2. Banana|3. Cherry', 'for-each unrolls the node-set with position()');
        await waitForElement(twin, 'ul li');
        assertEqual(renderedHtml(legacy), renderedHtml(twin), 'legacy for-each renders identically to the unrolled CEM-ML twin');
    },
};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

let runtimeSequence = 0;

function makeRuntime(prefix: string, options: Omit<CemElementRuntimeOptions, 'declarationTag'> = {}): CemElementRuntime {
    runtimeSequence += 1;
    return new CemElementRuntime({ ...options, declarationTag: `legacy-decl-${prefix}-${runtimeSequence}` });
}

/** Register a legacy HTML+XSLT template (set as `innerHTML`, no `type` → `legacy-xslt` mode). */
function defineLegacy(runtime: CemElementRuntime, tag: string, legacyHtml: string): void {
    const declaration = document.createElement('div');
    declaration.setAttribute('tag', tag);
    const template = document.createElement('template');
    template.innerHTML = legacyHtml;
    declaration.appendChild(template);
    assert(runtime.registerDeclaration(declaration), `registered legacy <${tag}>`);
}

/** Register a canonical CEM-ML twin (`type="text/cem-ml"`). */
function defineCemMl(runtime: CemElementRuntime, tag: string, cemMl: string): void {
    const declaration = document.createElement('div');
    declaration.setAttribute('tag', tag);
    const template = document.createElement('template');
    template.setAttribute('type', 'text/cem-ml');
    template.textContent = cemMl;
    declaration.appendChild(template);
    assert(runtime.registerDeclaration(declaration), `registered cem-ml <${tag}>`);
}

function instance(tag: string, attributes: Record<string, string>, payload: string): HTMLElement {
    const element = document.createElement(tag);
    for (const [name, value] of Object.entries(attributes)) {
        element.setAttribute(name, value);
    }
    if (payload) {
        element.textContent = payload;
    }
    return element;
}

function section(label: string): HTMLElement {
    const root = document.createElement('section');
    root.setAttribute('aria-label', label);
    return root;
}

/**
 * The rendered light DOM of an instance, normalized for legacy-vs-twin comparison: the inert
 * data-island and cem-internal `data-cem-*` frame attributes are removed, whitespace-only text
 * nodes (insignificant CEM-ML structural spacing) are dropped, and remaining text is collapsed.
 */
function renderedHtml(instance: HTMLElement): string {
    const clone = instance.cloneNode(true) as HTMLElement;
    clone.querySelectorAll('template').forEach((node) => node.remove());
    for (const element of [clone, ...Array.from(clone.querySelectorAll('*'))]) {
        for (const attribute of Array.from(element.attributes)) {
            if (attribute.name.startsWith('data-cem-')) {
                element.removeAttribute(attribute.name);
            }
        }
    }
    normalizeWhitespaceNodes(clone);
    return clone.innerHTML.trim();
}

/** Drop whitespace-only text nodes and collapse whitespace runs in the rest. */
function normalizeWhitespaceNodes(root: Node): void {
    const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
    const texts: Text[] = [];
    for (let node = walker.nextNode(); node; node = walker.nextNode()) {
        texts.push(node as Text);
    }
    for (const text of texts) {
        const value = text.textContent ?? '';
        if (value.trim() === '') {
            text.remove();
        } else {
            text.textContent = value.replace(/\s+/g, ' ').trim();
        }
    }
}

async function waitForElement(root: ParentNode, selector: string, timeout = 2000): Promise<Element> {
    const start = Date.now();
    for (;;) {
        const found = root.querySelector(selector);
        if (found) {
            return found;
        }
        if (Date.now() - start > timeout) {
            throw new Error(`timed out waiting for \`${selector}\``);
        }
        await new Promise((resolve) => setTimeout(resolve, 16));
    }
}

function requiredElement(root: ParentNode, selector: string): Element {
    const found = root.querySelector(selector);
    if (!found) {
        throw new Error(`expected element \`${selector}\``);
    }
    return found;
}

function assert(condition: unknown, message: string): asserts condition {
    if (!condition) {
        throw new Error(message);
    }
}

function assertEqual(actual: unknown, expected: unknown, label: string): void {
    if (actual !== expected) {
        throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
    }
}
