import {
    CemElementRuntime,
    type CemProducedElementBehavior,
    type DataIslandSnapshot,
} from '@epa-wg/cem-elements';

export interface ComponentHarness {
    readonly root: HTMLElement;
    cleanup(): void;
    query<T extends Element = Element>(selector: string): T;
    render(markup: string): Promise<HTMLElement>;
}

export interface ComponentHarnessOptions {
    runtime?: CemElementRuntime;
}

export interface SubstrateComponentDeclaration {
    tag: string;
    cemMl: string;
    behavior?: CemProducedElementBehavior;
    behaviorIdentity?: string;
}

export interface CapturedFormFile {
    kind: 'file';
    lastModified: number;
    name: string;
    size: number;
    type: string;
}

export type CapturedFormValue = string | CapturedFormFile;

export interface FormSnapshot {
    entries: Array<[string, CapturedFormValue]>;
    valid: boolean;
}

export interface SubstrateComponentHarness extends ComponentHarness {
    readonly runtime: CemElementRuntime;
    register(declaration: SubstrateComponentDeclaration): Promise<HTMLElement>;
    resetForm(form: HTMLFormElement, instances?: readonly HTMLElement[]): Promise<FormSnapshot>;
    settle(instance: HTMLElement): Promise<void>;
    snapshot(instance: HTMLElement): DataIslandSnapshot;
}

export interface ComponentEventOptions<TDetail> {
    detail?: TDetail;
    requireBubbles?: boolean;
    requireComposed?: boolean;
    timeoutMs?: number;
}

export interface VisualSnapshot {
    html: string;
    rect: {
        height: number;
        width: number;
    };
    styles: Record<string, string>;
    tagName: string;
    text: string;
}

const ARIA_IDREF_ATTRIBUTES = [
    'aria-activedescendant',
    'aria-controls',
    'aria-describedby',
    'aria-details',
    'aria-errormessage',
    'aria-flowto',
    'aria-labelledby',
    'aria-owns',
    'for',
] as const;

const DEFAULT_VISUAL_STYLE_PROPERTIES = [
    'background-color',
    'border-bottom-color',
    'border-left-color',
    'border-right-color',
    'border-top-color',
    'color',
    'display',
    'font-size',
    'height',
    'line-height',
    'outline-color',
    'outline-style',
    'outline-width',
    'visibility',
    'width',
] as const;

const VOLATILE_RUNTIME_ATTRIBUTES = [
    'data-cem-data-revision',
    'data-cem-hydration',
    'data-cem-render-node-id',
    'data-cem-render-scope',
    'data-cem-source-fidelity',
    'data-cem-source-frame',
    'data-cem-template-artifact-id',
] as const;

interface RuntimeHarnessScope {
    readonly root: HTMLElement;
    readonly runtime: CemElementRuntime;
}

const runtimeHarnessScopes = new Set<RuntimeHarnessScope>();

export function createComponentHarness(options: ComponentHarnessOptions = {}): ComponentHarness {
    assertBrowserDom();

    const root = document.createElement('div');
    root.setAttribute('data-cem-component-harness', '');
    document.body.append(root);
    const runtimeScope = options.runtime ? { root, runtime: options.runtime } : undefined;
    if (runtimeScope) {
        runtimeHarnessScopes.add(runtimeScope);
    }

    return {
        root,
        cleanup() {
            if (runtimeScope) {
                runtimeHarnessScopes.delete(runtimeScope);
            }
            root.remove();
        },
        query<T extends Element = Element>(selector: string): T {
            const element = root.querySelector<T>(selector);

            if (!element) {
                throw new Error(`Expected harness fixture to contain ${selector}`);
            }

            return element;
        },
        async render(markup: string): Promise<HTMLElement> {
            root.innerHTML = markup;
            await nextRenderFrame();

            const element = root.firstElementChild;

            if (!(element instanceof HTMLElement)) {
                throw new Error('Expected harness markup to produce a root HTMLElement');
            }

            return element;
        },
    };
}

/**
 * Browser harness for real CEM-ML declarations rendered by the production
 * `<cem-element>` substrate. Registered produced tags remain document-global,
 * so fixture tags must be unique within a browser test document.
 */
export function createSubstrateComponentHarness(
    runtime = new CemElementRuntime(),
): SubstrateComponentHarness {
    const base = createComponentHarness();
    const registeredTags = new Set<string>();

    const settle = async (instance: HTMLElement): Promise<void> => {
        // Attribute observers and event-driven slice updates schedule work in a
        // microtask before replacing the runtime's current settlement promise.
        await nextRenderFrame();
        await runtime.whenRenderSettled(instance);
        await nextRenderFrame();
        await runtime.whenRenderSettled(instance);
    };

    return {
        ...base,
        runtime,
        async register(declaration) {
            if (declaration.behavior && !declaration.behaviorIdentity) {
                throw new Error(`Behavior-backed fixture ${declaration.tag} requires behaviorIdentity`);
            }

            const element = document.createElement('cem-element');
            element.setAttribute('tag', declaration.tag);
            const template = document.createElement('template');
            template.setAttribute('type', 'text/cem-ml');
            template.textContent = declaration.cemMl;
            element.append(template);

            const accepted = runtime.registerDeclaration(element, {
                behavior: declaration.behavior,
                behaviorIdentity: declaration.behaviorIdentity,
            });
            await runtime.whenDeclarationSettled(element);
            const hardDiagnostics = runtime
                .diagnosticsFor(element)
                .filter(({ severity }) => severity === 'error' || severity === 'fatal');

            if (!accepted || hardDiagnostics.length > 0 || !customElements.get(declaration.tag)) {
                const summary = runtime
                    .diagnosticsFor(element)
                    .map(({ code, message }) => `${code}: ${message}`)
                    .join('; ');
                throw new Error(`CEM substrate fixture ${declaration.tag} did not register${summary ? `: ${summary}` : ''}`);
            }

            registeredTags.add(declaration.tag);
            return element;
        },
        async render(markup) {
            const root = await base.render(markup);
            const instances = Array.from(registeredTags).flatMap((tag) => {
                const matches = root.matches(tag) ? [root] : [];
                return [...matches, ...Array.from(root.querySelectorAll<HTMLElement>(tag))];
            });
            await Promise.all(instances.map(settle));
            return root;
        },
        async resetForm(form, instances = []) {
            form.reset();
            await Promise.all(instances.map(settle));
            return captureFormSnapshot(form);
        },
        settle,
        snapshot(instance) {
            return runtime.snapshotInstance(instance);
        },
    };
}

export async function nextRenderFrame(): Promise<void> {
    if (runtimeHarnessScopes.size === 0) {
        await nextAnimationFrame();
        return;
    }

    let previousInstanceCount = -1;
    for (let pass = 0; pass < 8; pass += 1) {
        await nextAnimationFrame();
        const instanceCount = await settleRuntimeHarnessScopes();
        if (instanceCount === previousInstanceCount) {
            return;
        }
        previousInstanceCount = instanceCount;
    }

    throw new Error('CEM component harness did not reach a stable nested runtime instance set');
}

async function nextAnimationFrame(): Promise<void> {
    await Promise.resolve();

    if (typeof requestAnimationFrame === 'function') {
        await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    }

    await Promise.resolve();
}

async function settleRuntimeHarnessScopes(): Promise<number> {
    const instances = Array.from(runtimeHarnessScopes).flatMap(({ root, runtime }) => {
        const hosts = new Set(
            Array.from(root.querySelectorAll<HTMLTemplateElement>('template[data-cem-island="instance"]'))
                .map((island) => island.parentElement)
                .filter((instance): instance is HTMLElement => instance instanceof HTMLElement),
        );
        return Array.from(hosts).map((instance) => ({ instance, runtime }));
    });
    const settlements = instances.map(({ instance, runtime }) => runtime.whenRenderSettled(instance));
    await Promise.all(settlements);
    return instances.length;
}

export function assertLightDomRendered(host: HTMLElement): void {
    if (host.shadowRoot) {
        throw new Error(`${host.tagName.toLowerCase()} must render without shadow DOM`);
    }

    const visibleNodes = Array.from(host.childNodes).filter((node) => {
        if (node instanceof HTMLTemplateElement) {
            return false;
        }

        return node.textContent?.trim() || node instanceof Element;
    });

    if (visibleNodes.length === 0) {
        throw new Error(`${host.tagName.toLowerCase()} must expose light-DOM output`);
    }
}

export async function expectComponentEvent<TDetail>(
    target: EventTarget,
    eventName: string,
    action: () => Promise<void> | void,
    options: ComponentEventOptions<TDetail> = {},
): Promise<CustomEvent<TDetail>> {
    const { timeoutMs = 250, requireBubbles = true, requireComposed = true } = options;

    const event = await new Promise<CustomEvent<TDetail>>((resolve, reject) => {
        const timer = setTimeout(() => {
            target.removeEventListener(eventName, listener);
            reject(new Error(`Expected ${eventName} to be dispatched within ${timeoutMs}ms`));
        }, timeoutMs);

        const listener = (rawEvent: Event) => {
            clearTimeout(timer);
            target.removeEventListener(eventName, listener);

            if (!(rawEvent instanceof CustomEvent)) {
                reject(new Error(`Expected ${eventName} to be a CustomEvent`));
                return;
            }

            resolve(rawEvent as CustomEvent<TDetail>);
        };

        target.addEventListener(eventName, listener, { once: true });

        try {
            void Promise.resolve(action()).catch(reject);
        } catch (error) {
            reject(error);
        }
    });

    if (requireBubbles && !event.bubbles) {
        throw new Error(`${eventName} must bubble`);
    }

    if (requireComposed && !event.composed) {
        throw new Error(`${eventName} must be composed`);
    }

    assertJsonSerializable(event.detail, `${eventName}.detail`);

    if (options.detail !== undefined && stableStringify(event.detail) !== stableStringify(options.detail)) {
        throw new Error(
            `Expected ${eventName} detail ${stableStringify(options.detail)}, received ${stableStringify(event.detail)}`,
        );
    }

    return event;
}

export function accessibleName(element: Element): string {
    const labelledBy = splitIdRefs(element.getAttribute('aria-labelledby'));
    if (labelledBy.length > 0) {
        return labelledBy
            .map((id) => element.ownerDocument.getElementById(id)?.textContent?.trim() ?? '')
            .filter(Boolean)
            .join(' ')
            .trim();
    }

    const ariaLabel = element.getAttribute('aria-label')?.trim();
    if (ariaLabel) {
        return ariaLabel;
    }

    const id = element.getAttribute('id');
    if (id) {
        const label = element.ownerDocument.querySelector<HTMLLabelElement>(`label[for="${cssEscape(id)}"]`);
        const text = label?.textContent?.trim();
        if (text) {
            return text;
        }
    }

    const wrappingLabel = element.closest('label');
    const wrappingLabelText = wrappingLabel ? labelTextWithoutControl(wrappingLabel, element) : '';
    if (wrappingLabelText) {
        return wrappingLabelText;
    }

    return normalizeText(element.textContent ?? '');
}

export function assertAccessibleName(element: Element, expected?: string): string {
    const name = accessibleName(element);

    if (!name) {
        throw new Error(`${element.tagName.toLowerCase()} must resolve an accessible name`);
    }

    if (expected !== undefined && name !== expected) {
        throw new Error(`Expected accessible name "${expected}", received "${name}"`);
    }

    return name;
}

export function assertAriaReferenceIntegrity(root: ParentNode): void {
    const document = ownerDocumentFor(root);
    const brokenReferences: string[] = [];

    for (const element of elementsUnder(root)) {
        for (const attribute of ARIA_IDREF_ATTRIBUTES) {
            const value = element.getAttribute(attribute);

            for (const id of splitIdRefs(value)) {
                if (!document.getElementById(id)) {
                    brokenReferences.push(`${element.tagName.toLowerCase()}[${attribute}="${id}"]`);
                }
            }
        }
    }

    if (brokenReferences.length > 0) {
        throw new Error(`Broken ARIA/reference targets: ${brokenReferences.join(', ')}`);
    }
}

export async function assertFocusVisible(element: HTMLElement): Promise<void> {
    element.focus();
    await nextRenderFrame();

    if (element.ownerDocument.activeElement !== element) {
        throw new Error(`${element.tagName.toLowerCase()} must receive focus`);
    }

    const styles = getComputedStyle(element);
    const hasOutline = styles.outlineStyle !== 'none' && styles.outlineWidth !== '0px';
    const hasBoxShadow = styles.boxShadow !== 'none';

    if (!hasOutline && !hasBoxShadow) {
        throw new Error(`${element.tagName.toLowerCase()} must expose a visible focus indicator`);
    }
}

export function captureVisualSnapshot(
    element: HTMLElement,
    styleProperties: readonly string[] = DEFAULT_VISUAL_STYLE_PROPERTIES,
): VisualSnapshot {
    const rect = element.getBoundingClientRect();
    const computed = getComputedStyle(element);
    const styles: Record<string, string> = {};

    for (const property of styleProperties) {
        styles[property] = computed.getPropertyValue(property);
    }

    return {
        html: normalizeVisualHtml(element),
        rect: {
            height: round(rect.height),
            width: round(rect.width),
        },
        styles,
        tagName: element.tagName.toLowerCase(),
        text: normalizeText(element.textContent ?? ''),
    };
}

/** Capture the browser-owned successful-controls and constraint-validity view. */
export function captureFormSnapshot(form: HTMLFormElement): FormSnapshot {
    return {
        entries: Array.from(new FormData(form).entries(), ([name, value]) => [
            name,
            typeof value === 'string'
                ? value
                : {
                    kind: 'file' as const,
                    lastModified: value.lastModified,
                    name: value.name,
                    size: value.size,
                    type: value.type,
                },
        ]),
        valid: form.checkValidity(),
    };
}

function assertBrowserDom(): void {
    if (typeof document === 'undefined') {
        throw new Error('The CEM component harness requires a browser DOM');
    }
}

function assertJsonSerializable(value: unknown, label: string): void {
    try {
        JSON.stringify(value);
    } catch (error) {
        throw new Error(`${label} must be JSON-serializable: ${String(error)}`, { cause: error });
    }
}

function cssEscape(value: string): string {
    if (typeof CSS !== 'undefined' && typeof CSS.escape === 'function') {
        return CSS.escape(value);
    }

    return value.replaceAll('"', '\\"');
}

function elementsUnder(root: ParentNode): Element[] {
    const elements = Array.from(root.querySelectorAll('*'));

    if (root instanceof Element) {
        elements.unshift(root);
    }

    return elements;
}

function normalizeVisualHtml(element: HTMLElement): string {
    const clone = element.cloneNode(true) as HTMLElement;

    for (const current of elementsUnder(clone)) {
        for (const attribute of VOLATILE_RUNTIME_ATTRIBUTES) {
            current.removeAttribute(attribute);
        }
    }
    for (const template of clone.querySelectorAll('template[data-cem-island]')) {
        template.remove();
    }

    return clone.outerHTML.replace(/\s+/g, ' ').replace(/> </g, '><').trim();
}

function labelTextWithoutControl(label: HTMLLabelElement, control: Element): string {
    const clone = label.cloneNode(true) as HTMLLabelElement;
    const controls = Array.from(label.querySelectorAll('button, input, meter, output, progress, select, textarea'));
    const controlIndex = controls.indexOf(control);

    if (controlIndex >= 0) {
        clone.querySelectorAll('button, input, meter, output, progress, select, textarea')[controlIndex]?.remove();
    }

    return normalizeText(clone.textContent ?? '');
}

function normalizeText(text: string): string {
    return text.replace(/\s+/g, ' ').trim();
}

function ownerDocumentFor(root: ParentNode): Document {
    if (root instanceof Document) {
        return root;
    }

    const document = (root as Node).ownerDocument;

    if (!document) {
        throw new Error('Expected reference root to be attached to a document');
    }

    return document;
}

function round(value: number): number {
    return Math.round(value * 100) / 100;
}

function splitIdRefs(value: string | null): string[] {
    return value?.trim().split(/\s+/).filter(Boolean) ?? [];
}

function stableStringify(value: unknown): string {
    return JSON.stringify(value, (_key, child) => {
        if (!child || typeof child !== 'object' || Array.isArray(child)) {
            return child;
        }

        return Object.fromEntries(Object.entries(child).sort(([left], [right]) => left.localeCompare(right)));
    });
}
