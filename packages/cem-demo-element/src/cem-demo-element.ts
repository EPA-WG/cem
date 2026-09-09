import {
    highlightCemSource,
    renderCemMlSource,
    type CemMlDiagnostic,
    type CemMlHtmlRenderResult,
} from './cem-ml-runtime.js';
import { highlightedSource } from './source-highlight.js';

const STYLE_MARKER = 'data-cem-demo-element-styles';
const REGION_NAMES = ['legend', 'description', 'text', 'demo', 'status'] as const;
type RegionName = typeof REGION_NAMES[number];

export type CemDemoState = 'idle' | 'loading' | 'ready' | 'error';
export type CemDemoSource = string | Node | null | undefined;

export interface CemDemoRenderDetail {
    type: string;
    sourceUrl?: string;
    result?: CemMlHtmlRenderResult;
}

export class CemDemoElement extends HTMLElement {
    static readonly version = '0.1.1';
    static readonly observedAttributes = [
        'source',
        'type',
        'src',
        'demo',
        'text',
        'legend',
        'description',
    ];

    #initialized = false;
    #sourceValue: string | undefined;
    #sourceUrl: string | undefined;
    #detectedType: string | undefined;
    #lastResult: CemMlHtmlRenderResult | undefined;
    #regions = new Map<RegionName, HTMLElement>();
    #abortController: AbortController | undefined;
    #generation = 0;
    #updateComplete: Promise<void> = Promise.resolve();

    get source(): string | undefined {
        return this.#sourceValue;
    }

    set source(value: CemDemoSource) {
        this.#sourceValue = sourceText(value);
        this.#sourceUrl = undefined;
        this.#detectedType = undefined;
        if (this.#initialized) this.#queueRender();
    }

    get src(): string | null {
        return this.getAttribute('src');
    }

    set src(value: string | null) {
        if (value === null) this.removeAttribute('src');
        else this.setAttribute('src', value);
    }

    get type(): string {
        return this.getAttribute('type') ?? 'html';
    }

    set type(value: string | null) {
        if (value === null) this.removeAttribute('type');
        else this.setAttribute('type', value);
    }

    get legend(): string | null {
        return this.getAttribute('legend');
    }

    set legend(value: string | null) {
        if (value === null) this.removeAttribute('legend');
        else this.setAttribute('legend', value);
    }

    get description(): string | null {
        return this.getAttribute('description');
    }

    set description(value: string | null) {
        if (value === null) this.removeAttribute('description');
        else this.setAttribute('description', value);
    }

    get state(): CemDemoState {
        return (this.dataset.state as CemDemoState | undefined) ?? 'idle';
    }

    get updateComplete(): Promise<void> {
        return this.#updateComplete;
    }

    get lastResult(): CemMlHtmlRenderResult | undefined {
        return this.#lastResult;
    }

    connectedCallback(): void {
        installStyles(this.ownerDocument);
        if (!this.#initialized) this.#initialize();
        const src = this.src;
        this.#updateComplete = src ? this.#loadSource(src) : this.#render(++this.#generation);
    }

    disconnectedCallback(): void {
        this.#abortController?.abort();
    }

    attributeChangedCallback(name: string, oldValue: string | null, newValue: string | null): void {
        if (!this.#initialized || oldValue === newValue) return;
        if (name === 'src') {
            this.#updateComplete = newValue
                ? this.#loadSource(newValue)
                : this.#render(++this.#generation);
            return;
        }
        if (name === 'source') {
            this.#sourceValue = newValue ?? '';
            this.#sourceUrl = undefined;
        }
        if (name === 'type') this.#detectedType = undefined;
        this.#queueRender();
    }

    /** Re-render source presentation and any CEM-ML live output. */
    render(): Promise<void> {
        this.#queueRender();
        return this.#updateComplete;
    }

    #initialize(): void {
        const authoredHtml = this.innerHTML;
        const sourceControl = this.querySelector('[slot="source"]')
            ?? this.querySelector('template');
        if (this.#sourceValue === undefined) {
            this.#sourceValue = this.hasAttribute('source')
                ? this.getAttribute('source') ?? ''
                : sourceControl
                    ? sourceText(sourceControl)
                    : authoredHtml;
        }

        if (sourceControl) this.#initializeTemplateLayout(sourceControl);
        else this.#initializeBodyLayout();
        this.#initialized = true;
    }

    #initializeTemplateLayout(sourceControl: Element): void {
        for (const name of REGION_NAMES) this.#regions.set(name, this.#ensureRegion(name, sourceControl));
        this.#configureStatusRegion();
        const demo = this.#requiredRegion('demo');
        if (sourceControl instanceof HTMLTemplateElement) {
            demo.replaceChildren(sourceControl.content.cloneNode(true));
        }
    }

    #initializeBodyLayout(): void {
        const authoredNodes = Array.from(this.childNodes);
        this.replaceChildren();
        for (const name of REGION_NAMES) {
            const region = this.ownerDocument.createElement('div');
            region.slot = name;
            region.dataset.cemDemoRegion = name;
            this.#regions.set(name, region);
            this.append(region);
        }
        this.#configureStatusRegion();
        this.#requiredRegion('demo').append(...authoredNodes);
    }

    #ensureRegion(name: RegionName, sourceControl: Element): HTMLElement {
        const existing = directSlottedChild(this, name);
        if (existing) return existing;

        const region = this.ownerDocument.createElement('div');
        region.slot = name;
        region.dataset.cemDemoRegion = name;
        if (name === 'legend') this.prepend(region);
        else if (name === 'description') this.#requiredRegion('legend').after(region);
        else this.insertBefore(region, sourceControl);
        return region;
    }

    #queueRender(): void {
        const generation = ++this.#generation;
        this.#updateComplete = Promise.resolve().then(() => this.#render(generation));
    }

    async #loadSource(url: string): Promise<void> {
        const generation = ++this.#generation;
        this.#abortController?.abort();
        const controller = new AbortController();
        this.#abortController = controller;
        this.#setState('loading');
        try {
            const response = await fetch(url, { signal: controller.signal });
            if (!response.ok) throw new Error(`Could not load ${url}: HTTP ${response.status}`);
            const source = await response.text();
            if (generation !== this.#generation) return;
            this.#sourceValue = source;
            this.#sourceUrl = response.url || new URL(url, this.ownerDocument.baseURI).href;
            this.#detectedType = detectType(
                response.headers.get('content-type'),
                this.#sourceUrl
            );
            await this.#render(generation);
        } catch (error) {
            if (controller.signal.aborted || generation !== this.#generation) return;
            this.#showError(error);
        }
    }

    async #render(generation: number): Promise<void> {
        this.#renderHeading();
        const source = this.#sourceValue ?? '';
        const type = this.#effectiveType();
        this.#setState('loading');
        await this.#renderSource(source, type);
        if (generation !== this.#generation) return;

        if (type !== 'cem-ml') {
            this.#lastResult = undefined;
            if (this.src !== null) {
                const demo = this.#requiredRegion('demo');
                if (type === 'html') {
                    demo.replaceChildren(trustedHtmlFragment(
                        this.ownerDocument,
                        source,
                        this.#sourceUrl
                    ));
                } else {
                    demo.replaceChildren();
                }
            }
            this.#renderDiagnostics([]);
            this.#setState('ready');
            this.dispatchEvent(new CustomEvent<CemDemoRenderDetail>('cem-demo-render', {
                detail: { type, sourceUrl: this.#sourceUrl },
            }));
            return;
        }

        this.#requiredRegion('demo').replaceChildren();
        this.#setState('loading');
        try {
            const result = await renderCemMlSource(source, this.#sourceUrl);
            if (generation !== this.#generation) return;
            this.#lastResult = result;
            this.#renderDiagnostics(result.diagnostics);
            if (result.status !== 'rendered') {
                this.#setState('error');
                this.dispatchEvent(new CustomEvent<CemDemoRenderDetail>('cem-demo-error', {
                    detail: { type, sourceUrl: this.#sourceUrl, result },
                }));
                return;
            }
            this.#requiredRegion('demo').innerHTML = result.html;
            this.#setState('ready');
            this.dispatchEvent(new CustomEvent<CemDemoRenderDetail>('cem-demo-render', {
                detail: { type, sourceUrl: this.#sourceUrl, result },
            }));
        } catch (error) {
            if (generation !== this.#generation) return;
            this.#showError(error);
        }
    }

    #renderHeading(): void {
        const legend = this.#requiredRegion('legend');
        const description = this.#requiredRegion('description');
        const legendValue = this.getAttribute('legend');
        const descriptionValue = this.getAttribute('description');
        if (legendValue !== null) legend.innerHTML = `<h3>${legendValue}</h3>`;
        else if (legend.dataset.cemDemoRegion) legend.replaceChildren();
        if (descriptionValue !== null) description.innerHTML = `<div>${descriptionValue}</div>`;
        else if (description.dataset.cemDemoRegion) description.replaceChildren();
    }

    async #renderSource(source: string, type: string): Promise<void> {
        const code = this.ownerDocument.createElement('code');
        code.className = `cem-source-code language-${type}`;
        code.dataset.language = type;
        if (type === 'html' || type === 'cem-ml') {
            try {
                const highlighted = await highlightCemSource(
                    source,
                    type === 'html' ? 'text/html' : 'application/cem',
                    this.#sourceUrl
                );
                code.innerHTML = highlighted.html;
            } catch {
                code.innerHTML = highlightedSource(source, type);
            }
        } else {
            code.innerHTML = highlightedSource(source, type);
        }
        const pre = this.ownerDocument.createElement('pre');
        pre.append(code);
        this.#requiredRegion('text').replaceChildren(pre);
    }

    #renderDiagnostics(diagnostics: readonly CemMlDiagnostic[]): void {
        const status = this.#requiredRegion('status');
        status.replaceChildren();
        if (diagnostics.length === 0) return;
        const list = this.ownerDocument.createElement('ul');
        for (const diagnostic of diagnostics) {
            const item = this.ownerDocument.createElement('li');
            item.dataset.severity = diagnostic.severity;
            item.textContent = `${diagnostic.code}: ${diagnostic.message}`;
            list.append(item);
        }
        status.append(list);
    }

    #showError(error: unknown): void {
        this.#lastResult = undefined;
        const diagnostic: CemMlDiagnostic = {
            code: 'cem.demo.render_failed',
            severity: 'error',
            message: error instanceof Error ? error.message : String(error),
        };
        this.#renderDiagnostics([diagnostic]);
        this.#setState('error');
        this.dispatchEvent(new CustomEvent('cem-demo-error', { detail: { error } }));
    }

    #effectiveType(): string {
        const requested = this.type.toLowerCase();
        if (requested === 'auto') return this.#detectedType ?? 'text';
        return normalizeType(requested);
    }

    #setState(state: CemDemoState): void {
        this.dataset.state = state;
        this.toggleAttribute('aria-busy', state === 'loading');
    }

    #requiredRegion(name: RegionName): HTMLElement {
        const region = this.#regions.get(name);
        if (!region) throw new Error(`cem-demo-element ${name} region is not initialized`);
        return region;
    }

    #configureStatusRegion(): void {
        const status = this.#requiredRegion('status');
        status.setAttribute('role', 'status');
        status.setAttribute('aria-live', 'polite');
    }
}

export function defineCemDemoElement(registry: CustomElementRegistry = customElements): void {
    if (!registry.get('cem-demo-element')) registry.define('cem-demo-element', CemDemoElement);
}

function sourceText(value: CemDemoSource): string | undefined {
    if (value === null || value === undefined) return undefined;
    if (typeof value === 'string') return value;
    if (value instanceof HTMLTemplateElement) return value.innerHTML;
    if (value instanceof Element) return value.innerHTML;
    return value.nodeValue ?? value.textContent ?? '';
}

function directSlottedChild(parent: Element, slot: RegionName): HTMLElement | undefined {
    return Array.from(parent.children).find(
        (child): child is HTMLElement => child instanceof HTMLElement && child.slot === slot
    );
}

function detectType(contentType: string | null, url: string): string {
    const mediaType = contentType?.split(';', 1)[0]?.trim().toLowerCase() ?? '';
    if (mediaType.includes('cem')) return 'cem-ml';
    if (mediaType.includes('html') || mediaType.includes('xml') || mediaType.includes('svg')) return 'html';
    if (mediaType.includes('json') || mediaType.includes('javascript') || mediaType.includes('typescript')) return 'js';
    if (mediaType.includes('css') || mediaType.includes('scss') || mediaType.includes('less')) return 'css';

    const extension = new URL(url, document.baseURI).pathname.split('.').pop()?.toLowerCase();
    if (extension === 'cem') return 'cem-ml';
    if (extension === 'html' || extension === 'htm' || extension === 'xml' || extension === 'svg') return 'html';
    if (extension === 'json' || extension === 'js' || extension === 'mjs' || extension === 'cjs' || extension === 'ts') return 'js';
    if (extension === 'css' || extension === 'scss' || extension === 'less') return 'css';
    return 'text';
}

function normalizeType(type: string): string {
    if (type === 'cem' || type === 'cemml' || type === 'application/cem') return 'cem-ml';
    if (type === 'javascript' || type === 'typescript' || type === 'json') return 'js';
    if (type === 'markup' || type === 'xhtml' || type === 'svg' || type === 'xml') return 'html';
    return type;
}

const HTML_URL_ATTRIBUTES: Readonly<Record<string, readonly string[]>> = {
    a: ['href'],
    area: ['href'],
    audio: ['src'],
    base: ['href'],
    blockquote: ['cite'],
    button: ['formaction'],
    del: ['cite'],
    embed: ['src'],
    form: ['action'],
    iframe: ['src'],
    img: ['src'],
    input: ['formaction', 'src'],
    ins: ['cite'],
    link: ['href'],
    object: ['data'],
    q: ['cite'],
    script: ['src'],
    source: ['src'],
    track: ['src'],
    video: ['poster', 'src'],
};

function trustedHtmlFragment(
    document: Document,
    source: string,
    sourceUrl: string | undefined
): DocumentFragment {
    const template = document.createElement('template');
    template.innerHTML = source;
    if (!sourceUrl) return template.content;

    for (const element of template.content.querySelectorAll<HTMLElement>('*')) {
        for (const attribute of HTML_URL_ATTRIBUTES[element.localName] ?? []) {
            const value = element.getAttribute(attribute);
            if (value === null) continue;
            try {
                element.setAttribute(attribute, new URL(value, sourceUrl).href);
            } catch {
                // Preserve values that are not URL references for their element.
            }
        }
    }
    return template.content;
}

function installStyles(document: Document): void {
    if (document.head.querySelector(`style[${STYLE_MARKER}]`)) return;
    const style = document.createElement('style');
    style.setAttribute(STYLE_MARKER, '');
    style.textContent = `
cem-demo-element {
    --cem-demo-border: var(--cem-palette-creativity-x, #7553a6);
    --cem-demo-heading: var(--cem-palette-conservative, #ece9f1);
    --cem-demo-code: var(--cem-palette-comfort, #f7f7f8);
    --cem-color-syntax-punctuation: color-mix(
        in srgb,
        var(--cem-palette-conservative-x, #1a1c18) 68%,
        var(--cem-demo-code)
    );
    --cem-color-syntax-name: var(--cem-action-primary-active-background, #002f65);
    --cem-color-syntax-attribute: var(--cem-action-destructive-pending-background, #502400);
    --cem-color-syntax-string: var(--cem-action-contextual-pending-background, #6a1b9a);
    --cem-color-syntax-number: var(--cem-palette-creativity-x, #6a1b9a);
    --cem-color-syntax-keyword: var(--cem-action-primary-pending-background, #002f65);
    --cem-color-syntax-comment: color-mix(
        in srgb,
        var(--cem-palette-calm-x, #006a6a) 90%,
        var(--cem-demo-code)
    );
    --cem-color-syntax-text: var(--cem-palette-comfort-text, #202124);
    --cem-color-syntax-raw: var(--cem-color-syntax-punctuation);
    --cem-color-diagnostic-error: var(--cem-action-destructive-hover-background, #b42318);
    display: flex;
    min-width: 0;
    flex-direction: column;
    border: 1px dashed var(--cem-demo-border);
    border-radius: 1rem;
    overflow: hidden;
}
cem-demo-element > [slot="legend"],
cem-demo-element > [slot="description"] {
    margin: 0;
    background: var(--cem-demo-heading);
}
cem-demo-element > [slot="legend"] > h3 {
    margin: 0;
    padding: 0.75rem 1rem 0.35rem;
}
cem-demo-element > [slot="description"] {
    padding: 0.15rem 1rem 0.75rem;
}
cem-demo-element > [slot="legend"]:empty,
cem-demo-element > [slot="description"]:empty,
cem-demo-element > [slot="status"]:empty {
    display: none;
}
cem-demo-element > :not([slot]) {
    padding-inline: 1rem;
}
cem-demo-element > [slot="text"] {
    min-width: 0;
    background: var(--cem-demo-code);
}
cem-demo-element > [slot="text"] pre {
    box-sizing: border-box;
    max-width: 100%;
    margin: 0;
    padding: 1rem;
    overflow: auto;
    tab-size: 4;
    white-space: pre;
}
cem-demo-element > [slot="text"] code {
    color: var(--cem-color-syntax-text);
    font: 0.875rem/1.45 ui-monospace, SFMono-Regular, Consolas, monospace;
}
cem-demo-element > [slot="text"] code.cem-source-code {
    color: var(--cem-color-syntax-raw);
}
cem-demo-element code.cem-source-code > b {
    color: var(--cem-color-syntax-name);
    font-weight: 700;
}
cem-demo-element code.cem-source-code > var {
    color: var(--cem-color-syntax-attribute);
    font-weight: 700;
}
cem-demo-element code.cem-source-code > strong {
    color: var(--cem-color-syntax-keyword);
    font-weight: 600;
}
cem-demo-element code.cem-source-code > i {
    color: var(--cem-color-syntax-string);
}
cem-demo-element code.cem-source-code > u {
    color: var(--cem-color-syntax-number);
}
cem-demo-element code.cem-source-code > small {
    color: var(--cem-color-syntax-comment);
    font-size: inherit;
    font-style: italic;
}
cem-demo-element code.cem-source-code > samp {
    color: var(--cem-color-syntax-text);
    font: inherit;
}
cem-demo-element code.cem-source-code > mark {
    background: transparent;
    color: var(--cem-color-diagnostic-error);
    font-weight: 600;
}
cem-demo-element > [slot="demo"] {
    min-width: 0;
    padding: 1rem;
}
cem-demo-element > [slot="status"] {
    padding: 0 1rem 0.75rem;
    color: var(--cem-color-diagnostic-error);
}
@media (prefers-color-scheme: dark) {
    cem-demo-element {
        --cem-demo-border: var(--cem-palette-creativity-x, #bfa1ea);
        --cem-demo-heading: var(--cem-palette-conservative, #302a38);
        --cem-demo-code: var(--cem-palette-comfort, #1d1d20);
        --cem-color-syntax-punctuation: color-mix(
            in srgb,
            var(--cem-palette-conservative-x, #f1f1eb) 60%,
            var(--cem-demo-code)
        );
        --cem-color-syntax-name: var(--cem-action-primary-active-background, #d7e3ff);
        --cem-color-syntax-attribute: var(--cem-action-destructive-pending-background, #f0f070);
        --cem-color-syntax-string: var(--cem-action-contextual-pending-background, #e1bee7);
        --cem-color-syntax-number: var(--cem-palette-creativity-x, #e1bee7);
        --cem-color-syntax-keyword: var(--cem-action-primary-pending-background, #d7e3ff);
        --cem-color-syntax-comment: color-mix(
            in srgb,
            var(--cem-palette-calm-x, #00fbfb) 55%,
            var(--cem-demo-code)
        );
        --cem-color-syntax-text: var(--cem-palette-comfort-text, #e8eaed);
        --cem-color-syntax-raw: var(--cem-color-syntax-punctuation);
        --cem-color-diagnostic-error: var(--cem-action-destructive-hover-background, #ffb4ab);
    }
}
`;
    document.head.append(style);
}
