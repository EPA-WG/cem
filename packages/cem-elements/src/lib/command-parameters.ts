/** Desktop v1 command resources, consumed from the CEM-ML XML projection. */
// Same-origin frame runtimes can load independent module instances. The symbol marks
// only host-created parameter controls and does not survive HTML serialization.
const privateDialog = Symbol.for('cem.elements.privateCommandDialog');
type Marked = Element & { [privateDialog]?: true };
export function isCemParameterElement(node: Element): boolean {
    return (node as Marked)[privateDialog] === true;
}
export function isCemParameterEvent(event: Event): boolean {
    return event.composedPath().some(node => (node as Marked)[privateDialog] === true);
}

function required(element: Element, attribute: string): string {
    const value = element.getAttribute(attribute);
    if (!value) throw Error('cem.command.invalid-resource');
    return value;
}
function unique(elements: Element[]): Element;
function unique(elements: Element[], optional: true): Element | undefined;
function unique(elements: Element[], optional = false): Element | undefined {
    if (elements.length === 1) return elements[0];
    if (!elements.length && optional) return;
    throw Error('cem.command.ambiguous-resource');
}
function children(element: Element, tag: string): Element[] {
    return Array.from(element.children).filter(node => node.localName === tag);
}

interface Parameter {
    name: string;
    label: string;
    type: 'enum' | 'string';
    sensitive: boolean;
    required: boolean;
    minLength: number;
    choices: Map<string, string>;
}

export function readCommandResource(manifest: Element, id: string, profile: string) {
    if (manifest.localName !== 'manifest' || manifest.getAttribute('version') !== '1') throw Error('cem.command.manifest-version');
    const command = unique(children(manifest, 'command').filter(node => node.getAttribute('id') === id));
    const revision = required(command, 'revision');
    const presentation = unique(children(command, 'presentation'));
    const schema = unique(children(command, 'parameters'));
    const parameters = new Map<string, Parameter>();
    for (const node of children(schema, 'parameter')) {
        const name = required(node, 'name'), type = required(node, 'type');
        const minLength = Number(node.getAttribute('min-length') ?? 0);
        if (parameters.has(name) || !['enum', 'string'].includes(type) || !Number.isSafeInteger(minLength) || minLength < 0)
            throw Error('cem.command.invalid-parameter');
        const choices = new Map<string, string>();
        for (const choice of children(node, 'choice')) {
            const value = required(choice, 'value');
            if (choices.has(value)) throw Error('cem.command.duplicate-choice');
            choices.set(value, required(choice, 'label-key'));
        }
        if ((type === 'enum') !== !!choices.size) throw Error('cem.command.invalid-choices');
        parameters.set(name, { name, label: required(node, 'label-key'), type: type as Parameter['type'],
            minLength, required: node.getAttribute('required') === 'true', choices,
            sensitive: node.getAttribute('sensitive') === 'true' || node.getAttribute('delivery') === 'endpoint-only' });
    }
    const groups = new Map<string, Map<string, string>>();
    for (const group of children(manifest, 'resources')) {
        const lang = required(group, 'lang'), messages = new Map<string, string>();
        if (groups.has(lang)) throw Error('cem.command.duplicate-locale');
        for (const msg of children(group, 'message')) {
            const key = required(msg, 'key');
            if (messages.has(key)) throw Error('cem.command.duplicate-message');
            messages.set(key, msg.textContent ?? '');
        }
        groups.set(lang, messages);
    }
    const defaultLocale = required(manifest, 'default-locale');
    if (!groups.has(defaultLocale)) throw Error('cem.command.default-locale');
    const shortcuts = unique(children(manifest, 'shortcut-profile').filter(node => node.getAttribute('id') === profile));
    const binding = unique(children(shortcuts, 'binding').filter(node => node.getAttribute('command-id') === id
        && node.getAttribute('command-revision') === revision));
    if (binding.getAttribute('match') !== 'key' || binding.getAttribute('target') !== 'focused-content'
        || binding.getAttribute('repeat') !== 'ignore') throw Error('cem.command.unsupported-binding');
    const modifiers = unique(children(binding, 'modifiers'));
    const key = required(binding, 'key').toLowerCase();
    const ctrl = modifiers.getAttribute('control') === 'true', shift = modifiers.getAttribute('shift') === 'true';
    const alt = modifiers.getAttribute('alt') === 'true', meta = modifiers.getAttribute('meta') === 'true';
    const shortcut = [ctrl && 'Control', alt && 'Alt', shift && 'Shift', meta && 'Meta', key.toUpperCase()].filter(Boolean).join('+');
    const template = unique(children(manifest, 'template').filter(node => node.getAttribute('resource-id') === id
        && node.getAttribute('command-revision') === revision && node.getAttribute('kind') === 'parameter-form'), true);
    return { id, revision, parameters, defaultLocale, template: template?.cloneNode(true) as Element | undefined,
        title: required(presentation, 'name-key'), menu: required(presentation, 'menu-name-key'),
        description: required(presentation, 'description-key'), shortcut,
        text(lang: string, name: string) { return groups.get(lang)?.get(name) ?? groups.get(defaultLocale)?.get(name) ?? name; },
        matches(event: KeyboardEvent) {
            return !event.defaultPrevented && !event.repeat && !event.isComposing && event.keyCode !== 229
                && !event.getModifierState('AltGraph') && event.key.toLowerCase() === key
                && event.ctrlKey === ctrl && event.shiftKey === shift && event.altKey === alt && event.metaKey === meta;
        },
    };
}

type Resource = ReturnType<typeof readCommandResource>;
const tags = new Set(['span', 'form', 'label', 'select', 'option', 'input', 'button']);
const attrs = new Set(['slot', 'id', 'name', 'type', 'value', 'aria-labelledby', 'text-key', 'submit-role',
    'required', 'disabled', 'selected', 'autocomplete', 'formnovalidate']);

/** Build only inertly described native controls. The XML is never injected as HTML. */
function copyControl(document: Document, source: Element): HTMLElement {
    if (!tags.has(source.localName)) throw Error('cem.command.unsupported-form');
    const node = document.createElement(source.localName);
    for (const attribute of source.attributes) {
        if (attrs.has(attribute.name)) node.setAttribute(attribute.name, attribute.value);
        else if (!(source.localName === 'form' && attribute.name === 'method' && attribute.value === 'dialog'))
            throw Error('cem.command.unsupported-form');
    }
    if (source.children.length) for (const child of source.children) node.append(copyControl(document, child));
    else node.textContent = source.textContent;
    return node;
}

export function openCommandParameters(root: HTMLElement, resource: Resource, locale: string,
    accept: (values: Readonly<Record<string, string>>) => void, cancel: () => void, seed?: readonly [string, string]) {
    const document = root.ownerDocument;
    const dialog = document.createElement('dialog');
    dialog.setAttribute('data-cem-command-parameters', resource.id);
    if (resource.template) {
        const source = unique(children(resource.template, 'cem-dialog'));
        if (source.getAttribute('mode') !== 'nonmodal') throw Error('cem.command.unsupported-form');
        for (const child of source.children) dialog.append(copyControl(document, child));
    } else {
        const form = document.createElement('form');
        dialog.append(form);
        for (const field of resource.parameters.values()) {
            const label = document.createElement('label'), text = document.createElement('span');
            text.setAttribute('text-key', field.label);
            const control = field.type === 'enum' ? document.createElement('select') : document.createElement('input');
            if (control.localName === 'select') {
                control.append(document.createElement('option'));
                for (const [value, labelKey] of field.choices) {
                    const option = document.createElement('option');
                    option.value = value; option.setAttribute('text-key', labelKey); control.append(option);
                }
            }
            control.name = field.name; label.append(text, control); form.append(label);
        }
        const button = document.createElement('button');
        button.type = 'submit'; button.setAttribute('submit-role', 'accept'); button.setAttribute('text-key', resource.title);
        form.append(button);
    }
    // IDREFs are document-wide in light DOM. Concurrent forms can use the same template.
    const ids = new Map<string, string>();
    for (const node of dialog.querySelectorAll('[id]')) {
        if (ids.has(node.id)) throw Error('cem.command.duplicate-form-id');
        const scoped = 'cem-form-' + crypto.randomUUID();
        ids.set(node.id, scoped); node.id = scoped;
    }
    for (const node of dialog.querySelectorAll('[aria-labelledby]')) {
        const names = required(node, 'aria-labelledby').trim().split(/\s+/);
        if (names.some(name => !ids.has(name))) throw Error('cem.command.invalid-form-label');
        node.setAttribute('aria-labelledby', names.map(name => ids.get(name)).join(' '));
    }
    const form = unique([...dialog.querySelectorAll('form')]) as HTMLFormElement;
    const controls = new Map<string, HTMLInputElement | HTMLSelectElement>();
    for (const element of form.elements) {
        if (element.localName === 'button' && element.getAttribute('type') === 'submit'
            && ['accept', 'cancel'].includes(element.getAttribute('submit-role') ?? '')) continue;
        const name = required(element, 'name'), field = resource.parameters.get(name);
        if (!field || controls.has(name) || element.localName !== (field.type === 'enum' ? 'select' : 'input')
            || (element.localName === 'input' && (element as HTMLInputElement).type !== 'text'))
            throw Error('cem.command.invalid-form-binding');
        const control = element as HTMLInputElement | HTMLSelectElement;
        control.required = field.required;
        if (control.localName === 'input') (control as HTMLInputElement).minLength = field.minLength;
        // Keep the binding in this collector. Native FormData must omit the draft
        // even if an unrelated collector enumerates every form in the document.
        control.removeAttribute('name');
        controls.set(name, control);
    }
    if (controls.size !== resource.parameters.size || !form.querySelector('button[submit-role="accept"]'))
        throw Error('cem.command.incomplete-form');

    function localize(lang: string) {
        dialog.lang = lang;
        dialog.setAttribute('aria-label', resource.text(lang, resource.title));
        for (const node of dialog.querySelectorAll('[text-key]')) node.textContent = resource.text(lang, required(node, 'text-key'));
    }
    form.addEventListener('submit', event => {
        event.preventDefault();
        if (!dialog.open || dialog.parentElement !== root || !root.isConnected || !dialog.contains(form)) return;
        if (event.submitter?.getAttribute('submit-role') === 'cancel') return cancel();
        const values: Record<string, string> = Object.create(null);
        for (const [name, control] of controls) {
            const field = resource.parameters.get(name);
            if (!field || control.form !== form || !form.contains(control)) return;
            const value = control.value;
            if (!value && !field.required) continue;
            if (control.disabled || value.length < field.minLength || (field.required && !value)
                || (field.type === 'enum' && !field.choices.has(value))) return;
            values[name] = value;
        }
        accept(Object.freeze(values));
    });
    dialog.addEventListener('close', cancel);
    dialog.addEventListener('keydown', event => {
        if (event.key === 'Escape' && !event.defaultPrevented && !event.isComposing) {
            event.preventDefault(); cancel();
        }
    });
    // Keep text, composition, keystrokes and clipboard events out of bubbling view bindings.
    for (const event of ['beforeinput','input','change','keydown','keyup','keypress','compositionstart','compositionupdate',
        'compositionend','paste','copy','cut','submit','formdata','click']) dialog.addEventListener(event, e => e.stopPropagation());
    const initial = seed && controls.get(seed[0]);
    if (seed && initial) initial.value = seed[1];
    for (const element of [dialog, ...dialog.querySelectorAll('*')]) {
        Object.defineProperty(element, privateDialog, { value: true });
    }
    localize(locale);
    root.append(dialog);
    try { dialog.show(); } catch (error) { dialog.remove(); throw error; }
    return { localize, fence(pending: boolean) {
        for (const button of form.querySelectorAll<HTMLButtonElement>('button[submit-role="accept"]')) {
            button.disabled = pending;
        }
    }, close() {
        dialog.removeEventListener('close', cancel);
        for (const control of controls.values()) control.value = '';
        dialog.close(); dialog.remove();
    } };
}
